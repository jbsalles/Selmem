//! P6 directed forgetting. P7 confidence is not access.

use selmem::{
    Channel, DriftKind, EncodeInput, EntityProfile, Narrator, RecallBias, RecallWrite,
    RuleNarrator, SelectiveMemory,
};

fn hour<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.40;
    ev.arousal = 0.50;
    ev.self_relevance = 0.80;
    ev.permanence = 0.40;
    ev.schema = Some("office".into());
    ev.channel = Channel::Selfhood;
    ev
}

#[test]
fn suppress_leaves_the_hour_but_drops_it_from_remember() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hour(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let hits = mem.remember("cancelled project team");
    assert!(
        hits.iter().any(|h| h.trace_id == id),
        "uns suppressed hour is recallable"
    );
    assert!(mem.suppress(&id));
    let t = &mem.store.traces[&id];
    assert!(t.suppressed);
    assert_eq!(t.status, selmem::TraceStatus::Active);
    assert!(t.drifts.iter().any(|d| d.kind == DriftKind::Suppress));
    let hits = mem.remember("cancelled project team");
    assert!(
        !hits.iter().any(|h| h.trace_id == id),
        "suppressed hour must not win Observed recall"
    );
    let (_, dump) = mem.remember_with(
        "cancelled project team",
        RecallWrite::ReadOnly,
        RecallBias::ForceMarked,
        &[id.clone()],
    );
    assert!(
        dump.selected.iter().any(|s| s == &id),
        "ForceMarked still reaches a suppressed hour"
    );
}

#[test]
fn unsuppress_returns_the_hour_to_recall() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hour("The project was cancelled in front of the team."))
        .trace_id
        .expect("kept");
    assert!(mem.suppress(&id));
    assert!(mem.unsuppress(&id));
    assert!(!mem.store.traces[&id].suppressed);
    let hits = mem.remember("cancelled project team");
    assert!(hits.iter().any(|h| h.trace_id == id));
}

#[test]
fn suppress_is_not_release() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hour("The project was cancelled in front of the team."))
        .trace_id
        .expect("kept");
    let n_arch = mem.store.archives.len();
    assert!(mem.suppress(&id));
    assert!(mem.store.traces.contains_key(&id));
    assert_eq!(mem.store.archives.len(), n_arch);
}

#[test]
fn confidence_starts_at_one_and_falls_when_the_gist_is_rewritten() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hour("The project was cancelled in front of the team."))
        .trace_id
        .expect("kept");
    {
        let t = &mem.store.traces[&id];
        assert!((t.confidence - 1.0).abs() < 1e-4);
        assert!((t.access - 1.0).abs() < 1e-4);
    }
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = t.core.clone();
        t.fidelity = 0.28;
        t.anchor = 0.20;
        t.permanence = 0.30;
    }
    mem.store.add_axiom(selmem::IdentityAxiom {
        id: "ax_office".into(),
        statement: "Credit gone in public.".into(),
        support_trace_ids: vec![id.clone()],
        valence: -0.6,
        strength: 0.4,
        created_at: 1,
        superseded_by: None,
        schema: Some("office".into()),
        layer: selmem::AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    assert!(selmem::dream::confab::fill_if_hole(
        &mut mem.store,
        &id,
        &mem.profile
    ));
    let t = &mem.store.traces[&id];
    assert!(
        t.confidence < t.access,
        "a filled hole is easy to keep in mind and not to be trusted: conf={} acc={}",
        t.confidence,
        t.access
    );
    assert!(t.confidence < 0.40, "got {}", t.confidence);
}

#[test]
fn low_confidence_mouth_speaks_the_core() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hour("The project was cancelled in front of the team."))
        .trace_id
        .expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "Even that, with time, took on another softness.".into();
        t.confidence = 0.20;
        t.fidelity = 0.80;
        t.access = 0.90;
    }
    let spoken = RuleNarrator.reconstruct(
        &mem.store.traces[&id],
        &mem.mood,
        "what happened",
    );
    assert_eq!(spoken, mem.store.traces[&id].core);
}

#[test]
fn file_roundtrip_keeps_suppress_and_confidence() {
    let dir = std::env::temp_dir().join(format!("selmem-supp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::open(&path, p.clone()).unwrap();
    let id = mem
        .live_with(hour("The project was cancelled in front of the team."))
        .trace_id
        .expect("kept");
    assert!(mem.suppress(&id));
    mem.store.traces.get_mut(&id).unwrap().confidence = 0.33;
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, p).unwrap();
    let t = &loaded.store.traces[&id];
    assert!(t.suppressed);
    assert!((t.confidence - 0.33).abs() < 1e-3);
    let _ = std::fs::remove_dir_all(&dir);
}
