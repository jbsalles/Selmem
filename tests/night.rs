//! Budget, spoken rehearsal, merge/axiom lineage.

use selmem::{
    evaluate_budget, EntityProfile, NightKind, RecallBias, SelectiveMemory, TraceStatus,
    SHALLOW_PASSES, NIGHT_PASSES,
};
use selmem::EncodeInput;

fn dull<'a>(text: &'a str, schema: &str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.10;
    ev.arousal = 0.12;
    ev.disgust = 0.00;
    ev.self_relevance = 0.70;
    ev.permanence = 0.80;
    ev.schema = Some(schema.into());
    ev
}

fn charged<'a>(text: &'a str, schema: &str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.70;
    ev.arousal = 0.80;
    ev.disgust = 0.55;
    ev.self_relevance = 0.90;
    ev.permanence = 0.85;
    ev.schema = Some(schema.into());
    ev
}

#[test]
fn shallow_passes_are_weather_and_release() {
    assert_eq!(SHALLOW_PASSES, &["weather", "release"]);
    assert_eq!(
        NIGHT_PASSES,
        &["weather", "rewrite", "merge", "ladder", "release"]
    );
}

#[test]
fn calm_hours_wait_for_the_count() {
    let mut p = EntityProfile::tender("Claire");
    p.deep_min_hours = 3;
    p.deep_min_charge = 1.2;
    let mut mem = SelectiveMemory::new(p);
    assert!(mem.live_with(dull("A quiet coffee by the window.", "daily")).kept);
    assert!(mem.live_with(dull("Another quiet coffee, same window.", "daily")).kept);
    let b = evaluate_budget(&mem.store, &mem.profile);
    assert_eq!(b.new_hours, 2);
    assert!(b.charge < 1.2);
    assert_eq!(b.kind, NightKind::Shallow);
    let report = mem.sleep();
    assert_eq!(report.kind, NightKind::Shallow);
    assert_eq!(report.merged, 0);
    assert!(mem.store.last_deep_at.is_none());
    assert!(mem.live_with(dull("A third quiet coffee.", "daily")).kept);
    let report = mem.sleep();
    assert_eq!(report.kind, NightKind::Deep);
    assert!(mem.store.last_deep_at.is_some());
}

#[test]
fn a_charged_hour_pays_for_a_deep_night() {
    let mut p = EntityProfile::tender("Claire");
    p.deep_min_hours = 3;
    p.deep_min_charge = 1.2;
    let mut mem = SelectiveMemory::new(p);
    assert!(mem
        .live_with(charged(
            "The project was cancelled and they stole the credit.",
            "injustice",
        ))
        .kept);
    let b = evaluate_budget(&mem.store, &mem.profile);
    assert_eq!(b.new_hours, 1);
    assert!(b.charge >= 1.2);
    assert_eq!(b.kind, NightKind::Deep);
    let report = mem.sleep();
    assert_eq!(report.kind, NightKind::Deep);
}

#[test]
fn second_night_without_new_hours_is_shallow() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    assert!(mem.live_with(dull("You stayed in the rain by the window.", "loyalty")).kept);
    let first = mem.sleep();
    assert_eq!(first.kind, NightKind::Deep);
    let second = mem.sleep();
    assert_eq!(second.kind, NightKind::Shallow);
    assert_eq!(second.merged, 0);
}

#[test]
fn remember_does_not_count_as_spoken_utility() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let id = mem
        .live_with(charged(
            "You stayed in the rain by the window and did not leave.",
            "loyalty",
        ))
        .trace_id
        .unwrap();
    assert_eq!(mem.store.traces[&id].rehearsals, 0);
    let hits = mem.remember("rain window loyalty");
    assert!(!hits.is_empty());
    assert_eq!(mem.store.traces[&id].rehearsals, 0);
    let _ = mem.speak("What happened in the rain?");
    assert!(
        mem.store.traces[&id].rehearsals >= 1,
        "live speak must stamp the selected hour"
    );
}

