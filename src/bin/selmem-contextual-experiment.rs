//! GENESIS contextual belief experiment: synthetic memory-state qualification and
//! optional paid, matched actor choice probes. Requires the contextual scope patch.
//! Only Rust + the SelMem crate; no external Cargo dependencies.
use selmem::util::json;

use json::{num, obj, text, Json};
use selmem::core::model::{Bearer, StakeKind};
use selmem::core::self_model::{consolidate_with_scorer, view_scoped};
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{
    Attribution, ContextFacet, EncodeInput, EntityProfile, MemoryContextOptions, PropositionLabel,
    PropositionScorer, ScopeStatus, SelectiveMemory, SelfMemory, SelfRole, SemanticInterpreter,
};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const QUIET: &str = "I chose a quiet venue.";
const LIVELY: &str = "I chose a lively venue.";

/// The scorer is a **scripted witness**, not an independent LLM judge.
struct FixtureWitness;
impl PropositionScorer for FixtureWitness {
    fn name(&self) -> &str {
        "contextual-scripted-witness"
    }
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
    fn score_self_model(
        &self,
        statement: &str,
        expectation: &str,
        traces: &[&selmem::MemoryTrace],
    ) -> PropositionLabel {
        if statement.to_ascii_lowercase().contains("prefer") || traces.len() < 2 {
            return PropositionLabel::Unknown;
        }
        if [QUIET, LIVELY].contains(&expectation) && traces.iter().all(|t| t.gist == expectation) {
            PropositionLabel::Entail
        } else {
            PropositionLabel::Unknown
        }
    }
    fn score_expectation(&self, expectation: &str, event: &str) -> PropositionLabel {
        if expectation == event {
            PropositionLabel::Entail
        } else if [QUIET, LIVELY].contains(&expectation) && [QUIET, LIVELY].contains(&event) {
            PropositionLabel::Contradict
        } else {
            PropositionLabel::Unknown
        }
    }
}

#[derive(Clone)]
struct Config {
    out: PathBuf,
    actor: bool,
    provider: String,
    model: String,
    repeats: usize,
    max_http_calls: Option<usize>,
    seed: u64,
}
impl Config {
    fn parse() -> Result<Self, String> {
        let mut c = Self {
            out: PathBuf::new(),
            actor: false,
            provider: "grok".into(),
            model: "grok-4.3".into(),
            repeats: 1,
            max_http_calls: None,
            seed: 1729,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            if flag == "--help" || flag == "-h" {
                println!("GENESIS contextual experiment\n  --out NEW_DIR [--actor --provider grok|openai --model MODEL --max-http-calls N]\n  [--repeats N] [--seed N]\nWithout --actor: offline organ qualification, no API requests.\n--actor: 128 * repeats model calls (two mirrored histories, four controls, two checkpoints, two contexts, two paraphrases, two option orders).\nRequires GROK_API_KEY or OPENAI_API_KEY in the environment. No credentials are written to disk.");
                std::process::exit(0);
            }
            if flag == "--actor" {
                c.actor = true;
                continue;
            }
            let value = it
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--out" => c.out = PathBuf::from(value),
                "--provider" => c.provider = value,
                "--model" => c.model = value,
                "--repeats" => c.repeats = value.parse().map_err(|_| "invalid repeats")?,
                "--max-http-calls" => {
                    c.max_http_calls = Some(value.parse().map_err(|_| "invalid max-http-calls")?)
                }
                "--seed" => c.seed = value.parse().map_err(|_| "invalid seed")?,
                _ => return Err(format!("unknown flag: {flag}")),
            }
        }
        if c.out.as_os_str().is_empty() || c.out.exists() {
            return Err("--out must name a new, nonexistent directory".into());
        }
        if c.repeats == 0 || c.repeats > 20 {
            return Err("repeats must be 1..=20".into());
        }
        if c.actor {
            if !["grok", "openai"].contains(&c.provider.as_str()) {
                return Err("--provider must be grok or openai".into());
            }
            if c.model.trim().is_empty() {
                return Err("empty model".into());
            }
            let required = 128usize.checked_mul(c.repeats).ok_or("trial overflow")?;
            if c.max_http_calls.unwrap_or(0) < required {
                return Err(format!("actor phase needs {required} model calls; explicitly supply --max-http-calls {required} or higher"));
            }
        }
        Ok(c)
    }
}

