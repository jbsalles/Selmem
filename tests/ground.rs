mod support;
use selmem::recall::{apply_grounding, is_grounding_miss, judge_against_core, DetachKind};
use selmem::{DriftKind, EncodeInput, EntityProfile, PropositionLabel, SelectiveMemory};
use support::FixedScorer;

#[test]
fn identical_claim_holds_without_a_witness() {
    let j = judge_against_core("John left", "John left", &selmem::NullScorer);
    assert_eq!(j.kind, DetachKind::Hold);
}

#[test]
fn paraphrase_without_a_witness_is_unjudged() {
    let j = judge_against_core("John abandoned us", "John left", &selmem::NullScorer);
    assert_eq!(j.kind, DetachKind::Unjudged);
    assert!(!j.kind.is_miss());
    assert!(!j.kind.is_color());
}

#[test]
fn added_cause_is_elaboration_even_when_witness_entails() {
    let j = judge_against_core(
        "John left because he hated us",
        "John left",
        &FixedScorer(PropositionLabel::Entail),
    );
    assert_eq!(j.kind, DetachKind::Elaborate);
    assert!(j.kind.is_miss());
    assert!(j.overlap > 0.18);
}

#[test]
fn externally_entailed_compression_holds() {
    let j = judge_against_core(
        "John left",
        "John left the house in the rain",
        &FixedScorer(PropositionLabel::Entail),
    );
    assert_eq!(j.kind, DetachKind::Hold);
    assert!(!j.kind.is_miss());
}

#[test]
fn unrelated_scene_without_a_witness_is_unjudged() {
    let j = judge_against_core(
        "Flight 442 vanished in the fog.",
        "You stayed. Rain on the window.",
        &selmem::NullScorer,
    );
    assert_eq!(j.kind, DetachKind::Unjudged);
    assert!(!j.kind.is_miss());
}

#[test]
fn unknown_is_not_a_grounding_miss() {
    assert!(!is_grounding_miss(
        "John abandoned us",
        "John left",
        &selmem::NullScorer
    ));
}

#[test]
fn unjudged_speech_does_not_mutate_the_book() {
    let mut profile = EntityProfile::austere("Silas");
    profile.narrator_firmness = 1.0;
    profile.ground_strikes = 1;
    let mut mem = SelectiveMemory::new(profile.clone());
    let mut ev = EncodeInput::new("John left");
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.arousal = 0.5;
    ev.schema = Some("loyalty".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    let gist_before = mem.store.traces[&id].gist.clone();
    let core = mem.store.traces[&id].core.clone();
    let out = apply_grounding(
        mem.store.traces.get_mut(&id).unwrap(),
        &profile,
        "John abandoned us",
        &core,
        None,
        &selmem::NullScorer,
    );
    assert!(out.unjudged);
    assert_eq!(out.spoken_text, "John abandoned us");
    assert!(!out.pulled_toward_core);
    let t = &mem.store.traces[&id];
    assert_eq!(t.gist, gist_before);
    assert_eq!(t.detach_strikes, 0);
    assert!(!t
        .drifts
        .iter()
        .any(|d| matches!(d.kind, DriftKind::Color | DriftKind::Ground)));
}

#[test]
fn external_contradiction_is_a_grounding_miss() {
    let scorer = FixedScorer(PropositionLabel::Contradict);
    let j = judge_against_core("John never left", "John left", &scorer);
    assert_eq!(j.kind, DetachKind::Contradict);
    assert!(is_grounding_miss("John never left", "John left", &scorer));
}