#[test]
fn isolated_speak_still_does_not_rehearse() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let id = mem
        .live_with(charged(
            "The project was cancelled without a hearing.",
            "injustice",
        ))
        .trace_id
        .unwrap();
    let before = mem.store.traces[&id].rehearsals;
    let _ = mem.speak_isolated("Was the cancellation unjust?");
    assert_eq!(mem.store.traces[&id].rehearsals, before);
}

#[test]
fn merge_keeps_lineage_on_the_keeper_and_the_axiom() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    profile.merge_similarity = 0.15;
    let mut mem = SelectiveMemory::new(profile);
    let mut ids = Vec::new();
    for text in [
        "You stayed in the rain by the window.",
        "That rain again: you stayed by the window and did not leave.",
    ] {
        let mut ev = EncodeInput::new(text);
        ev.valence = 0.35;
        ev.arousal = 0.35;
        ev.self_relevance = 0.7;
        ev.schema = Some("loyalty".into());
        ids.push(mem.live_with(ev).trace_id.expect("kept"));
    }
    let report = mem.sleep_deep();
    assert!(report.merged >= 1, "two close rain hours must fuse");
    let lineage = mem.store.lineage(&[ids[0].clone()]);
    assert!(
        ids.iter().all(|id| lineage.iter().any(|x| x == id)),
        "merge edges must keep the rain family reachable, got {lineage:?}"
    );
    let myths: Vec<String> = mem
        .store
        .traces
        .values()
        .filter(|t| t.status == TraceStatus::Myth)
        .map(|t| t.id.clone())
        .collect();
    assert!(!myths.is_empty(), "absorbed hours become Myth");
    for m in &myths {
        assert!(
            mem.store.edges.values().any(|n| n.contains(m)),
            "merged myth {m} must stay on an edge"
        );
    }
    assert!(
        mem.who_am_i().iter().all(|ax| !ax.support_trace_ids.is_empty()),
        "ladder may not mint an orphan axiom"
    );
}

#[test]
fn live_speak_raises_axiom_strength_only_with_the_cut() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    for text in [
        "You stayed in the rain by the window.",
        "Again you waited in the rain and did not leave.",
    ] {
        assert!(mem.live_with(charged(text, "loyalty")).kept);
    }
    mem.sleep_deep();
    let before = mem.store.max_axiom_strength();
    assert!(before > 0.0, "ladder must mint");
    let id = mem
        .store
        .traces
        .values()
        .find(|t| t.schema.as_deref() == Some("loyalty"))
        .map(|t| t.id.clone())
        .expect("hour");
    mem.note_spoken(&id);
    assert_eq!(
        mem.store.max_axiom_strength(),
        before,
        "default cut does not move strength"
    );
    mem.cut.util_to_strength = true;
    mem.note_spoken(&id);
    assert!(
        mem.store.max_axiom_strength() > before,
        "util cut must step strength"
    );
}

#[test]
fn isolated_note_path_is_speak_only() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    mem.cut.util_to_strength = true;
    assert!(mem.live_with(charged("The project was cancelled.", "injustice")).kept);
    mem.sleep_deep();
    let before = mem.store.max_axiom_strength();
    let _ = mem.speak_isolated("Was it unjust?");
    assert_eq!(mem.store.max_axiom_strength(), before);
}

#[test]
fn merge_veto_keeps_pinned_hours_apart() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    profile.merge_similarity = 0.15;
    let mut mem = SelectiveMemory::new(profile);
    mem.cut.merge_support_veto = true;
    let mut ids = Vec::new();
    for text in [
        "You stayed in the rain by the window.",
        "That rain again: you stayed by the window and did not leave.",
    ] {
        let mut ev = dull(text, "loyalty");
        ev.permanence = 0.40;
        let id = mem.live_with(ev).trace_id.expect("kept");
        ids.push(id);
    }
    mem.pin(&ids[0]);
    let report = mem.sleep_deep();
    assert_eq!(report.merged, 0, "pin must veto merge");
    assert!(mem.store.merges_refused >= 1);
}

