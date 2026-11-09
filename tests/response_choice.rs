use selmem::recall::choice::{
    self,
    json::{self, num, obj, text, Json},
    ChoiceMode, ChoiceOptions, Influence,
};
use selmem::{
    Config, FailurePolicy, HttpNarrator, MemoryContext, MemoryContextOptions, MemoryReply, Mood,
    Narrator, WorkingTalk,
};
use std::sync::{Arc, Mutex};
fn input() -> Json {
    json::parse(r#"{"human":"Design a group activity.","working_conversation":"","memory":{"episodes":[{"trace_id":"t1","attribution":"internal","narrative":"Shared tools helped us finish."},{"trace_id":"t2","attribution":"external","narrative":"A neighbor worked alone."}],"dispositions":[],"associations":[{"a":"t1","b":"t2","weight":0.5}]}}"#).unwrap()
}
fn generated() -> Json {
    json::parse(r#"{"source_reviews":[{"source_id":"t1","decision":"used","reason":"Sharing tools is relevant to the current activity.","orientation_ids":["O1"]},{"source_id":"t2","decision":"irrelevant","reason":"No transferable orientation is claimed from this isolated report.","orientation_ids":[]}],"orientations":[{"id":"O1","criterion":"Share responsibility","consequence":"Rotate the coordinator","limit":"Allow participants to opt out","source_ids":["t1"],"evidence":[{"source_id":"t1","quote":"Shared tools helped us finish."}]}],"candidates":[{"id":"A","kind":"direct","mechanism":"An assigned coordinator gives each person a task","answer":"Assign each person a separate task.","orientation_ids":[],"source_ids":[]},{"id":"B","kind":"oriented","mechanism":"Pairs rotate responsibility for a shared task","answer":"Rotate pairs with an optional observer role.","orientation_ids":["O1"],"source_ids":["t1"]},{"id":"C","kind":"associative","mechanism":"Alternate private preparation and a joint assembly","answer":"Prepare alone then assemble together.","orientation_ids":[],"source_ids":["t1","t2"]}]}"#).unwrap()
}
fn evaluated() -> Json {
    json::parse(r#"{"evaluations":[{"id":"A","utility":4,"feasibility":4,"originality":3,"alignment":[-1],"violations":[],"reason":"Clear assigned roles."},{"id":"B","utility":4,"feasibility":4,"originality":2,"alignment":[2],"violations":[],"reason":"Rotating pairs implement shared responsibility."},{"id":"C","utility":3,"feasibility":3,"originality":3,"alignment":[0],"violations":[],"reason":"Private preparation limits resource sharing."}]}"#).unwrap()
}
fn quality_response(payload: &Json) -> Json {
    let rows = payload
        .get("answers")
        .unwrap()
        .array()
        .unwrap()
        .iter()
        .map(|row| {
            let answer = row.get("answer").unwrap().string().unwrap();
            let (utility, feasibility, originality) = if answer.starts_with("Assign") {
                (4, 4, 3)
            } else if answer.starts_with("Rotate") {
                (4, 4, 2)
            } else {
                (3, 3, 3)
            };
            (
                row.get("id").unwrap().string().unwrap().to_string(),
                obj([
                    ("utility", num(utility)),
                    ("feasibility", num(feasibility)),
                    ("originality", num(originality)),
                    ("violations", Json::Array(vec![])),
                    ("reason", text("Task-only transport fixture.")),
                ]),
            )
        })
        .collect();
    obj([("quality", Json::Object(rows))])
}
fn grounding_response(payload: &Json) -> Json {
    let rows = payload
        .get("answers")
        .unwrap()
        .array()
        .unwrap()
        .iter()
        .map(|row| {
            let answer = row.get("answer").unwrap().string().unwrap();
            let alignment = if answer.starts_with("Rotate") {
                2
            } else if answer.starts_with("Assign") {
                -1
            } else {
                0
            };
            let evidence = if alignment == 0 {
                vec![]
            } else {
                vec![obj([
                    ("orientation_id", text("O1")),
                    ("quote", text(answer)),
                ])]
            };
            (
                row.get("id").unwrap().string().unwrap().to_string(),
                obj([
                    ("alignment", Json::Array(vec![num(alignment)])),
                    ("evidence", Json::Array(evidence)),
                    (
                        "reason",
                        text("Quoted the same anonymous standalone answer."),
                    ),
                ]),
            )
        })
        .collect();
    obj([
        (
            "orientation_reviews",
            obj([(
                "O1",
                obj([
                    ("premise", text("faithful")),
                    ("lesson", text("plausible")),
                    ("scope", text("applicable")),
                    ("limit_polarity", text("exception")),
                    ("reason", text("A bounded lesson, not a universal rule.")),
                ]),
            )]),
        ),
        (
            "activation_review",
            obj([
                ("verdict", text("adequate")),
                ("reason", text("All supplied sources were reviewed.")),
            ]),
        ),
        ("alignments", Json::Object(rows)),
    ])
}

#[test]
fn weight_changes_choice_without_more_generations() {
    let c = choice::parse_candidates(&generated().encode(), &input(), 180).unwrap();
    let e = choice::parse_evaluations(&evaluated().encode(), &c).unwrap();
    assert_eq!(choice::select(&e, Influence::None).unwrap(), 0);
    assert_eq!(choice::select(&e, Influence::Moderate).unwrap(), 1);
    assert_eq!(choice::select(&e, Influence::Strong).unwrap(), 1);
}
#[test]
fn hard_constraints_and_quality_floor_override_memory() {
    let c = choice::parse_candidates(&generated().encode(), &input(), 180).unwrap();
    let mut e = choice::parse_evaluations(&evaluated().encode(), &c).unwrap();
    e[1].violations.push("Needs unavailable equipment".into());
    assert_ne!(choice::select(&e, Influence::Strong).unwrap(), 1);
    e[1].violations.clear();
    e[1].utility = 1.0;
    assert_ne!(choice::select(&e, Influence::Strong).unwrap(), 1);
    for r in &mut e {
        r.violations.push("constraint".into());
    }
    assert!(choice::select(&e, Influence::Strong).is_err());
}
#[test]
fn unknown_sources_and_invented_associations_are_rejected() {
    let raw = generated().encode().replace("\"t1\"", "\"invented\"");
    assert!(choice::parse_candidates(&raw, &input(), 180).is_err());
    let mut i = input();
    i.get("memory").unwrap();
    if let Json::Object(ref mut root) = i {
        if let Json::Object(m) = root.get_mut("memory").unwrap() {
            m.insert("associations".into(), Json::Array(vec![]));
        }
    }
    assert!(choice::parse_candidates(&generated().encode(), &i, 180).is_err());
}
#[test]
fn duplicates_oversize_and_score_errors_are_rejected() {
    let g = generated();
    let c = choice::parse_candidates(&g.encode(), &input(), 180).unwrap();
    assert!(choice::parse_candidates(&g.encode(), &input(), 1).is_err());
    assert!(choice::parse_candidates(
        &g.encode().replace(
            "Prepare alone then assemble together.",
            "Assign each person a separate task."
        ),
        &input(),
        180
    )
    .is_err());
    assert!(choice::parse_evaluations(
        &evaluated()
            .encode()
            .replace("\"utility\":4", "\"utility\":5"),
        &c
    )
    .is_err());
    assert!(choice::parse_evaluations(
        &evaluated()
            .encode()
            .replace("\"alignment\":[2]", "\"alignment\":[]"),
        &c
    )
    .is_err());
}
#[test]
fn config_and_routing_are_explicit() {
    let o = ChoiceOptions::from_config(&Config::empty()).unwrap();
    assert!(o.active("Design a process"));
    assert!(o.active("Propose a group activity"));
    assert!(!o.active("What time is it?"));
    assert!(!o.active("The designer arrived"));
    assert!(ChoiceOptions::from_config(&Config::parse("memory_influence=invalid")).is_err());
    assert!(ChoiceOptions::from_config(&Config::parse("candidate_words=0")).is_err());
    assert_eq!(
        ChoiceOptions::from_config(&Config::parse("response_selection=direct"))
            .unwrap()
            .mode,
        ChoiceMode::Direct
    );
}
#[test]
fn production_pipeline_separates_generation_quality_and_grounding() {
    let mut stages = vec![];
    let result = choice::run(
        &input(),
        "base",
        &ChoiceOptions::default(),
        |stage, _, raw, _| {
            stages.push(stage.to_string());
            let payload = json::parse(raw)?;
            Ok(match stage {
                "candidates" => generated(),
                "quality" => quality_response(&payload),
                "grounding" => grounding_response(&payload),
                _ => panic!("unexpected stage"),
            }
            .encode())
        },
    )
    .unwrap();
    assert_eq!(stages, vec!["candidates", "quality", "grounding"]);
    assert_eq!(
        result.answer,
        "Rotate pairs with an optional observer role."
    );
    assert_eq!(result.audit.get("selected").unwrap().string().unwrap(), "B");
    assert_eq!(
        result
            .audit
            .get("weight_sweep")
            .unwrap()
            .array()
            .unwrap()
            .len(),
        3
    );
    assert!(result.audit.get("quality_audit").is_ok());
    assert!(result.audit.get("grounding_audit").is_ok());
}
#[test]
fn selection_transport_is_real_and_only_selected_answer_escapes() {
    let seen = Arc::new(Mutex::new(vec![]));
    let seen2 = seen.clone();
    let mut context = MemoryContext::default();
    // No memory is needed for the multi-candidate control. It must not invent orientations.
    let mut g = generated();
    if let Json::Object(root) = &mut g {
        root.insert("orientations".into(), Json::Array(vec![]));
        root.insert("source_reviews".into(), Json::Array(vec![]));
        if let Json::Array(rows) = root.get_mut("candidates").unwrap() {
            for (i, row) in rows.iter_mut().enumerate() {
                if let Json::Object(r) = row {
                    r.insert("source_ids".into(), Json::Array(vec![]));
                    r.insert("orientation_ids".into(), Json::Array(vec![]));
                    if i > 0 {
                        r.insert("kind".into(), text("exploratory"));
                    }
                }
            }
        }
    }
    let h = HttpNarrator::parse("http://example.invalid", "grok-4.3", None)
        .unwrap()
        .with_policy(FailurePolicy::Error)
        .with_transport(Arc::new(move |_, _, body| {
            let wire = json::parse(body).unwrap();
            let payload = json::parse(
                wire.get("messages").unwrap().array().unwrap()[1]
                    .get("content")
                    .unwrap()
                    .string()
                    .unwrap(),
            )
            .unwrap();
            let mut calls = seen2.lock().unwrap();
            let reply = if calls.is_empty() {
                g.clone()
            } else {
                quality_response(&payload)
            };
            calls.push(wire);
            Ok(obj([(
                "choices",
                Json::Array(vec![obj([
                    ("finish_reason", text("stop")),
                    ("message", obj([("content", text(reply.encode()))])),
                ])]),
            )])
            .encode())
        }));
    let mood = Mood::default();
    let talk = WorkingTalk::default();
    let opts = MemoryContextOptions::default();
    let r = MemoryReply {
        user: "Design an activity",
        memories: &[],
        axioms: &[],
        mood: &mood,
        talk: &talk,
        reading: "",
        context: &mut context,
        options: &opts,
    };
    assert_eq!(
        h.reply_with_context(&r),
        "Assign each person a separate task."
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(h.failure_log().calls, 2);
    assert!(h.last_choice_audit().unwrap().contains("weight_sweep"));
}
#[test]
fn partial_pipeline_failure_does_not_silently_issue_a_third_call() {
    let n = Arc::new(Mutex::new(0));
    let n2 = n.clone();
    let h = HttpNarrator::parse("http://example.invalid", "grok-4.3", None)
        .unwrap()
        .with_policy(FailurePolicy::Error)
        .with_transport(Arc::new(move |_, _, _| {
            *n2.lock().unwrap() += 1;
            Ok(
                "{\"choices\":[{\"finish_reason\":\"length\",\"message\":{\"content\":\"{}\"}}]}"
                    .into(),
            )
        }));
    let mood = Mood::default();
    let talk = WorkingTalk::default();
    let opts = MemoryContextOptions::default();
    let context = MemoryContext::default();
    let r = MemoryReply {
        user: "Design an activity",
        memories: &[],
        axioms: &[],
        mood: &mood,
        talk: &talk,
        reading: "",
        context: &context,
        options: &opts,
    };
    assert!(h.reply_with_context(&r).starts_with("[llm-error]"));
    assert_eq!(*n.lock().unwrap(), 1);
    assert_eq!(h.failure_log().failures, 1);
}

#[test]
fn failed_selection_is_not_recorded_as_a_conversation_turn() {
    let h = HttpNarrator::parse("http://example.invalid", "grok-4.3", None)
        .unwrap()
        .with_policy(FailurePolicy::Error)
        .with_transport(Arc::new(|_, _, _| Err("HTTP 503 local test".into())));
    let mut organ = selmem::SelectiveMemory::new(selmem::EntityProfile::new("Claire"))
        .with_narrator(Box::new(h));
    let reply = organ.speak("Design an activity");
    assert!(reply.starts_with("[llm-error]"));
    assert!(organ.talk.turns.is_empty());
    assert!(!organ.talk.render().contains("llm-error"));
    assert!(!organ.mouth_held());
}

#[test]
fn no_relevant_graph_edge_can_use_an_exploratory_candidate() {
    let mut g = generated();
    if let Json::Object(root) = &mut g {
        if let Json::Array(rows) = root.get_mut("candidates").unwrap() {
            if let Json::Object(c) = &mut rows[2] {
                c.insert("kind".into(), text("exploratory"));
                c.insert("source_ids".into(), Json::Array(vec![]));
            }
        }
    }
    assert!(choice::parse_candidates(&g.encode(), &input(), 180).is_ok());
}

#[test]
fn keyed_evaluations_preserve_scores_and_require_all_three_candidates() {
    let c = choice::parse_candidates(&generated().encode(), &input(), 180).unwrap();
    let rows = evaluated()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()
        .to_vec();
    let mut map = std::collections::BTreeMap::new();
    for mut row in rows {
        if let Json::Object(fields) = &mut row {
            let id = fields.remove("id").unwrap().string().unwrap().to_string();
            map.insert(id, row);
        }
    }
    let complete = obj([("evaluations", Json::Object(map.clone()))]);
    let e = choice::parse_evaluations(&complete.encode(), &c).unwrap();
    assert_eq!(e[1].alignment, vec![2.0]);
    assert_eq!(choice::select(&e, Influence::Moderate).unwrap(), 1);
    map.remove("C");
    assert!(
        choice::parse_evaluations(&obj([("evaluations", Json::Object(map))]).encode(), &c)
            .unwrap_err()
            .contains("received 2")
    );
}
#[test]
fn complete_legacy_arrays_can_reorder_but_never_omit_or_duplicate_ids() {
    let c = choice::parse_candidates(&generated().encode(), &input(), 180).unwrap();
    let mut rows = evaluated()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()
        .to_vec();
    rows.swap(0, 2);
    let e = choice::parse_evaluations(
        &obj([("evaluations", Json::Array(rows.clone()))]).encode(),
        &c,
    )
    .unwrap();
    assert_eq!(e[0].id, "A");
    rows.pop();
    assert!(choice::parse_evaluations(
        &obj([("evaluations", Json::Array(rows.clone()))]).encode(),
        &c
    )
    .is_err());
    rows.push(rows[0].clone());
    assert!(
        choice::parse_evaluations(&obj([("evaluations", Json::Array(rows))]).encode(), &c).is_err()
    );
}
#[test]
fn evaluation_contract_has_three_entries_even_without_orientations() {
    let mut c = choice::parse_candidates(&generated().encode(), &input(), 180).unwrap();
    c.orientations.clear();
    let i = choice::evaluation_input(&input(), &c);
    let contract = i.get("response_contract").unwrap();
    assert_eq!(
        contract.get("alignment_length").unwrap().number().unwrap(),
        0.0
    );
    let template = contract.get("evaluations_template").unwrap();
    for id in ["A", "B", "C"] {
        assert!(template
            .get(id)
            .unwrap()
            .get("alignment")
            .unwrap()
            .array()
            .unwrap()
            .is_empty());
    }
}
