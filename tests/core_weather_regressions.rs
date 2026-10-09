use selmem::{EncodeInput, EntityProfile, SelectiveMemory};

const EVENT: &str = "In front of Alice, Bob and the rest of the team the file is cancelled and given to someone else. They say your effort did not enter the decision. You are not allowed to speak.";

#[test]
fn short_event_keeps_occurrence_and_negated_consequence() {
    let core = selmem::encode::core::extractive_core(EVENT);
    assert!(core.split_whitespace().count() <= 64);
    assert!(core.contains("file is cancelled"));
    assert!(core.contains("not allowed to speak"));
    assert!(selmem::accept_core(&core, EVENT).is_some());
    assert!(selmem::accept_core("You are allowed to speak.", EVENT).is_none());
}

#[test]
fn weather_does_not_keep_only_a_comma_separated_subject() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("test"));
    let mut input = EncodeInput::new(EVENT);
    input.valence = -0.78;
    input.arousal = 0.82;
    input.disgust = 0.58;
    input.self_relevance = 0.92;
    input.permanence = 0.86;
    let id = mem.live_with(input).trace_id.unwrap();
    assert!(mem.pin(&id));
    let core = mem.store.traces[&id].core.clone();
    mem.advance_hours(24.0 * 365.0);
    // Exercise the same weather path that previously produced "In front of Alice.".
    let profile = mem.profile.clone();
    let trace = mem.store.traces.get_mut(&id).unwrap();
    let _clock = selmem::core::model::ClockGuard::push(mem.clock.clone());
    assert!(selmem::dream::drift::weather(trace, &profile, false).is_some());
    assert_eq!(trace.core, core);
    assert!(trace.gist.contains("cancelled"));
    assert!(trace.gist.contains("not allowed"));
    assert_ne!(trace.gist, "In front of Alice.");
}

#[test]
fn weather_keeps_extra_detail_when_the_remaining_gist_contains_the_core() {
    let event = "The client left because of the copier on 3 January. That date is why the file was late. You keep 3 January.";
    let mut mem = SelectiveMemory::new(EntityProfile::tender("test"));
    let mut input = EncodeInput::new(event);
    input.permanence = 0.95;
    let id = mem.live_with(input).trace_id.unwrap();
    mem.advance_hours(24.0 * 365.0);
    let profile = mem.profile.clone();
    let trace = mem.store.traces.get_mut(&id).unwrap();
    trace.core = "That date is why the file was late.".into();
    assert!(selmem::accept_core(&trace.core, event).is_some());
    let _clock = selmem::core::model::ClockGuard::push(mem.clock.clone());
    assert!(selmem::dream::drift::weather(trace, &profile, false).is_some());
    assert!(trace.gist.contains("3 January"));
    assert!(trace.gist.contains(&trace.core));
}

#[test]
fn precise_cues_can_evoke_an_old_pinned_episode_without_reviving_unrelated_ones() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("test")).detach_clock();
    let mut input = EncodeInput::new(EVENT);
    input.permanence = 0.95;
    input.arousal = 0.82;
    input.self_relevance = 0.92;
    let id = mem.live_with(input).trace_id.unwrap();
    assert!(mem.pin(&id));
    mem.advance_hours(24.0 * 365.0);
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.fidelity = 0.25;
        t.gist = t.core.clone();
        t.last_recalled_at = None;
    }
    let (hits, _) = mem.remember_with(
        "cancelled file team not allowed to speak",
        selmem::RecallWrite::ReadOnly,
        selmem::RecallBias::Observed,
        &[],
    );
    assert!(hits.iter().any(|h| h.trace_id == id));
    let (hits, _) = mem.remember_with(
        "What happened to the budget?",
        selmem::RecallWrite::ReadOnly,
        selmem::RecallBias::Observed,
        &[],
    );
    assert!(hits.is_empty());
}
