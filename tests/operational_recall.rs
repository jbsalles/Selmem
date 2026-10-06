use selmem::{Channel, EncodeInput, EntityProfile, SelectiveMemory};

#[test]
fn old_world_fact_is_queryable_without_self_relevance_and_unrelated_facts_are_excluded() {
    let mut memory = SelectiveMemory::new(EntityProfile::tender("facts")).detach_clock();
    for event in [
        "The appointment is Tuesday in room B.",
        "The invoice amount is ninety euros.",
    ] {
        let mut input = EncodeInput::new(event);
        input.channel = Channel::World;
        input.self_relevance = 0.0;
        input.permanence = 0.0;
        assert!(memory.live_with(input).kept);
    }
    memory.advance_hours(24.0 * 400.0);
    let hits = memory.remember("appointment Tuesday");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].narrative.contains("Tuesday"));
    assert!(!hits[0].narrative.contains("invoice"));
}

#[test]
fn world_conversations_still_fade_and_leave_the_living_book() {
    let mut memory = SelectiveMemory::new(EntityProfile::tender("conversation")).detach_clock();
    memory.clock.origin_real = 4_000_000_000;
    memory.clock.jump = 0;
    let mut input = EncodeInput::new("The copier jammed in room B this morning.");
    input.channel = Channel::World;
    input.source = "talk";
    input.permanence = 0.35;
    let id = memory.live_with(input).trace_id.expect("kept conversation");
    for _ in 0..60 {
        memory.advance_hours(24.0);
        memory.fade_sitting();
    }
    assert!(!memory.store.traces.contains_key(&id));
    assert!(memory.remember("copier room").is_empty());
}

#[test]
fn kept_sitting_uses_the_memory_clock() {
    let mut memory = SelectiveMemory::new(EntityProfile::tender("clock")).detach_clock();
    memory.clock.origin_real = 4_000_000_000;
    memory.clock.jump = 86_400;
    memory.speak("The copier jammed in room B this morning.");
    assert_eq!(memory.keep_sitting().0, 1);
    let trace = memory.store.traces.values().next().unwrap();
    assert_eq!(trace.created_at, 4_000_086_400);
}
