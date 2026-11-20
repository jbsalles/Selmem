use selmem::core::model::{Bearer, StakeKind};
use selmem::{
    EncodeInput, EntityProfile, EventSemantics, LexicalInterpreter, RecallBias, RecallWrite,
    SelectiveMemory, SemanticInterpreter,
};

struct Annotated;
impl SemanticInterpreter for Annotated {
    fn interpret_event(&self, event: &str) -> Result<EventSemantics, String> {
        let mut s = LexicalInterpreter.interpret_event("a neutral hour")?;
        s.core.entities = vec!["Alice".into()];
        s.core.actions = vec!["refuser".into()];
        s.stake_kind = StakeKind::Limit;
        s.bearer = Bearer::Other;
        s.valence = -0.8;
        s.arousal = 0.8;
        s.core.claim = event.into();
        Ok(s)
    }
}

#[test]
fn observed_excerpt_keeps_action_context_and_result_after_sleep_and_reload() {
    let mut profile = EntityProfile::new("workshop");
    profile.encode_threshold = 0.0;
    profile.merge_similarity = 0.0;
    let mut organ = SelectiveMemory::new(profile.clone()).detach_clock();
    let intake = |organ: &mut SelectiveMemory, id: &str, count| {
        let excerpt = format!(
            "I cut a blank band across the strokes. The print hung under one lamp. Jo saw {count} of 6 visitors point to the turn first."
        );
        let raw = format!(
            "I carried the registered relief proof from the drying shelf to the bench before the session. {excerpt} I cleared the bench."
        );
        let mut input = EncodeInput::observed(&raw, id);
        input.schema = Some("print trials".into());
        let decision = organ.live_with_observed_excerpt(input, &excerpt).unwrap();
        assert_eq!(organ.store.archives[&decision.archive_id].verbatim, raw);
        (decision.trace_id.unwrap(), excerpt)
    };
    let first = intake(&mut organ, "trial-a", 5);
    let second = intake(&mut organ, "trial-b", 1);
    for _ in 0..4 {
        organ.advance_hours(24.0);
        organ.sleep();
    }
    let path = std::env::temp_dir().join(format!("selmem-excerpt-{}.snapshot", std::process::id()));
    organ.path = Some(path.clone());
    organ.save().unwrap();
    let mut organ = SelectiveMemory::open(&path, profile).unwrap();
    std::fs::remove_file(path).unwrap();
    for (id, excerpt) in [first, second] {
        let stored = &organ.store.traces[&id];
        assert_eq!(stored.core, excerpt);
        assert_eq!(stored.reality.claim, excerpt);
        assert_eq!(stored.semantic.claim, excerpt);
        let (recalled, _) = organ.remember_with(
            "print trials",
            RecallWrite::ReadOnly,
            RecallBias::ForceMarked,
            std::slice::from_ref(&id),
        );
        assert_eq!(recalled[0].trace_id, id);
        assert_eq!(recalled[0].narrative, excerpt);
    }
}

#[test]
fn observed_excerpt_rejects_changed_counts_partial_sentences_and_duplicate_ids() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("workshop"));
    let raw = "I did not cut a blank band. Jo saw five visitors point to the turn first.";
    for excerpt in [
        "cut a blank band.",
        "I did not cut a blank band. Jo saw one visitor point to the turn first.",
        "I did not cut a blank band",
    ] {
        let rejected = organ.live_with_observed_excerpt(EncodeInput::observed(raw, "a"), excerpt);
        assert!(rejected.is_err());
    }
    assert!(organ.store.traces.is_empty());
    assert!(organ.store.archives.is_empty());
    assert!(organ
        .live_with_observed_excerpt(EncodeInput::new(raw), raw)
        .is_err());
    let mut input = EncodeInput::observed(raw, "a");
    input.permanence = 1.0;
    assert!(organ.live_with_observed_excerpt(input, raw).unwrap().kept);
    assert!(organ
        .live_with_observed_excerpt(EncodeInput::observed(raw, "a"), raw)
        .is_err());
    assert_eq!(organ.store.traces.len(), 1);
    assert_eq!(organ.store.archives.len(), 1);
}

#[test]
fn semantic_annotations_override_english_keywords() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("semantic"))
        .with_semantic_interpreter(Box::new(Annotated));
    let mut input = EncodeInput::new("Alice renouvelle sa promesse.");
    input.permanence = 1.0;
    let d = organ.live_with(input);
    let trace = &organ.store.traces[d.trace_id.as_ref().unwrap()];
    assert_eq!(trace.stake_kind, StakeKind::Limit);
    assert_eq!(trace.bearer, Bearer::Other);
    assert_eq!(trace.semantic.actions, vec!["refuser"]);
    assert_eq!(trace.semantic.entities, vec!["Alice"]);
}

#[test]
fn invalid_semantics_do_not_write_archive_trace_or_talk() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("invalid"));
    let mut s = Annotated.interpret_event("event").unwrap();
    s.arousal = f32::NAN;
    let mut input = EncodeInput::new("event");
    input.semantics = Some(s);
    let before = format!("{:?}", organ.talk);
    let d = organ.live_with(input);
    assert!(!d.kept);
    assert!(organ.store.traces.is_empty());
    assert_eq!(before, format!("{:?}", organ.talk));
}