#[test]
fn two_schema_hours_mint_an_axiom() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    for text in [
        "They left for three days without a word. The plate stayed on the table.",
        "The door clicked at two in the morning. No note on the table.",
    ] {
        let mut ev = charged(text, "hearth");
        ev.schema = Some("hearth".into());
        assert!(mem.live_with(ev).kept);
    }
    mem.sleep_deep();
    assert!(
        !mem.who_am_i().is_empty(),
        "two hearth hours must mint a motif or belief"
    );
}

#[test]
fn motif_mints_below_the_belief_cap() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    for text in [
        "You stayed in the rain by the window.",
        "Again you waited in the rain and did not leave.",
    ] {
        assert!(mem.live_with(charged(text, "loyalty")).kept);
    }
    mem.sleep_deep();
    let s = mem.store.max_axiom_strength();
    assert!(s > 0.0, "must mint");
    assert!(
        s <= selmem::AxiomLayer::Motif.strength_cap() + 0.001,
        "two hours are a motif, strength={s}"
    );
}

#[test]
fn spoken_utility_can_raise_a_motif() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    mem.cut.util_to_strength = true;
    for text in [
        "You stayed in the rain by the window.",
        "Again you waited in the rain and did not leave.",
    ] {
        assert!(mem.live_with(charged(text, "loyalty")).kept);
    }
    mem.sleep_deep();
    let before = mem.store.max_axiom_strength();
    let id = mem
        .store
        .traces
        .values()
        .find(|t| t.schema.as_deref() == Some("loyalty"))
        .map(|t| t.id.clone())
        .expect("hour");
    mem.note_spoken(&id);
    let after = mem.store.max_axiom_strength();
    assert!(after > before, "util must raise a motif: {before} → {after}");
    assert!(after <= selmem::AxiomLayer::Motif.strength_cap() + 0.001);
}

#[test]
fn second_night_keeps_a_strengthened_schema() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    mem.cut.util_to_strength = true;
    for text in [
        "You stayed in the rain by the window.",
        "Again you waited in the rain and did not leave.",
    ] {
        assert!(mem.live_with(charged(text, "loyalty")).kept);
    }
    mem.sleep_deep();
    let id = mem
        .store
        .traces
        .values()
        .find(|t| t.schema.as_deref() == Some("loyalty"))
        .map(|t| t.id.clone())
        .expect("hour");
    mem.note_spoken(&id);
    let after_use = mem.store.max_axiom_strength();
    assert!(mem
        .live_with(charged("A third night in the rain by the window.", "loyalty"))
        .kept);
    assert!(mem
        .live_with(charged("Still in the rain. You did not leave.", "loyalty"))
        .kept);
    mem.sleep_deep();
    let after_night = mem.store.max_axiom_strength();
    assert!(
        after_night + 0.001 >= after_use,
        "same schema must not remint under the spoken strength: use={after_use} night={after_night}"
    );
    assert_eq!(mem.store.living_axioms().len(), 1, "one living axiom per schema");
}

#[test]
fn axioms_only_mouth_does_not_name_the_scene() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    let mut mem = SelectiveMemory::new(profile);
    for text in [
        "You stayed in the rain by the window.",
        "Again you waited in the rain and did not leave.",
    ] {
        assert!(mem.live_with(charged(text, "loyalty")).kept);
    }
    mem.sleep_deep();
    let (reply, _) =
        mem.speak_isolated_axioms("What do you do when someone leaves?", RecallBias::Observed, &[]);
    let low = reply.to_lowercase();
    assert!(
        !low.contains("window") && !low.contains("rain"),
        "axioms-only must not leak the gist: {reply}"
    );
    let ax = mem.who_am_i()[0].statement.clone();
    assert!(
        reply.contains(&ax),
        "mouth should carry the axiom `{ax}`, got {reply}"
    );
}
