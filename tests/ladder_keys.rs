use selmem::{EncodeInput, EntityProfile, SelectiveMemory};

#[test]
fn repeated_consolidation_reuses_each_schema_and_stake_key() {
    let mut memory = SelectiveMemory::new(EntityProfile::tender("keys"));
    for (schema, mark) in [
        ("office:access", "boundary"),
        ("office:access", "promise"),
        ("office", "access:boundary"),
    ] {
        for event in ["You stayed with the team.", "You remained beside the team."] {
            let mut input = EncodeInput::new(event);
            input.valence = 0.6;
            input.arousal = 0.7;
            input.self_relevance = 0.9;
            input.permanence = 1.0;
            input.schema = Some(schema.into());
            let id = memory.live_with(input).trace_id.expect("kept");
            memory.store.traces.get_mut(&id).unwrap().stake_mark = mark.into();
        }
    }
    selmem::dream::ladder::run(&mut memory.store, &selmem::RuleNarrator);
    let mut before: Vec<_> = memory
        .store
        .living_axioms()
        .iter()
        .map(|a| a.id.clone())
        .collect();
    before.sort();
    assert_eq!(before.len(), 3, "distinct marks remain independent");
    assert!(memory
        .store
        .living_axioms()
        .iter()
        .any(|a| a.schema.as_deref() == Some("office") && a.stake_mark == "access:boundary"));
    selmem::dream::ladder::run(&mut memory.store, &selmem::RuleNarrator);
    let mut after: Vec<_> = memory
        .store
        .living_axioms()
        .iter()
        .map(|a| a.id.clone())
        .collect();
    after.sort();
    assert_eq!(after, before, "same keys must reuse the living axioms");
}
