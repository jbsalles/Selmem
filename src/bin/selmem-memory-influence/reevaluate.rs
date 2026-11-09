//! Evaluate frozen candidate pools without regenerating answers or mutating the source.
use super::*;
pub fn run(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let source = fs::canonicalize(
        o.cases_from
            .as_ref()
            .ok_or("reevaluate requires --cases-from SOURCE_LOT")?,
    )
    .map_err(|e| e.to_string())?;
    let (cases, provenance) = frozen_cases(&source)?;
    for name in ["all-proposals.json", "result.json"] {
        let meta = fs::symlink_metadata(source.join(name)).map_err(|e| e.to_string())?;
        ensure(
            meta.file_type().is_file() && meta.len() < 10_000_000,
            "source outputs must be bounded regular files",
        )?;
    }
    let source_result = load(&source.join("result.json"))?;
    ensure(
        s(&source_result, "status")? == "collected_unrated"
            && source_result.get("completed")?.number()? == 72.0,
        "source collection incomplete",
    )?;
    ensure(
        o.mock || source_result.get("mock")? == &Json::Bool(false),
        "real reevaluation requires a real source",
    )?;
    let originals = load(&source.join("all-proposals.json"))?;
    let pools = validate_source(&cases, &originals)?;
    let grounding_calls = pools
        .iter()
        .flatten()
        .filter(|(input, _)| {
            let m = input.get("memory").unwrap();
            !m.get("episodes").unwrap().array().unwrap().is_empty()
                || !m.get("dispositions").unwrap().array().unwrap().is_empty()
        })
        .count();
    let reused = if o.reuse_quality {
        Some(quality_reuse::prepare(&source, &originals, &pools)?)
    } else {
        None
    };
    let quality_calls = if o.reuse_quality { 0 } else { 48 };
    let nominal = quality_calls + grounding_calls;
    let cfg = if o.config.is_file() {
        selmem::Config::load_path(&o.config).map_err(|e| e.to_string())?
    } else {
        ensure(o.mock || o.prepare, "runtime config missing")?;
        selmem::Config::empty()
    };
    let mouth = cfg
        .mouth(
            None,
            Some(o.model.clone()),
            Some(o.provider.clone()),
            &o.model,
        )?
        .ok_or("HTTP provider required")?;
    if !o.mock && !o.prepare {
        ensure(
            mouth.api_key.as_ref().is_some_and(|k| !k.trim().is_empty())
                || mouth.url.starts_with("http://"),
            "provider API key missing",
        )?;
    }
    if o.reuse_quality {
        let old = load(&source.join("design.json"))?;
        ensure(
            s(&old, "model")? == mouth.model
                && s(&old, "endpoint")? == mouth.url
                && s(&old, "reasoning_effort")? == cfg.reasoning(),
            "quality reuse requires unchanged model, endpoint and reasoning settings",
        )?;
    }
    let parent = fs::canonicalize(
        o.out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(|e| e.to_string())?;
    let out = parent.join(o.out.file_name().ok_or("invalid output")?);
    ensure(
        !out.starts_with(&source) && !source.starts_with(&out),
        "output and source must be separate directories",
    )?;
    if out.exists() {
        ensure(o.resume, "output exists; use fresh directory or --resume")?;
        ensure(
            fs::symlink_metadata(&out)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_dir(),
            "output must be a regular directory",
        )?;
    } else {
        ensure(!o.resume, "resume output missing")?;
        fs::create_dir(&out).map_err(|e| e.to_string())?;
    }
    let lock = out.join("run.lock");
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock)
        .map_err(|_| "output locked")?;
    let _lock = Lock(lock);
    let design=obj([("version",text("memory-influence-1")),("mode",text("frozen-pool-reevaluation-022")),("implementation",text(implementation())),("fixture_sha256",text(sha256::hex(FIXTURE.as_bytes()))),("cases_sha256",text(hash_file(&source.join("cases.json"))?)),("source_proposals_sha256",text(hash_file(&source.join("all-proposals.json"))?)),("source_result_sha256",text(hash_file(&source.join("result.json"))?)),("case_source",provenance),("model",text(&mouth.model)),("endpoint",text(&mouth.url)),("reasoning_effort",text(cfg.reasoning())),("evaluation_temperature",num(0)),("mock",Json::Bool(o.mock)),("nominal_calls",num(nominal as u32)),("quality_calls",num(quality_calls as u32)),("reuse_quality",Json::Bool(o.reuse_quality)),("quality_reuse_sha256",reused.as_ref().map(|r|text(sha256::hex(r.encode().as_bytes()))).unwrap_or(Json::Null)),("grounding_calls",num(grounding_calls as u32)),("quality_system",text(choice::QUALITY)),("grounding_system",text(choice::GROUNDING)),("planned_proposals",num(72)),("evaluation_system",text(choice::EVALUATE)),("selection_protocol",text(choice::VERSION)),("provenance",text("24 direct answers and 48 candidate pools reused unchanged; only evaluation and selection rerun. Not a fresh generation trial."))]);
    if o.resume {
        ensure(
            load(&out.join("cases.json"))? == cases,
            "output cases changed",
        )?;
        if let Some(manifest) = &reused {
            ensure(
                load(&out.join("quality-reuse.json"))? == *manifest,
                "quality reuse provenance changed",
            )?;
        }
        resume_design(&out, &design)?;
    } else {
        write(&out.join("design.json"), &design)?;
        fs::copy(source.join("cases.json"), out.join("cases.json")).map_err(|e| e.to_string())?;
    }
    if let Some(manifest) = &reused {
        let file = out.join("quality-reuse.json");
        if o.resume {
            ensure(
                load(&file)? == *manifest,
                "quality reuse provenance changed",
            )?;
        } else {
            write(&file, manifest)?;
        }
    }
    fs::create_dir_all(out.join("cache")).map_err(|e| e.to_string())?;
    if o.prepare {
        println!("Prepared frozen pools: {nominal} nominal assessment calls ({quality_calls} quality + {grounding_calls} grounding; reuse_quality={}), zero generation/direct calls; no HTTP calls made.",o.reuse_quality);
        return Ok(());
    }
    let used = if out.join("http.jsonl").exists() {
        fs::read_to_string(out.join("http.jsonl"))
            .map_err(|e| e.to_string())?
            .lines()
            .map(json::parse)
            .collect::<Result<Vec<_>>>()?
            .iter()
            .filter(|r| s(r, "event").ok() == Some("start"))
            .count()
    } else {
        0
    };
    let mut api = Api {
        out: out.clone(),
        url: mouth.url,
        key: mouth.api_key,
        model: mouth.model,
        reasoning: cfg.reasoning(),
        mock: o.mock,
        max: o.max,
        used,
        transport: None,
    };
    let mut proposals = vec![];
    for (original, pool) in originals.array()?.iter().zip(pools) {
        let mut p = original.clone();
        if let Some((input, c)) = pool {
            let condition = s(original, "condition")?;
            let options = ChoiceOptions {
                mode: ChoiceMode::Always,
                influence: if condition == "candidates_empty" {
                    Influence::None
                } else {
                    Influence::Moderate
                },
                max_words: 180,
            };
            let job = format!(
                "{}-{}-{}-{condition}",
                s(original, "history")?,
                s(original.get("task")?, "id")?,
                original.get("repeat")?.number()? as u32
            );
            let result = choice::assess(&input, &c, &options, |stage, system, raw, cap| {
                let payload = json::parse(raw)?;
                if stage == "quality" && o.reuse_quality {
                    ensure(
                        payload
                            == *original
                                .get("audit")?
                                .get("quality_audit")?
                                .get("request")?,
                        "quality reuse request changed after preflight",
                    )?;
                    return Ok(original
                        .get("audit")?
                        .get("quality_audit")?
                        .get("response")?
                        .encode());
                }

                api.call(
                    &job,
                    stage,
                    system,
                    &payload,
                    cap,
                    100 + original.get("repeat")?.number()? as u32,
                    |r| choice::validate_assessment(stage, r, &payload, 180),
                )
            })?;
            if o.reuse_quality {
                ensure(
                    result.audit.get("quality_audit")?
                        == original.get("audit")?.get("quality_audit")?,
                    "quality audit changed despite reuse",
                )?;
            }
            if let Json::Object(fields) = &mut p {
                fields.insert("proposal".into(), text(result.answer));
                fields.insert("audit".into(), result.audit);
            }
        }
        proposals.push(p);
        write(
            &out.join("progress.json"),
            &obj([
                ("completed", num(proposals.len() as u32)),
                ("http_attempts", num(api.used as u32)),
                ("mock", Json::Bool(o.mock)),
            ]),
        )?;
    }
    let mut blind = proposals
        .iter()
        .map(|p| {
            Ok(obj([
                ("blind_id", p.get("blind_id")?.clone()),
                ("task", p.get("task")?.clone()),
                ("proposal", p.get("proposal")?.clone()),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    blind.sort_by_key(|p| s(p, "blind_id").unwrap().to_string());
    write(
        &out.join("selection-summary.json"),
        &selection_summary(&proposals)?,
    )?;
    write(&out.join("all-proposals.json"), &arr(proposals))?;
    write(&out.join("blind-proposals.json"), &arr(blind))?;
    write(
        &out.join("result.json"),
        &obj([
            ("status", text("collected_unrated")),
            ("completed", num(72)),
            ("mock", Json::Bool(o.mock)),
            ("http_attempts", num(api.used as u32)),
            ("reused_direct_answers", num(24)),
            ("reused_candidate_pools", num(48)),
            (
                "reused_quality_results",
                num(if o.reuse_quality { 48 } else { 0 }),
            ),
        ]),
    )?;
    println!(
        "Reevaluated frozen pools in {} (mock={})",
        out.display(),
        o.mock
    );
    Ok(())
}
fn validate_source(
    cases: &Json,
    proposals: &Json,
) -> Result<Vec<Option<(Json, choice::Candidates)>>> {
    ensure(
        proposals.array()?.len() == 72,
        "expected 72 source proposals",
    )?;
    let mut seen = BTreeSet::new();
    let mut pools = vec![];
    for p in proposals.array()? {
        let h = s(p, "history")?;
        let task = p.get("task")?;
        let repeat = p.get("repeat")?;
        let condition = s(p, "condition")?;
        ensure(
            ["direct_memory", "candidates_empty", "candidates_memory"].contains(&condition),
            "unknown source arm",
        )?;
        let case = cases
            .array()?
            .iter()
            .find(|c| {
                s(c, "history").ok() == Some(h)
                    && c.get("task").ok() == Some(task)
                    && c.get("repeat").ok() == Some(repeat)
            })
            .ok_or("source proposal has no frozen case")?;
        let job = format!(
            "{h}-{}-{}-{condition}",
            s(task, "id")?,
            repeat.number()? as u32
        );
        ensure(seen.insert(job.clone()), "duplicate source cell")?;
        ensure(
            s(p, "blind_id")? == format!("I{}", &sha256::hex(job.as_bytes())[..12]),
            "source blind ID mismatch",
        )?;
        ensure(
            !s(p, "proposal")?.trim().is_empty()
                && s(p, "proposal")?.split_whitespace().count() <= 180,
            "invalid source answer",
        )?;
        if condition == "direct_memory" {
            ensure(p.get("audit")? == &Json::Null, "unexpected direct audit")?;
            pools.push(None);
        } else {
            let audit = p.get("audit")?;
            ensure(
                [
                    "selmem-response-choice-2",
                    "selmem-response-choice-3",
                    "selmem-response-choice-4",
                    "selmem-response-choice-5",
                ]
                .contains(&s(audit, "version")?),
                "unsupported source selection protocol",
            )?;
            let input = if condition == "candidates_empty" {
                empty_input(case.get("input")?)?
            } else {
                case.get("input")?.clone()
            };
            let c = choice::parse_candidates(&audit.get("generation")?.encode(), &input, 180)?;
            ensure(
                c.candidates.iter().any(|c| {
                    c.id == s(audit, "selected").unwrap_or("")
                        && c.answer == s(p, "proposal").unwrap_or("")
                }),
                "source selected answer disagrees with frozen pool",
            )?;
            pools.push(Some((input, c)));
        }
    }
    Ok(pools)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frozen_grid_rejects_duplicate_cells() {
        let cases = prepare_cases().unwrap();
        let mut proposals = vec![];
        for case in cases.array().unwrap() {
            for condition in ["direct_memory", "candidates_empty", "candidates_memory"] {
                let h = s(case, "history").unwrap();
                let task = case.get("task").unwrap();
                let repeat = case.get("repeat").unwrap();
                let job = format!(
                    "{h}-{}-{}-{condition}",
                    s(task, "id").unwrap(),
                    repeat.number().unwrap() as u32
                );
                let input = if condition == "candidates_empty" {
                    empty_input(case.get("input").unwrap()).unwrap()
                } else {
                    case.get("input").unwrap().clone()
                };
                let c = choice::parse_candidates(
                    &mock_generation(&input).unwrap().encode(),
                    &input,
                    180,
                )
                .unwrap();
                proposals.push(obj([
                    ("history", text(h)),
                    ("task", task.clone()),
                    ("repeat", repeat.clone()),
                    ("condition", text(condition)),
                    (
                        "blind_id",
                        text(format!("I{}", &sha256::hex(job.as_bytes())[..12])),
                    ),
                    (
                        "proposal",
                        text(if condition == "direct_memory" {
                            "Direct answer."
                        } else {
                            &c.candidates[0].answer
                        }),
                    ),
                    (
                        "audit",
                        if condition == "direct_memory" {
                            Json::Null
                        } else {
                            obj([
                                ("version", text("selmem-response-choice-2")),
                                ("generation", c.to_json()),
                                ("selected", text("A")),
                            ])
                        },
                    ),
                ]));
            }
        }
        let pools = validate_source(&cases, &arr(proposals.clone())).unwrap();
        assert_eq!(pools.iter().filter(|p| p.is_some()).count(), 48);
        proposals[1] = proposals[0].clone();
        assert!(validate_source(&cases, &arr(proposals)).is_err());
    }
}
