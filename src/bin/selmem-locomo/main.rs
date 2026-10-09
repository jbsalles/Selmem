//! LoCoMo-derived paired pilot. Rust-only orchestration; no Python or crate dependencies.
mod json;
mod memory;
mod report;
mod reasoning;
mod strategy_report;
mod sha256;
mod preflight;
mod context;
mod sampling;
use json::{num, obj, text, Json};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
const PROTOCOL: &str = include_str!("../../../data/locomo_strategy.json");
// Immutable provenance label for the frozen organ; not a CLI or directory version.
const ORGAN_REVISION: &str = "whole-context-topic-controls-v6";

struct Options {
    dry: bool,
    panel: String,
    plug: Option<String>,
    model: Option<String>,
    config: Option<PathBuf>,
    repeats: Option<usize>,
    temperature: Option<f64>,
    exploratory: bool,
    max_calls: usize,
    dataset: PathBuf,
    output: PathBuf,
    reuse_memories: Option<PathBuf>,
}
fn options() -> Result<Options, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let mut o = Options {
        dry: false,
        panel: "all".into(),
        plug: None,
        model: None,
        config: None,
        repeats: None,
        temperature: None,
        exploratory: false,
        max_calls: 300,
        dataset: ".cache/locomo10.json".into(),
        reuse_memories: None,
        output: format!("experiments/strategy/{stamp}-{}", std::process::id()).into(),
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => o.dry = true,
            "--wow" => {}, // Compatibility alias: strategy is now the only experiment.
            "--exploratory" => o.exploratory = true,
            "--temperature" => {
                let t: f64 = args.next().ok_or("missing value for --temperature")?.parse().map_err(|_| "invalid temperature")?;
                if !t.is_finite() || !(0.0..=2.0).contains(&t) { return Err("temperature must be between 0 and 2".into()); }
                o.temperature = Some(t);
            }
            "--panel" => { o.panel = args.next().ok_or("missing value for --panel")?; }
            "--reuse-memories" => {
                o.reuse_memories = Some(args.next().ok_or("missing value for --reuse-memories")?.into());
            }
            "--help" | "-h" => {
                println!("Sampling: --temperature 0 for a stability check; use --panel all --repeats 5 for controls. Paid strategy comparisons require --panel all and >=3 repeats unless --exploratory is explicitly set. Text distances are diagnostics, not behavioral evidence.");
                println!("Strategy experiment: [--panel all|core|controls]. Default: all 10 conditions, 6 probes, 5 repeats, 300-call ceiling. Core/controls require --exploratory for paid calls. --wow is a compatibility alias. Reuse only completed snapshots from the same protocol and frozen organ with --reuse-memories DIR.");
                println!("cargo run --release --bin selmem-locomo -- [--dry-run] [--config FILE] [--plug NAME] [--model MODEL] [--repeats N] [--max-calls N] [--dataset FILE] [--output NEW_DIR]\nLLM configuration: existing .selmem / SELMEM_CONFIG mechanism. --plug and --model are optional overrides. Default: 5 repeats, 300-call ceiling; --dry-run makes zero calls.");
                std::process::exit(0);
            }
            "--plug" | "--model" | "--config" | "--repeats" | "--max-calls" | "--dataset"
            | "--output" => {
                let v = args
                    .next()
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                match arg.as_str() {
                    "--plug" => o.plug = Some(v),
                    "--model" => o.model = Some(v),
                    "--config" => o.config = Some(v.into()),
                    "--repeats" => o.repeats = Some(v.parse().map_err(|_| "invalid repeats")?),
                    "--max-calls" => o.max_calls = v.parse().map_err(|_| "invalid max-calls")?,
                    "--dataset" => o.dataset = v.into(),
                    _ => o.output = v.into(),
                }
            }
            _ => return Err(format!("unknown option {arg}")),
        }
    }
    if !["core", "controls", "all"].contains(&o.panel.as_str()) {
        return Err("panel must be core, controls or all".into());
    }
    Ok(o)
}
fn configured_mouth(
    config: &selmem::Config,
    model: Option<String>,
    plug: Option<String>,
) -> Result<selmem::config::Mouth, String> {
    let mouth = config
        .mouth(None, model, plug, "")?
        .ok_or("configure an LLM in .selmem (default_llm or llm); no rule fallback")?;
    if mouth.model.trim().is_empty() {
        return Err("configure a model in .selmem or use --model".into());
    }
    // Credentials, endpoint and defaults all follow Config::mouth, including
    // custom plugs and local endpoints that do not need an API key.
    Ok(mouth)
}
const STRATEGY_CONDITIONS: &[&str] = &[
    "full_a", "full_b", "transplant_a_from_b", "transplant_b_from_a",
    "nosleep_a", "nosleep_b", "full_null_a", "full_null_b", "raw_a", "raw_b",
];

