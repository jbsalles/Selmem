//! Isolate admission, reconstruction and decision use without changing SelMem.
use super::{
    json::{num, obj, text, Json},
    runner::*,
    transport,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
const CUE: &str = "Neri north route report independently checked open closed episode";
fn source_cells(design: &Json, history: &str) -> Result<Vec<Json>> {
    Ok(array(design, "schedule")?
        .iter()
        .filter(|c| s(c, "actor").ok() == Some("Neri") && s(c, "history").ok() == Some(history))
        .cloned()
        .collect())
}
fn event(observation: &Json) -> Result<Json> {
    let episode = n(observation, "episode")? as usize;
    Ok(obj([
        ("id", text(format!("report-{episode:02}"))),
        (
            "text",
            text(format!(
                "You independently checked {}'s report: {} Your independent observation was: {}",
                s(observation, "actor")?,
                s(observation, "report")?,
                s(observation, "independent_observation")?
            )),
        ),
        ("recall_query", text(CUE)),
    ]))
}
fn rebuild(cell: &Json, evidence: Json) -> Result<Json> {
    let mut prompt = cell.get("prompt")?.clone();
    prompt.put("past_observations", evidence);
    Ok(prompt)
}
fn run(source: &Path, out: &Path, config: Json, provider: &mut Provider, mock: bool) -> Result<()> {
    if out.exists() {
        return Err("learning output must be fresh".into());
    }
    let qualification = load(&source.join("positive.json"))?;
    if !mock
        && (!flag(&qualification, "qualified")
            || flag(&qualification, "mock")
            || s(&qualification, "status")? != "complete")
    {
        return Err("source model must pass the real positive control".into());
    }
    let design = load(&source.join("positive-design.json"))?;
    if s(&design, "version")? != "positive-1" {
        return Err("unsupported positive control version".into());
    }
    if !mock
        && (provider.model != s(design.get("provider")?, "model_requested")?
            || s(&config, "memory_model")? != provider.model
            || s(&config, "scorer_model")? != provider.model)
    {
        return Err("all models must match qualified positive control".into());
    }
    for h in ["reliable", "unreliable", "empty"] {
        if source_cells(&design, h)?.len() != 8 {
            return Err("requires the original balanced Neri panel".into());
        }
    }
    let hashes = arr(["positive.json", "positive-design.json"]
        .into_iter()
        .map(|file| {
            Ok(obj([
                ("file", text(file)),
                ("sha256", text(file_hash(&source.join(file))?)),
            ]))
        })
        .collect::<Result<Vec<_>>>()?);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut fixtures = Vec::new();
    for h in ["reliable", "unreliable"] {
        let cells = source_cells(&design, h)?;
        let observations = array(cells[0].get("prompt")?, "past_observations")?;
        if observations.len() != 12 {
            return Err("requires twelve externally authored observations".into());
        }
        for (i, o) in observations.iter().enumerate() {
            if n(o, "episode")? != (i + 1) as f64 || s(o, "actor")? != "Neri" {
                return Err("invalid episode provenance".into());
            }
        }
        fixtures.push(obj([
            ("history", text(h)),
            ("observations", arr(observations.iter().cloned())),
            (
                "events",
                arr(observations.iter().map(event).collect::<Result<Vec<_>>>()?),
            ),
        ]));
    }
    write(&out.join("learning-design.json"),&obj([
        ("version",text("learning-2-observation-support")),("config",config.clone()),("provider",provider.metadata()),("source_files",hashes.clone()),("fixtures",arr(fixtures.clone())),
        ("planned_encodings",num(24.)),("planned_readouts",num(4.)),("planned_decisions",num(72.)),("shuffle_seed",num(20261017.)),
        ("scope",text("One Neri history pair, twelve reports each. Episode identity is event.id provenance, outside the repeated factual proposition. Native tick/live/recall after every report; no sleep within twelve steps. Final readonly recalls on fresh identical snapshot clones under native/strict. Kept can mean new trace or support update; inspect new_trace and salience_evaluated. Faithful and admitted-faithful controls come from external fixtures, never runtime archive retrieval. Probe evidence is frozen across decisions; only the evidence field differs. No history label or oracle is sent to model.")),
        ("interpretation",text("Compare full external-history oracle and observed choices; admitted-only control diagnoses gate loss, native/strict diagnose recall. The recalled sample can be smaller than twelve, so identical choices at cost 3 can be rational. Do not count duplicate recalls as independent past episodes. No population, persistence, generalization, creativity or specific reconstruction-benefit claim.")),
    ]))?;
    let mut report = obj([("status", text("running")), ("mock", Json::Bool(mock))]);
    write(&out.join("learning.json"), &report)?;
    let mut receipts = Vec::new();
    let mut readouts = Vec::new();
    let mut decisions = Vec::new();
    let mut snapshot_hashes = Vec::new();
    let result: Result<()> = (|| {
        let mut contexts = Vec::new();
        let mut rehearsal = Provider::new(true, "mock".into(), String::new(), None, out);
        for fixture in &fixtures {
            let history = s(fixture, "history")?;
            let memory = Memory::new("selmem_full", out.join(format!("{history}.vault")), &config);
            memory.init()?;
            let mut admitted = Vec::new();
            for (i, e) in array(fixture, "events")?.iter().enumerate() {
                super::progress::line(format!("LEARNING history={history} episode={}/12", i + 1));
                let receipt = memory.ingest(e, i + 1, &mut rehearsal)?;
                if flag(&receipt, "kept") {
                    admitted.push(array(fixture, "observations")?[i].clone());
                }
                super::progress::line(format!(
                    "GATE history={history} episode={} kept={} score={} reason={}",
                    i + 1,
                    flag(&receipt, "kept"),
                    n(&receipt, "score")?,
                    s(&receipt, "reason")?
                ));
                let row = obj([
                    ("history", text(history)),
                    ("event", e.clone()),
                    ("receipt", receipt),
                ]);
                log(&out.join("learning-encoding.jsonl"), &row)?;
                receipts.push(row);
            }
            contexts.push((
                history.to_string(),
                "faithful".to_string(),
                fixture.get("observations")?.clone(),
            ));
            contexts.push((
                history.to_string(),
                "admitted-faithful".to_string(),
                arr(admitted),
            ));
            let original_hash = file_hash(&memory.path)?;
            snapshot_hashes.push(obj([
                ("file", text(format!("{history}.vault"))),
                ("sha256", text(&original_hash)),
            ]));
            for policy in ["native", "strict"] {
                super::progress::line(format!("FINAL READOUT history={history} policy={policy}"));
                let mut cfg = config.clone();
                cfg.put("reconstruction_policy", text(policy));
                let source_memory = Memory::new("selmem_full", memory.path.clone(), &cfg);
                let clone = source_memory.copy(out.join(format!("{history}-{policy}.vault")))?;
                if file_hash(&clone.path)? != original_hash {
                    return Err("clone bytes differ".into());
                }
                let (context, audit) = clone.context(CUE)?;
                if file_hash(&clone.path)? != original_hash
                    || file_hash(&memory.path)? != original_hash
                {
                    return Err("readonly probe mutated memory".into());
                }
                let row = obj([
                    ("history", text(history)),
                    ("policy", text(policy)),
                    ("snapshot_hash", text(&original_hash)),
                    ("context", context.clone()),
                    ("audit", audit),
                ]);
                log(&out.join("learning-readouts.jsonl"), &row)?;
                readouts.push(row);
                // Leave identity, counts and provenance as actually recalled. No synthetic count injection.
                contexts.push((history.to_string(), policy.to_string(), context));
            }
        }
        contexts.push(("empty".into(), "empty".into(), arr([])));
        let mut schedule = Vec::new();
        for (history, condition, evidence) in contexts {
            for cell in source_cells(&design, &history)? {
                schedule.push(obj([
                    ("history", text(&history)),
                    ("condition", text(&condition)),
                    ("source_cell", cell.clone()),
                    ("prompt", rebuild(&cell, evidence.clone())?),
                ]));
            }
        }
        Rng::new(20261017).shuffle(&mut schedule);
        write(&out.join("learning-schedule.json"), &arr(schedule.clone()))?;
        for (i, cell) in schedule.into_iter().enumerate() {
            super::progress::line(format!(
                "LEARNING DECISION {}/72 history={} condition={} cost={}",
                i + 1,
                s(&cell, "history")?,
                s(&cell, "condition")?,
                n(cell.get("source_cell")?, "cost")?
            ));
            let answer = provider.ask(cell.get("prompt")?.clone(), "decision")?;
            let choice = s(&answer, "choice")?;
            if !["follow", "verify", "alternative", "stop"].contains(&choice) {
                return Err("invalid choice".into());
            }
            let correct = choice == s(cell.get("source_cell")?, "expected")?;
            let row = obj([
                ("cell", cell),
                ("answer", answer),
                ("full_history_oracle_correct", Json::Bool(correct)),
            ]);
            log(&out.join("learning-decisions.jsonl"), &row)?;
            decisions.push(row);
        }
        Ok(())
    })();
    let sources_ok = hashes.array()?.iter().all(|h| {
        file_hash(&source.join(s(h, "file").unwrap()))
            .ok()
            .as_deref()
            == Some(s(h, "sha256").unwrap())
    });
    let snapshots_ok = snapshot_hashes.iter().all(|h| {
        file_hash(&out.join(s(h, "file").unwrap())).ok().as_deref() == Some(s(h, "sha256").unwrap())
    });
    report.put("source_unchanged", Json::Bool(sources_ok));
    report.put(
        "life_snapshots_unchanged_after_probes",
        Json::Bool(snapshots_ok),
    );
    report.put("life_snapshots", arr(snapshot_hashes));
    report.put("completed_encodings", num(receipts.len() as f64));
    report.put("completed_readouts", num(readouts.len() as f64));
    report.put("completed_decisions", num(decisions.len() as f64));
    report.put("http_attempts", num(transport::used() as f64));
    report.put(
        "admission",
        arr(["reliable", "unreliable"].into_iter().map(|h| {
            obj([
                ("history", text(h)),
                (
                    "completed",
                    num(receipts
                        .iter()
                        .filter(|r| s(r, "history").ok() == Some(h))
                        .count() as f64),
                ),
                (
                    "kept",
                    num(receipts
                        .iter()
                        .filter(|r| {
                            s(r, "history").ok() == Some(h)
                                && flag(r.get("receipt").unwrap(), "kept")
                        })
                        .count() as f64),
                ),
            ])
        })),
    );
    let mut summaries = Vec::new();
    for h in ["reliable", "unreliable", "empty"] {
        for c in ["faithful", "admitted-faithful", "native", "strict", "empty"] {
            let rows: Vec<_> = decisions
                .iter()
                .filter(|r| {
                    let x = r.get("cell").unwrap();
                    s(x, "history").ok() == Some(h) && s(x, "condition").ok() == Some(c)
                })
                .collect();
            if rows.is_empty() {
                continue;
            }
            let choices = obj(["follow", "verify", "alternative", "stop"].map(|choice| {
                (
                    choice,
                    num(rows
                        .iter()
                        .filter(|r| s(r.get("answer").unwrap(), "choice").ok() == Some(choice))
                        .count() as f64),
                )
            }));
            summaries.push(obj([
                ("history", text(h)),
                ("condition", text(c)),
                ("completed", num(rows.len() as f64)),
                (
                    "full_history_oracle_correct",
                    num(rows
                        .iter()
                        .filter(|r| flag(r, "full_history_oracle_correct"))
                        .count() as f64),
                ),
                ("choices", choices),
            ]));
        }
    }
    report.put("by_condition", arr(summaries));
    let final_result = result.and_then(|_| {
        if sources_ok && snapshots_ok {
            Ok(())
        } else {
            Err("snapshot or source changed".into())
        }
    });
    report.put(
        "status",
        text(if final_result.is_err() {
            "failed"
        } else if mock {
            "technical_rehearsal_complete"
        } else {
            "complete"
        }),
    );
    report.put(
        "error",
        final_result.as_ref().err().map(text).unwrap_or(Json::Null),
    );
    write(&out.join("learning.json"), &report)?;
    final_result
}
pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let mut flags = BTreeMap::new();
    let mut mock = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--mock" {
            mock = true;
            i += 1;
            continue;
        }
        let key = &args[i];
        if ![
            "--source",
            "--out",
            "--config",
            "--llm",
            "--model",
            "--plug",
            "--max-http-calls",
            "--observation-evidence",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("unknown option {key}"));
        }
        if flags
            .insert(key.clone(), args.get(i + 1).ok_or("missing value")?.clone())
            .is_some()
        {
            return Err("duplicate option".into());
        }
        i += 2;
    }
    let source = PathBuf::from(flags.get("--source").ok_or("--source required")?);
    let out = PathBuf::from(flags.get("--out").ok_or("--out required")?);
    let limit = flags
        .get("--max-http-calls")
        .map(|v| v.parse::<usize>().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(800);
    if !(1..=800).contains(&limit) {
        return Err("budget must be 1..800".into());
    }
    flags.insert(
        "--memory-backend".into(),
        if mock { "rules" } else { "http" }.into(),
    );
    flags.insert(
        "--lifecycle-backend".into(),
        if mock { "rules" } else { "http" }.into(),
    );
    flags.insert("--reconstruction-policy".into(), "native".into());
    let mut config = campaign_config(selmem::Config::get(), &flags, true, mock)?;
    let enabled = match flags.get("--observation-evidence").map(String::as_str) {
        Some("on") => true,
        Some("off") => false,
        Some(_) => return Err("--observation-evidence must be on|off".into()),
        None => match config.get("observation_evidence") {
            Ok(Json::Bool(v)) => *v,
            Err(_) => true,
            _ => return Err("observation_evidence must be boolean".into()),
        },
    };
    config.put("observation_evidence", Json::Bool(enabled));
    // Prespecified no-sleep twelve-episode learning assay; organ thresholds remain native.
    config.put("sleep_every", num(100000.));
    config.put("hours_per_interaction", num(1.));
    let mouth = if mock {
        None
    } else {
        Some(configured_mouth(selmem::Config::get(), &flags)?.ok_or("configure provider")?)
    };
    let mut p = Provider::new(
        mock,
        mouth
            .as_ref()
            .map(|m| m.model.clone())
            .unwrap_or("mock".into()),
        mouth.as_ref().map(|m| m.url.clone()).unwrap_or_default(),
        None,
        &out,
    )
    .with_api_key(mouth.and_then(|m| m.api_key));
    super::progress::enable(true);
    let _session = transport::begin(&out, limit);
    run(&source, &out, config, &mut p, mock)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_local_http_life_and_readonly_controls_preserve_snapshots() {
        let source = super::super::lifecycle_tests::root();
        super::super::positive_control::cli(vec![
            "--mock".into(),
            "--out".into(),
            source.to_string_lossy().into(),
        ])
        .unwrap();
        let server = super::super::lifecycle_tests::Server::new(false);
        let out = super::super::lifecycle_tests::root();
        let mut cfg =
            super::super::json::parse(include_str!("../../../experiments/genesis/config.json"))
                .unwrap();
        cfg = merge(&cfg, &super::super::lifecycle_tests::config(&server));
        cfg.put("observation_evidence", Json::Bool(true));
        cfg.put("sleep_every", num(100000.));
        cfg.put("hours_per_interaction", num(1.));
        let mut p = Provider::new(true, "mock".into(), String::new(), None, &out);
        let _session = transport::begin(&out, 800);
        run(&source, &out, cfg, &mut p, true).unwrap();
        let r = load(&out.join("learning.json")).unwrap();
        assert_eq!(n(&r, "completed_encodings").unwrap(), 24.);
        assert_eq!(n(&r, "completed_readouts").unwrap(), 4.);
        assert_eq!(n(&r, "completed_decisions").unwrap(), 72.);
        for history in ["reliable", "unreliable"] {
            let state = Memory::new(
                "selmem_full",
                out.join(format!("{history}.vault")),
                &obj([]),
            )
            .call("inspect", obj([]))
            .unwrap();
            assert_eq!(array(&state, "traces").unwrap().len(), 1);
            assert_eq!(
                n(
                    &array(&state, "observation_evidence").unwrap()[0],
                    "observations"
                )
                .unwrap(),
                12.
            );
        }
        assert!(flag(&r, "source_unchanged"));
        assert!(flag(&r, "life_snapshots_unchanged_after_probes"));
        assert_eq!(s(&r, "status").unwrap(), "technical_rehearsal_complete");
        let schedule = load(&out.join("learning-schedule.json")).unwrap();
        assert_eq!(schedule.array().unwrap().len(), 72);
        for cell in schedule
            .array()
            .unwrap()
            .iter()
            .filter(|c| s(c, "condition").unwrap() == "native")
        {
            assert!(cell
                .get("prompt")
                .unwrap()
                .get("past_observations")
                .unwrap()
                .get("memories")
                .is_ok());
            let prompt = cell.get("prompt").unwrap();
            for key in ["expected", "history", "condition", "oracle_value"] {
                assert!(prompt.get(key).is_err());
            }
        }
    }
    #[test]
    fn external_episode_identity_is_explicit() {
        let o = obj([
            ("episode", num(12.)),
            ("actor", text("Neri")),
            ("report", text("The north route is open.")),
            (
                "independent_observation",
                text("The north route was closed."),
            ),
        ]);
        let e = event(&o).unwrap();
        assert_eq!(s(&e, "id").unwrap(), "report-12");
        assert!(!s(&e, "text").unwrap().contains("episode 12"));
        assert!(s(&e, "text").unwrap().contains("closed"));
    }
    #[test]
    fn evidence_replacement_does_not_leak_full_history() {
        let cell = obj([(
            "prompt",
            obj([
                ("past_observations", arr([text("secret twelve successes")])),
                ("world_rules", text("fixed")),
            ]),
        )]);
        let p = rebuild(&cell, obj([("memories", arr([text("one report")]))])).unwrap();
        assert!(!p.encode().contains("secret"));
        assert_eq!(s(&p, "world_rules").unwrap(), "fixed");
    }
    #[test]
    fn unqualified_source_fails_before_http() {
        let source = super::super::lifecycle_tests::root();
        fs::create_dir_all(&source).unwrap();
        write(
            &source.join("positive.json"),
            &obj([
                ("qualified", Json::Bool(false)),
                ("status", text("complete")),
                ("mock", Json::Bool(false)),
            ]),
        )
        .unwrap();
        let out = super::super::lifecycle_tests::root();
        let mut p = Provider::new(true, "mock".into(), String::new(), None, &out);
        let _session = transport::begin(&out, 800);
        assert!(run(&source, &out, obj([]), &mut p, false).is_err());
        assert_eq!(transport::used(), 0);
        assert!(!out.exists());
    }
}
