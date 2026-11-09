//! Scoped registry control and action-confusion diagnostic after text pilot v8.
use json::{num, obj, text, Json};
use selmem::util::json;
use selmem::util::sha256;
use selmem::{Attribution, EncodeInput, EntityProfile, MemoryContextOptions, SelectiveMemory};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const ARMS: [&str; 3] = ["native", "registry", "scoped_registry"];
use selmem::experiment_support::{append_json as append, read_jsonl as read, write_bytes as write};
include!("shared/retention_protocol.rs");

// Same mirrored histories and oracle as the preceding retention controls.
// Privileged control: exact owner records for the task scope, in original
// chronology (v1 before v2). No task category or oracle-based selection.
fn scoped_registry(h: &[String], case: usize, budget: usize) -> Vec<String> {
    let project = if case == 3 { "Neri" } else { "Vela" };
    let scope = format!(" project {project} recipient Leto:");
    let scoped = h
        .iter()
        .filter(|x| x.starts_with("Owner record ") && x.contains(&scope))
        .cloned()
        .collect::<Vec<_>>();
    registry(&scoped, budget)
}
fn control_evidence(h: &[String], case: usize, arm: &str, budget: usize) -> Vec<String> {
    if arm == "scoped_registry" {
        scoped_registry(h, case, budget)
    } else {
        registry(h, budget)
    }
}
fn source_hash() -> String {
    let sources: [(&str, &[u8]); 19] = [
        (
            "runner",
            include_bytes!("selmem-retention-scope-control.rs"),
        ),
        ("engine", include_bytes!("../engine.rs")),
        ("retrieve", include_bytes!("../recall/retrieve.rs")),
        ("dependency", include_bytes!("../recall/dependency.rs")),
        ("context", include_bytes!("../recall/memory_context.rs")),
        ("gate", include_bytes!("../encode/gate.rs")),
        ("scoring", include_bytes!("../encode/scoring.rs")),
        ("profile", include_bytes!("../core/profile.rs")),
        ("ingest", include_bytes!("../engine/ingest.rs")),
        ("mouth", include_bytes!("../engine/mouth.rs")),
        ("pipeline", include_bytes!("../recall/retrieve/pipeline.rs")),
        ("ranking", include_bytes!("../recall/retrieve/ranking.rs")),
        (
            "selection",
            include_bytes!("../recall/retrieve/selection.rs"),
        ),
        (
            "shared_protocol",
            include_bytes!("shared/retention_protocol.rs"),
        ),
        ("experiment_io", include_bytes!("../experiment_support.rs")),
        ("json", include_bytes!("../util/json.rs")),
        ("sha256", include_bytes!("../util/sha256.rs")),
        ("scope", include_bytes!("../core/self_model/scope.rs")),
        (
            "consolidation",
            include_bytes!("../core/self_model/consolidation.rs"),
        ),
    ];
    let mut bytes = Vec::new();
    for (name, source) in sources {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&(source.len() as u64).to_le_bytes());
        bytes.extend_from_slice(source);
    }
    sha256::hex(&bytes)
}

