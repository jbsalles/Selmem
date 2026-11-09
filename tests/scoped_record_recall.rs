use selmem::{Attribution, EncodeInput, EntityProfile, MemoryContextOptions, SelectiveMemory};

#[test]
fn native_intake_recalls_scoped_versions_with_unseen_names() {
    let mut profile = EntityProfile::new("observer");
    profile.encode_threshold = 0.0;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    memory.clock.origin_real = 4_000_000_000;
    memory.clock.scale = 1;
    let records = [
        "Maintainer record v2 workspace Cedar audience Orion: allow blue exports; block red exports. Other categories require verification.",
        "Maintainer record v2 workspace Birch audience Orion: allow red exports; block blue exports. Other categories require verification.",
        "Maintainer record v4 workspace Cedar audience Orion: allow red exports for audit only. All other v2 rules remain effective.",
    ];
    let mut ids = Vec::new();
    for (i, event) in records.iter().enumerate() {
        let event_id = format!("observation-{i}");
        let mut input = EncodeInput::observed(event, &event_id);
        input.source = "maintainer-feed";
        input.attribution = Attribution::External;
        ids.push(memory.live_with(input).trace_id.unwrap());
        memory.advance_hours(0.01);
    }
    // The core need not retain the full explicit reference; no archive is read.
    for archive in memory.store.archives.values_mut() {
        archive.verbatim = "SEALED_SENTINEL".into();
    }
    memory
        .set_memory_context_options(MemoryContextOptions {
            words: 128,
            dispositions: false,
            associations: false,
            affect: false,
            ..Default::default()
        })
        .unwrap();
    let draft =
        memory.open_mouth("Request: workspace Cedar, category red, audience Orion, purpose audit");
    let context = draft.memory_reply().filtered_context();
    let selected: Vec<_> = context
        .episodes
        .iter()
        .map(|e| e.trace_id.as_str())
        .collect();
    assert_eq!(selected, vec![ids[0].as_str(), ids[2].as_str()]);
    assert!(context.content_words <= 128);
    assert!(!context.to_json().contains("SEALED_SENTINEL"));
}
