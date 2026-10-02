use selmem::{EncodeInput, EntityProfile, ExperimentRng, SelectiveMemory};

#[test]
fn release_drops_pending_night_id() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("T"));
    let dec = mem.live_with(EncodeInput::new("she left the plate on the table"));
    let id = dec.trace_id.expect("kept");
    assert!(mem.store.pending_night.iter().any(|x| x == &id));
    mem.store.release_trace(&id);
    assert!(!mem.store.pending_night.iter().any(|x| x == &id));
}

#[test]
fn encode_keeps_observation_apart_from_interpretation() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("T"));
    let dec = mem.live_with(EncodeInput::new("she left the plate on the table"));
    let id = dec.trace_id.expect("kept");
    let t = mem.store.traces.get(&id).unwrap();
    assert_eq!(t.observation_id, t.archive_id);
    assert!(!t.interpretation.statement.is_empty());
    assert_eq!(t.operations[0].kind, "encode");
    assert_eq!(t.operations[0].origin.token(), "event");
    let verbatim = mem.store.archive_verbatim(&id).unwrap();
    assert!(verbatim.contains("plate"));
}

#[test]
fn experiment_rng_is_stable_per_seed() {
    let mut a = ExperimentRng::from_seed(1);
    let mut b = ExperimentRng::from_seed(1);
    let mut c = ExperimentRng::from_seed(2);
    assert_eq!(a.next_u64(), b.next_u64());
    assert_ne!(a.next_u64(), c.next_u64());
}

#[test]
fn semantic_core_names_actor_and_action_without_replacing_lexical_core() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("T"));
    let dec = mem.live_with(EncodeInput::new("She left because she no longer trusted him"));
    let id = dec.trace_id.expect("kept");
    let t = mem.store.traces.get(&id).unwrap();
    assert!(t.semantic.entities.iter().any(|e| e.eq_ignore_ascii_case("She")));
    assert!(t.semantic.actions.iter().any(|a| a == "left"));
    assert!(!t.core.is_empty());
    assert_eq!(t.semantic.claim, t.core);
}

#[test]
fn detached_clocks_do_not_share_a_jump() {
    let mut a = SelectiveMemory::new(EntityProfile::tender("A")).detach_clock();
    let mut b = SelectiveMemory::new(EntityProfile::tender("B")).detach_clock();
    let before = a.clock.now();
    a.advance_hours(48.0);
    assert!(a.clock.now() >= before + 47 * 3600);
    assert!(b.clock.now() + 3600 < a.clock.now());
}

#[test]
fn different_cause_is_not_the_same_claim() {
    let j = selmem::recall::judge_against_core(
        "He left because he hated me",
        "He left because I misunderstood him",
    );
    assert_eq!(j.kind, selmem::recall::DetachKind::Elaborate);
    assert!(j.causal_match < 0.5);
}

#[test]
fn paraphrase_of_leaving_is_a_reframe_not_a_departure() {
    let j = selmem::recall::judge_against_core("She walked away", "She abandoned me");
    assert_ne!(j.kind, selmem::recall::DetachKind::Depart, "{:?}", j.kind);
    assert!(j.entity_match > 0.0);
}

#[test]
fn reinterpret_keeps_the_reality_claim() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("T"));
    let id = mem
        .live_with(EncodeInput::new("She left because she no longer trusted him"))
        .trace_id
        .expect("kept");
    let anchor = mem.store.traces[&id].reality.claim.clone();
    assert!(mem.reinterpret(&id, "Her leaving came from a loss of trust"));
    let t = &mem.store.traces[&id];
    assert_eq!(t.reality.claim, anchor);
    assert_eq!(t.interpretation.statement, "Her leaving came from a loss of trust");
    assert!(t.operations.iter().any(|o| o.kind == "reinterpret"));
    assert!(t.drifts.iter().all(|d| d.kind != selmem::DriftKind::Ground));
}

#[test]
fn similar_hours_with_opposite_valence_do_not_merge() {
    let mut profile = EntityProfile::tender("T");
    profile.merge_similarity = 0.1;
    profile.encode_threshold = 0.1;
    let mut mem = SelectiveMemory::new(profile);
    for (text, v) in [("You stayed by the window in the rain", 0.55), ("You stayed by the window in the rain and did not leave", -0.62)] {
        let mut ev = EncodeInput::new(text);
        ev.valence = v;
        ev.arousal = 0.4;
        ev.self_relevance = 0.8;
        ev.schema = Some("loyalty".into());
        assert!(mem.live_with(ev).kept);
    }
    let report = mem.sleep();
    assert_eq!(report.merged, 0);
    assert!(mem.store.merges_refused >= 1);
}
