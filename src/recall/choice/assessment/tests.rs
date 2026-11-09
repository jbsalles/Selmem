use super::*;
fn put(v: &mut Json, k: &str, x: Json) {
    if let Json::Object(m) = v {
        m.insert(k.into(), x);
    } else {
        panic!("object")
    }
}
fn input() -> Json {
    obj([
        ("human", text("Plan a one-hour community meeting.")),
        (
            "memory",
            obj([
                ("episodes", arr([])),
                ("dispositions", arr([])),
                ("associations", arr([])),
            ]),
        ),
    ])
}
fn candidates() -> Candidates {
    let rows = (0..3).map(|i| {
        obj([
            ("id", text(["A", "B", "C"][i])),
            ("kind", text(if i == 0 { "direct" } else { "exploratory" })),
            (
                "mechanism",
                text(["Private roles", "Public voting", "Pair feedback"][i]),
            ),
            (
                "answer",
                text(
                    [
                        "Allocate roles privately.",
                        "Vote publicly on each proposal.",
                        "Use pair feedback followed by a rota.",
                    ][i],
                ),
            ),
            ("orientation_ids", arr([])),
            ("source_ids", arr([])),
        ])
    });
    parse_candidates(
        &obj([
            ("orientations", arr([])),
            ("source_reviews", arr([])),
            ("candidates", arr(rows)),
        ])
        .encode(),
        &input(),
        180,
    )
    .unwrap()
}
fn quality(value: f64) -> Json {
    obj([(
        "quality",
        Json::Object(
            SLOTS
                .iter()
                .map(|id| {
                    (
                        id.to_string(),
                        obj([
                            ("utility", num(value)),
                            ("feasibility", num(3)),
                            ("originality", num(3)),
                            ("violations", arr([])),
                            ("reason", text("Task-only test score.")),
                        ]),
                    )
                })
                .collect(),
        ),
    )])
}
#[test]
fn quality_request_is_invariant_to_memory_labels_and_persona() {
    let c = candidates();
    let mut changed = input();
    put(&mut changed, "memory", text("secret traumatic history"));
    put(&mut changed, "persona", text("secret identity"));
    let a = quality_input(&input(), &c, 180).unwrap();
    let mut labeled = c.clone();
    labeled.candidates[0].kind = "oriented".into();
    labeled.candidates[0].mechanism = "secret preference".into();
    assert_eq!(a, quality_input(&changed, &labeled, 180).unwrap());
    let s = a.encode();
    assert!(!s.contains("secret"));
    assert!(!s.contains("orientation"));
    assert!(!s.contains("mechanism"));
    assert!(!s.contains("source_id"));
    assert_eq!(a.get("answers").unwrap().array().unwrap().len(), 3);
}
#[test]
fn anonymous_scores_map_back_to_the_correct_answer() {
    let c = candidates();
    let mut q = quality(2.0);
    let top = order(&c).iter().position(|&i| i == 2).unwrap();
    if let Json::Object(mut rows) = q.get("quality").unwrap().clone() {
        put(rows.get_mut(SLOTS[top]).unwrap(), "utility", num(4));
        put(&mut q, "quality", Json::Object(rows));
    }
    let mut stages = vec![];
    let result = assess(&input(), &c, &ChoiceOptions::default(), |stage, _, _, _| {
        stages.push(stage.to_string());
        Ok(q.encode())
    })
    .unwrap();
    assert_eq!(stages, vec!["quality"]);
    assert_eq!(result.answer, c.candidates[2].answer);
    assert_eq!(result.audit.get("selected").unwrap().string().unwrap(), "C");
}
#[test]
fn sixty_five_minute_agenda_is_rejected_despite_perfect_model_scores() {
    let mut c = candidates();
    c.candidates[0].answer="60-minute agenda: 5-min intro, 20-min priorities, 20-min vote, 15-min allocation, 5-min sign-off. Post results.".into();
    assert_eq!(schedule(&c.candidates[0].answer), Some((60, 65)));
    let result = assess(&input(), &c, &ChoiceOptions::default(), |_, _, _, _| {
        Ok(quality(4.0).encode())
    })
    .unwrap();
    assert_ne!(result.audit.get("selected").unwrap().string().unwrap(), "A");
    let rows = result
        .audit
        .get("effective_evaluation")
        .unwrap()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap();
    assert!(rows[0].get("violations").unwrap().array().unwrap()[0]
        .string()
        .unwrap()
        .contains("listed 65"));
    assert_eq!(
        schedule("60-minute agenda: 5-min intro, untimed free discussion."),
        None
    );
    assert_eq!(
        schedule("60-minute agenda: 20-min intro, 40-min work."),
        Some((60, 60))
    );
}
#[test]
fn missing_duplicate_or_memory_scoring_fields_are_rejected() {
    let mut q = quality(3.0);
    if let Json::Object(mut rows) = q.get("quality").unwrap().clone() {
        rows.remove("Q3");
        put(&mut q, "quality", Json::Object(rows));
    }
    assert!(parse_quality(&q.encode()).is_err());
    let mut q = quality(3.0);
    if let Json::Object(mut rows) = q.get("quality").unwrap().clone() {
        put(rows.get_mut("Q1").unwrap(), "alignment", arr([]));
        put(&mut q, "quality", Json::Object(rows));
    }
    assert!(parse_quality(&q.encode()).is_err());
}
#[test]
fn grounding_cannot_receive_or_overwrite_task_quality_and_empty_orientations_are_not_empty_memory()
{
    let mut input = input();
    put(
        &mut input,
        "memory",
        obj([
            (
                "episodes",
                arr([obj([
                    ("trace_id", text("t1")),
                    ("narrative", text("A past community event.")),
                ])]),
            ),
            ("dispositions", arr([])),
            ("associations", arr([])),
        ]),
    );
    let mut c = candidates();
    c.source_reviews = vec![SourceReview {
        source_id: "t1".into(),
        decision: "irrelevant".into(),
        reason: "Technical fixture with no orientation.".into(),
        orientations: vec![],
    }];
    let ground = obj([
        ("orientation_reviews", obj([])),
        (
            "activation_review",
            obj([
                ("verdict", text("missed_opportunity")),
                (
                    "reason",
                    text("Delivered episode exists despite no orientations."),
                ),
            ]),
        ),
        (
            "alignments",
            Json::Object(
                ["R1", "R2", "R3"]
                    .iter()
                    .map(|id| {
                        (
                            id.to_string(),
                            obj([
                                ("alignment", arr([])),
                                ("evidence", arr([])),
                                ("reason", text("No orientation to align with.")),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ]);
    let mut stages = vec![];
    let result = assess(&input, &c, &ChoiceOptions::default(), |stage, _, raw, _| {
        stages.push(stage.to_string());
        let p = json::parse(raw)?;
        if stage == "quality" {
            assert!(p.get("memory").is_err());
            Ok(quality(2.0).encode())
        } else {
            assert!(p.get("quality").is_err());
            assert!(p
                .get("response_contract")?
                .get("evaluations_template")
                .is_err());
            validate_assessment(stage, &ground.encode(), &p, 180)?;
            let mut bad = ground.clone();
            put(&mut bad, "quality", quality(4.0));
            assert!(validate_assessment(stage, &bad.encode(), &p, 180).is_err());
            Ok(ground.encode())
        }
    })
    .unwrap();
    assert_eq!(stages, vec!["quality", "grounding"]);
    for e in result
        .audit
        .get("evaluation")
        .unwrap()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()
    {
        assert_eq!(e.get("utility").unwrap().number().unwrap(), 2.0);
    }
    assert_eq!(
        result
            .audit
            .get("activation_review")
            .unwrap()
            .get("verdict")
            .unwrap()
            .string()
            .unwrap(),
        "missed_opportunity"
    );
}
#[test]
fn shorter_listed_agendas_allow_unallocated_time() {
    let mut c = candidates();
    c.candidates[0].answer =
        "60-minute agenda: 5-min intro, 20-min discussion. Leave the remaining time for decisions."
            .into();
    let result = assess(&input(), &c, &ChoiceOptions::default(), |_, _, _, _| {
        Ok(quality(3.0).encode())
    })
    .unwrap();
    let a = &result
        .audit
        .get("effective_evaluation")
        .unwrap()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()[0];
    assert!(a.get("violations").unwrap().array().unwrap().is_empty());
}
fn lived_fixture() -> (Json, Candidates) {
    let mut input = input();
    put(
        &mut input,
        "memory",
        obj([
            (
                "episodes",
                arr([obj([
                    ("trace_id", text("t1")),
                    (
                        "narrative",
                        text("Shared planning gave clear responsibilities and an on-time event."),
                    ),
                ])]),
            ),
            ("dispositions", arr([])),
            ("associations", arr([])),
        ]),
    );
    let mut c = candidates();
    c.candidates[0].answer = "Rotate roles and update a public checklist.".into();
    c.orientations = vec![Orientation {
        id: "O1".into(),
        criterion: "Distribute roles and track progress publicly.".into(),
        consequence: "Use rotating roles and a public checklist.".into(),
        limit: "when fewer than three can attend".into(),
        sources: vec!["t1".into()],
        evidence: vec![Evidence {
            source_id: "t1".into(),
            quote: "Shared planning gave clear responsibilities and an on-time event.".into(),
        }],
    }];
    c.source_reviews = vec![SourceReview {
        source_id: "t1".into(),
        decision: "used".into(),
        reason: "Technical source rationale.".into(),
        orientations: vec!["O1".into()],
    }];
    c.candidates[1].kind = "oriented".into();
    c.candidates[1].orientations = vec!["O1".into()];
    c.candidates[1].sources = vec!["t1".into()];
    (input, c)
}
fn anonymous_ground(c: &Candidates, payload: &Json) -> Json {
    let alignments = payload
        .get("answers")
        .unwrap()
        .array()
        .unwrap()
        .iter()
        .map(|a| {
            let positive = a.get("answer").unwrap().string().unwrap() == c.candidates[0].answer;
            (
                a.get("id").unwrap().string().unwrap().to_string(),
                obj([
                    ("alignment", arr([num(if positive { 2 } else { 0 })])),
                    (
                        "evidence",
                        if positive {
                            arr([obj([
                                ("orientation_id", text("O1")),
                                ("quote", text(&c.candidates[0].answer)),
                            ])])
                        } else {
                            arr([])
                        },
                    ),
                    (
                        "reason",
                        text("Behavioral fit, independent of reference declarations."),
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
                    ("reason", text("Technical review of the provided source.")),
                ]),
            )]),
        ),
        (
            "activation_review",
            obj([
                ("verdict", text("adequate")),
                ("reason", text("No omitted orientation in this fixture.")),
            ]),
        ),
        ("alignments", Json::Object(alignments)),
    ])
}
#[test]
fn grounding_payload_is_invariant_to_candidate_labels_references_and_source_rationales() {
    let (input, c) = lived_fixture();
    let original = grounding_input(&input, &c);
    let mut changed = c.clone();
    changed.candidates[0].id = "secret-original-label".into();
    changed.candidates[0].kind = "secret-kind".into();
    changed.candidates[0].mechanism = "secret-mechanism".into();
    changed.candidates[0].orientations = vec!["secret-reference".into()];
    changed.candidates[0].sources = vec!["secret-citation".into()];
    changed.source_reviews[0].reason = "Prefer candidate secret-original-label".into();
    assert_eq!(original, grounding_input(&input, &changed));
    assert!(!original.encode().contains("secret"));
    assert!(original.get("generation").is_err());
    assert!(original
        .get("response_contract")
        .unwrap()
        .get("candidate_ids")
        .is_err());
    for answer in original.get("answers").unwrap().array().unwrap() {
        assert_eq!(answer.object().unwrap().len(), 2);
    }
}
#[test]
fn undeclared_behavior_can_receive_alignment_but_another_answers_quote_cannot() {
    let (input, c) = lived_fixture();
    assert!(c.candidates[0].orientations.is_empty());
    let payload = grounding_input(&input, &c);
    let good = anonymous_ground(&c, &payload);
    validate_assessment("grounding", &good.encode(), &payload, 180).unwrap();
    let result = assess(&input, &c, &ChoiceOptions::default(), |stage, _, raw, _| {
        if stage == "quality" {
            Ok(quality(3.0).encode())
        } else {
            Ok(anonymous_ground(&c, &json::parse(raw)?).encode())
        }
    })
    .unwrap();
    let a = &result
        .audit
        .get("evaluation")
        .unwrap()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()[0];
    assert_eq!(
        a.get("alignment").unwrap().array().unwrap()[0]
            .number()
            .unwrap(),
        2.0
    );
    let slot = GROUND_SLOTS[ground_order(&c).iter().position(|&i| i == 0).unwrap()];
    let mut bad = good.clone();
    if let Json::Object(mut rows) = bad.get("alignments").unwrap().clone() {
        put(
            rows.get_mut(slot).unwrap(),
            "evidence",
            arr([obj([
                ("orientation_id", text("O1")),
                ("quote", text(&c.candidates[1].answer)),
            ])]),
        );
        put(&mut bad, "alignments", Json::Object(rows));
    }
    assert!(
        validate_assessment("grounding", &bad.encode(), &payload, 180)
            .unwrap_err()
            .contains("OWN anonymous answer")
    );
    let mut missing = good.clone();
    if let Json::Object(mut rows) = missing.get("alignments").unwrap().clone() {
        put(rows.get_mut(slot).unwrap(), "evidence", arr([]));
        put(&mut missing, "alignments", Json::Object(rows));
    }
    assert!(validate_assessment("grounding", &missing.encode(), &payload, 180).is_err());
}
#[test]
fn reusable_quality_audit_must_match_task_scores_mapping_and_checks() {
    let c = candidates();
    let options = ChoiceOptions::default();
    let result = assess(&input(), &c, &options, |_, _, _, _| {
        Ok(quality(3.0).encode())
    })
    .unwrap();
    let audit = result.audit.get("quality_audit").unwrap();
    validate_quality_audit(&input(), &c, &options, audit).unwrap();
    let mut changed = input();
    put(&mut changed, "human", text("A changed task."));
    assert!(validate_quality_audit(&changed, &c, &options, audit).is_err());
    for field in ["mapping", "mapped_checked_scores", "response"] {
        let mut bad = audit.clone();
        put(&mut bad, field, obj([]));
        assert!(validate_quality_audit(&input(), &c, &options, &bad).is_err());
    }
}

fn empty_axis_ground() -> Json {
    obj([
        ("orientation_reviews", obj([])),
        (
            "activation_review",
            obj([
                ("verdict", text("missed_opportunity")),
                ("reason", text("No transferable criterion examined.")),
            ]),
        ),
        (
            "alignments",
            Json::Object(
                GROUND_SLOTS
                    .iter()
                    .map(|id| {
                        (
                            id.to_string(),
                            obj([
                                ("alignment", num(0)),
                                ("evidence", arr([])),
                                ("reason", text("No orientations supplied.")),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}
#[test]
fn empty_axis_scalar_zero_keeps_quality_raw_response_and_negative_activation() {
    let mut input = input();
    put(
        &mut input,
        "memory",
        obj([
            (
                "episodes",
                arr([obj([
                    ("trace_id", text("t1")),
                    ("narrative", text("A past community event.")),
                ])]),
            ),
            ("dispositions", arr([])),
            ("associations", arr([])),
        ]),
    );
    let mut c = candidates();
    c.source_reviews = vec![SourceReview {
        source_id: "t1".into(),
        decision: "irrelevant".into(),
        reason: "Fixture abstention.".into(),
        orientations: vec![],
    }];
    let raw = empty_axis_ground();
    let payload = grounding_input(&input, &c);
    validate_assessment("grounding", &raw.encode(), &payload, 180).unwrap();
    let effective = blind_grounding(&raw.encode(), &payload).unwrap();
    for slot in GROUND_SLOTS {
        assert_eq!(
            effective
                .get("alignments")
                .unwrap()
                .get(slot)
                .unwrap()
                .get("alignment")
                .unwrap(),
            &arr([])
        );
    }
    let result = assess(&input, &c, &ChoiceOptions::default(), |stage, _, _, _| {
        Ok(if stage == "quality" {
            quality(2.0).encode()
        } else {
            raw.encode()
        })
    })
    .unwrap();
    assert_eq!(result.audit.get("grounding_audit").unwrap(), &raw);
    assert_eq!(
        result
            .audit
            .get("grounding_normalizations")
            .unwrap()
            .array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        result.audit.get("activation_review").unwrap(),
        raw.get("activation_review").unwrap()
    );
    let quality_audit = result.audit.get("quality_audit").unwrap();
    validate_quality_audit(&input, &c, &ChoiceOptions::default(), quality_audit).unwrap();
    assert_eq!(quality_audit.get("response").unwrap(), &quality(2.0));
    for e in result
        .audit
        .get("evaluation")
        .unwrap()
        .get("evaluations")
        .unwrap()
        .array()
        .unwrap()
    {
        assert!(e.get("alignment").unwrap().array().unwrap().is_empty());
        assert_eq!(e.get("utility").unwrap(), &num(2));
    }
}
#[test]
fn scalar_compatibility_is_limited_to_numeric_zero_on_an_empty_axis() {
    let payload = grounding_input(&input(), &candidates());
    for value in [num(1), num(-1), Json::Null, text("0"), arr([num(0)])] {
        let mut raw = empty_axis_ground();
        if let Json::Object(mut rows) = raw.get("alignments").unwrap().clone() {
            put(rows.get_mut("R1").unwrap(), "alignment", value);
            put(&mut raw, "alignments", Json::Object(rows));
        }
        assert!(
            validate_assessment("grounding", &raw.encode(), &payload, 180)
                .unwrap_err()
                .contains("grounding.R1.alignment")
        );
    }
    let (input, c) = lived_fixture();
    let payload = grounding_input(&input, &c);
    let mut raw = anonymous_ground(&c, &payload);
    if let Json::Object(mut rows) = raw.get("alignments").unwrap().clone() {
        put(rows.get_mut("R1").unwrap(), "alignment", num(3));
        put(&mut raw, "alignments", Json::Object(rows));
    }
    assert!(
        validate_assessment("grounding", &raw.encode(), &payload, 180)
            .unwrap_err()
            .contains("grounding.R1.alignment")
    );
    let mut raw = empty_axis_ground();
    if let Json::Object(mut rows) = raw.get("alignments").unwrap().clone() {
        put(rows.get_mut("R1").unwrap(), "evidence", Json::Null);
        put(&mut raw, "alignments", Json::Object(rows));
    }
    assert!(validate_assessment(
        "grounding",
        &raw.encode(),
        &grounding_input(&self::input(), &candidates()),
        180
    )
    .unwrap_err()
    .contains("grounding.R1.evidence"));
}

#[test]
fn scalar_scores_are_preserved_only_for_exactly_one_axis() {
    for n in [-2.0, -0.5, 0.0, 0.5, 2.0] {
        assert_eq!(scalar_alignment(&num(n), 1), Some(arr([num(n)])));
        assert_eq!(scalar_alignment(&num(n), 2), None);
    }
    for value in [
        num(-2.1),
        num(2.1),
        Json::Number(f64::NAN),
        Json::Number(f64::INFINITY),
        Json::Null,
        text("2"),
        arr([num(2)]),
    ] {
        assert_eq!(scalar_alignment(&value, 1), None);
    }
    let (input, c) = lived_fixture();
    let payload = grounding_input(&input, &c);
    let vector = anonymous_ground(&c, &payload);
    let mut scalar = vector.clone();
    if let Json::Object(fields) = &mut scalar {
        if let Some(Json::Object(rows)) = fields.get_mut("alignments") {
            for row in rows.values_mut() {
                let n = row.get("alignment").unwrap().array().unwrap()[0].clone();
                put(row, "alignment", n);
            }
        }
    }
    validate_assessment("grounding", &scalar.encode(), &payload, 180).unwrap();
    let run = |ground: &Json| {
        assess(&input, &c, &ChoiceOptions::default(), |stage, _, _, _| {
            Ok(if stage == "quality" {
                quality(3.0).encode()
            } else {
                ground.encode()
            })
        })
        .unwrap()
    };
    let a = run(&vector);
    let b = run(&scalar);
    assert_eq!(a.answer, b.answer);
    for field in [
        "quality_audit",
        "evaluation",
        "effective_evaluation",
        "selected",
        "activation_review",
    ] {
        assert_eq!(a.audit.get(field).unwrap(), b.audit.get(field).unwrap());
    }
    assert_eq!(b.audit.get("grounding_audit").unwrap(), &scalar);
    assert_eq!(
        b.audit
            .get("grounding_normalizations")
            .unwrap()
            .array()
            .unwrap()
            .len(),
        3
    );
    let mut multi = payload.clone();
    if let Json::Object(fields) = &mut multi {
        if let Some(Json::Object(contract)) = fields.get_mut("response_contract") {
            contract.insert("orientation_ids".into(), arr([text("O1"), text("O2")]));
        }
    }
    assert!(
        validate_assessment("grounding", &scalar.encode(), &multi, 180)
            .unwrap_err()
            .contains("alignment")
    );
}
#[test]
fn all_three_captured_responses_reject_the_noncontiguous_quote_after_scalar_normalization() {
    let fixture = json::parse(include_str!("single-axis-errors-023.json")).unwrap();
    let rows = fixture.get("journal").unwrap().array().unwrap();
    let payload = json::parse(
        rows[4]
            .get("request")
            .unwrap()
            .get("messages")
            .unwrap()
            .array()
            .unwrap()[1]
            .get("content")
            .unwrap()
            .string()
            .unwrap(),
    )
    .unwrap();
    for row in &rows[4..7] {
        let raw = row
            .get("response")
            .unwrap()
            .get("choices")
            .unwrap()
            .array()
            .unwrap()[0]
            .get("message")
            .unwrap()
            .get("content")
            .unwrap()
            .string()
            .unwrap();
        let error = validate_assessment("grounding", raw, &payload, 180).unwrap_err();
        assert!(error.contains("grounding.R2.evidence[O1].quote"));
        assert!(error.contains("exact contiguous"));
        let mut corrected = json::parse(raw).unwrap();
        if let Json::Object(root) = &mut corrected {
            if let Some(Json::Object(alignments)) = root.get_mut("alignments") {
                let row = alignments.get_mut("R2").unwrap();
                if let Json::Object(row) = row {
                    if let Some(Json::Array(evidence)) = row.get_mut("evidence") {
                        put(
                            &mut evidence[0],
                            "quote",
                            text("Each team elects facilitator, scribe, time-keeper, observer."),
                        );
                    }
                }
            }
        }
        validate_assessment("grounding", &corrected.encode(), &payload, 180).unwrap();
        assert_eq!(
            json::parse(raw)
                .unwrap()
                .get("alignments")
                .unwrap()
                .get("R1")
                .unwrap()
                .get("alignment")
                .unwrap(),
            &num(2)
        );
    }
}