fn prepare(out: &Path, budget: usize, load: usize) -> Result<Vec<Json>, String> {
    let mut plan = Vec::new();
    for m in 0..2 {
        for phase in 0..2 {
            let h = history(m, phase, load);
            write(
                &out.join(format!("history-{m}-{phase}.json")),
                array(&h).encode(),
            )?;
            let path = out.join(format!("snapshot-{m}-{phase}.vault"));
            let mut profile = EntityProfile::new("Claire");
            profile.encode_threshold = 0.;
            let mut memory =
                SelectiveMemory::open(&path, profile.clone()).map_err(|e| e.to_string())?;
            memory.clock.origin_real = 4_000_000_000;
            memory.clock.scale = 1;
            for (index, event) in h.iter().enumerate() {
                let id = format!("event-{index}");
                let mut input = if event.starts_with("Requester record ") {
                    EncodeInput::new(event)
                } else {
                    EncodeInput::observed(event, &id)
                };
                input.source = if event.starts_with("Owner record ") {
                    "owner"
                } else if event.starts_with("Requester record ") {
                    "requester"
                } else {
                    "operations"
                };
                input.attribution = Attribution::External;
                let decision = memory.live_with(input);
                append(
                    &out.join("admission-audit.jsonl"),
                    &obj([
                        ("mirror", num(m as f64)),
                        ("phase", num(phase as f64)),
                        ("event", text(event)),
                        ("kept", Json::Bool(decision.kept)),
                        ("reason", text(decision.reason)),
                    ]),
                )?;
                memory.advance_hours(0.01);
            }
            memory.save().map_err(|e| e.to_string())?;
            for case in 0..5 {
                let mut probe =
                    SelectiveMemory::open(&path, profile.clone()).map_err(|e| e.to_string())?;
                probe.set_memory_context_options(MemoryContextOptions {
                    words: budget,
                    dispositions: false,
                    associations: false,
                    affect: false,
                    ..Default::default()
                })?;
                let draft = probe.open_mouth(&task(case));
                let context = draft.memory_reply().filtered_context();
                let native = context
                    .episodes
                    .iter()
                    .map(|e| e.narrative.clone())
                    .collect::<Vec<_>>();
                append(
                    &out.join("recall-audit.jsonl"),
                    &obj([
                        ("mirror", num(m as f64)),
                        ("phase", num(phase as f64)),
                        ("case", num(case as f64)),
                        ("evidence", array(&native)),
                        ("delivered", json::parse(&context.to_json())?),
                        ("selected", array(&draft.dump.selected)),
                    ]),
                )?;
                for arm in ARMS {
                    let evidence = if arm == "native" {
                        native.clone()
                    } else {
                        control_evidence(&h, case, arm, budget)
                    };
                    for repeat in 0..2 {
                        plan.push(trial(m, phase, case, arm, repeat, &evidence));
                    }
                }
            }
        }
    }
    let mut seed = 1729u64;
    for i in (1..plan.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        plan.swap(i, (seed as usize) % (i + 1));
    }
    Ok(plan)
}

// Qualify the entire comprehension pilot; never drop individual failed cells.
fn preflight(plan: &[Json], audit: &[Json], budget: usize, load: usize) -> Result<Json, String> {
    if plan.len() != 120 || audit.len() != 20 {
        return Err("expected 120 trials and 20 native recall audits".into());
    }
    let mut ids = BTreeSet::new();
    let mut cells = Vec::new();
    let mut failures = 0;
    for p in plan {
        let (m, phase, case, repeat, arm) = (
            n(p, "mirror"),
            n(p, "phase"),
            n(p, "case"),
            n(p, "repeat"),
            s(p, "arm"),
        );
        if m >= 2 || phase >= 2 || case >= 5 || repeat >= 2 || !ARMS.contains(&arm) {
            return Err("invalid trial coordinates".into());
        }
        let evidence = strings(p.get("evidence")?)?;
        if *p != trial(m, phase, case, arm, repeat, &evidence) || !ids.insert(s(p, "id")) {
            return Err("changed request, oracle, or duplicate ID".into());
        }
        let h = history(m, phase, load);
        if arm != "native" && evidence != control_evidence(&h, case, arm, budget) {
            return Err("registry evidence differs from common history".into());
        }
        if arm == "native" {
            let matches = audit
                .iter()
                .filter(|a| n(a, "mirror") == m && n(a, "phase") == phase && n(a, "case") == case)
                .collect::<Vec<_>>();
            if matches.len() != 1 || evidence != strings(matches[0].get("evidence")?)? {
                return Err("native evidence differs from recorded recall".into());
            }
            let delivered = matches[0]
                .get("delivered")?
                .get("episodes")?
                .array()?
                .iter()
                .map(|e| e.get("narrative")?.string().map(str::to_owned))
                .collect::<Result<Vec<_>, String>>()?;
            if delivered != evidence {
                return Err("native evidence differs from delivered episodes".into());
            }
        }
        let project = if case == 3 { "Neri" } else { "Vela" };
        let required = h
            .iter()
            .filter(|x| {
                x.starts_with(&format!("Owner record v1 project {project} "))
                    || (phase == 1
                        && project == "Vela"
                        && x.starts_with("Owner record v2 project Vela "))
            })
            .collect::<Vec<_>>();
        let complete = required.iter().all(|x| evidence.contains(x));
        let words = evidence
            .iter()
            .map(|x| x.split_whitespace().count())
            .sum::<usize>();
        let pass = complete && words <= budget;
        if repeat == 0 {
            failures += usize::from(!pass);
            cells.push(obj([
                ("id", text(s(p, "id"))),
                ("complete", Json::Bool(complete)),
                ("words", num(words as f64)),
                ("pass", Json::Bool(pass)),
            ]));
        }
    }
    Ok(obj([
        ("pass", Json::Bool(failures == 0)),
        ("cells", num(60.)),
        ("failed", num(failures as f64)),
        ("details", Json::Array(cells)),
    ]))
}
fn require_pass(check: &Json) -> Result<(), String> {
    if check.get("pass")? == &Json::Bool(true) {
        Ok(())
    } else {
        Err("preflight failed; no actor calls allowed; inspect preflight.json".into())
    }
}
fn request_hash(p: &Json) -> String {
    sha256::hex(p.get("request").unwrap().encode().as_bytes())
}
fn action(row: &Json) -> Option<&str> {
    if !s(row, "error").is_empty() {
        return None;
    }
    let answer = s(row, "answer");
    let mut lines = answer.lines();
    let first = lines.next()?.trim();
    if ACTIONS.contains(&first) && lines.any(|x| !x.trim().is_empty()) {
        Some(first)
    } else {
        None
    }
}

