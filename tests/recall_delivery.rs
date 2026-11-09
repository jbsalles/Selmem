use selmem::{
    Channel, EncodeInput, EntityProfile, MemoryContextFormat, MemoryContextOptions, SelectiveMemory,
};

fn fixture(format: MemoryContextFormat, words: usize) -> SelectiveMemory {
    let mut profile = EntityProfile::new("observer");
    profile.encode_threshold = 0.0;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    for event in [
        "Workshop opening time is nine in the morning.",
        "Workshop repair materials are stored in the blue cabinet.",
        "Workshop emergency exit leads into the north courtyard.",
        "Workshop beginner classes take place every Saturday.",
    ] {
        let mut input = EncodeInput::new(event);
        input.channel = Channel::World;
        input.permanence = 1.0;
        assert!(memory.live_with(input).kept);
    }
    memory
        .set_memory_context_options(MemoryContextOptions {
            format,
            words,
            ..Default::default()
        })
        .unwrap();
    memory
}

#[test]
fn structured_conversation_delivers_four_selected_episodes() {
    let mut memory = fixture(MemoryContextFormat::Structured, 128);
    let draft = memory.open_mouth("Tell me about the workshop.");
    assert_eq!(draft.dump.selected.len(), 4);
    let context = draft.memory_reply().filtered_context();
    assert_eq!(context.episodes.len(), 4);
    assert_eq!(
        context
            .episodes
            .iter()
            .map(|e| &e.trace_id)
            .collect::<Vec<_>>(),
        draft.dump.selected.iter().collect::<Vec<_>>()
    );
    assert!(context.content_words <= 128);
}

#[test]
fn legacy_keeps_two_scenes_and_structured_keeps_its_word_limit() {
    let mut memory = fixture(MemoryContextFormat::Legacy, 128);
    assert_eq!(
        memory
            .open_mouth("Tell me about the workshop.")
            .memories
            .len(),
        2
    );
    let mut memory = fixture(MemoryContextFormat::Structured, 32);
    let draft = memory.open_mouth("Tell me about the workshop.");
    let context = draft.memory_reply().filtered_context();
    assert!(context.content_words <= 32);
    assert!(context
        .episodes
        .iter()
        .all(|e| draft.dump.selected.contains(&e.trace_id)));
}
