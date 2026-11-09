use selmem::*;
fn fixture() -> SelectiveMemory {
    let mut p = EntityProfile::new("Claire");
    p.encode_threshold = 0.;
    let mut m = SelectiveMemory::new(p);
    for i in 0..16 {
        let event=format!("Warehouse constraint {i}: station {i} needs its own safety checklist before starting work. Supervisor number {i} approves its completion.");
        let source = format!("operator-{i}");
        let mut input = EncodeInput::observed(&event, &source);
        input.source = &source;
        input.attribution = Attribution::External;
        m.live_with(input);
    }
    m
}
#[test]
fn larger_delivery_reaches_past_four_episodes_and_restores_profile() {
    let mut m = fixture();
    m.set_memory_context_options(MemoryContextOptions {
        words: 64000,
        max_episodes: 12,
        episode_words: 800,
        token_budget: Some(8000),
        dispositions: false,
        affect: false,
        associations: false,
        ..Default::default()
    })
    .unwrap();
    let d = m.open_mouth("Warehouse constraint station checklist supervisor safety");
    let c = d.memory_reply().filtered_context();
    assert!(
        c.episodes.len() > 4,
        "selected {} delivered {}",
        d.dump.selected.len(),
        c.episodes.len()
    );
    assert!(c.episodes.len() <= 12);
    assert!(c.to_json().len() <= 8000);
    assert_eq!(m.profile.max_recall, 4);
}
#[test]
fn serialized_metadata_counts_and_refiltering_is_idempotent() {
    let mut m = fixture();
    for budget in [1000, 2000, 4000, 8000] {
        m.set_memory_context_options(MemoryContextOptions {
            words: 64000,
            max_episodes: 16,
            episode_words: 800,
            token_budget: Some(budget),
            ..Default::default()
        })
        .unwrap();
        let d = m.open_mouth("Warehouse constraint station checklist supervisor safety");
        let reply = d.memory_reply();
        let c = reply.filtered_context();
        assert!(c.to_json().len() <= budget);
        assert_eq!(c.to_json(), c.filtered(&d.memory_context_options).to_json());
        m.cancel_mouth(&d);
    }
}
#[test]
fn custom_counter_and_config_validation() {
    let c=Config::parse("memory_context_tokens=8000\nmemory_context_words=64000\nmemory_context_max_episodes=24\nmemory_context_episode_words=800");
    let o = MemoryContextOptions::from_config(&c).unwrap();
    assert_eq!(o.token_budget, Some(8000));
    assert_eq!(o.max_episodes, 24);
    let mut invalid = o.clone();
    invalid.max_episodes = 65;
    assert!(invalid.validate().is_err());
    invalid = o.clone();
    invalid.format = MemoryContextFormat::Legacy;
    assert!(invalid.validate().is_err());
    let mut m = fixture();
    let o = MemoryContextOptions {
        token_counter: |s| s.len() / 2 + 1,
        token_budget: Some(1000),
        words: 64000,
        max_episodes: 16,
        ..Default::default()
    };
    m.set_memory_context_options(o.clone()).unwrap();
    let d = m.open_mouth("Warehouse constraint checklist supervisor");
    let c = d.memory_reply().filtered_context();
    assert!((o.token_counter)(&c.to_json()) <= 1000);
}