fn observed(
    m: &mut SelectiveMemory,
    tag: &str,
    action: &str,
    company: &str,
    extra: Option<(&str, &str)>,
) -> Result<String, String> {
    let mut input = EncodeInput::observed(action, tag);
    input.source = "genesis-scripted-tool";
    input.attribution = Attribution::Internal;
    input.schema = Some("venue".into());
    input.permanence = 0.9;
    input.personal.scope_facets = vec![ContextFacet::new("company", company)];
    if let Some((dimension, value)) = extra {
        input
            .personal
            .scope_facets
            .push(ContextFacet::new(dimension, value));
    }
    let mut sem = selmem::LexicalInterpreter::default().interpret_event(action)?;
    sem.schema = Some("venue".into());
    sem.stake_kind = StakeKind::Decision;
    sem.bearer = Bearer::Self_;
    sem.core.claim = action.into();
    input.semantics = Some(sem);
    let decision = m.live_with(input);
    if !decision.kept {
        return Err(format!("rejected observation {tag}: {}", decision.reason));
    }
    decision
        .trace_id
        .ok_or_else(|| format!("missing trace for {tag}"))
}
fn seed_parent(m: &mut SelectiveMemory, a: &str, a1: String, a2: String) -> Result<String, String> {
    let mut input = EncodeInput::new("I have repeatedly chosen a venue of one kind.");
    input.attribution = Attribution::Internal;
    input.schema = Some("venue".into());
    input.personal = SelfMemory {
        role: SelfRole::Belief,
        context: "venue".into(),
        expectation: a.into(),
        certainty: 0.65,
        importance: 0.8,
        support_ids: vec![a1, a2],
        ..SelfMemory::default()
    };
    m.live_with(input)
        .trace_id
        .ok_or("failed to seed broad self-belief".into())
}
fn consolidate(m: &mut SelectiveMemory) {
    consolidate_with_scorer(&mut m.store, &RuleNarrator, &HashEmbedder, &FixtureWitness);
}
fn checkpoint(
    m: &mut SelectiveMemory,
    parent: &str,
    history: &str,
    enabled: bool,
    phase: &str,
    log: &mut BufWriter<File>,
) -> Result<(usize, usize), String> {
    consolidate(m);
    let p = m.store.traces.get(parent).ok_or("missing parent")?;
    let children: Vec<_> = m
        .store
        .traces
        .values()
        .filter(|t| t.personal.scope_parent.as_deref() == Some(parent) && !t.personal.retired)
        .collect();
    let candidates = children
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Candidate)
        .count();
    let qualified = children
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Conditional)
        .count();
    let conflicted = children
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Conflicted)
        .count();
    let scoped_ids = |company: &str| {
        view_scoped(
            &m.store,
            "venue",
            Some("venue"),
            &[ContextFacet::new("company", company)],
        )
        .into_iter()
        .map(|t| text(&t.id))
        .collect::<Vec<_>>()
    };
    let row = obj([
        ("kind", text("organ")),
        ("history", text(history)),
        ("condition", text(if enabled { "on" } else { "off" })),
        ("phase", text(phase)),
        ("parent_id", text(parent)),
        ("parent_status", text(p.personal.scope_status.token())),
        ("parent_certainty", num(p.personal.certainty as f64)),
        ("active_children", num(children.len() as f64)),
        ("candidates", num(candidates as f64)),
        ("qualified", num(qualified as f64)),
        ("conflicted", num(conflicted as f64)),
        ("alone_ids", Json::Array(scoped_ids("alone"))),
        ("friends_ids", Json::Array(scoped_ids("friends"))),
    ]);
    writeln!(log, "{}", row.encode()).map_err(|e| e.to_string())?;
    Ok((candidates, qualified))
}
fn snapshot(m: &SelectiveMemory) -> Result<(), String> {
    m.save().map_err(|e| format!("saving checkpoint: {e}"))
}
fn build_history(
    out: &Path,
    reverse: bool,
    enabled: bool,
    log: &mut BufWriter<File>,
) -> Result<(), String> {
    let hist = if reverse { "reversed" } else { "normal" };
    let state = if enabled { "on" } else { "off" };
    let a = if reverse { LIVELY } else { QUIET };
    let b = if reverse { QUIET } else { LIVELY };
    let path = out.join(format!("{hist}_{state}_qualified.vault"));
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let mut m = SelectiveMemory::open(&path, profile)
        .map_err(|e| e.to_string())?
        .with_scorer(Box::new(FixtureWitness));
    m.store.contextual_scope.enabled = enabled;
    let q1 = observed(&mut m, "initial-1", a, "alone", None)?;
    let q2 = observed(&mut m, "initial-2", a, "alone", None)?;
    let parent = seed_parent(&mut m, a, q1, q2)?;
    checkpoint(&mut m, &parent, hist, enabled, "formed", log)?;
    observed(&mut m, "contrary-1", b, "friends", None)?;
    observed(&mut m, "contrary-2", b, "friends", None)?;
    let (candidate, qualified) = checkpoint(&mut m, &parent, hist, enabled, "candidate", log)?;
    if enabled && (candidate, qualified) != (2, 0) {
        return Err(format!(
            "{hist}: expected exactly two hidden candidates, got ({candidate},{qualified})"
        ));
    }
    if !enabled && (candidate, qualified) != (0, 0) {
        return Err("OFF control formed scoped beliefs".into());
    }
    observed(&mut m, "prospective-1", a, "alone", None)?;
    observed(&mut m, "prospective-2", b, "friends", None)?;
    let (_, qualified) = checkpoint(&mut m, &parent, hist, enabled, "qualified", log)?;
    if enabled && qualified != 2 {
        return Err(format!("{hist}: qualification failed"));
    }
    if !enabled && qualified != 0 {
        return Err("OFF control unexpectedly qualified".into());
    }
    snapshot(&m)?;
    // A repeatable contrary event tests whether the branch can be contradicted.
    observed(&mut m, "recover-conflict-1", a, "friends", None)?;
    observed(&mut m, "recover-conflict-2", a, "friends", None)?;
    checkpoint(&mut m, &parent, hist, enabled, "recovery_conflict", log)?;
    for (id, company) in [
        ("recover-a1", "alone"),
        ("recover-a2", "alone"),
        ("recover-b1", "friends"),
        ("recover-b2", "friends"),
    ] {
        observed(&mut m, id, a, company, None)?;
    }
    checkpoint(&mut m, &parent, hist, enabled, "recovered", log)?;
    if enabled && m.store.traces[&parent].personal.scope_status != ScopeStatus::General {
        return Err(format!("{hist}: recovery did not restore general state"));
    }
    // Save second checkpoint under an independent path using a new vault file.
    let recovered = out.join(format!("{hist}_{state}_recovered.vault"));
    let original = m.path.replace(recovered);
    snapshot(&m)?;
    m.path = original;
    Ok(())
}
fn sham(out: &Path, log: &mut BufWriter<File>) -> Result<(), String> {
    // Confounded factors: company and time move in lockstep. Must abstain.
    let path = out.join("sham_confounded.vault");
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let mut m = SelectiveMemory::open(&path, profile)
        .map_err(|e| e.to_string())?
        .with_scorer(Box::new(FixtureWitness));
    m.store.contextual_scope.enabled = true;
    let q1 = observed(&mut m, "sham-q1", QUIET, "alone", Some(("time", "day")))?;
    let q2 = observed(&mut m, "sham-q2", QUIET, "alone", Some(("time", "day")))?;
    let parent = seed_parent(&mut m, QUIET, q1, q2)?;
    observed(
        &mut m,
        "sham-b1",
        LIVELY,
        "friends",
        Some(("time", "night")),
    )?;
    observed(
        &mut m,
        "sham-b2",
        LIVELY,
        "friends",
        Some(("time", "night")),
    )?;
    let (c, q) = checkpoint(&mut m, &parent, "confounded", true, "negative_control", log)?;
    if c != 0 || q != 0 {
        return Err("confounded sham incorrectly formed scope".into());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Arm {
    On,
    Off,
    EpisodesOnly,
    Bare,
}
impl Arm {
    fn token(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
            Self::EpisodesOnly => "episodes_only",
            Self::Bare => "bare",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Qualified,
    Recovered,
}
impl Phase {
    fn token(self) -> &'static str {
        match self {
            Self::Qualified => "qualified",
            Self::Recovered => "recovered",
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Trial {
    reverse: bool,
    arm: Arm,
    phase: Phase,
    friends: bool,
    wording: usize,
    order: usize,
    repeat: usize,
}
impl Trial {
    fn tag(self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.reverse,
            self.arm.token(),
            self.phase.token(),
            self.friends,
            self.wording,
            self.order,
            self.repeat
        )
    }
    fn expected(self) -> &'static str {
        let a = if self.reverse { LIVELY } else { QUIET };
        let b = if self.reverse { QUIET } else { LIVELY };
        if self.phase == Phase::Recovered || !self.friends {
            a
        } else {
            b
        }
    }
    fn task(self) -> String {
        let setting = if self.friends {
            if self.wording == 0 {
                "You're going out with close friends."
            } else {
                "You are arranging an evening together with people you know well."
            }
        } else if self.wording == 0 {
            "You're going out on your own."
        } else {
            "You're arranging an evening by yourself."
        };
        let (first, second) = if self.order == 0 {
            ("a quiet venue", "a lively venue")
        } else {
            ("a lively venue", "a quiet venue")
        };
        format!("{setting} You can pick one of two equally accessible places with the same price and quality. Which would you choose?\n1. {first}\n2. {second}\nReply with only 1 or 2.")
    }
    fn action_for(self, raw: &str) -> Option<&'static str> {
        let raw = raw.trim();
        match (self.order, raw) {
            (0, "1") | (1, "2") => Some(QUIET),
            (0, "2") | (1, "1") => Some(LIVELY),
            _ => None,
        }
    }
}
fn actor_trials(repeats: usize) -> Vec<Trial> {
    let mut out = Vec::new();
    for reverse in [false, true] {
        for arm in [Arm::On, Arm::Off, Arm::EpisodesOnly, Arm::Bare] {
            for phase in [Phase::Qualified, Phase::Recovered] {
                for friends in [false, true] {
                    for wording in 0..2 {
                        for order in 0..2 {
                            for repeat in 0..repeats {
                                out.push(Trial {
                                    reverse,
                                    arm,
                                    phase,
                                    friends,
                                    wording,
                                    order,
                                    repeat,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
fn rank(seed: u64, key: &str) -> u64 {
    // Deterministic, noncryptographic schedule. Stable for published seed.
    let mut h = seed ^ 0xcbf29ce484222325;
    for b in key.bytes() {
        h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
    }
    h
}
fn actor(
    t: Trial,
    dir: &Path,
    provider: &str,
    model: &str,
    key: &str,
) -> Result<(String, Vec<String>), String> {
    let history = if t.reverse { "reversed" } else { "normal" };
    let state = if t.arm == Arm::Off { "off" } else { "on" };
    let snapshot = dir.join(format!("{history}_{state}_{}.vault", t.phase.token()));
    let mut m = if t.arm == Arm::Bare {
        SelectiveMemory::new(EntityProfile::new("Claire"))
    } else {
        SelectiveMemory::open(&snapshot, EntityProfile::new("Claire")).map_err(|e| e.to_string())?
    };
    m.store.contextual_scope.enabled = t.arm != Arm::Off && t.arm != Arm::Bare;
    let options = MemoryContextOptions {
        dispositions: t.arm != Arm::EpisodesOnly && t.arm != Arm::Bare,
        episodes: t.arm != Arm::Bare,
        associations: false,
        affect: false,
        ..MemoryContextOptions::default()
    };
    m.set_memory_context_options(options)?;
    let task = t.task();
    let context = if t.friends { "friends" } else { "alone" };
    let draft = m.open_mouth_scoped(&task, &[ContextFacet::new("company", context)])?;
    let selected = draft
        .memory_context
        .self_model
        .iter()
        .map(|s| s.trace_id.clone())
        .collect::<Vec<_>>();
    let system = draft.memory_reply().system_prompt();
    let user = draft.memory_reply().user_prompt();
    m.cancel_mouth(&draft);
    let request=obj([
        ("model",text(model)), ("temperature",num(0.0)), ("max_tokens",num(24.0)),
        ("messages",Json::Array(vec![
            obj([("role",text("system")),("content",text(format!("{system}\nFor this response, output exactly one digit: 1 or 2. No explanation.")))]),
            obj([("role",text("user")),("content",text(user))]),
        ])),
    ]);
    let endpoint = if provider == "grok" {
        "https://api.x.ai/v1/chat/completions"
    } else {
        "https://api.openai.com/v1/chat/completions"
    };
    let raw = selmem::net::httpx::post_json(endpoint, Some(key), &request.encode())?;
    let tree = json::parse(&raw)?;
    let content = tree
        .get("choices")?
        .array()?
        .first()
        .ok_or("empty choices")?
        .get("message")?
        .get("content")?
        .string()?
        .to_string();
    Ok((content, selected))
}
fn run_actor(c: &Config, dir: &Path) -> Result<(), String> {
    let env_name = if c.provider == "grok" {
        "GROK_API_KEY"
    } else {
        "OPENAI_API_KEY"
    };
    let key = std::env::var(env_name).map_err(|_| format!("{env_name} not set"))?;
    if key.trim().is_empty() {
        return Err(format!("{env_name} is empty"));
    }
    let mut trials = actor_trials(c.repeats);
    let count = trials.len();
    if count > c.max_http_calls.unwrap_or(0) {
        return Err("model-call budget exceeded".into());
    }
    trials.sort_by_key(|t| (rank(c.seed, &t.tag()), t.tag()));
    let file = File::create(c.out.join("choices.jsonl")).map_err(|e| e.to_string())?;
    let mut out = BufWriter::new(file);
    let mut totals: BTreeMap<(String, String, String, String), (usize, usize, usize)> =
        BTreeMap::new();
    for (index, t) in trials.iter().copied().enumerate() {
        let (raw, selected, error) = match actor(t, dir, &c.provider, &c.model, &key) {
            Ok((raw, selected)) => (raw, selected, String::new()),
            Err(e) => (String::new(), Vec::new(), e),
        };
        let answer = t.action_for(&raw);
        let correct = answer == Some(t.expected());
        let mapkey = (
            if t.reverse {
                "reversed".to_string()
            } else {
                "normal".to_string()
            },
            t.arm.token().to_string(),
            t.phase.token().to_string(),
            if t.friends {
                "friends".into()
            } else {
                "alone".into()
            },
        );
        let totals_row = totals.entry(mapkey).or_default();
        totals_row.0 += 1;
        if answer.is_some() {
            totals_row.1 += 1;
        }
        if correct {
            totals_row.2 += 1;
        }
        let row = obj([
            ("kind", text("choice")),
            ("trial", num(index as f64)),
            (
                "history",
                text(if t.reverse { "reversed" } else { "normal" }),
            ),
            ("condition", text(t.arm.token())),
            ("phase", text(t.phase.token())),
            ("company", text(if t.friends { "friends" } else { "alone" })),
            ("wording", num(t.wording as f64)),
            ("order", num(t.order as f64)),
            ("repeat", num(t.repeat as f64)),
            ("expected_action", text(t.expected())),
            ("chosen_action", text(answer.unwrap_or(""))),
            ("raw_answer", text(&raw)),
            ("valid", Json::Bool(answer.is_some())),
            ("concordant", Json::Bool(correct)),
            (
                "self_trace_ids",
                Json::Array(selected.into_iter().map(text).collect()),
            ),
            ("error", text(&error)),
        ]);
        writeln!(out, "{}", row.encode()).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        if !error.is_empty() {
            return Err(format!(
                "actor call failed at {} (no retry): {}",
                t.tag(),
                error
            ));
        }
        eprintln!("actor {}/{count} {}", index + 1, t.tag());
    }
    let mut summary =
        BufWriter::new(File::create(c.out.join("summary.jsonl")).map_err(|e| e.to_string())?);
    for ((history, arm, phase, company), (n, valid, correct)) in totals {
        let row = obj([
            ("history", text(history)),
            ("arm", text(arm)),
            ("phase", text(phase)),
            ("company", text(company)),
            ("attempts", num(n as f64)),
            ("valid", num(valid as f64)),
            ("concordant", num(correct as f64)),
            ("concordance_per_attempt", num(correct as f64 / n as f64)),
        ]);
        writeln!(summary, "{}", row.encode()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn run(c: Config) -> Result<(), String> {
    fs::create_dir(&c.out).map_err(|e| e.to_string())?;
    let snapshots = c.out.join("snapshots");
    fs::create_dir(&snapshots).map_err(|e| e.to_string())?;
    let mut log =
        BufWriter::new(File::create(c.out.join("organ.jsonl")).map_err(|e| e.to_string())?);
    for reverse in [false, true] {
        for enabled in [false, true] {
            build_history(&snapshots, reverse, enabled, &mut log)?;
        }
    }
    sham(&snapshots, &mut log)?;
    log.flush().map_err(|e| e.to_string())?;
    let metadata = obj([
        ("version", text("genesis-contextual-experiment-1")),
        ("synthetic", Json::Bool(true)),
        ("actor_enabled", Json::Bool(c.actor)),
        (
            "provider",
            text(if c.actor { c.provider.as_str() } else { "none" }),
        ),
        (
            "model",
            text(if c.actor { c.model.as_str() } else { "none" }),
        ),
        (
            "planned_http_calls",
            num(if c.actor {
                actor_trials(c.repeats).len() as f64
            } else {
                0.0
            }),
        ),
        ("seed", num(c.seed as f64)),
    ]);
    fs::write(c.out.join("manifest.json"), metadata.encode()).map_err(|e| e.to_string())?;
    if c.actor {
        run_actor(&c, &snapshots)?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = Config::parse().and_then(run) {
        eprintln!("GENESIS contextual experiment: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trial_matrix_and_option_counterbalance() {
        let trials = actor_trials(1);
        assert_eq!(trials.len(), 128);
        let mut counts = BTreeMap::new();
        for t in &trials {
            *counts
                .entry((t.reverse, t.arm, t.phase, t.friends))
                .or_insert(0usize) += 1;
            let expected = if t.order == 0 { "1" } else { "2" };
            assert_eq!(t.action_for(expected), Some(QUIET));
        }
        assert!(counts.values().all(|v| *v == 4));
    }
    #[test]
    fn sequence_order_is_stable() {
        let a = actor_trials(2);
        assert_eq!(rank(17, &a[0].tag()), rank(17, &a[0].tag()));
        assert_ne!(a[0].tag(), a[1].tag());
    }
    #[test]
    fn offline_qualification_is_end_to_end_and_free() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "selmem-contextual-exp-{}-{stamp}",
            std::process::id()
        ));
        let c = Config {
            out: dir.clone(),
            actor: false,
            provider: "grok".into(),
            model: "grok-4.3".into(),
            repeats: 1,
            max_http_calls: None,
            seed: 1729,
        };
        let result = run(c);
        let read = fs::read_to_string(dir.join("organ.jsonl"));
        let _ = fs::remove_dir_all(&dir);
        result.unwrap();
        let evidence = read.unwrap();
        assert!(evidence.contains("qualified"));
        assert!(evidence.contains("negative_control"));
    }
}
