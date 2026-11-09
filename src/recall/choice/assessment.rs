//! Task-only quality scoring, separately reviewed memory alignment.
use super::*;
pub const QUALITY: &str = include_str!("../../../data/response-quality-blind.txt");
pub const GROUNDING: &str = include_str!("../../../data/response-grounding-blind.txt");
const SLOTS: [&str; 3] = ["Q1", "Q2", "Q3"];
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];
// Reproducible counterbalancing independent of memory, labels and declared kinds.
// This is not a secret random assignment or proof of position-bias removal.
fn order(c: &Candidates) -> [usize; 3] {
    let mut h = 14695981039346656037u64;
    for candidate in &c.candidates {
        for b in candidate.answer.bytes().chain([0]) {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    ORDERS[(h % 6) as usize]
}
fn schedule(answer: &str) -> Option<(u32, u32)> {
    // A deliberately narrow, complete grammar: N-minute agenda: N-min label, ... .
    // Other prose and range-based schedules are left to the fallible task reviewer.
    let (declared, rest) = answer.split_once("-minute agenda:")?;
    let declared = declared.trim().parse::<u32>().ok()?;
    let first = rest.split('.').next()?.trim();
    let mut total = 0u32;
    for part in first.split(',') {
        let (minutes, label) = part.trim().split_once("-min ")?;
        if label.trim().is_empty() {
            return None;
        }
        total = total.checked_add(minutes.parse::<u32>().ok()?)?;
    }
    Some((declared, total))
}
fn facts(answer: &str, max_words: usize) -> Json {
    obj([
        ("word_count", num(answer.split_whitespace().count() as u32)),
        ("word_limit", num(max_words as u32)),
        (
            "recognized_schedule",
            schedule(answer)
                .map(|(declared, total)| {
                    obj([
                        ("declared_minutes", num(declared)),
                        ("listed_minutes", num(total)),
                    ])
                })
                .unwrap_or(Json::Null),
        ),
    ])
}
pub fn quality_input(input: &Json, c: &Candidates, max_words: usize) -> Result<Json, String> {
    let rows = order(c).into_iter().enumerate().map(|(slot, index)| {
        obj([
            ("id", text(SLOTS[slot])),
            ("answer", text(&c.candidates[index].answer)),
            (
                "checked_facts",
                facts(&c.candidates[index].answer, max_words),
            ),
        ])
    });
    Ok(obj([
        ("task", input.get("human")?.clone()),
        ("answers", arr(rows)),
        (
            "response_contract",
            obj([(
                "quality",
                Json::Object(
                    SLOTS
                        .iter()
                        .map(|id| {
                            (
                                id.to_string(),
                                obj([
                                    ("utility", Json::Null),
                                    ("feasibility", Json::Null),
                                    ("originality", Json::Null),
                                    ("violations", arr([])),
                                    ("reason", text("")),
                                ]),
                            )
                        })
                        .collect(),
                ),
            )]),
        ),
    ]))
}
fn parse_quality(raw: &str) -> Result<Vec<Evaluation>, String> {
    require(raw.len() <= 25_000, "quality response too large")?;
    let root = json::parse(raw)?;
    require(
        root.object()?.len() == 1,
        "quality response must contain only quality",
    )?;
    let rows = root.get("quality")?.object()?;
    require(rows.len() == 3, "quality requires exactly Q1, Q2, Q3")?;
    SLOTS
        .iter()
        .map(|id| {
            let row = rows.get(*id).ok_or("missing quality slot")?;
            let fields = row.object()?;
            require(
                fields.len() == 5
                    && [
                        "utility",
                        "feasibility",
                        "originality",
                        "violations",
                        "reason",
                    ]
                    .iter()
                    .all(|k| fields.contains_key(*k)),
                "quality fields must exclude memory alignment and labels",
            )?;
            let violations = strings(row, "violations")?;
            require(
                violations.len() <= 12
                    && violations
                        .iter()
                        .all(|s| !s.trim().is_empty() && s.len() <= 500),
                "invalid task violations",
            )?;
            Ok(Evaluation {
                id: id.to_string(),
                utility: score(row, "utility", 0.0, 4.0)?,
                feasibility: score(row, "feasibility", 0.0, 4.0)?,
                originality: score(row, "originality", 0.0, 4.0)?,
                alignment: vec![],
                violations,
                reason: bounded(row, "reason", 100)?,
            })
        })
        .collect()
}
const GROUND_SLOTS: [&str; 3] = ["R1", "R2", "R3"];
fn ground_order(c: &Candidates) -> [usize; 3] {
    let mut ids = order(c);
    ids.rotate_left(1);
    ids
}
fn grounding_input(input: &Json, c: &Candidates) -> Json {
    let generation = c.to_json();
    let template = evaluation_input(input, c);
    let contract = template.get("response_contract").expect("local contract");
    obj([
        ("task", input.get("human").expect("validated task").clone()),
        (
            "memory",
            input.get("memory").expect("validated packet").clone(),
        ),
        (
            "orientations",
            generation
                .get("orientations")
                .expect("local orientations")
                .clone(),
        ),
        // Omit free-text source-review reasons, which can name candidate labels.
        (
            "source_reviews",
            arr(c.source_reviews.iter().map(|r| {
                obj([
                    ("source_id", text(&r.source_id)),
                    ("decision", text(&r.decision)),
                    ("orientation_ids", arr(r.orientations.iter().map(text))),
                ])
            })),
        ),
        (
            "answers",
            arr(ground_order(c)
                .into_iter()
                .enumerate()
                .map(|(slot, index)| {
                    obj([
                        ("id", text(GROUND_SLOTS[slot])),
                        ("answer", text(&c.candidates[index].answer)),
                    ])
                })),
        ),
        (
            "response_contract",
            obj([
                (
                    "orientation_ids",
                    contract.get("orientation_ids").unwrap().clone(),
                ),
                ("alignment_length", num(c.orientations.len() as u32)),
                (
                    "orientation_reviews_template",
                    contract
                        .get("orientation_reviews_template")
                        .unwrap()
                        .clone(),
                ),
                (
                    "activation_review_template",
                    contract.get("activation_review_template").unwrap().clone(),
                ),
                (
                    "alignments_template",
                    Json::Object(
                        GROUND_SLOTS
                            .iter()
                            .map(|id| {
                                (
                                    id.to_string(),
                                    obj([
                                        (
                                            "alignment",
                                            arr(c.orientations.iter().map(|_| Json::Null)),
                                        ),
                                        ("evidence", arr([])),
                                        ("reason", text("")),
                                    ]),
                                )
                            })
                            .collect(),
                    ),
                ),
            ]),
        ),
    ])
}
fn normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn scalar_alignment(value: &Json, axes: usize) -> Option<Json> {
    match value {
        Json::Number(n) if n.is_finite() && axes == 0 && *n == 0.0 => Some(arr([])),
        Json::Number(n) if n.is_finite() && axes == 1 && (-2.0..=2.0).contains(n) => {
            Some(arr([num(*n)]))
        }
        _ => None,
    }
}
fn blind_grounding(raw: &str, payload: &Json) -> Result<Json, String> {
    require(raw.len() <= 50_000, "grounding response too large")?;
    let mut root = json::parse(raw)?;
    let fields = root.object()?;
    require(
        fields.len() == 3
            && ["orientation_reviews", "activation_review", "alignments"]
                .iter()
                .all(|k| fields.contains_key(*k)),
        "grounding requires only orientation_reviews, activation_review and alignments",
    )?;
    let ids = payload
        .get("response_contract")?
        .get("orientation_ids")?
        .array()?
        .iter()
        .map(|v| v.string().map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;
    // Normalize only unambiguous scalar spellings in this parsed copy. Preserve
    // raw audits, and never broadcast a score across multiple orientations.
    if let Json::Object(fields) = &mut root {
        if let Some(Json::Object(rows)) = fields.get_mut("alignments") {
            for row in rows.values_mut() {
                if let Json::Object(fields) = row {
                    if let Some(effective) = fields
                        .get("alignment")
                        .and_then(|v| scalar_alignment(v, ids.len()))
                    {
                        fields.insert("alignment".into(), effective);
                    }
                }
            }
        }
    }
    let rows = root.get("alignments")?.object()?;
    require(rows.len() == 3, "grounding requires exactly R1, R2, R3")?;
    for slot in GROUND_SLOTS {
        let row = rows.get(slot).ok_or("missing anonymous grounding slot")?;
        require(
            row.object()?.len() == 3,
            "alignment entries require only alignment, evidence and reason",
        )?;
        bounded(row, "reason", 70)?;
        let alignment = row.get("alignment")?.array().map_err(|_| {
            format!(
                "grounding.{slot}.alignment must be an array with {} entries",
                ids.len()
            )
        })?;
        require(
            alignment.len() == ids.len(),
            &format!("grounding.{slot}.alignment requires {} entries", ids.len()),
        )?;
        let mut expected = BTreeSet::new();
        for (i, value) in alignment.iter().enumerate() {
            let n = value.number()?;
            require(
                n.is_finite() && (-2.0..=2.0).contains(&n),
                "alignment out of bounds",
            )?;
            if n != 0.0 {
                expected.insert(ids[i].clone());
            }
        }
        let answer = payload
            .get("answers")?
            .array()?
            .iter()
            .find(|v| v.get("id").and_then(Json::string).ok() == Some(slot))
            .ok_or("anonymous answer absent")?;
        let answer = normalized(answer.get("answer")?.string()?);
        let evidence = row.get("evidence")?.array().map_err(|_| {
            format!("grounding.{slot}.evidence must be an array; use [] when empty")
        })?;
        require(
            evidence.len() == expected.len(),
            "every nonzero alignment needs exactly one answer excerpt",
        )?;
        let mut seen = BTreeSet::new();
        for excerpt in evidence {
            require(
                excerpt.object()?.len() == 2,
                "alignment evidence requires orientation_id and quote only",
            )?;
            let id = string(excerpt, "orientation_id")?;
            require(
                expected.contains(&id) && seen.insert(id.clone()),
                "unknown, zero-alignment or duplicate evidence orientation",
            )?;
            let quote = normalized(&bounded(excerpt, "quote", 80)?);
            require(quote.chars().count()>=8&&answer.contains(&quote),&format!("grounding.{slot}.evidence[{id}].quote must be one exact contiguous excerpt of its OWN anonymous answer; copy 1..80 words (at least 8 characters), without inserted ellipses or joined passages"))?;
        }
    }
    Ok(root)
}
fn combined_grounding(
    root: &Json,
    c: &Candidates,
    quality: &[Evaluation],
    mapping: [usize; 3],
) -> Result<Judgment, String> {
    let rows = root.get("alignments")?.object()?;
    let mut evaluations = vec![];
    for (index, e) in quality.iter().enumerate() {
        let slot = mapping
            .iter()
            .position(|&i| i == index)
            .ok_or("invalid anonymous mapping")?;
        let row = rows
            .get(GROUND_SLOTS[slot])
            .ok_or("missing anonymous alignment")?;
        evaluations.push(obj([
            ("id", text(&e.id)),
            ("utility", num(e.utility)),
            ("feasibility", num(e.feasibility)),
            ("originality", num(e.originality)),
            ("alignment", row.get("alignment")?.clone()),
            ("violations", arr(e.violations.iter().map(text))),
            ("reason", text(&e.reason)),
        ]));
    }
    let parsed = obj([
        (
            "orientation_reviews",
            root.get("orientation_reviews")?.clone(),
        ),
        ("activation_review", root.get("activation_review")?.clone()),
        ("evaluations", arr(evaluations)),
    ]);
    parse_judgment(&parsed.encode(), c)
}
fn grounding_judgment(
    raw: &str,
    c: &Candidates,
    quality: &[Evaluation],
    payload: &Json,
) -> Result<Judgment, String> {
    combined_grounding(&blind_grounding(raw, payload)?, c, quality, ground_order(c))
}
pub fn validate_assessment(
    stage: &str,
    raw: &str,
    payload: &Json,
    _max_words: usize,
) -> Result<(), String> {
    match stage {
        "quality" => parse_quality(raw).map(|_| ()),
        "grounding" => {
            // Shape validation only: neutral local IDs and no reliance declarations.
            // Actual selection always maps onto the original Rust-only catalog.
            let root = blind_grounding(raw, payload)?;
            let orientations = payload
                .get("orientations")?
                .array()?
                .iter()
                .map(|o| {
                    Ok(Orientation {
                        id: string(o, "id")?,
                        criterion: String::new(),
                        consequence: String::new(),
                        limit: String::new(),
                        sources: vec![],
                        evidence: vec![],
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let candidates = ["A", "B", "C"]
                .iter()
                .map(|id| Candidate {
                    id: id.to_string(),
                    kind: String::new(),
                    mechanism: String::new(),
                    answer: String::new(),
                    orientations: vec![],
                    sources: vec![],
                })
                .collect();
            let c = Candidates {
                orientations,
                candidates,
                source_reviews: vec![],
            };
            let dummy = c
                .candidates
                .iter()
                .map(|c| Evaluation {
                    id: c.id.clone(),
                    utility: 3.0,
                    feasibility: 3.0,
                    originality: 3.0,
                    alignment: vec![],
                    violations: vec![],
                    reason: "Validation placeholder; never used for selection.".into(),
                })
                .collect::<Vec<_>>();
            combined_grounding(&root, &c, &dummy, [0, 1, 2]).map(|_| ())
        }
        _ => Err("unknown assessment stage".into()),
    }
}
fn checked_quality(
    raw: &str,
    c: &Candidates,
    options: &ChoiceOptions,
) -> Result<(Vec<Evaluation>, [usize; 3]), String> {
    let blinded = parse_quality(raw)?;
    let mut quality = vec![blinded[0].clone(); 3];
    let mapping = order(c);
    for (slot, &index) in mapping.iter().enumerate() {
        quality[index] = blinded[slot].clone();
        quality[index].id = c.candidates[index].id.clone();
        quality[index].alignment = vec![0.0; c.orientations.len()];
        let answer = &c.candidates[index].answer;
        if answer.split_whitespace().count() > options.max_words {
            quality[index]
                .violations
                .push("word_limit: checked word count exceeds configured limit".into());
        }
        if let Some((declared, total)) = schedule(answer) {
            if total > declared {
                quality[index].violations.push(format!(
                    "schedule_total: declared {declared} minutes, listed {total} minutes"
                ));
            }
        }
    }
    Ok((quality, mapping))
}
/// Both calls are stateless. Quality sees no memory, kinds, criteria or original IDs.
pub fn assess(
    input: &Json,
    c: &Candidates,
    options: &ChoiceOptions,
    mut call: impl FnMut(&str, &str, &str, u32) -> Result<String, String>,
) -> Result<ChoiceResult, String> {
    options.validate()?;
    let payload = quality_input(input, c, options.max_words)?;
    let raw = call("quality", QUALITY, &payload.encode(), 1600)?;
    let (quality, mapping) = checked_quality(&raw, c, options)?;
    let memory = input.get("memory")?;
    let has_sources = !memory.get("episodes")?.array()?.is_empty()
        || !memory.get("dispositions")?.array()?.is_empty();
    let (judgment, grounding_audit) = if has_sources {
        let p = grounding_input(input, c);
        let raw = call("grounding", GROUNDING, &p.encode(), 3000)?;
        (
            grounding_judgment(&raw, c, &quality, &p)?,
            json::parse(&raw)?,
        )
    } else {
        require(
            c.orientations.is_empty(),
            "orientations without delivered sources",
        )?;
        (
            Judgment {
                raw: quality.clone(),
                effective: quality.clone(),
                reviews: vec![],
                activation: obj([
                    ("verdict", text("adequate")),
                    (
                        "reason",
                        text("No delivered episodes or dispositions; grounding call skipped."),
                    ),
                ]),
            },
            Json::Null,
        )
    };
    let mut result = finish_choice(c, &judgment, options)?;
    if let Json::Object(a) = &mut result.audit {
        a.insert(
            "assessment_protocol".into(),
            text("task-only-quality-and-blind-grounding-022"),
        );
        a.insert(
            "quality_audit".into(),
            quality_audit(&payload, &raw, c, &quality, mapping)?,
        );
        let normalizations=GROUND_SLOTS.iter().filter_map(|slot|{
            let value=grounding_audit.get("alignments").and_then(|v|v.get(slot)).and_then(|v|v.get("alignment")).ok()?;
            scalar_alignment(value,c.orientations.len()).map(|effective|obj([("slot",text(*slot)),("field",text("alignment")),("raw",value.clone()),("effective",effective),("reason",text(if c.orientations.is_empty(){"No orientations: numeric zero denotes an empty axis, not a score."}else{"Exactly one orientation: preserve the numeric score in a singleton vector."}))]))
        }).collect::<Vec<_>>();
        a.insert(
            "grounding_validation_protocol".into(),
            text("scalar-axis-024"),
        );
        a.insert("grounding_normalizations".into(), arr(normalizations));
        a.insert("grounding_audit".into(), grounding_audit);
        a.insert(
            "grounding_request".into(),
            if has_sources {
                grounding_input(input, c)
            } else {
                Json::Null
            },
        );
        a.insert(
            "grounding_mapping".into(),
            if has_sources {
                Json::Object(
                    ground_order(c)
                        .iter()
                        .enumerate()
                        .map(|(slot, &i)| {
                            (GROUND_SLOTS[slot].to_string(), text(&c.candidates[i].id))
                        })
                        .collect(),
                )
            } else {
                obj([])
            },
        );
    }
    Ok(result)
}
fn quality_audit(
    payload: &Json,
    raw: &str,
    c: &Candidates,
    quality: &[Evaluation],
    mapping: [usize; 3],
) -> Result<Json, String> {
    Ok(obj([
        ("request", payload.clone()),
        ("response", json::parse(raw)?),
        (
            "mapping",
            Json::Object(
                mapping
                    .iter()
                    .enumerate()
                    .map(|(slot, &i)| (SLOTS[slot].to_string(), text(&c.candidates[i].id)))
                    .collect(),
            ),
        ),
        ("mapped_checked_scores", evaluation_json(quality)),
    ]))
}
/// Exact 021 quality audit compatibility; never reuse selection/grounding scores.
pub fn validate_quality_audit(
    input: &Json,
    c: &Candidates,
    options: &ChoiceOptions,
    audit: &Json,
) -> Result<(), String> {
    options.validate()?;
    let payload = quality_input(input, c, options.max_words)?;
    let raw = audit.get("response")?.encode();
    let (quality, mapping) = checked_quality(&raw, c, options)?;
    require(
        quality_audit(&payload, &raw, c, &quality, mapping)? == *audit,
        "quality source request, response, mapping or checked scores changed",
    )
}
#[cfg(test)]
mod tests;
