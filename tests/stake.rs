use selmem::core::model::{derive_stake, StakeKind};
use selmem::{EncodeInput, EntityProfile, SelectiveMemory};

fn hour<'a>(event: &'a str, valence: f32) -> EncodeInput<'a> {
    let mut input = EncodeInput::new(event);
    input.valence = valence;
    input.arousal = 0.7;
    input.self_relevance = 0.9;
    input.permanence = 0.8;
    input.schema = Some("mandate".into());
    input
}

#[test]
fn same_sign_different_stake_stays_two_traces() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("Ada"));
    mem.live_with(hour("The mandate was withdrawn after the review.", -0.6));
    mem.live_with(hour("The promise was kept after the review.", -0.6));
    mem.sleep();
    let n = mem.store.traces.values().filter(|t| t.status != selmem::core::model::TraceStatus::Myth).count();
    assert!(n >= 2, "two stakes must not collapse, living={n}");
}

#[test]
fn withdrawn_and_cancelled_do_not_share_an_axiom() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("Ada"));
    for _ in 0..3 {
        mem.live_with(hour("The mandate was withdrawn after the review.", -0.7));
        mem.live_with(hour("The mandate was cancelled after the review.", -0.7));
        mem.sleep();
    }
    let axioms: Vec<_> = mem.store.living_axioms().into_iter().map(|a| a.statement.clone()).collect();
    let joined = axioms.join(" | ");
    assert!(joined.contains("withdrawn") && joined.contains("cancelled"), "{joined}");
    assert!(
        axioms.iter().all(|s| !(s.contains("withdrawn") && s.contains("cancelled"))),
        "one axiom melted both marks: {joined}"
    );
}

#[test]
fn axiom_carries_stake_vector() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("Ada"));
    for _ in 0..3 {
        mem.live_with(hour("The mandate was withdrawn after the review.", -0.7));
        mem.sleep();
    }
    let ax = mem.store.living_axioms().into_iter().find(|a| a.statement.contains("stake=")).expect("axiom");
    assert_eq!(ax.stake_kind, StakeKind::Limit);
    assert!(ax.statement.contains("stake=limit"), "{}", ax.statement);
    assert!(ax.statement.contains("bearer="));
    assert!(ax.statement.contains("loss="));
}

#[test]
fn limit_outlasts_mood() {
    assert!(StakeKind::Limit.survival() < StakeKind::Mood.survival());
}

#[test]
fn marked_absence_is_a_trace() {
    let (kind, _, _, _, absence) = derive_stake("The reply never arrived.");
    assert_eq!(kind, StakeKind::Absence);
    assert!(absence.is_some());
    let mut mem = SelectiveMemory::new(EntityProfile::new("Ada"));
    mem.live_with(hour("The reply never arrived.", -0.4));
    assert!(mem.store.traces.values().any(|t| t.absence.is_some()));
}

