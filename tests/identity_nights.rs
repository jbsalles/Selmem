//! Constitutive agent identity vs fallible relationship and sleep memories.
use selmem::{Attribution, Channel, EntityProfile, SelectiveMemory, TraceStatus};

fn organ() -> SelectiveMemory {
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.05;
    SelectiveMemory::new(profile)
}

#[test]
fn agents_own_name_survives_empty_or_forgotten_book() {
    let mut m = organ();
    let d = m.open_mouth("Comment je m'appelle ?");
    let system = d.memory_reply().system_prompt();
    assert!(system.contains("Claire"));
    assert!(system.to_ascii_lowercase().contains("not the human's"));
    m.cancel_mouth(&d);
    m.reset();
    let d = m.open_mouth("Who are you?");
    assert!(d.memory_reply().system_prompt().contains("Claire"));
    m.cancel_mouth(&d);
}

#[test]
fn interlocutor_name_is_an_external_fallible_episode() {
    let mut m = organ();
    m.talk
        .record("Je m'appelle Victor", "Ravi de te rencontrer.");
    let (kept, _) = m.keep_sitting();
    assert_eq!(kept, 1);
    let t = m
        .store
        .traces
        .values()
        .find(|t| t.source == "talk" && t.schema.as_deref() == Some("interlocutor-name"))
        .unwrap();
    assert_eq!(t.channel, Channel::Selfhood);
    assert_eq!(t.attribution, Attribution::External);
    assert!(t.gist.contains("Victor"));
    assert_eq!(m.profile.name, "Claire");
    let id = t.id.clone();
    m.clear_talk();
    let d = m.open_mouth("Comment je m\x27appelle ?");
    assert!(d.memory_reply().user_prompt().contains("Victor"));
    m.cancel_mouth(&d);
    m.store.traces.get_mut(&id).unwrap().status = TraceStatus::Latent;
    assert_eq!(m.profile.name, "Claire");
    m.clear_talk();
    let d = m.open_mouth("Comment je m\x27appelle ?");
    assert!(!d.memory_reply().user_prompt().contains("Victor"));
    m.cancel_mouth(&d);
}

#[test]
fn night_recollection_is_counted_from_surviving_episodes() {
    let mut m = organ();
    let a = m.record_sleep_memory().expect("first sleep remembered");
    m.advance_hours(24.0);
    let b = m.record_sleep_memory().expect("second sleep remembered");
    assert_ne!(a, b);
    assert_eq!(m.remembered_nights(), 2);
    let d = m.open_mouth("Combien de nuits avons-nous passees ?");
    assert!(d.memory_reply().user_prompt().contains("at least 2 nights"));
    m.cancel_mouth(&d);
    m.store.traces.get_mut(&a).unwrap().status = TraceStatus::Latent;
    assert_eq!(m.remembered_nights(), 1);
    m.store.traces.get_mut(&b).unwrap().suppressed = true;
    assert_eq!(m.remembered_nights(), 0);
    let d = m.open_mouth("Combien de nuits ?");
    assert!(!d.memory_reply().user_prompt().contains("at least 2 nights"));
    m.cancel_mouth(&d);
}