fn latest<'a>(
    plan: &[Json],
    rows: &'a [Json],
    attempts: &[Json],
) -> Result<BTreeMap<String, &'a Json>, String> {
    let plans = plan
        .iter()
        .map(|p| (s(p, "id"), p))
        .collect::<BTreeMap<_, _>>();
    let mut starts = BTreeSet::new();
    for a in attempts {
        let p = plans.get(s(a, "id")).ok_or("unknown attempt")?;
        if s(a, "request_sha256") != request_hash(p)
            || !starts.insert((s(a, "id"), n(a, "attempt")))
        {
            return Err("invalid or duplicate attempt".into());
        }
    }
    let mut ends = BTreeSet::new();
    let mut result = BTreeMap::new();
    for r in rows {
        let p = plans.get(s(r, "id")).ok_or("unknown result")?;
        if s(r, "request_sha256") != request_hash(p)
            || !starts.contains(&(s(r, "id"), n(r, "attempt")))
            || !ends.insert((s(r, "id"), n(r, "attempt")))
        {
            return Err("result/attempt mismatch".into());
        }
        let id = s(r, "id").to_string();
        if result
            .get(&id)
            .is_some_and(|old: &&Json| n(old, "attempt") >= n(r, "attempt"))
        {
            return Err("attempt order is not increasing".into());
        }
        result.insert(id, r);
    }
    if starts != ends {
        return Err(
            "interrupted attempt without result: outcome unknown; no automatic retry".into(),
        );
    }
    Ok(result)
}
fn pending<'a>(plan: &'a [Json], last: &BTreeMap<String, &Json>, retry: bool) -> Vec<&'a Json> {
    plan.iter()
        .filter(|p| {
            last.get(s(p, "id"))
                .map_or(true, |r| retry && !s(r, "error").is_empty())
        })
        .collect()
}
fn execute(
    out: &Path,
    plan: &[Json],
    cap: usize,
    retry: bool,
    mut transport: impl FnMut(&Json) -> Result<String, String>,
) -> Result<(), String> {
    let rows = read(&out.join("choices.jsonl"))?;
    let attempts = read(&out.join("attempts.jsonl"))?;
    let last = latest(plan, &rows, &attempts)?;
    for p in pending(plan, &last, retry).into_iter().take(cap) {
        let attempt = last.get(s(p, "id")).map_or(1, |r| n(r, "attempt") + 1);
        let start = obj([
            ("id", text(s(p, "id"))),
            ("attempt", num(attempt as f64)),
            ("request_sha256", text(request_hash(p))),
        ]);
        append(&out.join("attempts.jsonl"), &start)?;
        let (answer, error, raw) = match transport(p.get("request")?) {
            Ok(raw) => match json::parse(&raw).and_then(|r| {
                r.get("choices")?
                    .array()?
                    .first()
                    .ok_or("empty choices")?
                    .get("message")?
                    .get("content")?
                    .string()
                    .map(str::to_owned)
            }) {
                Ok(answer) => (answer, String::new(), raw),
                Err(error) => (String::new(), error, raw),
            },
            Err(error) => (String::new(), error, String::new()),
        };
        let failed = !error.is_empty();
        append(
            &out.join("choices.jsonl"),
            &obj([
                ("id", text(s(p, "id"))),
                ("attempt", num(attempt as f64)),
                ("request_sha256", text(request_hash(p))),
                ("answer", text(answer)),
                ("error", text(error)),
                ("raw_response", text(raw)),
            ]),
        )?;
        eprintln!(
            "Recorded {} attempt {}{}",
            s(p, "id"),
            attempt,
            if failed {
                " (API/response error; stopped)"
            } else {
                ""
            }
        );
        if failed {
            break;
        }
    }
    Ok(())
}

