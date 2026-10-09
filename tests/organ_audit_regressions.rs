use selmem::core::model::{
    AxiomLayer, Bearer, ClockGuard, IdentityAxiom, LossKind, StakeKind, TraceStatus,
};
use selmem::recall::reading::ReadingProfile;
use selmem::{
    Attribution, EncodeInput, EntityProfile, Mood, RecallBias, RecallWrite, SelectiveMemory,
};

fn memory() -> SelectiveMemory {
    let mut p = EntityProfile::new("Claire");
    p.encode_threshold = 0.01;
    let mut m = SelectiveMemory::new(p).detach_clock();
    m.clock.origin_real = 4_102_444_800;
    m.clock.jump = 0;
    m.clock.scale = 1;
    m
}
fn add(m: &mut SelectiveMemory, text: &str, valence: f32) -> String {
    let mut e = EncodeInput::new(text);
    e.valence = valence;
    e.schema = Some("observed-belonging".into());
    e.attribution = Attribution::External;
    e.self_relevance = 0.95;
    e.arousal = 0.8;
    m.live_with(e).trace_id.expect("kept")
}
#[test]
fn painting_is_not_pain_and_variants_still_match() {
    for s in [
        "Caroline said: Any more paintings coming up?",
        "Melanie said: I painted the lake.",
        "The campaign opened.",
        "The presentation starts.",
    ] {
        let (v, _, d, schema) = selmem::encode::affect::guess(s);
        assert_eq!(v, 0.0, "{s}");
        assert_eq!(d, 0.0, "{s}");
        assert!(schema.is_none(), "{s}");
    }
    for s in [
        "It was painful.",
        "I was humiliated.",
        "They betrayed me.",
        "They laughed at me.",
    ] {
        assert!(selmem::encode::affect::guess(s).0 < 0.0, "{s}");
    }
    assert!(selmem::encode::affect::guess("Thank you for staying faithful.").0 > 0.0);
}
#[test]
fn old_observations_continue_maintenance_without_rewriting_quotes() {
    let mut m = memory();
    let id = add(
        &mut m,
        "Caroline said: The assignment ended painfully.",
        -0.6,
    );
    m.store.traces.get_mut(&id).unwrap().permanence = 0.0;
    m.store.traces.get_mut(&id).unwrap().anchor = 0.0;
    m.store.traces.get_mut(&id).unwrap().disgust = 0.4;
    let gist = m.store.traces[&id].gist.clone();
    m.advance_hours(200.0 * 24.0);
    let _clock = ClockGuard::push(m.clock.clone());
    let before = m.store.traces[&id].fidelity;
    let r = selmem::dream::weather::run(&mut m.store, &m.profile);
    let t = &m.store.traces[&id];
    assert!(r.weathered > 0);
    assert!(t.fidelity < before);
    assert!(t.disgust < 0.4);
    assert_eq!(t.gist, gist);
    assert_ne!(t.status, TraceStatus::Active);
}
#[test]
fn metadata_and_singleton_motifs_do_not_boost_irrelevant_traces() {
    let mut m = memory();
    let id = add(&mut m, "Caroline said: Good to see you.", 0.4);
    m.store.add_axiom(IdentityAxiom {
        id: "motif".into(),
        statement: "observed-belonging".into(),
        support_trace_ids: vec![id.clone()],
        valence: 0.4,
        strength: 1.0,
        created_at: 0,
        superseded_by: None,
        schema: Some("observed-belonging".into()),
        layer: AxiomLayer::Motif,
        stake_kind: StakeKind::None,
        bearer: Bearer::Other,
        loss_kind: LossKind::None,
        stake_mark: String::new(),
    });
    let (_, dump) = m.remember_with(
        "According to the observed conversations, what happened during the hike?",
        RecallWrite::ReadOnly,
        RecallBias::Observed,
        &[],
    );
    let t = dump.candidates.iter().find(|t| t.trace_id == id).unwrap();
    assert!((t.score - t.base_score * t.anchor).abs() < 1e-6);
}
#[test]
fn disposition_keeps_the_question_subject_and_excludes_boilerplate() {
    let mut m = memory();
    let c = add(&mut m, "Caroline said: The hike made me afraid.", -0.7);
    let other = add(&mut m, "Melanie said: The hike made me happy.", 0.9);
    let all = vec![c, other];
    let p = ReadingProfile::for_query(
        &m.store,
        &Mood::default(),
        "How did Caroline feel about the hike?",
        &all,
    );
    assert_eq!(p.salient.len(), 1);
    assert!(p.valence_bias < 0.0);
    assert_eq!(p.mood, "guarded");
    assert!(ReadingProfile::for_query(
        &m.store,
        &Mood::default(),
        "Design a cooperative game with string.",
        &all
    )
    .is_empty());
}
#[test]
fn factual_support_recall_does_not_depend_on_observed_metadata() {
    let mut m = memory();
    let id = add(
        &mut m,
        "Caroline said: The support group made me feel accepted.",
        0.7,
    );
    let (_, a) = m.remember_with(
        "What effect did the support group have on Caroline?",
        RecallWrite::ReadOnly,
        RecallBias::Observed,
        &[],
    );
    let (_, b) = m.remember_with("According to the observed conversations, what effect did the support group have on Caroline?", RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(a.selected.contains(&id));
    let ca = a.candidates.iter().find(|c| c.trace_id == id).unwrap();
    let cb = b.candidates.iter().find(|c| c.trace_id == id).unwrap();
    assert!(
        (ca.score - ca.base_score * ca.anchor - (cb.score - cb.base_score * cb.anchor)).abs()
            < 1e-6
    );
}
