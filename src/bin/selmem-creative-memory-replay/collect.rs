use super::*;
use selmem::{
    MemoryContext, MemoryContextFormat, MemoryContextOptions, MemoryReply, Mood, WorkingTalk,
};
// Reuse the tested bounded transport and source-validation code unchanged.
#[allow(dead_code)]
mod frozen {
    include!("../selmem-creative-replay/collect.rs");
    pub(super) fn source_manifest(p: &Path) -> Result<Json> {
        manifest(p)
    }
    pub(super) fn check_source(p: &Path, d: &Json) -> Result<()> {
        validate_source(p, d)
    }
    pub(super) struct Transport(Api);
    impl Transport {
        pub(super) fn new(
            out: PathBuf,
            url: String,
            key: Option<String>,
            model: String,
            reasoning: String,
            mock: bool,
            max: usize,
        ) -> Result<Self> {
            let used = lines(&out.join("http.jsonl"))?
                .iter()
                .filter(|r| s(r, "event").ok() == Some("start"))
                .count();
            Ok(Self(Api {
                scope: String::new(),
                out,
                url,
                key,
                model,
                reasoning,
                mock,
                max,
                used,
            }))
        }
        pub(super) fn proposal(
            &mut self,
            scope: &str,
            system: &str,
            input: Json,
            seed: u32,
        ) -> Result<Json> {
            self.0.scope = scope.into();
            self.0.call(
                "proposal",
                system,
                input,
                450,
                seed,
                obj([(
                    "proposal",
                    text("Technical mock proposal; not experimental evidence."),
                )]),
                |v| {
                    require(
                        words(s(v, "proposal")?) > 0 && words(s(v, "proposal")?) <= 180,
                        "proposal length invalid",
                    )
                },
            )
        }
        pub(super) fn used(&self) -> usize {
            self.0.used
        }
    }
}
struct Opt {
    source: PathBuf,
    previous: PathBuf,
    out: PathBuf,
    config: PathBuf,
    llm: String,
    model: String,
    max: usize,
    resume: bool,
    prepare: bool,
    mock: bool,
}
fn options(args: Vec<String>) -> Result<Opt> {
    let mut values = BTreeMap::new();
    let mut flags = BTreeSet::new();
    let mut it = args.into_iter();
    while let Some(k) = it.next() {
        if matches!(k.as_str(), "--resume" | "--prepare-only" | "--mock") {
            require(flags.insert(k), "duplicate flag")?;
        } else {
            require(
                matches!(
                    k.as_str(),
                    "--source"
                        | "--previous"
                        | "--out"
                        | "--config"
                        | "--llm"
                        | "--model"
                        | "--max-http-calls"
                ),
                "unknown option",
            )?;
            let v = it.next().ok_or("missing option value")?;
            require(values.insert(k, v).is_none(), "duplicate option")?;
        }
    }
    let get = |k: &str| {
        values
            .get(k)
            .cloned()
            .ok_or_else(|| format!("required {k}"))
    };
    Ok(Opt {
        source: get("--source")?.into(),
        previous: get("--previous")?.into(),
        out: get("--out")?.into(),
        config: values
            .get("--config")
            .cloned()
            .unwrap_or(".selmem".into())
            .into(),
        llm: values.get("--llm").cloned().unwrap_or("grok".into()),
        model: values.get("--model").cloned().unwrap_or("grok-4.3".into()),
        max: values
            .get("--max-http-calls")
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| "invalid budget")?
            .unwrap_or(200),
        resume: flags.contains("--resume"),
        prepare: flags.contains("--prepare-only"),
        mock: flags.contains("--mock"),
    })
}
fn previous_manifest(p: &Path) -> Result<Json> {
    let mut paths = vec![
        "replay-design.json".to_string(),
        "replay.json".into(),
        "http.jsonl".into(),
        "responses.jsonl".into(),
        "all-proposals.json".into(),
    ];
    for folder in ["contexts", "cache"] {
        for e in fs::read_dir(p.join(folder)).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            require(
                e.file_type().map_err(|e| e.to_string())?.is_file(),
                "previous lot contains non-regular entry",
            )?;
            paths.push(format!(
                "{folder}/{}",
                e.file_name().to_str().ok_or("non UTF8 name")?
            ));
        }
    }
    paths.sort();
    let mut out = BTreeMap::new();
    for name in paths {
        let path = p.join(&name);
        require(
            !fs::symlink_metadata(&path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink(),
            "previous symlink forbidden",
        )?;
        require(
            fs::canonicalize(&path)
                .map_err(|e| e.to_string())?
                .starts_with(p),
            "previous path escapes lot",
        )?;
        out.insert(name, text(hash(&path)?));
    }
    Ok(Json::Object(out))
}
fn retelling_cache(previous: &Path) -> Result<Vec<Json>> {
    let mut successful = BTreeMap::new();
    for row in lines(&previous.join("responses.jsonl"))? {
        let choices = array(row.get("response")?, "choices")?;
        let choice = choices.first().ok_or("empty previous choices")?;
        if let Ok(v) = json::parse(s(choice.get("message")?, "content")?) {
            successful.insert(s(&row, "signature")?.to_string(), v);
        }
    }
    let mut retellings = vec![];
    for e in fs::read_dir(previous.join("cache")).map_err(|e| e.to_string())? {
        let p = e.map_err(|e| e.to_string())?.path();
        let v = load(&p)?;
        let signature = p
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("bad cache name")?;
        require(
            successful.get(signature) == Some(&v),
            "previous cache does not match a recorded successful response",
        )?;
        if v.get("memories").is_ok() {
            retellings.push(v);
        }
    }
    require(
        !retellings.is_empty(),
        "previous lot has no validated retellings",
    )?;
    Ok(retellings)
}
fn join_entries(entries: &[String]) -> String {
    let mut out = String::new();
    for e in entries {
        if words(&out) + words(e) <= 600 {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(e);
        }
    }
    out
}
fn trace<'a>(state: &'a Json, id: &str) -> Result<&'a Json> {
    array(state, "traces")?
        .iter()
        .find(|t| s(t, "id").ok() == Some(id))
        .ok_or_else(|| format!("trace not in inspection: {id}"))
}
fn ids(original: &Json) -> Result<Vec<String>> {
    array(original.get("audit")?, "selected_ids")?
        .iter()
        .map(|v| v.string().map(str::to_string))
        .collect()
}
fn narratives(
    old: &Json,
    original: &Json,
    state: &Json,
    retellings: &[Json],
) -> Result<(Vec<String>, Vec<String>)> {
    let selected = ids(original)?;
    let gists: Vec<String> = selected
        .iter()
        .map(|id| trace(state, id).and_then(|t| s(t, "gist").map(str::to_string)))
        .collect::<Result<_>>()?;
    require(
        selected.len() <= 4,
        "saved selection exceeds organ delivery cap",
    )?;
    if s(old, "condition")? == "after_no_recon" {
        require(
            join_entries(&gists) == s(old, "background")?,
            "saved direct context differs from selected lived gists",
        )?;
        return Ok((selected, gists));
    }
    let audit = array(old, "audit")?;
    require(audit.len() == selected.len(), "guard audit length mismatch")?;
    for (a, id) in audit.iter().zip(&selected) {
        require(s(a, "trace_id")? == id, "guard audit selection mismatch")?;
        require(
            boolean(a, "fallback_to_lived_gist")? == (s(a, "verdict")? != "retelling"),
            "guard verdict/fallback mismatch",
        )?;
    }
    let mut matches = vec![];
    for cached in retellings {
        let rows = array(cached, "memories")?;
        if rows.len() != selected.len() {
            continue;
        }
        let mut seen = BTreeSet::new();
        if rows.iter().any(|r| {
            s(r, "trace_id")
                .map(|id| !selected.iter().any(|x| x == id) || !seen.insert(id))
                .unwrap_or(true)
        }) {
            continue;
        }
        let mut candidate = vec![];
        for (i, id) in selected.iter().enumerate() {
            let row = rows
                .iter()
                .find(|r| s(r, "trace_id").ok() == Some(id))
                .ok_or("missing cached trace")?;
            candidate.push(if boolean(&audit[i], "fallback_to_lived_gist")? {
                gists[i].clone()
            } else {
                s(row, "narrative")?.into()
            });
        }
        if join_entries(&candidate) == s(old, "background")? {
            matches.push(candidate);
        }
    }
    let chosen = matches
        .first()
        .ok_or("could not reconstruct exact previous guarded background from validated retellings")?
        .clone();
    require(
        matches.iter().all(|m| m == &chosen),
        "ambiguous previous retelling mapping",
    )?;
    Ok((selected, chosen))
}
fn finite(v: &Json) -> Result<f64> {
    let n = match v {
        Json::String(s) => s.parse::<f64>().map_err(|_| "invalid inspection number")?,
        _ => v.number()?,
    };
    require(n.is_finite(), "nonfinite inspection number")?;
    Ok(n)
}
fn mood(state: &Json) -> Result<Json> {
    let m = array(state, "mood")?;
    require(m.len() == 3, "invalid saved mood")?;
    Ok(obj([
        ("valence", num(finite(&m[0])?)),
        ("arousal", num(finite(&m[1])?)),
        ("disgust", num(finite(&m[2])?)),
    ]))
}
fn take_words(s: &str, left: &mut usize, cap: usize) -> (String, bool) {
    let n = words(s);
    let k = n.min(*left).min(cap);
    *left -= k;
    if n == k {
        (s.into(), false)
    } else {
        (
            s.split_whitespace().take(k).collect::<Vec<_>>().join(" "),
            true,
        )
    }
}
fn packet(
    state: &Json,
    original: &Json,
    selected: &[String],
    texts: &[String],
    opts: &MemoryContextOptions,
) -> Result<Json> {
    let mut left = opts.words;
    let mut episodes = vec![];
    if opts.episodes {
        for (id, narrative) in selected.iter().zip(texts) {
            let t = trace(state, id)?;
            require(
                !boolean(t, "suppressed")? && s(t, "status")? != "Latent",
                "saved selected trace is hidden or latent",
            )?;
            let (text, truncated) = take_words(narrative, &mut left, 160);
            if text.is_empty() {
                continue;
            }
            let affect = if opts.affect {
                obj([
                    ("valence", num(n(t, "valence")?)),
                    ("arousal", num(n(t, "arousal")?)),
                    ("disgust", Json::Null),
                ])
            } else {
                Json::Null
            };
            episodes.push(obj([
                ("trace_id", text_json(id)),
                ("narrative", text_json(text)),
                (
                    "attribution",
                    text_json(s(t, "attribution")?.to_ascii_lowercase()),
                ),
                ("channel", Json::Null),
                ("fidelity", num(n(t, "fidelity")?)),
                ("confidence", Json::Null),
                ("affect", affect),
                ("truncated", Json::Bool(truncated)),
            ]));
        }
    }
    let mut dispositions = vec![];
    if opts.dispositions {
        for allowed in array(original.get("audit")?, "axioms")?.iter().take(4) {
            let statement = allowed.string()?;
            let a = array(state, "axioms")?
                .iter()
                .find(|a| {
                    s(a, "statement").ok() == Some(statement)
                        && a.get("superseded_by").ok() == Some(&Json::Null)
                })
                .ok_or("allowed axiom absent or superseded")?;
            let support = array(a, "supports")?;
            let mut origins = BTreeSet::new();
            for id in support {
                let t = trace(state, id.string()?)?;
                require(
                    !boolean(t, "suppressed")? && s(t, "status")? != "Latent",
                    "allowed axiom has unavailable support",
                )?;
                origins.insert(s(t, "attribution")?.to_ascii_lowercase());
            }
            let origin = if origins.len() == 1 {
                origins.into_iter().next().unwrap()
            } else {
                "mixed_or_unspecified".into()
            };
            let (statement, truncated) = take_words(statement, &mut left, 80);
            if statement.is_empty() {
                continue;
            }
            dispositions.push(obj([
                ("axiom_id", text(s(a, "id")?)),
                ("statement", text(statement)),
                ("layer", text(s(a, "layer")?.to_ascii_lowercase())),
                ("strength", num(n(a, "strength")?)),
                ("attribution", text(origin)),
                ("support_trace_ids", arr(support.iter().take(32).cloned())),
                ("support_trace_count", num(support.len() as u32)),
                ("support_ids_truncated", Json::Bool(support.len() > 32)),
                ("truncated", Json::Bool(truncated)),
            ]));
        }
    }
    if opts.associations {
        require(n(state,"associations")?==0.0,"inspection only exports association count; nonempty graph requires a richer export, not guessed edges")?;
    }
    Ok(obj([
        (
            "version",
            text(selmem::recall::memory_context::MEMORY_CONTEXT_VERSION),
        ),
        ("episodes", arr(episodes)),
        ("dispositions", arr(dispositions)),
        ("contextual_statements", arr([])),
        ("associations", arr([])),
        (
            "affect",
            if opts.affect {
                mood(state)?
            } else {
                Json::Null
            },
        ),
        ("content_words", num((opts.words - left) as u32)),
    ]))
}
fn text_json(s: impl Into<String>) -> Json {
    text(s)
}
fn empty_packet() -> Json {
    json::parse(&MemoryContext::default().to_json()).unwrap()
}
fn final_system(opts: &MemoryContextOptions) -> String {
    let talk = WorkingTalk::default();
    let context = MemoryContext::default();
    let mood = Mood::default();
    let r = MemoryReply {
        user: "",
        memories: &[],
        axioms: &[],
        mood: &mood,
        talk: &talk,
        reading: "",
        context: &context,
        options: opts,
    };
    format!("{}\nRespect every task constraint. Return JSON {{\"proposal\":\"...\"}} with a nonempty proposal of at most 180 words.",r.system_prompt())
}
struct Cell {
    history: String,
    task: Json,
    condition: String,
    input: Json,
    audit: Json,
}
fn prepare_cells(source: &Path, previous: &Path, opts: &MemoryContextOptions) -> Result<Vec<Cell>> {
    let caches = retelling_cache(previous)?;
    let mut cells = vec![];
    for h in ["conv-26", "conv-30"] {
        for task in ["community", "learning", "repair"] {
            for condition in CONDITIONS {
                let old = load(&previous.join(format!("contexts/{h}-{task}-{condition}.json")))?;
                require(
                    s(&old, "history")? == h && s(&old, "condition")? == condition,
                    "previous context identity mismatch",
                )?;
                let original_condition = match condition {
                    "before_sleep_guarded" => "before_sleep",
                    "after_full_guarded" => "after_full",
                    "empty_two_step" => "empty",
                    x => x,
                };
                let original =
                    load(&source.join(format!("contexts/{h}-{task}-{original_condition}.json")))?;
                require(
                    old.get("task")? == original.get("task")?,
                    "task changed between sources",
                )?;
                let native = matches!(
                    condition,
                    "before_sleep_guarded" | "after_no_recon" | "after_full_guarded"
                );
                let memory = if native {
                    let state = load(&source.join(format!(
                        "checkpoints/{h}-{}.state.json",
                        if condition == "before_sleep_guarded" {
                            "no-sleep"
                        } else {
                            "after-sleep"
                        }
                    )))?;
                    let (selected, texts) = narratives(&old, &original, &state, &caches)?;
                    packet(&state, &original, &selected, &texts, opts)?
                } else {
                    empty_packet()
                };
                let auxiliary = match condition {
                    "rag" => {
                        require(
                            s(&old, "background")? == s(&original, "context")?,
                            "RAG background changed",
                        )?;
                        obj([
                            ("kind", text("retrieved_context")),
                            ("attribution", text("external")),
                            ("text", text(s(&old, "background")?)),
                        ])
                    }
                    "empty_two_step" => {
                        require(
                            boolean(old.get("audit")?, "accepted")?,
                            "previous draft rejected; cannot reuse it as accepted ideas",
                        )?;
                        obj([
                            ("kind", text("preliminary_ideas_not_memory")),
                            ("text", text(s(&old, "background")?)),
                        ])
                    }
                    _ => {
                        if condition == "empty" {
                            require(
                                s(&old, "background")?.is_empty(),
                                "empty control contains background",
                            )?;
                        }
                        Json::Null
                    }
                };
                let input = obj([
                    ("working_conversation", text("")),
                    ("memory", memory),
                    ("human", text(s(old.get("task")?, "prompt")?)),
                    ("auxiliary_context", auxiliary),
                ]);
                let audit = obj([
                    (
                        "previous_context",
                        text(format!("contexts/{h}-{task}-{condition}.json")),
                    ),
                    (
                        "frozen_reconstruction_reused",
                        Json::Bool(condition.ends_with("_guarded")),
                    ),
                    ("inspection_adapter", text("inspection-json-v1")),
                    (
                        "unavailable_metadata",
                        arr([
                            text("episode confidence"),
                            text("episode channel"),
                            text("episode disgust"),
                        ]),
                    ),
                    ("previous_guard_audit", old.get("audit")?.clone()),
                ]);
                cells.push(Cell {
                    history: h.into(),
                    task: old.get("task")?.clone(),
                    condition: condition.into(),
                    input,
                    audit,
                });
            }
        }
    }
    shuffle(&mut cells, 90421);
    Ok(cells)
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub fn cli(args: Vec<String>) -> Result<()> {
    let o = options(args)?;
    let source = fs::canonicalize(&o.source).map_err(|e| e.to_string())?;
    let previous = fs::canonicalize(&o.previous).map_err(|e| e.to_string())?;
    let parent = fs::canonicalize(
        o.out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(|e| format!("output parent must exist: {e}"))?;
    let out = parent.join(o.out.file_name().ok_or("invalid output")?);
    require(
        !out.starts_with(&source) && !out.starts_with(&previous),
        "output must be outside both input folders",
    )?;
    if out.exists() {
        require(
            !fs::symlink_metadata(&out)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink(),
            "output symlink forbidden",
        )?;
    }
    let source_manifest = frozen::source_manifest(&source)?;
    let previous_manifest = previous_manifest(&previous)?;
    let sd = load(&source.join("preview-design.json"))?;
    frozen::check_source(&source, &sd)?;
    let pd = load(&previous.join("replay-design.json"))?;
    let pr = load(&previous.join("replay.json"))?;
    require(
        s(&pd, "version")? == "creative-replay-1"
            && n(&pr, "completed")? == 72.0
            && s(&pr, "status")? == "collected_unrated",
        "previous must be a completed replay-1 lot",
    )?;
    require(
        pd.get("source_manifest")? == &source_manifest,
        "previous lot was not based on these exact saved source files",
    )?;
    if !o.mock {
        require(
            !boolean(&pd, "mock")? && !boolean(&pr, "mock")?,
            "real replay requires a real previous lot",
        )?;
    }
    let (url, key, reasoning, opts) = if o.mock {
        (
            "mock://local".into(),
            None,
            "none".into(),
            MemoryContextOptions::default(),
        )
    } else {
        let cfg = selmem::Config::load_path(&o.config).map_err(|e| e.to_string())?;
        let raw = o.llm.starts_with("http");
        let mouth = cfg
            .mouth(
                raw.then(|| o.llm.clone()),
                Some(o.model.clone()),
                (!raw).then(|| o.llm.clone()),
                &o.model,
            )?
            .ok_or("HTTP mouth required")?;
        let r = cfg.reasoning();
        (
            mouth.url,
            mouth.api_key,
            if r == "off" { "none".into() } else { r },
            MemoryContextOptions::from_config(&cfg)?,
        )
    };
    require(
        opts.format == MemoryContextFormat::Structured,
        "this rerun requires memory_context=structured; remove old legacy overrides",
    )?;
    if !o.mock {
        require(
            s(&pd, "model")? == o.model
                && s(&pd, "endpoint")? == url
                && s(&pd, "reasoning_effort")? == reasoning,
            "model, endpoint and reasoning must match previous lot",
        )?;
    }
    let cells = prepare_cells(&source, &previous, &opts)?;
    let system = final_system(&opts);
    let protocol = obj([
        ("version", text("creative-memory-replay-1")),
        ("source", text(source.to_string_lossy())),
        ("previous", text(previous.to_string_lossy())),
        ("source_manifest", source_manifest.clone()),
        ("previous_manifest", previous_manifest.clone()),
        ("model", text(&o.model)),
        ("endpoint", text(&url)),
        ("reasoning_effort", text(&reasoning)),
        ("mock", Json::Bool(o.mock)),
        ("conditions", arr(CONDITIONS.map(text))),
        ("planned", num(72)),
        ("nominal_calls", num(72)),
        ("memory_context_options", json::parse(&opts.to_json())?),
        ("final_system_prompt", text(&system)),
        (
            "implementation_sha256",
            text(sha256::hex(
                concat!(
                    include_str!("../selmem-creative-memory-replay.rs"),
                    include_str!("collect.rs"),
                    include_str!("rate.rs"),
                    include_str!("../../util/json.rs"),
                    include_str!("../../util/sha256.rs"),
                    include_str!("../selmem-creative-replay/collect.rs"),
                    include_str!("../../recall/memory_context.rs")
                )
                .as_bytes(),
            )),
        ),
    ]);
    if o.resume {
        require(
            load(&out.join("replay-design.json"))? == protocol,
            "resume source/protocol/implementation mismatch",
        )?;
    } else {
        fs::create_dir(&out).map_err(|e| format!("fresh output required: {e}"))?;
        fresh(&out.join("replay-design.json"), &protocol)?;
    }
    let lock = out.join("run.lock");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| format!("run lock unavailable: {e}"))?;
    let _lock = Lock(lock);
    for c in &cells {
        commit(
            &out.join("contexts").join(format!(
                "{}-{}-{}.json",
                c.history,
                s(&c.task, "id")?,
                c.condition
            )),
            &obj([
                ("history", text(&c.history)),
                ("task", c.task.clone()),
                ("condition", text(&c.condition)),
                ("input", c.input.clone()),
                ("audit", c.audit.clone()),
            ]),
        )?;
    }
    if o.prepare {
        println!(
            "Prepared 72 structured proposals, 72 nominal calls; zero HTTP calls made: {}",
            out.display()
        );
        return Ok(());
    }
    let mut api = frozen::Transport::new(out.clone(), url, key, o.model, reasoning, o.mock, o.max)?;
    let mut proposals = vec![];
    for c in &cells {
        for repeat in 0..2 {
            let scope = format!("{}-{}-{}", c.history, s(&c.task, "id")?, c.condition);
            let v = api.proposal(&scope, &system, c.input.clone(), 100 + repeat)?;
            proposals.push(obj([
                ("history", text(&c.history)),
                ("task", c.task.clone()),
                ("condition", text(&c.condition)),
                ("repeat", num(repeat)),
                ("proposal", v.get("proposal")?.clone()),
            ]));
        }
    }
    require(
        frozen::source_manifest(&source)? == source_manifest
            && self::previous_manifest(&previous)? == previous_manifest,
        "input files changed during rerun",
    )?;
    shuffle(&mut proposals, 95487);
    let mut blind = vec![];
    let mut key = vec![];
    for (i, p) in proposals.iter_mut().enumerate() {
        let id = format!("M{:04}", i + 1);
        p.put("blind_id", text(&id));
        blind.push(obj([
            ("blind_id", text(&id)),
            ("task", p.get("task")?.clone()),
            ("proposal", p.get("proposal")?.clone()),
        ]));
        key.push(obj([
            ("blind_id", text(id)),
            ("history", p.get("history")?.clone()),
            ("condition", p.get("condition")?.clone()),
            ("repeat", p.get("repeat")?.clone()),
        ]));
    }
    commit(&out.join("all-proposals.json"), &arr(proposals))?;
    commit(&out.join("blind-proposals.json"), &arr(blind))?;
    commit(&out.join("private-blind-key.json"), &arr(key))?;
    commit(
        &out.join("replay.json"),
        &obj([
            ("status", text("collected_unrated")),
            ("mock", Json::Bool(o.mock)),
            ("completed", num(72)),
            ("http_attempts", num(api.used() as u32)),
            ("source_unchanged", Json::Bool(true)),
            ("previous_unchanged", Json::Bool(true)),
            ("reconstructions_reused", Json::Bool(true)),
        ]),
    )?;
    println!(
        "Collected 72 structured proposals in {} (mock={})",
        out.display(),
        o.mock
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> Json {
        obj([
            (
                "traces",
                arr([obj([
                    ("id", text("a")),
                    ("gist", text("An old lived scene.")),
                    ("suppressed", Json::Bool(false)),
                    ("status", text("Active")),
                    ("attribution", text("External")),
                    ("fidelity", num(0.6)),
                    ("valence", num(0.4)),
                    ("arousal", num(0.2)),
                ])]),
            ),
            ("axioms", arr([])),
            ("mood", arr([text("0.4"), text("0.2"), text("0.0")])),
            ("associations", num(0)),
        ])
    }
    fn original() -> Json {
        obj([(
            "audit",
            obj([("selected_ids", arr([text("a")])), ("axioms", arr([]))]),
        )])
    }
    #[test]
    fn only_validated_retellings_matching_the_exact_old_background_are_reused() {
        let old = obj([
            ("condition", text("after_full_guarded")),
            ("background", text("A retold past scene.")),
            (
                "audit",
                arr([obj([
                    ("trace_id", text("a")),
                    ("verdict", text("retelling")),
                    ("fallback_to_lived_gist", Json::Bool(false)),
                ])]),
            ),
        ]);
        let cache = obj([(
            "memories",
            arr([obj([
                ("trace_id", text("a")),
                ("narrative", text("A retold past scene.")),
            ])]),
        )]);
        let (_, n) = narratives(&old, &original(), &state(), &[cache]).unwrap();
        assert_eq!(n, vec!["A retold past scene."]);
        assert!(narratives(&old, &original(), &state(), &[]).is_err());
    }
    #[test]
    fn rejected_previous_retelling_reuses_the_gist_not_the_rejected_text() {
        let old = obj([
            ("condition", text("before_sleep_guarded")),
            ("background", text("An old lived scene.")),
            (
                "audit",
                arr([obj([
                    ("trace_id", text("a")),
                    ("verdict", text("unsupported")),
                    ("fallback_to_lived_gist", Json::Bool(true)),
                ])]),
            ),
        ]);
        let cache = obj([(
            "memories",
            arr([obj([
                ("trace_id", text("a")),
                ("narrative", text("A fabricated solution")),
            ])]),
        )]);
        let (_, n) = narratives(&old, &original(), &state(), &[cache]).unwrap();
        assert_eq!(n, vec!["An old lived scene."]);
    }
    #[test]
    fn missing_export_metadata_remains_null_and_external_stays_external() {
        let p = packet(
            &state(),
            &original(),
            &["a".into()],
            &["The old scene".into()],
            &MemoryContextOptions::default(),
        )
        .unwrap();
        let e = &array(&p, "episodes").unwrap()[0];
        assert_eq!(s(e, "attribution").unwrap(), "external");
        assert_eq!(e.get("confidence").unwrap(), &Json::Null);
        assert_eq!(e.get("channel").unwrap(), &Json::Null);
        assert_eq!(
            e.get("affect").unwrap().get("disgust").unwrap(),
            &Json::Null
        );
        assert!(array(&p, "dispositions").unwrap().is_empty());
    }
    #[test]
    fn nonempty_graph_is_not_silently_replaced_by_guessed_links() {
        let mut st = state();
        st.put("associations", num(2));
        assert!(packet(&st, &original(), &[], &[], &MemoryContextOptions::default()).is_err());
    }
    #[test]
    fn component_cuts_remove_native_state() {
        let mut opts = MemoryContextOptions::default();
        opts.episodes = false;
        opts.dispositions = false;
        opts.associations = false;
        opts.affect = false;
        let p = packet(
            &state(),
            &original(),
            &["a".into()],
            &["secret".into()],
            &opts,
        )
        .unwrap();
        assert!(array(&p, "episodes").unwrap().is_empty());
        assert_eq!(p.get("affect").unwrap(), &Json::Null);
        assert!(!p.encode().contains("secret"));
    }
    #[test]
    fn final_prompt_contains_the_real_organ_contract() {
        let s = final_system(&MemoryContextOptions::default());
        assert!(s.contains(selmem::recall::memory_context::MEMORY_USE_CONTRACT));
        assert!(!s.contains("Speak from the reading profile"));
        assert!(s.contains("180 words"));
    }
}
