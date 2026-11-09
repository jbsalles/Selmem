//! Matched native sleep interventions; references are external audit fixtures only.
use super::{
    json::{num, obj, text, Json},
    runner::*,
    transport,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
const CONDITIONS: [&str; 3] = ["none", "normal", "deep"];
const HORIZONS: [usize; 4] = [0, 1, 5, 20];
fn factual_prompt(probe: &Json, evidence: Json) -> Result<Json> {
    Ok(obj([("task",text("Answer the past factual question using only supplied fallible memories. Return {\"choice\": option_id}. Select unknown if evidence is insufficient; do not guess.")),("situation",probe.get("question")?.clone()),("past_observations",evidence),("options",probe.get("options")?.clone())]))
}
fn validate_fixture(f: &Json) -> Result<()> {
    if s(f, "version")? != "sleep-locomo-1"
        || s(f, "dataset_sha256")?
            != "79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4"
    {
        return Err("unsupported or unpinned LoCoMo fixture".into());
    }
    let mut histories = BTreeSet::new();
    for h in array(f, "histories")? {
        if !histories.insert(s(h, "id")?) {
            return Err("duplicate history".into());
        }
        let mut ids = BTreeSet::new();
        let mut events = BTreeSet::new();
        for e in array(h, "events")? {
            if !ids.insert(s(e, "dia_id")?)
                || !events.insert(s(e, "id")?)
                || s(e, "text")?.is_empty()
                || !flag(e, "external")
            {
                return Err("invalid LoCoMo observation".into());
            }
        }
        let mut probes = BTreeSet::new();
        for q in array(h, "probes")? {
            if !probes.insert(s(q, "id")?) || array(q, "evidence_ids")?.is_empty() {
                return Err("invalid probe".into());
            }
            for id in array(q, "evidence_ids")? {
                if !ids.contains(id.string()?) {
                    return Err("question evidence outside prefix".into());
                }
            }
            let options = array(q, "options")?;
            let mut choices = BTreeSet::new();
            for o in options {
                if !choices.insert(s(o, "id")?) {
                    return Err("duplicate option".into());
                }
            }
            let expected = s(q, "expected")?;
            if options
                .iter()
                .filter(|o| {
                    s(o, "id").ok() == Some(expected)
                        && s(o, "text").ok() == s(q, "reference_answer").ok()
                })
                .count()
                != 1
            {
                return Err("reference answer/choice mismatch".into());
            }
        }
        if array(h, "probes")?.is_empty() {
            return Err("empty LoCoMo panel".into());
        }
    }
    if histories.is_empty() {
        return Err("empty LoCoMo fixture".into());
    }
    Ok(())
}
fn assess(p: &mut Provider, prompt: Json, expected: &str) -> Result<Json> {
    let a = p.ask(prompt.clone(), "decision")?;
    let choice = s(&a, "choice")?;
    if !array(&prompt, "options")?
        .iter()
        .any(|o| s(o, "id").ok() == Some(choice))
    {
        return Err("choice outside panel".into());
    }
    Ok(obj([
        ("prompt", prompt),
        ("answer", a.clone()),
        ("correct", Json::Bool(choice == expected)),
        ("abstained", Json::Bool(choice == "unknown")),
    ]))
}
fn state_metrics(state: &Json) -> Result<Json> {
    let groups = array(state, "observation_evidence")?;
    let mut ids = BTreeSet::new();
    let mut occurrences = 0;
    for g in groups {
        if let Ok(observations) = array(g, "observation_ids") {
            for id in observations {
                occurrences += 1;
                ids.insert(format!("{}:{}", s(g, "source")?, id.string()?));
            }
        }
    }

    Ok(obj([
        ("traces", num(array(state, "traces")?.len() as f64)),
        ("axioms", num(array(state, "axioms")?.len() as f64)),
        ("support_groups", num(groups.len() as f64)),
        ("unique_support_identities", num(ids.len() as f64)),
        (
            "duplicate_support_identities",
            num((occurrences - ids.len()) as f64),
        ),
        (
            "support_observations",
            num(groups
                .iter()
                .map(|g| n(g, "observations"))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum::<f64>()),
        ),
    ]))
}
fn check_clock(states: &[Json]) -> Result<()> {
    let clocks = states
        .iter()
        .map(|s| n(s, "clock"))
        .collect::<Result<Vec<_>>>()?;
    if clocks.windows(2).any(|w| w[0] != w[1]) {
        return Err("intervention clocks differ".into());
    }
    Ok(())
}
struct History {
    id: String,
    domain: &'static str,
    memory: Memory,
    cells: Vec<Json>,
}
fn controlled_cells(schedule: &Json, h: &str) -> Result<Vec<Json>> {
    let mut seen = BTreeSet::new();
    let mut cells = Vec::new();
    for c in schedule.array()? {
        if s(c, "history")? == h && s(c, "condition")? == "faithful" {
            let original = c.get("source_cell")?;
            // Only one original repeat; both option orders and costs remain.
            if n(original, "repeat")? != 0. {
                continue;
            }
            let signature = format!("{}:{}", n(original, "cost")?, flag(original, "reverse"));
            if seen.insert(signature.clone()) {
                cells.push(obj([
                    ("id", text(signature)),
                    ("kind", text("decision")),
                    (
                        "query",
                        text("Neri north route report independently checked open closed"),
                    ),
                    ("expected", original.get("expected")?.clone()),
                    ("prompt", c.get("prompt")?.clone()),
                ]));
            }
        }
    }
    if cells.len() != 4 {
        return Err("requires complete original Neri learning schedule".into());
    }
    Ok(cells)
}
fn run(
    source: &Path,
    fixture_path: &Path,
    out: &Path,
    config: Json,
    p: &mut Provider,
    resume: Option<&Path>,
    prepare_only: bool,
) -> Result<()> {
    if out.exists() {
        return Err("sleep output must be fresh".into());
    }
    let fixture = load(fixture_path)?;
    validate_fixture(&fixture)?;
    let source_report = load(&source.join("learning.json"))?;
    let design = load(&source.join("learning-design.json"))?;
    let schedule = load(&source.join("learning-schedule.json"))?;
    if !p.mock && (s(&source_report, "status")? != "complete" || flag(&source_report, "mock")) {
        return Err("requires completed real learning source".into());
    }
    if s(&design, "version")? != "learning-2-observation-support"
        || !flag(design.get("config")?, "observation_evidence")
    {
        return Err("use learning-2 observation support ON source".into());
    }
    if !p.mock && s(design.get("provider")?, "model_requested")? != p.model {
        return Err("model must match learning source".into());
    }
    let mut source_hashes = vec![(fixture_path.to_path_buf(), file_hash(fixture_path)?)];
    for name in [
        "learning.json",
        "learning-design.json",
        "learning-schedule.json",
        "reliable.vault",
        "unreliable.vault",
    ] {
        let path = source.join(name);
        source_hashes.push((path.clone(), file_hash(&path)?));
    }
    let mut histories = Vec::new();
    for h in ["reliable", "unreliable"] {
        histories.push(History {
            id: h.into(),
            domain: "controlled",
            memory: Memory::new("selmem_full", source.join(format!("{h}.vault")), &config),
            cells: controlled_cells(&schedule, h)?,
        });
    }
    let mut qualification_expected = Vec::new();
    for h in &histories {
        for c in &h.cells {
            qualification_expected.push((
                h.id.clone(),
                s(c, "id")?.into(),
                c.get("prompt")?.clone(),
                s(c, "expected")?.into(),
            ));
        }
    }
    for h in array(&fixture, "histories")? {
        for q in array(h, "probes")? {
            qualification_expected.push((
                s(h, "id")?.into(),
                s(q, "id")?.into(),
                factual_prompt(
                    q,
                    arr(array(h, "events")?
                        .iter()
                        .map(|e| e.get("text").unwrap().clone())),
                )?,
                s(q, "expected")?.into(),
            ));
        }
    }
    if let Some(parent) = resume {
        super::sleep_resume::validate(
            parent,
            &fixture,
            &config,
            &source_hashes,
            p.mock,
            &p.model,
            qualification_expected,
        )?;
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    if let Some(parent) = resume {
        super::sleep_resume::copy_logs(parent, out)?;
    }
    let total_cells = histories.iter().map(|h| h.cells.len()).sum::<usize>()
        + array(&fixture, "histories")?
            .iter()
            .map(|h| array(h, "probes").map(|p| p.len()))
            .collect::<Result<Vec<_>>>()?
            .iter()
            .sum::<usize>();
    write(&out.join("sleep-design.json"),&obj([("version",text("sleep-quality-1")),("config",config.clone()),("provider",p.metadata()),("fixture",fixture.clone()),("source_hashes",arr(source_hashes.iter().map(|(f,h)|obj([("file",text(f.to_string_lossy())),("sha256",text(h))])))),("horizons",arr(HORIZONS.map(|h|num(h as f64)))),("conditions",arr(CONDITIONS.map(text))),("planned_decisions",num((total_cells*(1+CONDITIONS.len()*HORIZONS.len())) as f64)),("scope",text("Exploratory paired intervention, one common prefix per history. No new inputs after split. All arms advance 24 virtual hours per cycle; normal native sleep budget vs forced deep vs no sleep. T0 is repeated across clones as generation control, not independent lives. Native readonly recall only. MCQ factual correctness is not an open-ended hallucination measure or official LoCoMo score."))]))?;
    let mut rows = Vec::new();
    let mut readouts = 0;
    let mut nights = 0;
    let mut report = obj([("status", text("running")), ("mock", Json::Bool(p.mock))]);
    write(&out.join("sleep.json"), &report)?;
    let result: Result<()> = (|| {
        // Exact cached qualification is validated before any HTTP.
        if resume.is_none() {
            for h in &histories {
                for c in &h.cells {
                    let r = assess(p, c.get("prompt")?.clone(), s(c, "expected")?)?;
                    log(
                        &out.join("qualification.jsonl"),
                        &merge(
                            &r,
                            &obj([("history", text(&h.id)), ("id", c.get("id")?.clone())]),
                        ),
                    )?;
                    if !p.mock && !flag(&r, "correct") {
                        return Err(
                            "faithful controlled qualification failed; no interventions performed"
                                .into(),
                        );
                    }
                }
            }
            for h in array(&fixture, "histories")? {
                for q in array(h, "probes")? {
                    let r = assess(
                        p,
                        factual_prompt(
                            q,
                            arr(array(h, "events")?
                                .iter()
                                .map(|e| e.get("text").unwrap().clone())),
                        )?,
                        s(q, "expected")?,
                    )?;
                    log(
                        &out.join("qualification.jsonl"),
                        &merge(
                            &r,
                            &obj([
                                ("history", h.get("id")?.clone()),
                                ("id", q.get("id")?.clone()),
                            ]),
                        ),
                    )?;
                    if !p.mock && !flag(&r, "correct") {
                        return Err("faithful LoCoMo qualification failed; inspect distractors/ambiguity; no interventions performed".into());
                    }
                }
            }
        } else {
            super::progress::line("RESUME: reusing 20 validated faithful answers");
        }
        for h in array(&fixture, "histories")? {
            let id = s(h, "id")?;
            let events = array(h, "events")?
                .iter()
                .map(|e| Ok(merge(e, &obj([("recall_query", e.get("text")?.clone())]))))
                .collect::<Result<Vec<_>>>()?;
            let memory = super::sleep_resume::prepare(resume, out, id, &events, &config)?;
            let cells = array(h, "probes")?
                .iter()
                .map(|q| {
                    Ok(obj([
                        ("id", q.get("id")?.clone()),
                        ("kind", q.get("kind")?.clone()),
                        ("query", q.get("question")?.clone()),
                        ("expected", q.get("expected")?.clone()),
                        ("prompt", factual_prompt(q, arr([]))?),
                    ]))
                })
                .collect::<Result<Vec<_>>>()?;
            source_hashes.push((memory.path.clone(), file_hash(&memory.path)?));
            histories.push(History {
                id: id.into(),
                domain: "locomo",
                memory,
                cells,
            });
        }
        if prepare_only {
            return Ok(());
        }
        for h in &histories {
            let hash = file_hash(&h.memory.path)?;
            let mut arms = Vec::new();
            for condition in CONDITIONS {
                let arm = h
                    .memory
                    .copy(out.join(format!("{}-{condition}.vault", h.id)))?;
                if file_hash(&arm.path)? != hash {
                    return Err("clone identity failed".into());
                }
                arms.push((condition, arm));
            }
            for cycle in 0..=20 {
                if cycle > 0 {
                    // Counterbalance operation order. Condition labels never enter LLM prompts.
                    for offset in 0..3 {
                        let (condition, arm) = &arms[(cycle + offset) % 3];
                        arm.call("tick", obj([("hours", num(24.))]))?;
                        if *condition != "none" {
                            let before = arm.call("inspect", obj([]))?;
                            let after = arm
                                .call("sleep", obj([("deep", Json::Bool(*condition == "deep"))]))?;
                            nights += 1;
                            log(
                                &out.join("nights.jsonl"),
                                &obj([
                                    ("history", text(&h.id)),
                                    ("condition", text(*condition)),
                                    ("cycle", num(cycle as f64)),
                                    ("before", before),
                                    ("after", after),
                                ]),
                            )?;
                        }
                    }
                }
                let states = arms
                    .iter()
                    .map(|(_, a)| a.call("inspect", obj([])))
                    .collect::<Result<Vec<_>>>()?;
                check_clock(&states)?;
                if !HORIZONS.contains(&cycle) {
                    continue;
                }
                for offset in 0..3 {
                    let index = (cycle + offset) % 3;
                    let (condition, arm) = &arms[index];
                    super::progress::line(format!(
                        "SLEEP CHECKPOINT history={} condition={condition} cycle={cycle}/20",
                        h.id
                    ));
                    let arm_hash = file_hash(&arm.path)?;
                    log(
                        &out.join("states.jsonl"),
                        &obj([
                            ("history", text(&h.id)),
                            ("condition", text(*condition)),
                            ("cycle", num(cycle as f64)),
                            ("sha256", text(&arm_hash)),
                            ("metrics", state_metrics(&states[index])?),
                            ("state", states[index].clone()),
                        ]),
                    )?;
                    let mut shared = None;
                    for c in &h.cells {
                        let evidence = if h.domain == "controlled" && shared.is_some() {
                            shared.clone().unwrap()
                        } else {
                            let tmp = arm.copy(out.join("probe.vault"))?;
                            let (context, audit) = tmp.context(s(c, "query")?)?;
                            readouts += 1;
                            if file_hash(&tmp.path)? != arm_hash
                                || file_hash(&arm.path)? != arm_hash
                            {
                                return Err("readonly probe changed snapshot".into());
                            }
                            log(
                                &out.join("sleep-readouts.jsonl"),
                                &obj([
                                    ("history", text(&h.id)),
                                    ("condition", text(*condition)),
                                    ("cycle", num(cycle as f64)),
                                    ("id", c.get("id")?.clone()),
                                    ("snapshot_hash", text(&arm_hash)),
                                    ("context", context.clone()),
                                    ("audit", audit),
                                ]),
                            )?;
                            fs::remove_file(&tmp.path).map_err(|e| e.to_string())?;
                            context
                        };
                        if h.domain == "controlled" {
                            shared = Some(evidence.clone());
                        }
                        let mut prompt = c.get("prompt")?.clone();
                        prompt.put("past_observations", evidence);
                        let r = assess(p, prompt, s(c, "expected")?)?;
                        let row = merge(
                            &r,
                            &obj([
                                ("history", text(&h.id)),
                                ("domain", text(h.domain)),
                                ("condition", text(*condition)),
                                ("cycle", num(cycle as f64)),
                                ("id", c.get("id")?.clone()),
                                ("kind", c.get("kind")?.clone()),
                            ]),
                        );
                        log(&out.join("sleep-decisions.jsonl"), &row)?;
                        rows.push(row);
                    }
                    if file_hash(&arm.path)? != arm_hash {
                        return Err("decisions changed life".into());
                    }
                }
            }
        }
        Ok(())
    })();
    let intact = source_hashes
        .iter()
        .all(|(p, h)| file_hash(p).ok().as_ref() == Some(h));
    let result = result.and_then(|_| {
        if intact {
            Ok(())
        } else {
            Err("prefix or fixture changed".into())
        }
    });
    report.put(
        "status",
        text(if result.is_err() {
            "failed"
        } else if p.mock {
            "technical_rehearsal_complete"
        } else if prepare_only {
            "prepared"
        } else {
            "complete"
        }),
    );
    report.put(
        "error",
        result.as_ref().err().map(text).unwrap_or(Json::Null),
    );
    report.put("preparation_only", Json::Bool(prepare_only));
    if let Some(parent) = resume {
        let old = load(&parent.join("sleep.json"))?;
        let previous = n(&old, "http_attempts_total").or_else(|_| n(&old, "http_attempts"))?;
        report.put(
            "http_attempts_total",
            num(previous + transport::used() as f64),
        );
    }
    report.put("sources_unchanged", Json::Bool(intact));
    report.put("completed_decisions", num(rows.len() as f64));
    report.put("completed_readouts", num(readouts as f64));
    report.put("completed_nights", num(nights as f64));
    report.put("http_attempts", num(transport::used() as f64));
    report.put(
        "scores",
        arr(HORIZONS
            .into_iter()
            .flat_map(|cycle| {
                CONDITIONS.into_iter().flat_map(move |condition| {
                    ["controlled", "locomo"].map(move |domain| (cycle, condition, domain))
                })
            })
            .map(|(cycle, condition, domain)| {
                let r = rows
                    .iter()
                    .filter(|r| {
                        n(r, "cycle").ok() == Some(cycle as f64)
                            && s(r, "condition").ok() == Some(condition)
                            && s(r, "domain").ok() == Some(domain)
                    })
                    .collect::<Vec<_>>();
                obj([
                    ("cycle", num(cycle as f64)),
                    ("condition", text(condition)),
                    ("domain", text(domain)),
                    ("answered", num(r.len() as f64)),
                    (
                        "correct",
                        num(r.iter().filter(|r| flag(r, "correct")).count() as f64),
                    ),
                    (
                        "abstained",
                        num(r.iter().filter(|r| flag(r, "abstained")).count() as f64),
                    ),
                ])
            })),
    );
    write(
        &out.join("ancestry.json"),
        &arr(source_hashes
            .iter()
            .map(|(f, h)| obj([("file", text(f.to_string_lossy())), ("sha256", text(h))]))),
    )?;
    write(&out.join("sleep.json"), &report)?;
    super::progress::line(format!(
        "SLEEP RESULT status={} measured_decisions={} readouts={readouts} nights={nights} HTTP={}",
        s(&report, "status")?,
        rows.len(),
        transport::used()
    ));
    result
}
pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let mut flags = BTreeMap::new();
    let mut mock = false;
    let mut prepare_only = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--prepare-only" {
            prepare_only = true;
            i += 1;
            continue;
        }
        if args[i] == "--mock" {
            mock = true;
            i += 1;
            continue;
        }
        let k = &args[i];
        if ![
            "--source",
            "--locomo",
            "--out",
            "--config",
            "--llm",
            "--model",
            "--plug",
            "--max-http-calls",
            "--resume-from",
        ]
        .contains(&k.as_str())
        {
            return Err(format!("unknown option {k}"));
        }
        if flags
            .insert(
                k.clone(),
                args.get(i + 1).ok_or("missing option value")?.clone(),
            )
            .is_some()
        {
            return Err("duplicate option".into());
        }
        i += 2;
    }
    let source = PathBuf::from(flags.get("--source").ok_or("--source required")?);
    let fixture = PathBuf::from(flags.get("--locomo").ok_or("--locomo required")?);
    let out = PathBuf::from(flags.get("--out").ok_or("--out required")?);
    let limit = flags
        .get("--max-http-calls")
        .map(|v| v.parse::<usize>().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(3000);
    if !(1..=8000).contains(&limit) {
        return Err("budget must be 1..8000".into());
    }
    // Reuse prefix organ options, including optional co-recall. Only provider binding is refreshed.
    let design = load(&source.join("learning-design.json"))?;
    let mut config = design.get("config")?.clone();
    let binding = campaign_config(selmem::Config::get(), &flags, true, mock)?;
    for key in [
        "memory_backend",
        "lifecycle_backend",
        "reconstruction_policy",
        "memory_endpoint",
        "scorer_endpoint",
        "memory_model",
        "scorer_model",
        "memory_plug",
    ] {
        if let Ok(v) = binding.get(key) {
            config.put(key, v.clone());
        }
    }
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
    let _budget = transport::begin(&out, limit);
    run(
        &source,
        &fixture,
        &out,
        config,
        &mut p,
        flags.get("--resume-from").map(Path::new),
        prepare_only,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forced_deep_uses_native_night_and_preserves_parent() {
        let root = super::super::lifecycle_tests::root();
        fs::create_dir_all(&root).unwrap();
        let cfg = obj([
            ("memory_backend", text("rules")),
            ("associations_learn", Json::Bool(true)),
            ("associations_reconstruct", Json::Bool(true)),
            ("observation_evidence", Json::Bool(true)),
        ]);
        let parent = Memory::new("selmem_full", root.join("parent.vault"), &cfg);
        parent.init().unwrap();
        for i in 0..3 {
            parent
                .call(
                    "live",
                    obj([(
                        "event",
                        obj([
                            ("id", text(format!("o{i}"))),
                            ("text", text("Mira supplied six batteries to the workshop.")),
                            ("critical", Json::Bool(true)),
                        ]),
                    )]),
                )
                .unwrap();
        }
        let hash = file_hash(&parent.path).unwrap();
        let deep = parent.copy(root.join("deep.vault")).unwrap();
        let none = parent.copy(root.join("none.vault")).unwrap();
        for arm in [&deep, &none] {
            arm.call("tick", obj([("hours", num(24.))])).unwrap();
        }
        let state = deep
            .call("sleep", obj([("deep", Json::Bool(true))]))
            .unwrap();
        assert_eq!(s(state.get("night").unwrap(), "kind").unwrap(), "Deep");
        check_clock(&[state.clone(), none.call("inspect", obj([])).unwrap()]).unwrap();
        assert_eq!(
            n(&state_metrics(&state).unwrap(), "support_observations").unwrap(),
            3.
        );
        assert_eq!(file_hash(&parent.path).unwrap(), hash);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn factual_prompt_excludes_oracle_and_evidence_ids() {
        let q = obj([
            ("question", text("When?")),
            (
                "options",
                arr([obj([("id", text("a")), ("text", text("Tuesday"))])]),
            ),
            ("expected", text("a")),
            ("reference_answer", text("Tuesday")),
            ("evidence_ids", arr([text("D1:1")])),
        ]);
        let p = factual_prompt(&q, arr([])).unwrap();
        for k in ["expected", "reference_answer", "evidence_ids"] {
            assert!(p.get(k).is_err());
        }
    }
    #[test]
    fn unequal_virtual_time_is_rejected() {
        assert!(check_clock(&[obj([("clock", num(24.))]), obj([("clock", num(48.))])]).is_err());
    }
    #[test]
    fn support_metrics_count_observations_not_claim_copies() {
        let state = obj([
            ("traces", arr([])),
            ("axioms", arr([])),
            (
                "observation_evidence",
                arr([obj([("observations", num(12.))])]),
            ),
        ]);
        assert_eq!(
            n(&state_metrics(&state).unwrap(), "support_observations").unwrap(),
            12.
        );
    }
}