fn in_panel(name: &str, panel: &str) -> bool {
    if !STRATEGY_CONDITIONS.contains(&name) { return false; }
    let core = ["full_a", "full_b", "transplant_a_from_b", "transplant_b_from_a"].contains(&name);
    match panel { "core" => core, "controls" => !core, "all" => true, _ => false }
}

fn panel_condition_count(panel: &str) -> usize {
    STRATEGY_CONDITIONS.iter().filter(|name| in_panel(name, panel)).count()
}
#[cfg(test)]
fn context(r: &Json, budget: usize) -> Result<String, String> {
    Ok(context::build(r, budget)?.text)
}
fn dataset(path: &Path, p: &Json) -> Result<Json, String> {
    let expected = p.get("dataset_sha256")?.string()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        println!("Downloading official LoCoMo text dataset...");
        let result = Command::new("curl")
            .args([
                "-fLsS",
                "--max-time",
                "60",
                "--max-filesize",
                "16777216",
                p.get("dataset_url")?.string()?,
            ])
            .output()
            .map_err(|_| "curl is required for HTTPS")?;
        if !result.status.success() {
            return Err("LoCoMo download failed; use --dataset with the official file".into());
        }
        if sha256::hex(&result.stdout) != expected {
            return Err("dataset checksum mismatch".into());
        }
        fs::write(path, &result.stdout).map_err(|e| e.to_string())?;
    }
    let raw = fs::read(path).map_err(|e| e.to_string())?;
    if raw.len() > 16 * 1024 * 1024 || sha256::hex(&raw) != expected {
        return Err("dataset checksum mismatch".into());
    }
    let j = json::parse(std::str::from_utf8(&raw).map_err(|_| "invalid UTF-8 dataset")?)?;
    j.array()?
        .iter()
        .find(|s| {
            s.get("sample_id").and_then(Json::string).ok()
                == p.get("sample_id").and_then(Json::string).ok()
        })
        .cloned()
        .ok_or_else(|| "sample_id not found".into())
}
fn chat_request(
    url: &str, model: &str, p: &Json, ctx: &str, question: &str, reasoning_effort: &str,
) -> Result<Json, String> {
    Ok(obj([
        ("model", text(model)),
        ("reasoning_effort", text(reasoning_effort)),
        ("temperature", p.get("temperature")?.clone()),
        (
            if url.contains("api.x.ai/") {
                "max_tokens"
            } else {
                "max_completion_tokens"
            },
            p.get("max_completion_tokens")?.clone(),
        ),
        (
            "messages",
            Json::Array(vec![
                obj([
                    ("role", text("system")),
                    ("content", p.get("system")?.clone()),
                ]),
                obj([
                    ("role", text("user")),
                    ("content", text(format!("{ctx}\n\nQuestion:\n{question}"))),
                ]),
            ]),
        ),
    ]))
}

fn chat(
    url: &str,
    key: Option<&str>,
    model: &str,
    p: &Json,
    ctx: &str,
    question: &str,
    reasoning_effort: &str,
) -> Result<(String, Json, String, Json), String> {
    let body = chat_request(url, model, p, ctx, question, reasoning_effort)?;
    // Transport errors may echo provider bodies. Keep them out of experiment artifacts.
    let raw = selmem::net::httpx::post_json(url, key, &body.encode()).map_err(|_| {
        "LLM request failed: verify key, model, quota and parameter support; no fallback or retry"
    })?;
    decode_reply(&raw, model)
}
fn decode_reply(raw: &str, requested: &str) -> Result<(String, Json, String, Json), String> {
    let result = json::parse(raw).map_err(|_| "provider returned invalid JSON")?;
    let choice = result
        .get("choices")
        .and_then(Json::array)
        .ok()
        .and_then(|a| a.first())
        .ok_or("provider returned no choice; no rule fallback")?;
    let answer = choice.get("message")?.get("content")?.string()?;
    if answer.trim().is_empty() {
        return Err("provider returned empty text; no rule fallback".into());
    }
    Ok((
        answer.into(),
        result.get("usage").cloned().unwrap_or(Json::Null),
        result
            .get("model")
            .and_then(Json::string)
            .unwrap_or(requested)
            .into(),
        choice.get("finish_reason").cloned().unwrap_or(Json::Null),
    ))
}
type ProviderReply = (String, Json, String, Json);

