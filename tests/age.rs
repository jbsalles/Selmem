//! Survival slows erasure. Recency lowers weight. Use needs the context cloud.

use selmem::encode::scoring::{behavior_weight, hazard_scale};
use selmem::recall::retrieve::{context_cloud_pub, statement_anchored};
use selmem::{advance_hours, now_secs, EncodeInput, EntityProfile, SelectiveMemory};

fn hour<'a>(text: &'a str, schema: &str, relevance: f32) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.2;
    ev.arousal = 0.4;
    ev.self_relevance = relevance;
    ev.permanence = 0.5;
    ev.schema = Some(schema.into());
    ev
}

#[test]
fn hazard_falls_as_the_hour_survives() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender(""));
    mem.live("the draft was withdrawn after delivery");
    let id = mem.store.active_ids()[0].clone();
    let young = hazard_scale(mem.store.traces.get(&id).unwrap());
    assert!(young > 0.9, "day 0 hazard {young}");

    mem.store.traces.get_mut(&id).unwrap().created_at = now_secs().saturating_sub(30 * 86_400);
    let month = hazard_scale(mem.store.traces.get(&id).unwrap());
    assert!(month < young / 5.0, "a month must be much safer: {month} vs {young}");

    mem.store.traces.get_mut(&id).unwrap().created_at = now_secs().saturating_sub(365 * 86_400);
    let year = hazard_scale(mem.store.traces.get(&id).unwrap());
    assert!(year < 0.02, "a year is almost not erasable: {year}");
}

#[test]
fn recent_use_weighs_more_than_an_old_use() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender(""));
    mem.live("the board kept the credit");
    let id = mem.store.active_ids()[0].clone();
    let fresh = behavior_weight(mem.store.traces.get(&id).unwrap());
    mem.store.traces.get_mut(&id).unwrap().last_recalled_at = Some(now_secs().saturating_sub(200 * 86_400));
    let old = behavior_weight(mem.store.traces.get(&id).unwrap());
    assert!(fresh > 0.9, "just encoded weighs near 1: {fresh}");
    assert!(old < fresh, "an old use weighs less: {old} vs {fresh}");
    assert!(old >= 0.12, "years-old still has a floor: {old}");
}

#[test]
fn cloud_uses_a_neighbor_weighted_by_age() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender(""));
    mem.live_with(hour("the long assignment starts monday", "work", 0.9));
    mem.live_with(hour("the mandate was withdrawn after the draft", "work", 0.9));
    let ids = mem.store.active_ids();
    let neighbor = ids.iter().find(|id| {
        mem.store.traces.get(*id).unwrap().gist.contains("mandate")
            || mem.store.traces.get(*id).unwrap().core.contains("mandate")
    }).cloned();
    let Some(neighbor) = neighbor else {
        panic!("mandate hour was not kept");
    };
    let cloud = context_cloud_pub(&mem.store, "assignment");
    assert!(!cloud.is_empty(), "the query must touch a cloud");
    assert!(
        statement_anchored(&cloud, "the mandate was withdrawn"),
        "a same-schema neighbor must anchor through the touched hour"
    );
    let young = cloud.get("assignment").copied().unwrap_or(0.0);

    mem.store.traces.get_mut(&neighbor).unwrap().created_at = now_secs().saturating_sub(400 * 86_400);
    mem.store.traces.get_mut(&neighbor).unwrap().last_recalled_at = Some(now_secs().saturating_sub(400 * 86_400));
    let aged = context_cloud_pub(&mem.store, "assignment");
    let still = aged.get("assignment").copied().unwrap_or(0.0);
    assert!(still <= young + 0.001, "aging a neighbor must not raise the seed token");
    let _ = advance_hours;
}

#[test]
fn an_unrelated_statement_is_not_anchored() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender(""));
    mem.live("the long assignment starts monday");
    let cloud = context_cloud_pub(&mem.store, "assignment");
    assert!(!statement_anchored(&cloud, "thermostat setting returned"));
}