#[test]
fn malformed_llm_response_is_rejected() {
    use selmem::encode::semantic::parse_semantics;
    assert!(parse_semantics("valence=NaN", "event").is_err());
    assert!(parse_semantics("valence=0\nvalence=1", "event").is_err());
    let raw = "event_type=limit\nbearer=other\nloss=access\nagency=external\nabsence=none\nvalence=-0.7\narousal=0.5\ndisgust=0\nself_relevance=0.8\ngoal_relevance=0.6\nconfidence=0.7\nschema=access\nstake_mark=refus\nentities=Alice\nactions=refuser";
    let s = parse_semantics(raw, "Alice refuse l'accès.").unwrap();
    assert_eq!(s.stake_kind, StakeKind::Limit);
    assert_eq!(s.core.actions, vec!["refuser"]);
    assert!(parse_semantics(
        &raw.replace("event_type=limit", "event_type=bogus"),
        "event"
    )
    .is_err());
    assert!(parse_semantics(&raw.replace("arousal=0.5", "arousal=NaN"), "event").is_err());
}

#[test]
fn annotations_survive_snapshot_roundtrip() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("roundtrip"))
        .with_semantic_interpreter(Box::new(Annotated));
    let mut input = EncodeInput::new("Alice refuse.");
    input.permanence = 1.0;
    let d = organ.live_with(input);
    let path = std::env::current_dir()
        .unwrap()
        .join(format!("semantic-{}.snapshot", std::process::id()));
    organ.path = Some(path.clone());
    organ.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::new("roundtrip")).unwrap();
    std::fs::remove_file(path).unwrap();
    let trace = &loaded.store.traces[d.trace_id.as_ref().unwrap()];
    assert_eq!(trace.stake_kind, StakeKind::Limit);
    assert_eq!(trace.semantic.actions, vec!["refuser"]);
}

#[test]
fn multipart_uses_selected_backend_for_each_slice() {
    use selmem::encode::gate::encode_with_interpreter;
    use selmem::{HashEmbedder, MemoryStore};
    let event = (0..160)
        .map(|n| format!("Alice examine le dossier numéro {n}. "))
        .collect::<String>();
    let parts = selmem::split_event(&event, None);
    assert!(parts.len() > 1);
    let mut store = MemoryStore::new();
    let mut input = EncodeInput::new(&event);
    input.permanence = 1.0;
    let d = encode_with_interpreter(
        &mut store,
        &EntityProfile::new("parts"),
        input,
        &HashEmbedder,
        None,
        Some(&Annotated),
    );
    assert_eq!(d.kept_n, parts.len());
    for trace in store.traces.values() {
        assert_eq!(trace.stake_kind, StakeKind::Limit);
        assert_eq!(trace.semantic.actions, vec!["refuser"]);
    }
}

struct Broken;
impl SemanticInterpreter for Broken {
    fn interpret_event(&self, _: &str) -> Result<EventSemantics, String> {
        Err("backend unavailable".into())
    }
}

#[test]
fn backend_failure_is_not_silently_replaced_by_lexical_rules() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("broken"))
        .with_semantic_interpreter(Box::new(Broken));
    let before = format!("{:?}", organ.talk);
    let d = organ.live_with(EncodeInput::new("She made a promise."));
    assert!(!d.kept);
    assert!(d.reason.contains("backend unavailable"));
    assert!(organ.store.traces.is_empty());
    assert!(organ.store.archives.is_empty());
    assert_eq!(before, format!("{:?}", organ.talk));
}

#[test]
fn equivalent_features_have_equal_gate_score_across_languages() {
    let run = |event: &str| {
        let mut organ = SelectiveMemory::new(EntityProfile::new("invariant"))
            .with_semantic_interpreter(Box::new(Annotated));
        let mut input = EncodeInput::new(event);
        input.permanence = 1.0;
        let d = organ.live_with(input);
        let t = &organ.store.traces[d.trace_id.as_ref().unwrap()];
        (d.score, t.anchor, t.stake_kind)
    };
    assert_eq!(run("Alice refuses access."), run("Alice refuse l'accès."));
}

struct FailsOnSlice;
impl SemanticInterpreter for FailsOnSlice {
    fn interpret_event(&self, event: &str) -> Result<EventSemantics, String> {
        if event.split_whitespace().count() <= 80 {
            Err("slice failed".into())
        } else {
            Annotated.interpret_event(event)
        }
    }
}

#[test]
fn slice_failure_happens_before_any_live_state_write() {
    let mut organ = SelectiveMemory::new(EntityProfile::new("slices"))
        .with_semantic_interpreter(Box::new(FailsOnSlice));
    let event = (0..160)
        .map(|n| format!("Alice examine le dossier numéro {n}. "))
        .collect::<String>();
    let before = format!("{:?}", organ.talk);
    let d = organ.live_with(EncodeInput::new(&event));
    assert!(!d.kept);
    assert!(d.reason.contains("slice failed"));
    assert!(organ.store.traces.is_empty());
    assert!(organ.store.archives.is_empty());
    assert_eq!(before, format!("{:?}", organ.talk));
}
