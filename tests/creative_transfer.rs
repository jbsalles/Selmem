use selmem::{Attribution, EncodeInput, EntityProfile, LexicalInterpreter, RecallBias,
    RecallWrite, SelectiveMemory, SemanticInterpreter};

struct Annotated;
impl SemanticInterpreter for Annotated {
    fn interpret_event(&self, event: &str) -> Result<selmem::EventSemantics, String> {
        let mut s = LexicalInterpreter.interpret_event(event)?;
        s.attribution = Attribution::Internal;
        Ok(s)
    }
}

#[test]
fn explicit_external_source_survives_semantic_annotation() {
    let mut memory = SelectiveMemory::new(EntityProfile::new("observer"))
        .with_semantic_interpreter(Box::new(Annotated));
    let mut input = EncodeInput::new("Jon said: I teach dance in a studio.");
    input.attribution = Attribution::External;
    input.permanence = 1.0;
    let id = memory.live_with(input).trace_id.unwrap();
    assert_eq!(memory.store.traces[&id].attribution, Attribution::External);
}

#[test]
fn transfer_is_task_sensitive_and_does_not_turn_sources_into_factual_answers() {
    let mut profile = EntityProfile::new("observer");
    profile.max_recall = 1;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    let mut ids = Vec::new();
    for event in [
        "Conversation on 1 January 2023: Jon said: Neighbors repair broken tools in a workshop.",
        "Conversation on 1 January 2023: Gina said: Adults learn practical skills in a library.",
        "Conversation on 1 January 2023: Jon said: I am so excited and going for my dreams!",
    ] {
        let mut input = EncodeInput::new(event);
        input.attribution = Attribution::External;
        input.permanence = 1.0; // Engineered regression fixtures, not experimental exposure.
        ids.push(memory.live_with(input).trace_id.unwrap());
    }
    for (query, expected) in [("repair broken tools", &ids[0]), ("learn practical skills", &ids[1])] {
        let before: Vec<_> = memory.store.traces.values().map(|t| (t.id.clone(), t.gist.clone(), t.rehearsals)).collect();
        let (_, dump) = memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Transfer, &[]);
        assert_eq!(&dump.selected, &vec![expected.clone()]);
        let after: Vec<_> = memory.store.traces.values().map(|t| (t.id.clone(), t.gist.clone(), t.rehearsals)).collect();
        assert_eq!(before, after);
    }
    let (_, factual) = memory.remember_with("What did Gina repair?", RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(!factual.selected.contains(&ids[0]), "selected={:?}, traces={:?}", factual.selected, memory.store.traces.values().map(|t| (&t.core, t.attribution)).collect::<Vec<_>>());
    let (_, unrelated) = memory.remember_with("astronomy telescope galaxies", RecallWrite::ReadOnly, RecallBias::Transfer, &[]);
    assert!(unrelated.selected.is_empty());
}

#[test]
fn dates_and_speaker_wrappers_do_not_manufacture_novelty() {
    use selmem::encode::scoring::novelty;
    let old = vec!["Conversation on 1 January 2023: Jon said: Neighbors repair tools.".into()];
    assert_eq!(novelty("Conversation on 2 February 2024: Gina said: Neighbors repair tools.", &old), 0.0);
    assert_eq!(novelty("Conversation on 1 January 2023: Jon said: Astronomers observe galaxies.", &old), 1.0);
}

#[test]
fn hash_fallback_novelty_is_lexical_not_dense_collision_similarity() {
    use selmem::{HashEmbedder, MemoryStore};
    use selmem::encode::{gate::encode, scoring::{encode_score, novelty}};
    let profile = EntityProfile::new("gate");
    let mut store = MemoryStore::new();
    let mut first = EncodeInput::new("Conversation on 1 January 2023: Jon said: Neighbors repair tools.");
    first.permanence = 1.0;
    assert!(encode(&mut store, &profile, first, &HashEmbedder).kept);
    let next = "Conversation on 1 January 2023: Jon said: Astronomers observe galaxies.";
    let expected_novelty = novelty(next, &store.traces.values().map(|t| t.gist.clone()).collect::<Vec<_>>());
    let mut input = EncodeInput::new(next);
    input.permanence = 1.0;
    let expected = encode_score(&profile, input.arousal, expected_novelty,
        input.self_relevance, input.utility, input.goal_align, 1.0 - expected_novelty);
    let result = encode(&mut store, &profile, input, &HashEmbedder);
    assert!((result.score - expected).abs() < 1e-6);
}

#[test]
fn transfer_prefers_complementary_sources_over_an_identical_repeat() {
    let mut profile = EntityProfile::new("observer");
    profile.max_recall = 2;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    let mut ids = Vec::new();
    for event in ["Neighbors repair broken tools together.",
        "Neighbors repair broken tools together.",
        "Residents repair broken tools using shared parts."] {
        let mut input = EncodeInput::new(event);
        input.permanence = 1.0;
        ids.push(memory.live_with(input).trace_id.unwrap());
    }
    let (_, dump) = memory.remember_with("repair broken tools", RecallWrite::ReadOnly, RecallBias::Transfer, &[]);
    assert_eq!(dump.selected.len(), 2);
    assert!(dump.selected.contains(&ids[2]));
    assert_eq!(dump.selected.iter().filter(|id| **id == ids[0] || **id == ids[1]).count(), 1);
}
