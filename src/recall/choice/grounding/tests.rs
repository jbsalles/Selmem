use super::*;

const SUCCESS: &str = "I planned the community event privately and assigned each task without discussion. The written responsibilities were clear and the event was completed on time.";
fn put(v: &mut Json, key: &str, value: Json) {
    if let Json::Object(m) = v {
        m.insert(key.into(), value);
    } else {
        panic!("test object");
    }
}
fn input() -> Json {
    obj([
        ("human", text("Propose a meeting workflow.")),
        (
            "memory",
            obj([
                (
                    "episodes",
                    arr([obj([
                        ("trace_id", text("t1")),
                        ("narrative", text(SUCCESS)),
                        ("attribution", text("internal")),
                    ])]),
                ),
                ("dispositions", arr([])),
                ("associations", arr([])),
            ]),
        ),
    ])
}
fn generation(oriented: bool) -> Json {
    let orientation=obj([("id",text("O1")),("criterion",text("Retain clear written responsibility after a successful event.")),("consequence",text("Name a responsible coordinator.")),("limit",text("Do not infer this is always best.")),("source_ids",arr([text("t1")])),("evidence",arr([obj([("source_id",text("t1")),("quote",text("The written responsibilities were clear and the event was completed on time."))])]))]);
    obj([
        (
            "source_reviews",
            arr([obj([
                ("source_id", text("t1")),
                (
                    "decision",
                    text(if oriented { "used" } else { "irrelevant" }),
                ),
                ("reason", text("Task-specific source judgment.")),
                (
                    "orientation_ids",
                    if oriented { arr([text("O1")]) } else { arr([]) },
                ),
            ])]),
        ),
        (
            "orientations",
            if oriented {
                arr([orientation])
            } else {
                arr([])
            },
        ),
        (
            "candidates",
            arr((0..3).map(|i| {
                obj([
                    ("id", text(["A", "B", "C"][i])),
                    (
                        "kind",
                        text(if i == 0 {
                            "direct"
                        } else if i == 1 && oriented {
                            "oriented"
                        } else {
                            "exploratory"
                        }),
                    ),
                    (
                        "mechanism",
                        text(["Named roles", "Rotating roles", "Pair review"][i]),
                    ),
                    (
                        "answer",
                        text(
                            [
                                "Use clear responsibilities.",
                                "Name an accountable coordinator.",
                                "Use pair verification.",
                            ][i],
                        ),
                    ),
                    (
                        "orientation_ids",
                        if i == 1 && oriented {
                            arr([text("O1")])
                        } else {
                            arr([])
                        },
                    ),
                    (
                        "source_ids",
                        if i == 1 && oriented {
                            arr([text("t1")])
                        } else {
                            arr([])
                        },
                    ),
                ])
            })),
        ),
    ])
}
fn candidates(oriented: bool) -> Candidates {
    parse_candidates(&generation(oriented).encode(), &input(), 180).unwrap()
}
fn judgment(c: &Candidates, verdict: &str) -> Json {
    obj([
        (
            "orientation_reviews",
            Json::Object(
                c.orientations
                    .iter()
                    .map(|o| {
                        (
                            o.id.clone(),
                            obj([
                                (
                                    "premise",
                                    text(match verdict {
                                        "supported" => "faithful",
                                        "unsupported" => "contradicted",
                                        "uncertain" => "uncertain",
                                        _ => verdict,
                                    }),
                                ),
                                ("lesson", text("plausible")),
                                ("scope", text("applicable")),
                                ("limit_polarity", text("exception")),
                                (
                                    "reason",
                                    text("Checked the delivered outcome and task transfer."),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "activation_review",
            obj([
                ("verdict", text("adequate")),
                (
                    "reason",
                    text("No other obvious transferable source was omitted."),
                ),
            ]),
        ),
        (
            "evaluations",
            Json::Object(
                c.candidates
                    .iter()
                    .map(|c| {
                        (
                            c.id.clone(),
                            obj([
                                ("utility", num(3)),
                                ("feasibility", num(3)),
                                ("originality", num(3)),
                                ("alignment", arr([])),
                                ("violations", arr([])),
                                ("reason", text("Concrete task assessment.")),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}
fn evaluation(c: &Candidates, verdict: &str) -> Json {
    let mut j = judgment(c, verdict);
    if let Json::Object(rows) = j.get("evaluations").unwrap().clone() {
        let mut rows = rows;
        for (i, id) in ["A", "B", "C"].iter().enumerate() {
            put(
                rows.get_mut(*id).unwrap(),
                "alignment",
                arr(c
                    .orientations
                    .iter()
                    .map(|_| num(if i == 1 { 2 } else { 0 }))),
            );
        }
        put(&mut j, "evaluations", Json::Object(rows));
    }
    j
}
#[test]
fn excerpts_and_source_accounting_round_trip() {
    let c = candidates(true);
    let again = parse_candidates(&c.to_json().encode(), &input(), 180).unwrap();
    assert_eq!(again.orientations[0].evidence[0].source_id, "t1");
    assert_eq!(again.source_reviews.len(), 1);
}
#[test]
fn invented_or_outcome_reversed_quote_is_rejected() {
    let mut g = generation(true);
    if let Json::Array(rows) = g.get("orientations").unwrap().clone() {
        let mut rows = rows;
        put(
            &mut rows[0],
            "evidence",
            arr([obj([
                ("source_id", text("t1")),
                (
                    "quote",
                    text("The event failed because of private planning."),
                ),
            ])]),
        );
        put(&mut g, "orientations", arr(rows));
    }
    assert!(parse_candidates(&g.encode(), &input(), 180)
        .unwrap_err()
        .contains("exact excerpt"));
}
#[test]
fn source_id_alone_is_not_evidence() {
    let mut c = candidates(true);
    c.orientations[0].evidence.clear();
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180).is_err());
}
#[test]
fn exact_quote_does_not_certify_a_reversed_interpretation() {
    let mut c = candidates(true);
    c.orientations[0].criterion = "Avoid private planning after its past failure.".into();
    // Presence validation deliberately cannot prove semantic truth.
    let c = parse_candidates(&c.to_json().encode(), &input(), 180).unwrap();
    let j = parse_judgment(&evaluation(&c, "unsupported").encode(), &c).unwrap();
    assert_eq!(select(&j.effective, Influence::Strong).unwrap(), 0);
    assert_eq!(j.raw[1].alignment, vec![2.0]);
    assert_eq!(j.effective[1].alignment, vec![0.0]);
    assert!(j.effective[1]
        .violations
        .iter()
        .any(|s| s == "unverified_orientation:O1"));
}
#[test]
fn supported_orientation_can_change_the_choice() {
    let c = candidates(true);
    let j = parse_judgment(&evaluation(&c, "supported").encode(), &c).unwrap();
    assert_eq!(select(&j.effective, Influence::None).unwrap(), 0);
    assert_eq!(select(&j.effective, Influence::Moderate).unwrap(), 1);
}
#[test]
fn uncertainty_removes_bonus_without_excluding_reliance() {
    let c = candidates(true);
    let j = parse_judgment(&evaluation(&c, "uncertain").encode(), &c).unwrap();
    assert!(j.effective[1].violations.is_empty());
    assert_eq!(j.effective[1].alignment, vec![0.0]);
    let mut e = j.effective.clone();
    e[1].utility = 4.0;
    assert_eq!(select(&e, Influence::Strong).unwrap(), 1);
}
#[test]
fn evaluator_may_not_omit_reviews_or_activation_check() {
    let c = candidates(true);
    for field in ["orientation_reviews", "activation_review"] {
        let mut j = evaluation(&c, "supported");
        if let Json::Object(m) = &mut j {
            m.remove(field);
        }
        assert!(parse_judgment(&j.encode(), &c).is_err());
    }
}
#[test]
fn unknown_review_id_or_verdict_is_rejected() {
    let c = candidates(true);
    let mut j = evaluation(&c, "approved");
    assert!(parse_judgment(&j.encode(), &c).is_err());
    put(
        &mut j,
        "orientation_reviews",
        obj([(
            "O9",
            obj([
                ("verdict", text("supported")),
                ("reason", text("Wrong ID.")),
            ]),
        )]),
    );
    assert!(parse_judgment(&j.encode(), &c).is_err());
}
#[test]
fn empty_orientations_require_explained_abstentions() {
    let mut c = candidates(false);
    assert!(parse_judgment(&evaluation(&c, "supported").encode(), &c).is_ok());
    c.source_reviews.clear();
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180).is_err());
}
#[test]
fn unused_source_cannot_be_labeled_used_or_capacity() {
    let mut c = candidates(false);
    for decision in ["used", "capacity"] {
        c.source_reviews[0].decision = decision.into();
        assert!(parse_candidates(&c.to_json().encode(), &input(), 180).is_err());
    }
}
#[test]
fn duplicate_reviews_and_missing_candidate_sources_are_rejected() {
    let mut c = candidates(true);
    c.source_reviews.push(c.source_reviews[0].clone());
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180).is_err());
    let mut c = candidates(true);
    c.candidates[1].sources.clear();
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180).is_err());
}
#[test]
fn quality_and_constraint_floors_still_apply() {
    let c = candidates(true);
    let mut j = parse_judgment(&evaluation(&c, "supported").encode(), &c).unwrap();
    j.effective[1].feasibility = 1.0;
    assert_eq!(select(&j.effective, Influence::Strong).unwrap(), 0);
    j.effective[1].feasibility = 3.0;
    j.effective[1].violations.push("Over budget".into());
    assert_eq!(select(&j.effective, Influence::Strong).unwrap(), 0);
}
#[test]
fn production_uses_three_separate_calls_and_keeps_review_audit() {
    let c = candidates(true);
    let g = c.to_json().encode();
    let mut stages = vec![];
    let r = run(
        &input(),
        "Base",
        &ChoiceOptions::default(),
        |stage, _, raw, _| {
            stages.push(stage.to_string());
            Ok(if stage == "candidates" {
                g.clone()
            } else if stage == "quality" {
                quality_fixture().encode()
            } else {
                ground_only(&evaluation(&c, "unsupported"), &json::parse(raw)?, &c).encode()
            })
        },
    )
    .unwrap();
    assert_eq!(stages, vec!["candidates", "quality", "grounding"]);
    assert_eq!(r.answer, c.candidates[0].answer);
    assert_eq!(
        r.audit
            .get("orientation_reviews")
            .unwrap()
            .get("O1")
            .unwrap()
            .get("verdict")
            .unwrap()
            .string()
            .unwrap(),
        "unsupported"
    );
    assert!(r.audit.get("effective_evaluation").is_ok());
}
#[test]
fn failed_review_never_returns_an_unverified_winner() {
    let c = candidates(true);
    let mut calls = 0;
    let result = run(&input(), "Base", &ChoiceOptions::default(), |_, _, _, _| {
        calls += 1;
        Ok(if calls == 1 {
            c.to_json().encode()
        } else if calls == 2 {
            quality_fixture().encode()
        } else {
            evaluation_json(&[]).encode()
        })
    });
    assert!(result.is_err());
    assert_eq!(calls, 3);
}
#[test]
fn recorded_empty_control_failure_reports_the_exact_correction() {
    let input = obj([(
        "memory",
        obj([
            ("episodes", arr([])),
            ("dispositions", arr([])),
            ("associations", arr([])),
        ]),
    )]);
    let raw = include_str!("empty-control-016.json");
    let error = parse_candidates(raw, &input, 180).unwrap_err();
    assert!(error.contains("candidate B"));
    assert!(error.contains("expected exploratory"));
    assert!(error.contains("Generated orientations: 0"));
    let mut fixed = json::parse(raw).unwrap();
    if let Json::Array(rows) = fixed.get("candidates").unwrap().clone() {
        let mut rows = rows;
        put(&mut rows[1], "kind", text("exploratory"));
        put(&mut fixed, "candidates", arr(rows));
    }
    assert!(parse_candidates(&fixed.encode(), &input, 180).is_ok());
}
#[test]
fn production_contract_is_specific_to_empty_and_memory_inputs() {
    let empty = obj([(
        "memory",
        obj([
            ("episodes", arr([])),
            ("dispositions", arr([])),
            ("associations", arr([])),
        ]),
    )]);
    let system = generation_system_for("Base", &empty, 180).unwrap();
    assert!(system.contains("A=direct, B=exploratory, C=exploratory"));
    assert!(!system.contains("\"id\":\"B\",\"kind\":\"oriented\""));
    let memory = generation_system_for("Base", &input(), 180).unwrap();
    assert!(memory.contains("SOURCES BUT NO ASSOCIATION EDGES"));
    assert!(memory.contains("only if you generate at least one supported orientation"));
}
#[test]
fn all_three_recorded_metadata_failures_now_validate_without_editing() {
    let f = json::parse(include_str!("length-errors-017.json")).unwrap();
    let messages = f
        .get("request")
        .unwrap()
        .get("messages")
        .unwrap()
        .array()
        .unwrap();
    let input = json::parse(messages[1].get("content").unwrap().string().unwrap()).unwrap();
    for (i, response) in f
        .get("responses")
        .unwrap()
        .array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let raw = response.get("choices").unwrap().array().unwrap()[0]
            .get("message")
            .unwrap()
            .get("content")
            .unwrap()
            .string()
            .unwrap();
        let c = parse_candidates(raw, &input, 180).unwrap();
        assert_eq!(
            c.candidates[0].mechanism.split_whitespace().count(),
            [62, 73, 73][i]
        );
        assert!(c
            .candidates
            .iter()
            .all(|c| c.answer.split_whitespace().count() <= 180));
    }
}
#[test]
fn length_errors_identify_candidate_field_count_and_limit() {
    let mut c = candidates(false);
    c.candidates[0].mechanism = "word ".repeat(181);
    let e = parse_candidates(&c.to_json().encode(), &input(), 180).unwrap_err();
    assert!(e.contains("A.mechanism"));
    assert!(e.contains("1..=180"));
    assert!(e.contains("received 181"));
    c.candidates[0].mechanism = "  ".into();
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180)
        .unwrap_err()
        .contains("received 0"));
    c.candidates[0].mechanism = "A complete mechanism.".into();
    c.candidates[0].answer = "word ".repeat(181);
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180)
        .unwrap_err()
        .contains("A.answer"));
}
#[test]
fn other_text_limits_and_small_answer_budgets_are_preserved() {
    let mut c = candidates(true);
    c.orientations[0].criterion = "word ".repeat(41);
    assert!(parse_candidates(&c.to_json().encode(), &input(), 180)
        .unwrap_err()
        .contains("O1.criterion"));
    let mut c = candidates(false);
    c.candidates[0].mechanism = "word ".repeat(60);
    assert!(parse_candidates(&c.to_json().encode(), &input(), 32).is_ok());
}
#[test]
fn captured_duplicate_workflows_remain_rejected_with_actionable_pair_feedback() {
    let f = json::parse(include_str!("duplicate-errors-018.json")).unwrap();
    let messages = f
        .get("request")
        .unwrap()
        .get("messages")
        .unwrap()
        .array()
        .unwrap();
    let input = json::parse(messages[1].get("content").unwrap().string().unwrap()).unwrap();
    for response in f.get("responses").unwrap().array().unwrap() {
        let raw = response.get("choices").unwrap().array().unwrap()[0]
            .get("message")
            .unwrap()
            .get("content")
            .unwrap()
            .string()
            .unwrap();
        let error = parse_candidates(raw, &input, 180).unwrap_err();
        assert!(error.contains("candidate B duplicates candidate A"));
        assert!(error.contains("rewrite BOTH"));
        assert!(error.contains("not just the title"));
    }
}
#[test]
fn tentative_lessons_and_scope_uncertainty_keep_quality_eligibility() {
    let c = candidates(true);
    for (field, value) in [
        ("lesson", "unsupported"),
        ("scope", "inapplicable"),
        ("scope", "uncertain"),
        ("limit_polarity", "unclear"),
    ] {
        let mut raw = evaluation(&c, "supported");
        if let Json::Object(mut rows) = raw.get("orientation_reviews").unwrap().clone() {
            put(rows.get_mut("O1").unwrap(), field, text(value));
            put(&mut raw, "orientation_reviews", Json::Object(rows));
        }
        let j = parse_judgment(&raw.encode(), &c).unwrap();
        assert_eq!(j.raw[1].alignment, vec![2.0]);
        assert_eq!(j.effective[1].alignment, vec![0.0]);
        assert!(j.effective[1].violations.is_empty());
        let mut scores = j.effective.clone();
        scores[1].utility = 4.0;
        assert_eq!(select(&scores, Influence::Moderate).unwrap(), 1);
    }
}
#[test]
fn explicit_polarity_is_required_and_both_clear_polarities_can_apply() {
    let c = candidates(true);
    for polarity in ["exception", "requirement"] {
        let mut raw = evaluation(&c, "supported");
        if let Json::Object(mut rows) = raw.get("orientation_reviews").unwrap().clone() {
            put(
                rows.get_mut("O1").unwrap(),
                "limit_polarity",
                text(polarity),
            );
            put(&mut raw, "orientation_reviews", Json::Object(rows));
        }
        let j = parse_judgment(&raw.encode(), &c).unwrap();
        assert_eq!(select(&j.effective, Influence::Moderate).unwrap(), 1);
    }
    assert!(EVALUATE.contains("eight attendees"));
    assert!(EVALUATE.contains("preventive response"));
    assert!(EVALUATE.contains("standalone deliverable"));
}
fn quality_fixture() -> Json {
    obj([(
        "quality",
        Json::Object(
            ["Q1", "Q2", "Q3"]
                .iter()
                .map(|id| {
                    (
                        id.to_string(),
                        obj([
                            ("utility", num(3)),
                            ("feasibility", num(3)),
                            ("originality", num(3)),
                            ("violations", arr([])),
                            ("reason", text("Task-only technical fixture.")),
                        ]),
                    )
                })
                .collect(),
        ),
    )])
}
fn ground_only(j: &Json, payload: &Json, c: &Candidates) -> Json {
    obj([
        (
            "orientation_reviews",
            j.get("orientation_reviews").unwrap().clone(),
        ),
        (
            "activation_review",
            j.get("activation_review").unwrap().clone(),
        ),
        (
            "alignments",
            Json::Object(
                payload
                    .get("answers")
                    .unwrap()
                    .array()
                    .unwrap()
                    .iter()
                    .map(|answer| {
                        let original = c
                            .candidates
                            .iter()
                            .find(|candidate| {
                                candidate.answer == answer.get("answer").unwrap().string().unwrap()
                            })
                            .unwrap();
                        let e = j.get("evaluations").unwrap().get(&original.id).unwrap();
                        let alignment = e.get("alignment").unwrap();
                        let evidence = c
                            .orientations
                            .iter()
                            .zip(alignment.array().unwrap())
                            .filter(|(_, score)| score.number().unwrap() != 0.0)
                            .map(|(o, _)| {
                                obj([
                                    ("orientation_id", text(&o.id)),
                                    ("quote", text(&original.answer)),
                                ])
                            });
                        (
                            answer.get("id").unwrap().string().unwrap().into(),
                            obj([
                                ("alignment", alignment.clone()),
                                ("evidence", arr(evidence)),
                                ("reason", text("Anonymous grounding fixture.")),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}
