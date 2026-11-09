//! Regression of the entire in-memory store across read-only recall.
//! Unlike a trace-only comparison, this includes links, evidence and associations.
use selmem::{Attribution, EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory};

fn state(mem: &SelectiveMemory) -> String {
    let s = &mem.store;
    // Compare within one process: the HashMap iteration order is stable if
    // recall did not mutate anything. No serialization format assumptions.
    format!(
        "{:?}\n{:?}\n{:?}",
        (
            &s.normative,
            &s.contextual_scope,
            &s.self_consolidation_audit,
            &s.self_policy,
            &s.embedding_space,
            &s.observation_evidence,
            &s.associations,
            &s.traces,
            &s.archives
        ),
        (
            &s.axioms,
            &s.edges,
            &s.last_deep_at,
            &s.merges_refused,
            &s.pending_night,
            &s.centers,
            &s.measures
        ),
        (&mem.mood, &mem.recall_tally),
    )
}

#[test]
fn readonly_recall_preserves_all_store_fields_and_is_deterministic() {
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.0;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    memory.clock.origin_real = 4_102_444_800;
    memory.clock.scale = 1;
    memory.set_associations(true, true);
    memory.set_observation_evidence(true);
    for i in 0..12 {
        let event = format!("At the team meeting the project review {i} was completed.");
        let observation = format!("review-{i}");
        let mut input = EncodeInput::observed(&event, &observation);
        input.utility = 0.9;
        input.permanence = 0.9;
        input.self_relevance = 0.9;
        input.arousal = 0.7;
        input.attribution = Attribution::Internal;
        assert!(memory.live_with(input).kept, "fixture admission {i} failed");
    }
    assert!(!memory.store.traces.is_empty(), "no traces stored");
    let before = state(&memory);
    let query = "team meeting project review";
    let (first_memories, first) =
        memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(
        !first.selected.is_empty(),
        "fixture must actually exercise recall"
    );
    let (second_memories, second) =
        memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert_eq!(before, state(&memory), "ReadOnly recall mutated state");
    assert_eq!(
        first.selected, second.selected,
        "read-only selection changed"
    );
    assert_eq!(
        first_memories
            .iter()
            .map(|m| (&m.trace_id, &m.narrative))
            .collect::<Vec<_>>(),
        second_memories
            .iter()
            .map(|m| (&m.trace_id, &m.narrative))
            .collect::<Vec<_>>(),
        "read-only reconstruction changed"
    );
}