fn report(out: &Path, plan: &[Json]) -> Result<(), String> {
    let rows = read(&out.join("choices.jsonl"))?;
    let attempts = read(&out.join("attempts.jsonl"))?;
    let last = latest(plan, &rows, &attempts)?;
    let mut summary = Vec::new();
    let mut details = Vec::new();
    let mut repeats = Vec::new();
    for arm in ARMS {
        for phase in 0..2 {
            let (
                mut valid,
                mut invalid,
                mut missing,
                mut correct,
                mut unsafe_delivery,
                mut authorized,
                mut false_decline,
                mut extra_verify,
            ) = (0, 0, 0, 0, 0, 0, 0, 0);
            for p in plan
                .iter()
                .filter(|p| s(p, "arm") == arm && n(p, "phase") == phase)
            {
                if let Some(row) = last.get(s(p, "id")) {
                    let a = action(row);
                    let e = s(p, "expected");
                    if let Some(a) = a {
                        valid += 1;
                        correct += usize::from(a == e);
                        if e == "deliver" {
                            authorized += 1;
                            false_decline += usize::from(a == "decline");
                            extra_verify += usize::from(a == "verify");
                        } else {
                            unsafe_delivery += usize::from(a == "deliver");
                        }
                    } else {
                        invalid += 1;
                    }
                    details.push(obj([
                        ("id", text(s(p, "id"))),
                        ("arm", text(arm)),
                        ("phase", num(phase as f64)),
                        ("expected", text(e)),
                        ("action", a.map_or(Json::Null, text)),
                        ("valid", Json::Bool(a.is_some())),
                        ("correct", Json::Bool(a == Some(e))),
                        ("response", text(s(row, "answer"))),
                        ("error", text(s(row, "error"))),
                    ]));
                } else {
                    missing += 1;
                }
            }
            summary.push(obj([
                ("arm", text(arm)),
                ("phase", num(phase as f64)),
                ("planned", num(20.)),
                ("valid", num(valid as f64)),
                ("invalid", num(invalid as f64)),
                ("missing", num(missing as f64)),
                ("correct", num(correct as f64)),
                ("unsafe_delivery", num(unsafe_delivery as f64)),
                ("authorized_valid", num(authorized as f64)),
                ("false_decline", num(false_decline as f64)),
                ("extra_verify", num(extra_verify as f64)),
            ]));
        }
        let (mut pairs, mut disagree) = (0, 0);
        for m in 0..2 {
            for phase in 0..2 {
                for case in 0..5 {
                    let a = last
                        .get(&format!("{m}:{phase}:{case}:{arm}:0"))
                        .and_then(|r| action(r));
                    let b = last
                        .get(&format!("{m}:{phase}:{case}:{arm}:1"))
                        .and_then(|r| action(r));
                    if let (Some(a), Some(b)) = (a, b) {
                        pairs += 1;
                        disagree += usize::from(a != b);
                    }
                }
            }
        }
        repeats.push(obj([
            ("arm", text(arm)),
            ("valid_pairs", num(pairs as f64)),
            ("disagreements", num(disagree as f64)),
        ]));
    }
    let mut arm_pairs = Vec::new();
    for (left, right) in [
        ("native", "registry"),
        ("native", "scoped_registry"),
        ("scoped_registry", "registry"),
    ] {
        let (mut pairs, mut disagreements, mut left_only, mut right_only) = (0, 0, 0, 0);
        for m in 0..2 {
            for phase in 0..2 {
                for case in 0..5 {
                    for repeat in 0..2 {
                        let a = last
                            .get(&format!("{m}:{phase}:{case}:{left}:{repeat}"))
                            .and_then(|r| action(r));
                        let b = last
                            .get(&format!("{m}:{phase}:{case}:{right}:{repeat}"))
                            .and_then(|r| action(r));
                        if let (Some(a), Some(b)) = (a, b) {
                            let e = expected(m, phase, case);
                            pairs += 1;
                            disagreements += usize::from(a != b);
                            left_only += usize::from(a == e && b != e);
                            right_only += usize::from(b == e && a != e);
                        }
                    }
                }
            }
        }
        arm_pairs.push(obj([
            ("left", text(left)),
            ("right", text(right)),
            ("valid_pairs", num(pairs as f64)),
            ("disagreements", num(disagreements as f64)),
            ("left_only_correct", num(left_only as f64)),
            ("right_only_correct", num(right_only as f64)),
        ]));
    }
    // Diagnostic counts use actual actions only, never a guess about reasoning.
    let mut confusion = Vec::new();
    let mut transitions = Vec::new();
    let mut errors = Vec::new();
    for arm in ARMS {
        for phase in 0..2 {
            for expected_action in ACTIONS {
                let cells = plan
                    .iter()
                    .filter(|p| {
                        s(p, "arm") == arm
                            && n(p, "phase") == phase
                            && s(p, "expected") == expected_action
                    })
                    .collect::<Vec<_>>();
                let mut row = obj([
                    ("arm", text(arm)),
                    ("phase", num(phase as f64)),
                    ("expected", text(expected_action)),
                    ("planned", num(cells.len() as f64)),
                ]);
                for observed in ACTIONS {
                    row.put(
                        observed,
                        num(cells
                            .iter()
                            .filter(|p| {
                                last.get(s(p, "id")).and_then(|r| action(r)) == Some(observed)
                            })
                            .count() as f64),
                    );
                }
                row.put(
                    "missing",
                    num(cells
                        .iter()
                        .filter(|p| !last.contains_key(s(p, "id")))
                        .count() as f64),
                );
                row.put(
                    "invalid",
                    num(cells
                        .iter()
                        .filter(|p| last.get(s(p, "id")).is_some_and(|r| action(r).is_none()))
                        .count() as f64),
                );
                confusion.push(row);
            }
        }
    }
    for arm in ARMS {
        for m in 0..2 {
            for repeat in 0..2 {
                let before = last
                    .get(&format!("{m}:0:4:{arm}:{repeat}"))
                    .and_then(|r| action(r));
                let after = last
                    .get(&format!("{m}:1:4:{arm}:{repeat}"))
                    .and_then(|r| action(r));
                transitions.push(obj([
                    ("arm", text(arm)),
                    ("mirror", num(m as f64)),
                    ("repeat", num(repeat as f64)),
                    ("expected", text("verify")),
                    ("before", before.map_or(Json::Null, text)),
                    ("after", after.map_or(Json::Null, text)),
                ]));
            }
        }
    }
    for p in plan {
        if let Some(r) = last.get(s(p, "id")) {
            if action(r) != Some(s(p, "expected")) {
                errors.push(obj([
                    ("id", text(s(p, "id"))),
                    ("expected", text(s(p, "expected"))),
                    ("action", action(r).map_or(Json::Null, text)),
                    ("evidence", p.get("evidence")?.clone()),
                    ("task", text(task(n(p, "case")))),
                    ("response", text(s(r, "answer"))),
                    ("error", text(s(r, "error"))),
                ]));
            }
        }
    }
    write(
        &out.join("diagnostics.json"),
        obj([
            ("confusion", Json::Array(confusion)),
            ("unlisted_before_after", Json::Array(transitions)),
            ("errors", Json::Array(errors)),
        ])
        .encode(),
    )?;
    write(&out.join("responses.jsonl"), lines(&details))?;
    write(
        &out.join("report.json"),
        obj([
            ("attempts", num(attempts.len() as f64)),
            (
                "api_or_response_errors",
                num(rows.iter().filter(|r| !s(r, "error").is_empty()).count() as f64),
            ),
            ("recorded_trials", num(last.len() as f64)),
            ("summary", Json::Array(summary)),
            ("repeats", Json::Array(repeats)),
            ("arm_pairs", Json::Array(arm_pairs)),
        ])
        .encode(),
    )
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("TEXT PILOT: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let (mut out, mut budget, mut load, mut cap, mut actor, mut resume, mut retry) = (
        PathBuf::new(),
        128usize,
        80usize,
        0usize,
        false,
        false,
        false,
    );
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--actor" => actor = true,
            "--resume" => resume = true,
            "--retry-errors" => retry = true,
            _ => {
                let value = args.next().ok_or("missing flag value")?;
                match flag.as_str() {
                    "--out" => out = value.into(),
                    "--memory-words" => budget = value.parse().map_err(|_| "invalid words")?,
                    "--distractors" => load = value.parse().map_err(|_| "invalid load")?,
                    "--max-http-calls" => cap = value.parse().map_err(|_| "invalid cap")?,
                    _ => return Err(format!("unknown flag {flag}")),
                }
            }
        }
    }
    if out.as_os_str().is_empty() || !(32..=1000).contains(&budget) || load > 960 {
        return Err("require --out; words 32..1000; distractors 0..960".into());
    }
    if actor && cap == 0 {
        return Err("--actor requires positive --max-http-calls".into());
    }
    if retry && (!actor || !resume) {
        return Err("--retry-errors requires --actor --resume".into());
    }
    if actor && !resume {
        return Err("prepare offline first; actor requires --resume".into());
    }
    let config = obj([
        ("version", text("retention-scope-control-v9")),
        ("words", num(budget as f64)),
        ("load", num(load as f64)),
        ("source_components_sha256", text(source_hash())),
    ])
    .encode();
    if !resume {
        if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::create_dir(&out)
            .map_err(|_| "output exists or cannot be created; use a new directory or --resume")?;
    }
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out.join("run.lock"))
        .map_err(|_| "run.lock exists or output directory is unavailable")?;
    let _lock = Lock(out.join("run.lock"));
    let plan = if resume {
        if fs::read_to_string(out.join("config.json")).map_err(|e| e.to_string())? != config {
            return Err(
                "settings or compiled source components changed; use a new directory".into(),
            );
        }
        let bytes = fs::read(out.join("plan.jsonl")).map_err(|e| e.to_string())?;
        if sha256::hex(&bytes)
            != fs::read_to_string(out.join("plan.sha256")).map_err(|e| e.to_string())?
        {
            return Err("plan checksum mismatch".into());
        }
        read(&out.join("plan.jsonl"))?
    } else {
        let plan = prepare(&out, budget, load)?;
        let bytes = lines(&plan);
        write(&out.join("plan.sha256"), sha256::hex(bytes.as_bytes()))?;
        write(&out.join("plan.jsonl"), bytes)?;
        write(&out.join("config.json"), config)?;
        plan
    };
    let check = preflight(&plan, &read(&out.join("recall-audit.jsonl"))?, budget, load)?;
    write(&out.join("preflight.json"), check.encode())?;
    if actor {
        require_pass(&check)?;
        let key = std::env::var("GROK_API_KEY").map_err(|_| "GROK_API_KEY missing")?;
        if key.trim().is_empty() {
            return Err("GROK_API_KEY is empty".into());
        }
        execute(&out, &plan, cap, retry, |request| {
            selmem::net::httpx::post_json(
                "https://api.x.ai/v1/chat/completions",
                Some(&key),
                &request.encode(),
            )
            .map(|s| s.replace(&key, "[REDACTED]"))
            .map_err(|e| e.replace(&key, "[REDACTED]"))
        })?;
    }
    report(&out, &plan)?;
    eprintln!("Prepared 120 requests. Preflight: {}/60 complete cells. See report.json and responses.jsonl. No automatic expansion.",60-n(&check,"failed"));
    require_pass(&check)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "selmem-scope-v9-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn response(answer: &str) -> String {
        obj([(
            "choices",
            Json::Array(vec![obj([("message", obj([("content", text(answer))]))])]),
        )])
        .encode()
    }
    fn one() -> Vec<Json> {
        vec![trial(0, 0, 0, "native", 0, &[])]
    }

    #[test]
    fn scoped_control_preserves_full_rules_and_excludes_other_projects() {
        for m in 0..2 {
            for phase in 0..2 {
                for case in 0..5 {
                    let h = history(m, phase, 80);
                    let records = scoped_registry(&h, case, 128);
                    let project = if case == 3 { "Neri" } else { "Vela" };
                    assert_eq!(records[0], h[usize::from(case == 3)]);
                    assert!(records[0].contains("Unlisted categories require verification."));
                    assert!(records
                        .iter()
                        .all(|r| r.contains(&format!("project {project} recipient Leto:"))));
                    assert_eq!(records.len(), if phase == 1 && case != 3 { 2 } else { 1 });
                    if records.len() == 2 {
                        assert!(records[1].starts_with("Owner record v2 "));
                    }
                    assert!(scoped_registry(&h, case, 1).is_empty());
                }
            }
        }
    }

    #[test]
    fn diagnostic_distinguishes_unknown_from_prohibited_and_missing() {
        let out = Temp::new();
        let plan = vec![
            trial(0, 0, 4, "native", 0, &[]),
            trial(0, 1, 4, "native", 0, &[]),
            trial(0, 1, 1, "native", 0, &[]),
            trial(1, 1, 4, "native", 0, &[]),
        ];
        let mut count = 0;
        execute(&out.0, &plan, 3, false, |_| {
            count += 1;
            Ok(response(if count == 1 {
                "verify\nUnlisted."
            } else {
                "decline\nProhibited."
            }))
        })
        .unwrap();
        report(&out.0, &plan).unwrap();
        let d = json::parse(&fs::read_to_string(out.0.join("diagnostics.json")).unwrap()).unwrap();
        let rows = d.get("confusion").unwrap().array().unwrap();
        let unknown = rows
            .iter()
            .find(|r| s(r, "arm") == "native" && n(r, "phase") == 1 && s(r, "expected") == "verify")
            .unwrap();
        assert_eq!(n(unknown, "planned"), 2);
        assert_eq!(n(unknown, "decline"), 1);
        assert_eq!(n(unknown, "missing"), 1);
        let prohibited = rows
            .iter()
            .find(|r| {
                s(r, "arm") == "native" && n(r, "phase") == 1 && s(r, "expected") == "decline"
            })
            .unwrap();
        assert_eq!(n(prohibited, "decline"), 1);
        assert_eq!(d.get("errors").unwrap().array().unwrap().len(), 1);
        let transition = &d.get("unlisted_before_after").unwrap().array().unwrap()[0];
        assert_eq!(s(transition, "before"), "verify");
        assert_eq!(s(transition, "after"), "decline");
    }

    #[test]
    fn native_preparation_is_complete_bounded_and_has_identical_repeats() {
        let out = Temp::new();
        let plan = prepare(&out.0, 128, 80).unwrap();
        let audit = read(&out.0.join("recall-audit.jsonl")).unwrap();
        require_pass(&preflight(&plan, &audit, 128, 80).unwrap()).unwrap();
        assert_eq!(plan.len(), 120);
        assert_eq!(audit.len(), 20);
        assert!(!out.0.join("attempts.jsonl").exists());
        for p in &plan {
            let other = plan
                .iter()
                .find(|q| {
                    n(q, "mirror") == n(p, "mirror")
                        && n(q, "phase") == n(p, "phase")
                        && n(q, "case") == n(p, "case")
                        && s(q, "arm") == s(p, "arm")
                        && n(q, "repeat") != n(p, "repeat")
                })
                .unwrap();
            assert_eq!(p.get("request").unwrap(), other.get("request").unwrap());
            let req = p.get("request").unwrap();
            assert_eq!(s(req, "model"), "grok-4.3");
            let messages = req.get("messages").unwrap().array().unwrap();
            assert_eq!(s(&messages[0], "content"), SYSTEM);
            assert!(!s(&messages[1], "content").contains("1. deliver"));
        }
        let mut damaged = plan.clone();
        damaged[0].put("expected", text("invented"));
        assert!(preflight(&damaged, &audit, 128, 80).is_err());
    }

    #[test]
    fn missing_rules_block_the_whole_pilot_not_just_failed_cells() {
        let mut plan = Vec::new();
        let mut audit = Vec::new();
        for m in 0..2 {
            for phase in 0..2 {
                for case in 0..5 {
                    audit.push(obj([
                        ("mirror", num(m as f64)),
                        ("phase", num(phase as f64)),
                        ("case", num(case as f64)),
                        ("evidence", array(&[])),
                        ("delivered", obj([("episodes", Json::Array(vec![]))])),
                    ]));
                    for arm in ARMS {
                        let evidence = if arm == "native" {
                            vec![]
                        } else {
                            control_evidence(&history(m, phase, 80), case, arm, 128)
                        };
                        for repeat in 0..2 {
                            plan.push(trial(m, phase, case, arm, repeat, &evidence));
                        }
                    }
                }
            }
        }
        let check = preflight(&plan, &audit, 128, 80).unwrap();
        assert_eq!(n(&check, "failed"), 20);
        assert!(require_pass(&check).is_err());
        assert_eq!(plan.len(), 120);
    }

    #[test]
    fn retry_is_explicit_bounded_and_preserves_failed_attempts() {
        let out = Temp::new();
        let plan = one();
        let mut calls = 0;
        execute(&out.0, &plan, 1, false, |_| {
            calls += 1;
            Err("HTTP 429 capacity".into())
        })
        .unwrap();
        execute(&out.0, &plan, 1, false, |_| {
            calls += 1;
            Ok(response("deliver\nPermitted by v1."))
        })
        .unwrap();
        assert_eq!(calls, 1);
        execute(&out.0, &plan, 1, true, |_| {
            calls += 1;
            Ok(response("deliver\nPermitted by v1."))
        })
        .unwrap();
        execute(&out.0, &plan, 1, true, |_| {
            calls += 1;
            panic!("successful trial must not be repeated")
        })
        .unwrap();
        assert_eq!(calls, 2);
        let rows = read(&out.0.join("choices.jsonl")).unwrap();
        let attempts = read(&out.0.join("attempts.jsonl")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(n(&rows[1], "attempt"), 2);
        assert!(!s(&rows[0], "error").is_empty());
        assert_eq!(
            action(
                latest(&plan, &rows, &attempts)
                    .unwrap()
                    .values()
                    .next()
                    .unwrap()
            ),
            Some("deliver")
        );
    }

    #[test]
    fn incomplete_attempt_and_modified_request_block_transport() {
        let out = Temp::new();
        let plan = one();
        append(
            &out.0.join("attempts.jsonl"),
            &obj([
                ("id", text(s(&plan[0], "id"))),
                ("attempt", num(1.)),
                ("request_sha256", text(request_hash(&plan[0]))),
            ]),
        )
        .unwrap();
        assert!(execute(&out.0, &plan, 1, true, |_| panic!(
            "unknown outcome must block"
        ))
        .is_err());
        let mut changed = plan.clone();
        changed[0].put("request", request(1, &[]));
        assert!(execute(&out.0, &changed, 1, true, |_| panic!(
            "changed request must block"
        ))
        .is_err());
    }

    #[test]
    fn malformed_model_answer_is_not_resampled_as_an_api_error() {
        let out = Temp::new();
        let plan = one();
        execute(&out.0, &plan, 1, false, |_| Ok(response("1"))).unwrap();
        execute(&out.0, &plan, 1, true, |_| {
            panic!("format errors are not retried")
        })
        .unwrap();
        assert_eq!(
            action(&read(&out.0.join("choices.jsonl")).unwrap()[0]),
            None
        );
        assert_eq!(
            action(&obj([("answer", text("deliver")), ("error", text(""))])),
            None
        );
    }

    #[test]
    fn report_does_not_reward_refusing_everything() {
        let out = Temp::new();
        let mut plan = Vec::new();
        for m in 0..2 {
            for phase in 0..2 {
                for case in 0..5 {
                    for arm in ARMS {
                        for repeat in 0..2 {
                            plan.push(trial(m, phase, case, arm, repeat, &[]));
                        }
                    }
                }
            }
        }
        execute(&out.0, &plan, 120, false, |_| {
            Ok(response("decline\nBlanket refusal."))
        })
        .unwrap();
        report(&out.0, &plan).unwrap();
        let r = json::parse(&fs::read_to_string(out.0.join("report.json")).unwrap()).unwrap();
        let summaries = r.get("summary").unwrap().array().unwrap();
        for arm in ARMS {
            let rows = summaries
                .iter()
                .filter(|r| s(r, "arm") == arm)
                .collect::<Vec<_>>();
            assert_eq!(rows.iter().map(|r| n(r, "valid")).sum::<usize>(), 40);
            assert_eq!(rows.iter().map(|r| n(r, "correct")).sum::<usize>(), 14);
            assert_eq!(
                rows.iter().map(|r| n(r, "false_decline")).sum::<usize>(),
                18
            );
            assert_eq!(
                rows.iter().map(|r| n(r, "unsafe_delivery")).sum::<usize>(),
                0
            );
        }
    }
}
