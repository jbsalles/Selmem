use selmem::{accept_core, EncodeInput, EntityProfile, SelectiveMemory};

#[test]
fn extractive_core_preserves_the_event_relation_within_its_budget() {
    let event = "Yesterday at 5:30pm leaving the office Marc told me he was resigning because Sarah threatened to expose the accounting problem";
    let mut mem = SelectiveMemory::new(EntityProfile::tender("t"));
    let mut ev = EncodeInput::new(event);
    ev.permanence = 1.0;
    ev.self_relevance = 1.0;
    ev.arousal = 1.0;
    assert!(mem.live_with(ev).kept);
    let core = mem.store.traces.values().next().unwrap().core.clone();
    assert!(core.split_whitespace().count() <= 32);
    for fact in [
        "Marc",
        "resigning",
        "because",
        "Sarah",
        "threatened",
        "accounting problem",
    ] {
        assert!(core.contains(fact), "lost event relation {fact}: {core}");
    }
    assert!(accept_core(&core, event).is_some());
}

#[test]
fn accept_core_keeps_overlapping_facts() {
    let event = "Marc resigns because Sarah threatens to expose the accounts";
    assert!(accept_core("Marc resigns Sarah threatens accounts", event).is_some());
    assert!(accept_core("a dragon stole the moon last night", event).is_none());
    assert!(
        accept_core(
            "Trump signed the Abraham Accords normalizing Israel-Arab ties",
            "you know donald trump? why former?"
        )
        .is_none(),
        "pretrained world knowledge must not become the core"
    );
}
