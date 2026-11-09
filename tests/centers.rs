//! P4: schema centers. Periphery of an Internal hour can fall toward the hub.

use selmem::{Attribution, Channel, EncodeInput, EntityProfile, SelectiveMemory, TraceStatus};

fn charged<'a>(text: &'a str, schema: &str, attr: Attribution) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.70;
    ev.arousal = 0.80;
    ev.disgust = 0.55;
    ev.self_relevance = 0.90;
    ev.permanence = 0.85;
    ev.schema = Some(schema.into());
    ev.attribution = attr;
    ev.channel = Channel::Selfhood;
    ev
}

fn dull<'a>(text: &'a str, schema: &str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.05;
    ev.arousal = 0.12;
    ev.self_relevance = 0.40;
    ev.permanence = 0.35;
    ev.schema = Some(schema.into());
    ev.channel = Channel::Selfhood;
    ev
}

#[test]
fn two_schema_hours_mint_a_center_on_the_hub() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let hub = mem
        .live_with(charged(
            "The project was cancelled in front of the team.",
            "lyon-file",
            Attribution::Internal,
        ))
        .trace_id
        .expect("hub");
    let _ = mem.live_with(dull(
        "Someone reprints the agenda with the same items.",
        "lyon-file",
    ));
    let _ = mem.sleep_deep();
    let c = mem
        .store
        .centers
        .get("lyon-file")
        .expect("center after two hours");
    assert_eq!(c.hub_id.as_deref(), Some(hub.as_str()));
    assert!(
        c.core.to_lowercase().contains("cancel")
            || c.core.to_lowercase().contains("project")
            || c.core.to_lowercase().contains("team"),
        "hub core should be the prototype: {}",
        c.core
    );
}

#[test]
fn one_hour_without_axiom_has_no_center() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    assert!(
        mem.live_with(dull("The standup was at nine.", "daily"))
            .kept
    );
    let _ = mem.sleep_deep();
    assert!(
        !mem.store.centers.contains_key("daily"),
        "a single dull hour is not a schema"
    );
}

#[test]
fn faded_internal_periphery_falls_toward_the_center() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let _ = mem.live_with(charged(
        "The project was cancelled in front of the team.",
        "lyon-file",
        Attribution::Internal,
    ));
    let rim = mem
        .live_with(charged(
            "The printer in room B jammed and nobody came.",
            "lyon-file",
            Attribution::Internal,
        ))
        .trace_id
        .expect("rim");
    {
        let t = mem.store.traces.get_mut(&rim).unwrap();
        t.fidelity = 0.30;
        t.anchor = 0.20;
        t.permanence = 0.20;
    }
    selmem::dream::centers::refresh(&mut mem.store);
    let n = selmem::dream::centers::gravitate(&mut mem.store);
    assert!(n >= 1, "faded Internal rim must move");
    let t = &mem.store.traces[&rim];
    assert!(
        t.drifts.iter().any(|d| d.note == "schema-center"),
        "gravity writes a schema-center drift"
    );
    assert_eq!(t.status, TraceStatus::Active);
    let proto = &mem.store.centers["lyon-file"].core;
    assert_eq!(t.gist, *proto);
}

#[test]
fn external_rim_does_not_fall_toward_the_center() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let _ = mem.live_with(charged(
        "The project was cancelled in front of the team.",
        "lyon-file",
        Attribution::Internal,
    ));
    let rim = mem
        .live_with(charged(
            "The printer in room B jammed and nobody came.",
            "lyon-file",
            Attribution::External,
        ))
        .trace_id
        .expect("rim");
    {
        let t = mem.store.traces.get_mut(&rim).unwrap();
        t.fidelity = 0.30;
        t.anchor = 0.20;
    }
    let gist0 = mem.store.traces[&rim].gist.clone();
    selmem::dream::centers::refresh(&mut mem.store);
    let n = selmem::dream::centers::gravitate(&mut mem.store);
    assert_eq!(n, 0);
    assert_eq!(mem.store.traces[&rim].gist, gist0);
}

#[test]
fn file_roundtrip_keeps_the_center() {
    let dir = std::env::temp_dir().join(format!("selmem-center-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::open(&path, p.clone()).unwrap();
    let _ = mem.live_with(charged(
        "The project was cancelled in front of the team.",
        "lyon-file",
        Attribution::Internal,
    ));
    let _ = mem.live_with(dull(
        "Someone reprints the agenda with the same items.",
        "lyon-file",
    ));
    let _ = mem.sleep_deep();
    assert!(mem.store.centers.contains_key("lyon-file"));
    let core0 = mem.store.centers["lyon-file"].core.clone();
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, p).unwrap();
    assert_eq!(loaded.store.centers["lyon-file"].core, core0);
    let _ = std::fs::remove_dir_all(&dir);
}