// A provider failure is a missing observation, never a fabricated model answer.
fn keep_provider_reply(
    result: Result<ProviderReply, String>, out: &Path, manifest: &mut Json,
    failures: &mut Vec<Json>, mut call: Json,
) -> Result<Option<ProviderReply>, String> {
    let attempted = manifest.get("attempted_calls")?.number()? + 1.0;
    manifest.put("attempted_calls", num(attempted));
    match result {
        Ok(reply) => {
            let completed = manifest.get("completed_calls")?.number()? + 1.0;
            manifest.put("completed_calls", num(completed));
            Ok(Some(reply))
        }
        Err(reason) => {
            call.put("status", text("provider_error"));
            call.put("error", text(reason));
            call.put("usage", Json::Null); // Failed calls may still be billed.
            failures.push(call);
            manifest.put("failed_calls", num(failures.len() as f64));
            memory::append(&out.join("failures.jsonl"), failures.last().unwrap())?;
            memory::save(&out.join("failures.json"), &Json::Array(failures.clone()))?;
            Ok(None)
        }
    }
}
fn write_reports(out: &Path, manifest: &Json, rows: &[Json]) -> Result<(), String> {
    report::write(out, manifest, rows)?;
    {
        strategy_report::write(out, manifest, rows)?;
        memory::save(&out.join("sampling.json"), &sampling::summarize(manifest, rows))?;
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let o = options()?;
    if let Some(path) = &o.config {
        selmem::Config::load_path(path)
            .map_err(|_| "cannot read the requested --config file")?;
    }
    let protocol = PROTOCOL;
    let mut p = json::parse(protocol)?;
    if let Some(t) = o.temperature { p.put("temperature", num(t)); }
    let reasoning_effort = reasoning::validate(&selmem::Config::get().reasoning())?;
    if let Some(source) = &o.reuse_memories {
        reasoning::validate_source(source, &p, &sha256::hex(protocol.as_bytes()))?;
    }
    let repeats = o
        .repeats
        .unwrap_or(p.get("repetitions")?.number()? as usize);
    if !o.dry {
        sampling::validate_design(repeats, &o.panel, o.exploratory)?;
    }
    let probes = p.get("probes")?.array()?;
    let condition_count = panel_condition_count(&o.panel);
    let expected = condition_count
        .checked_mul(probes.len())
        .and_then(|v| v.checked_mul(repeats))
        .ok_or("call count overflow")?;
    if repeats == 0 || o.max_calls == 0 {
        return Err("repeats and max-calls must be positive".into());
    }
    if !o.dry && expected > o.max_calls {
        return Err(format!(
            "{expected} calls exceed --max-calls {}",
            o.max_calls
        ));
    }
    let bind = if o.dry {
        None
    } else {
        Some(configured_mouth(
            selmem::Config::get(),
            o.model.clone(),
            o.plug.clone(),
        )?)
    };
    if o.output.exists() {
        return Err("output already exists; choose a new directory".into());
    }
    fs::create_dir_all(&o.output).map_err(|e| e.to_string())?;
    let mut manifest = obj([
        ("panel", text(&o.panel)),
        ("condition_count", num(condition_count as f64)),
        ("protocol_sha256", text(sha256::hex(protocol.as_bytes()))),
        ("dataset_sha256", p.get("dataset_sha256")?.clone()),
        ("sample_id", p.get("sample_id")?.clone()),
        (
            "memory_engine",
            text("SelMem rules + HashEmbedder + NullScorer"),
        ),
        (
            "model",
            text(
                bind.as_ref()
                    .map(|b| b.model.as_str())
                    .unwrap_or("none (technical rehearsal)"),
            ),
        ),
        (
            "provider",
            bind.as_ref()
                .map(|b| text(if b.plug.is_empty() { "custom" } else { &b.plug }))
                .unwrap_or(Json::Null),
        ),
        ("organ_revision", text(ORGAN_REVISION)),
        ("sampling_design", text(if o.exploratory { "exploratory pilot" } else { "repeated comparison; blind behavioral evaluation required" })),
        ("memory_backends", obj([("narrator",text("rules")),("embedder",text("hash")),("scorer",text("null")),("semantic_interpreter",text("lexical extractive baseline"))])),
        ("audit", text("*.audit.jsonl: encode decisions, sleep counters, trace states; never fed to the organ")),
        ("repetitions", num(repeats as f64)),
        (
            "planned_calls",
            num(if o.dry { 0.0 } else { expected as f64 }),
        ),
        ("completed_calls", num(0.0)),
        ("attempted_calls", num(0.0)),
        ("failed_calls", num(0.0)),
        ("max_calls", num(o.max_calls as f64)),
        ("status", text("preparing")),
        ("context_budget_chars", p.get("context_chars")?.clone()),
        ("temperature", p.get("temperature")?.clone()),
        ("configured_reasoning_effort", text(&reasoning_effort)),
        ("reasoning_effort", if o.dry { Json::Null } else { text(&reasoning_effort) }),
        ("memory_source", o.reuse_memories.as_ref().map(|v| text(v.to_string_lossy())).unwrap_or(Json::Null)),
        (
            "max_completion_tokens",
            p.get("max_completion_tokens")?.clone(),
        ),
    ]);
    if let Ok(exe) = env::current_exe().and_then(fs::read) {
        manifest.put("binary_sha256", text(sha256::hex(&exe)));
    }
    if let Ok(rev) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        manifest.put(
            "git_revision",
            text(String::from_utf8_lossy(&rev.stdout).trim()),
        );
    }
    fs::write(o.output.join("protocol.json"), protocol).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    write_reports(&o.output, &manifest, &rows)?;
    let result = (|| -> Result<(), String> {
        let sample = dataset(&o.dataset, &p)?;
        let events = memory::timeline(&sample)?;
        let support = p.get("support_id")?.string()?;
        let disappointment = p.get("disappointment_id")?.string()?;
        for target in [support, disappointment] {
            if !events
                .iter()
                .any(|e| matches!(e,memory::Event::Turn{id,..} if id==target))
            {
                return Err("intervention event missing".into());
            }
        }
        let evidence = events
            .iter()
            .filter_map(|e| {
                if let memory::Event::Turn {
                    id,
                    source,
                    date,
                    speaker,
                    event,
                } = e
                {
                    if id == support || id == disappointment {
                        Some(obj([
                            ("dia_id", text(id)),
                            ("source", text(source)),
                            ("date", text(date)),
                            ("speaker", text(speaker)),
                            ("event", text(event)),
                        ]))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        memory::save(&o.output.join("evidence.json"), &Json::Array(evidence))?;
        let mut states: BTreeMap<String, (PathBuf, Option<PathBuf>)> = BTreeMap::new();
        for (arm, focus) in [
            ("a", Some(support)),
            ("b", Some(disappointment)),
            ("null_a", None),
            ("null_b", None),
        ] {
            let modes = if arm.starts_with("null") {
                vec!["full"]
            } else {
                vec!["full", "nosleep"]
            };
            for mode in modes {
                let name = format!("{mode}_{arm}");
                println!("Preparing {name}...");
                let snapshot = o.output.join(format!("{name}.selmem"));
                if let Some(source) = &o.reuse_memories {
                    reasoning::copy_snapshot(source, &name, &snapshot)?;
                } else {
                    let book = memory::train(&events, &p, focus, mode, &snapshot)?;
                    memory::save(&o.output.join(format!("{name}.book.json")), &book)?;
                }
                states.insert(name, (snapshot, None));
            }
        }
        states.insert(
            "transplant_a_from_b".into(),
            (states["full_a"].0.clone(), Some(states["full_b"].0.clone())),
        );
        states.insert(
            "transplant_b_from_a".into(),
            (states["full_b"].0.clone(), Some(states["full_a"].0.clone())),
        );
        let hashes = states.iter().filter(|(_, (_, donor))| donor.is_none())
            .map(|(name, (path, _))| fs::read(path)
                .map(|bytes| (name.clone(), text(sha256::hex(&bytes))))
                .map_err(|e| e.to_string()))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        manifest.put("memory_snapshot_sha256", Json::Object(hashes));
        let mut interpretation_audit = Vec::new();
        for (name, (snapshot, donor)) in &states {
            let source = donor.as_ref().unwrap_or(snapshot);
            let mem = selmem::SelectiveMemory::open(source, selmem::EntityProfile::new("Claire"))
                .map_err(|e| e.to_string())?;
            let mut entries = Vec::new();
            for trace in mem.store.traces.values() {
                if let Some(i) = selmem::recall::interpretation::stored(trace) {
                    let operation = trace.operations.iter().rev().find(|op|
                        op.kind == "situational-interpretation" && op.after == i.render(trace))
                        .ok_or("validated interpretation has no audit operation")?;
                    entries.push(obj([
                        ("trace_id", text(&trace.id)), ("subject", text(&i.subject)),
                        ("scope", text(i.scope())), ("evidence", text(&i.evidence)),
                        ("observation_id", text(trace.observation_id.as_deref().unwrap_or(&trace.id))),
                        ("source", text(&trace.source)), ("confidence", num(i.confidence as f64)),
                        ("statement", text(i.render(trace))),
                        ("operation", text(&operation.kind)), ("at", num(operation.at as f64)),
                        ("before", text(&operation.before)), ("after", text(&operation.after)),
                    ]));
                }
            }
            entries.sort_by_key(Json::encode);
            interpretation_audit.push(obj([
                ("condition", text(name)), ("interpretations", Json::Array(entries)),
            ]));
        }
        memory::save(&o.output.join("interpretations.json"), &Json::Array(interpretation_audit))?;
        let budget = p.get("context_chars")?.number()? as usize;
        let mut prepared = BTreeMap::new();
        for (name, (snapshot, donor)) in &states {
            for probe in probes {
                let retrieval_query = probe.object()?.get("retrieval_text")
                    .unwrap_or(probe.get("text")?).string()?;
                let mut r = memory::readout(snapshot, retrieval_query, donor.as_deref())?;
                r.put("organ_revision", text(ORGAN_REVISION));
                let packed = context::build(&r, budget)?;
                r.put("context_audit", packed.audit);
                let c = packed.text;
                prepared.insert(
                    (name.clone(), probe.get("id")?.string()?.to_string()),
                    (r, c),
                );
            }
        }
        {
            for (raw, source) in [("raw_a", "full_a"), ("raw_b", "full_b")] {
                for probe in probes {
                    let id = probe.get("id")?.string()?.to_string();
                    let mut r = strategy_report::raw_readout(&prepared[&(source.into(), id.clone())].0, &events)?;
                    r.put("organ_revision", text(ORGAN_REVISION));
                    let packed = context::build(&r, budget)?;
                    r.put("context_audit", packed.audit);
                    let c = packed.text;
                    prepared.insert((raw.into(), id), (r, c));
                }
                states.insert(raw.into(), (states[source].0.clone(), None));
            }
        }
        let clocks: std::collections::BTreeSet<_> = prepared
            .values()
            .map(|(r, _)| r.get("clock").map(Json::encode))
            .collect::<Result<_, _>>()?;
        if clocks.len() != 1 {
            return Err("unmatched probe clocks".into());
        }
        for probe in probes {
            let id = probe.get("id")?.string()?.to_string();
            for (donor, transplant) in [
                ("full_a", "transplant_b_from_a"),
                ("full_b", "transplant_a_from_b"),
            ] {
                if prepared[&(donor.into(), id.clone())]
                    != prepared[&(transplant.into(), id.clone())]
                {
                    return Err("transplant differs from donor; invalid comparison".into());
                }
            }
        }
        let null_equal = probes.iter().all(|probe| {
            let id = probe.get("id").unwrap().string().unwrap().to_string();
            prepared[&("full_null_a".into(), id.clone())].1
                == prepared[&("full_null_b".into(), id)].1
        });
        manifest.put(
            "checks",
            obj([
                ("matched_clocks", Json::Bool(true)),
                ("transplant_matches_donor", Json::Bool(true)),
                ("null_contexts_identical", Json::Bool(null_equal)),
            ]),
        );
        {
            let mut checks = Vec::new();
            let mut audit = Vec::new();
            for probe in probes {
                let id = probe.get("id")?.string()?;
                // Include creativity as transfer; only factual controls are exempt.
                let transfer = probe.get("kind")?.string()? != "fact";
                let mut signals = Vec::new();
                for name in states.keys() {
                    let (r, ctx) = &prepared[&(name.clone(), id.into())];
                    let audit = r.get("context_audit")?;
                    let has_evidence = audit.get("memories_included")?.number()? > 0.0
                        || audit.get("interpretations_included")?.number()? > 0.0
                        || audit.get("disposition_included")? == &Json::Bool(true);
                    signals.push(preflight::ContextSignal { condition: name.clone(), context: ctx.clone(), has_evidence });
                }
                let check = preflight::inspect(id, transfer, &signals);
                audit.push(obj([
                    ("probe", text(id)), ("transfer", Json::Bool(transfer)),
                    ("nonempty_conditions", num(check.nonempty as f64)),
                    ("unique_contexts", num(check.unique_contexts as f64)),
                    ("sleep_changes_context", Json::Bool(check.sleep_changes_context)),
                ]));
                checks.push(check);
            }
            let warnings = preflight::warnings(&checks);
            for warning in &warnings { eprintln!("Preflight: {warning}"); }
            let audit = obj([
                ("probes", Json::Array(audit)),
                ("warnings", Json::Array(warnings.iter().map(text).collect())),
                ("passed", Json::Bool(warnings.is_empty())),
                ("provider_calls", num(0.0)),
            ]);
            memory::save(&o.output.join("preflight.json"), &audit)?;
            manifest.put("preflight", audit);
            if !o.dry && !warnings.is_empty() {
                return Err("offline preflight failed; inspect preflight.json or rebuild with --dry-run before spending LLM calls".into());
            }
        }
        let mut tasks = Vec::new();
        for repeat in 1..=repeats {
            for name in states.keys().filter(|name| in_panel(name, &o.panel)) {
                for (i, _) in probes.iter().enumerate() {
                    tasks.push((name.clone(), i, repeat));
                }
            }
        }
        if tasks.len() != expected {
            return Err(format!("panel planning mismatch: {} tasks vs {expected} expected", tasks.len()));
        }
        // Fixed order randomization; it does not claim deterministic model sampling.
        let mut rng = 1729u64;
        for i in (1..tasks.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            tasks.swap(i, (rng as usize) % (i + 1));
        }
        manifest.put(
            "status",
            text(if o.dry {
                "technical rehearsal"
            } else {
                "running"
            }),
        );
        for (index, (name, pi, repeat)) in tasks.iter().enumerate() {
            let probe = &probes[*pi];
            let id = probe.get("id")?.string()?;
            let (readout, ctx) = &prepared[&(name.clone(), id.into())];
            let (answer, usage, model, finish) = if let Some(b) = &bind {
                let result = chat(
                    &b.url,
                    b.api_key.as_deref(),
                    &b.model,
                    &p,
                    ctx,
                    probe.get("text")?.string()?,
                    &reasoning_effort,
                );
                let call = obj([
                    ("id", text(format!("R{:03}", index + 1))),
                    ("condition", text(name)), ("probe", text(id)),
                    ("repetition", num(*repeat as f64)),
                    ("question", probe.get("text")?.clone()),
                    ("context", text(ctx)), ("readout", readout.clone()),
                    ("request_sha256", text(sha256::hex(chat_request(&b.url, &b.model, &p, ctx, probe.get("text")?.string()?, &reasoning_effort)?.encode().as_bytes()))),
                    ("reasoning_effort", text(&reasoning_effort)),
                ]);
                match keep_provider_reply(result, &o.output, &mut manifest, &mut failures, call)? {
                    Some(reply) => reply,
                    None => {
                        write_reports(&o.output, &manifest, &rows)?;
                        eprintln!("{}/{} {name} {id}: provider error recorded; continuing", index + 1, tasks.len());
                        continue;
                    }
                }
            } else {
                (
                    "[TECHNICAL ONLY: no LLM called]".into(),
                    Json::Null,
                    "none".into(),
                    Json::Null,
                )
            };
            rows.push(obj([
                ("id", text(format!("R{:03}", index + 1))),
                ("condition", text(name)),
                ("probe", text(id)),
                ("kind", probe.get("kind")?.clone()),
                ("question", probe.get("text")?.clone()),
                ("repetition", num(*repeat as f64)),
                ("response", text(answer)),
                ("context", text(ctx)),
                ("readout", readout.clone()),
                ("reasoning_effort", if o.dry { Json::Null } else { text(&reasoning_effort) }),
                ("request_sha256", if let Some(b) = &bind { text(sha256::hex(chat_request(&b.url, &b.model, &p, ctx, probe.get("text")?.string()?, &reasoning_effort)?.encode().as_bytes())) } else { Json::Null }),
                ("usage", usage),
                ("returned_model", text(model)),
                ("finish_reason", finish),
            ]));
            memory::append(&o.output.join("responses.jsonl"), rows.last().unwrap())?;
            write_reports(&o.output, &manifest, &rows)?;
            println!("{}/{} {name} {id}", index + 1, tasks.len());
        }
        manifest.put(
            "status",
            text(if o.dry {
                "technical rehearsal complete — no scientific result"
            } else if !failures.is_empty() {
                "complete with provider errors — missing responses; review failures.json"
            } else {
                "complete — awaiting blind evaluation"
            }),
        );
        Ok(())
    })();
    if let Err(e) = result {
        manifest.put(
            "status",
            text("failed — partial run, invalid for comparison"),
        );
        write_reports(&o.output, &manifest, &rows)?;
        return Err(format!("{e}. Partial artifacts: {}", o.output.display()));
    }
    write_reports(&o.output, &manifest, &rows)?;
    if !failures.is_empty() {
        eprintln!("Finished with {} provider errors. Successful responses: {}. See failures.json; failed calls were not retried.", failures.len(), rows.len());
    }
    println!("Open {}", o.output.join("report.html").display());
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_default_plug_model_and_key_from_config() {
        let cfg = selmem::Config::parse(
            "default_llm = grok\n[grok]\nmodel = configured-model\napi_key = fake-test-key\n",
        );
        let mouth = configured_mouth(&cfg, None, None).unwrap();
        assert_eq!(mouth.plug, "grok");
        assert_eq!(mouth.model, "configured-model");
        assert_eq!(mouth.api_key.as_deref(), Some("fake-test-key"));
    }
    #[test]
    fn optional_cli_overrides_use_the_same_config_mechanism() {
        let cfg = selmem::Config::parse("default_llm = grok\n[grok]\nmodel = grok-config\n[gpt]\nmodel = gpt-config\napi_key = fake-gpt-key\n");
        let mouth = configured_mouth(&cfg, Some("cli-model".into()), Some("gpt".into())).unwrap();
        assert_eq!(mouth.plug, "gpt");
        assert_eq!(mouth.model, "cli-model");
        assert_eq!(mouth.api_key.as_deref(), Some("fake-gpt-key"));
    }
    #[test]
    fn supports_custom_plugs_and_keyless_local_endpoints() {
        let cfg = selmem::Config::parse("default_llm = local\n[local]\nllm = http://127.0.0.1:11434/v1/chat/completions\nmodel = local-model\n");
        let mouth = configured_mouth(&cfg, None, None).unwrap();
        assert_eq!(mouth.plug, "local");
        assert_eq!(mouth.model, "local-model");
        assert!(mouth.api_key.is_none());
        let direct = selmem::Config::parse(
            "llm = http://127.0.0.1:11434/v1/chat/completions\nmodel = direct-model\n",
        );
        assert_eq!(
            configured_mouth(&direct, None, None).unwrap().model,
            "direct-model"
        );
    }
    #[test]
    fn missing_llm_config_does_not_select_rules_or_gpt() {
        assert!(configured_mouth(&selmem::Config::empty(), None, None).is_err());
    }
    #[test]
    fn request_transmits_the_configured_effort() {
        let p = json::parse(PROTOCOL).unwrap();
        for effort in ["none", "low"] {
            let body = chat_request("https://api.x.ai/v1/chat/completions", "grok-4.3", &p, "memories", "question", effort).unwrap();
            assert_eq!(body.get("reasoning_effort").unwrap().string().unwrap(), effort);
            assert_eq!(body.get("messages").unwrap().array().unwrap().len(), 2);
        }
    }
    #[test]
    fn strategy_protocol_and_panels() {
        let p = json::parse(PROTOCOL).unwrap();
        assert_eq!(p.get("probes").unwrap().array().unwrap().len(), 6);
        let names = ["full_a", "full_b", "transplant_a_from_b", "transplant_b_from_a", "nosleep_a", "nosleep_b", "selection_a", "selection_b", "full_null_a", "full_null_b", "raw_a", "raw_b"];
        assert_eq!(names.iter().filter(|n| in_panel(n, "core")).count(), 4);
        assert_eq!(names.iter().filter(|n| in_panel(n, "controls")).count(), 6);
        assert_eq!(names.iter().filter(|n| in_panel(n, "all")).count(), 10);
        for name in ["selection_a", "selection_b", "unknown"] {
            for panel in ["core", "controls", "all"] { assert!(!in_panel(name, panel)); }
        }
        assert_eq!(panel_condition_count("all") * 6 * 5, 300);
        assert_eq!(panel_condition_count("core") * 6 * 5, 120);
        assert_eq!(panel_condition_count("controls") * 6 * 5, 180);
        let transfer = p.get("probes").unwrap().array().unwrap().iter().filter(|q| q.get("kind").unwrap().string().unwrap() == "transfer");
        for q in transfer { let t = q.get("text").unwrap().string().unwrap(); assert!(!t.contains("hike")); assert!(!t.contains("support group")); }
    }
    #[test]
    fn provider_error_does_not_discard_successes_or_block_later_calls() {
        let root = std::env::temp_dir().join(format!("selmem-provider-errors-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut manifest = obj([("completed_calls", num(0.0)), ("attempted_calls", num(0.0)), ("failed_calls", num(0.0))]);
        let mut failures = Vec::new();
        let mut answers = Vec::new();
        let valid = r#"{"choices":[{"message":{"content":"valid answer"},"finish_reason":"stop"}],"usage":{"total_tokens":9}}"#;
        for (index, raw) in [valid, "<html>gateway response</html>", valid].into_iter().enumerate() {
            let result = decode_reply(raw, "model");
            if let Some(reply) = keep_provider_reply(result, &root, &mut manifest, &mut failures, obj([("id", text(format!("R{}", index + 1)))])).unwrap() {
                answers.push(reply);
            }
        }
        assert_eq!(answers.len(), 2);
        assert_eq!(manifest.get("completed_calls").unwrap().number().unwrap(), 2.0);
        assert_eq!(manifest.get("attempted_calls").unwrap().number().unwrap(), 3.0);
        assert_eq!(manifest.get("failed_calls").unwrap().number().unwrap(), 1.0);
        let saved = json::parse(&fs::read_to_string(root.join("failures.json")).unwrap()).unwrap();
        assert_eq!(saved.array().unwrap().len(), 1);
        assert_eq!(saved.array().unwrap()[0].get("id").unwrap().string().unwrap(), "R2");
        assert!(!saved.encode().contains("gateway response"));
        assert_eq!(fs::read_to_string(root.join("failures.jsonl")).unwrap().lines().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn no_fallback() {
        assert!(decode_reply(r#"{"choices":[{"message":{"content":""}}]}"#, "test").is_err());
        assert!(decode_reply(r#"{"error":{"message":"failed"}}"#, "test").is_err());
    }
    #[test]
    fn replies_and_usage() {
        let(a,u,m,_)=decode_reply(r#"{"model":"actual","choices":[{"message":{"content":"Bonjour"},"finish_reason":"stop"}],"usage":{"total_tokens":20}}"#,"requested").unwrap();
        assert_eq!(a, "Bonjour");
        assert_eq!(m, "actual");
        assert_eq!(u.get("total_tokens").unwrap().number().unwrap(), 20.0);
    }
    #[test]
    fn character_cap() {
        let r = obj([
            ("memories", Json::Array(vec![text("é".repeat(20000))])),
            ("axioms", Json::Array(vec![text("y".repeat(20000))])),
            ("reading_profile", text("z".repeat(20000))),
        ]);
        assert!(context(&r, 6000).unwrap().chars().count() <= 6000);
    }
}
