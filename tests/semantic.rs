use selmem::core::model::{Bearer, StakeKind};
use selmem::{
    EncodeInput, EntityProfile, EventSemantics, LexicalInterpreter, SelectiveMemory,
    SemanticInterpreter,
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
