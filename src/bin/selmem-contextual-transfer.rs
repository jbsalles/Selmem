//! GENESIS contextual transfer v3: long-horizon bounded memory, independent synthetic domains, mirrored lives,
//! matched observations, blind novel-choice prompts, and optional LLM veto.
//! No extra Cargo dependencies. Requires SelMem contextual self-belief extension.
use selmem::util::json;

use json::{num, obj, text, Json};
use selmem::core::model::{Bearer, StakeKind};
use selmem::core::self_model::{consolidate_with_scorer, view_scoped};
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{
    Attribution, ContextFacet, EncodeInput, EntityProfile, PropositionLabel, PropositionScorer,
    ScopeStatus, SelectiveMemory, SelfMemory, SelfRole, SemanticInterpreter,
};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct Domain {
    id: &'static str,
    topic: &'static str,
    a: &'static str,
    b: &'static str,
    // Held-out phrases must not reproduce either historical action verbatim.
    novel_a: &'static str,
    novel_b: &'static str,
    paraphrase_a: &'static str,
    paraphrase_b: &'static str,
}
const DOMAINS: [Domain; 12] = [
    Domain {
        id: "workspace",
        topic: "work setting",
        a: "I chose a secluded work desk.",
        b: "I chose a bustling coworking room.",
        novel_a: "a private focus booth",
        novel_b: "an energetic shared floor",
        paraphrase_a: "a tucked-away concentration cubicle",
        paraphrase_b: "a lively collaborative area",
    },
    Domain {
        id: "dining",
        topic: "dinner",
        a: "I chose a hushed little restaurant.",
        b: "I chose a crowded dining hall.",
        novel_a: "an intimate courtyard cafe",
        novel_b: "a busy food market",
        paraphrase_a: "a peaceful candlelit bistro",
        paraphrase_b: "a lively communal canteen",
    },
    Domain {
        id: "fitness",
        topic: "exercise",
        a: "I chose a solitary woodland walk.",
        b: "I chose a packed exercise class.",
        novel_a: "a remote riverside trail",
        novel_b: "an animated group workout",
        paraphrase_a: "an uncrowded nature path",
        paraphrase_b: "a high-energy training session",
    },
    Domain {
        id: "books",
        topic: "literary activity",
        a: "I chose a quiet reading library.",
        b: "I chose a lively book discussion.",
        novel_a: "a silent reading alcove",
        novel_b: "a lively literary gathering",
        paraphrase_a: "a calm individual reading session",
        paraphrase_b: "an animated readers' roundtable",
    },
    Domain {
        id: "museum",
        topic: "museum visit",
        a: "I chose an empty art gallery.",
        b: "I chose a crowded interactive exhibition.",
        novel_a: "a tranquil painting room",
        novel_b: "a busy hands-on exhibit",
        paraphrase_a: "a serene private collection",
        paraphrase_b: "a popular interactive science hall",
    },
    Domain {
        id: "music",
        topic: "musical performance",
        a: "I chose a soft acoustic recital.",
        b: "I chose a loud festival stage.",
        novel_a: "a mellow chamber performance",
        novel_b: "an energetic outdoor concert",
        paraphrase_a: "an intimate unplugged set",
        paraphrase_b: "a high-volume arena show",
    },
    Domain {
        id: "commute",
        topic: "city route",
        a: "I chose a peaceful side street.",
        b: "I chose a busy central boulevard.",
        novel_a: "a tranquil residential lane",
        novel_b: "a vibrant main avenue",
        paraphrase_a: "a quiet scenic back road",
        paraphrase_b: "a crowded downtown thoroughfare",
    },
    Domain {
        id: "lodging",
        topic: "weekend stay",
        a: "I chose a secluded cabin.",
        b: "I chose a social holiday resort.",
        novel_a: "a remote lakeside cottage",
        novel_b: "a lively communal hotel",
        paraphrase_a: "a peaceful rural hideaway",
        paraphrase_b: "a bustling vacation complex",
    },
    Domain {
        id: "lunch",
        topic: "lunchtime location",
        a: "I chose a calm botanical garden.",
        b: "I chose a packed city plaza.",
        novel_a: "a secluded greenhouse bench",
        novel_b: "a busy public square",
        paraphrase_a: "a peaceful riverside lawn",
        paraphrase_b: "a lively central courtyard",
    },
    Domain {
        id: "shopping",
        topic: "shopping",
        a: "I chose a small artisan boutique.",
        b: "I chose a crowded shopping mall.",
        novel_a: "a quiet independent shop",
        novel_b: "a bustling commercial center",
        paraphrase_a: "a peaceful specialty store",
        paraphrase_b: "a busy indoor marketplace",
    },
    Domain {
        id: "learning",
        topic: "learning activity",
        a: "I chose a silent individual tutorial.",
        b: "I chose a noisy group workshop.",
        novel_a: "a focused self-paced lesson",
        novel_b: "an interactive group seminar",
        paraphrase_a: "an independent quiet course",
        paraphrase_b: "a lively team training session",
    },
    Domain {
        id: "entertainment",
        topic: "evening entertainment",
        a: "I chose a calm film screening.",
        b: "I chose a crowded trivia party.",
        novel_a: "a quiet independent cinema",
        novel_b: "a lively quiz night",
        paraphrase_a: "an intimate documentary showing",
        paraphrase_b: "an animated social game event",
    },
];

#[derive(Clone)]
struct Config {
    out: PathBuf,
    pairs: usize,
    actor: bool,
    judge: bool,
    provider: String,
    model: String,
    judge_provider: String,
    judge_model: String,
    max_http_calls: usize,
    seed: u64,
    distractors: usize,
    memory_words: usize,
}
impl Config {
    fn parse() -> Result<Self, String> {
        let mut c = Self {
            out: PathBuf::new(),
            pairs: 12,
            actor: false,
            judge: false,
            provider: "grok".into(),
            model: "grok-4.3".into(),
            judge_provider: "openai".into(),
            judge_model: "gpt-4.1-mini".into(),
            max_http_calls: 0,
            seed: 1729,
            distractors: 160,
            memory_words: 64,
        };
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            if flag == "--help" || flag == "-h" {
                println!("GENESIS contextual transfer v3 (Rust only)\n--out NEW_DIR [--pairs 1..12] [--seed N]\n--actor --provider grok|openai --model MODEL --max-http-calls N\n[--judge --judge-provider grok|openai --judge-model MODEL]\nDefault offline qualification: 0 API calls.\n--distractors 160 --memory-words 64\nActor: 48*--pairs calls (two mirrored lives, two contexts, six arms, two counterbalanced wording/order cells).\nIndependent judge: +4*--pairs calls. All calls counted against the same strict cap.");
                std::process::exit(0);
            }
            if flag == "--actor" {
                c.actor = true;
                continue;
            }
            if flag == "--judge" {
                c.judge = true;
                continue;
            }
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--out" => c.out = PathBuf::from(value),
                "--pairs" => c.pairs = value.parse().map_err(|_| "invalid pairs")?,
                "--seed" => c.seed = value.parse().map_err(|_| "invalid seed")?,
                "--distractors" => {
                    c.distractors = value.parse().map_err(|_| "invalid distractors")?
                }
                "--memory-words" => {
                    c.memory_words = value.parse().map_err(|_| "invalid memory-words")?
                }
                "--provider" => c.provider = value,
                "--model" => c.model = value,
                "--judge-provider" => c.judge_provider = value,
                "--judge-model" => c.judge_model = value,
                "--max-http-calls" => {
                    c.max_http_calls = value.parse().map_err(|_| "invalid max-http-calls")?
                }
                _ => return Err(format!("unknown flag {flag}")),
            }
        }
        if c.out.as_os_str().is_empty() || c.out.exists() {
            return Err("--out must name a new directory".into());
        }
        if !(1..=DOMAINS.len()).contains(&c.pairs) {
            return Err("--pairs must be 1..=12".into());
        }
        if c.distractors < 40 || c.distractors > 500 {
            return Err("--distractors must be 40..=500".into());
        }
        if c.memory_words < 24 || c.memory_words > 256 {
            return Err("--memory-words must be 24..=256".into());
        }
        if c.judge && !c.actor {
            return Err("--judge requires --actor".into());
        }
        for p in [&c.provider, &c.judge_provider] {
            if p != "grok" && p != "openai" {
                return Err("provider must be grok or openai".into());
            }
        }
        if c.model.trim().is_empty() || c.judge_model.trim().is_empty() {
            return Err("empty model".into());
        }
        if c.judge && c.provider == c.judge_provider && c.model == c.judge_model {
            return Err("independent judge must use a different provider or model".into());
        }
        let needed = trials(c.pairs).len() + if c.judge { c.pairs * 4 } else { 0 };
        if c.actor && c.max_http_calls < needed {
            return Err(format!(
                "requires {needed} calls; supply --max-http-calls {needed} or greater"
            ));
        }
        Ok(c)
    }
}

struct FixtureWitness {
    a: &'static str,
    b: &'static str,
}
impl PropositionScorer for FixtureWitness {
    fn name(&self) -> &str {
        "genesis-transfer-structured-observation-oracle"
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
        if [self.a, self.b].contains(&expectation) && traces.iter().all(|t| t.gist == expectation) {
            PropositionLabel::Entail
        } else {
            PropositionLabel::Unknown
        }
    }
    fn score_expectation(&self, expectation: &str, event: &str) -> PropositionLabel {
        if expectation == event {
            PropositionLabel::Entail
        } else if [self.a, self.b].contains(&expectation) && [self.a, self.b].contains(&event) {
            PropositionLabel::Contradict
        } else {
            PropositionLabel::Unknown
        }
    }
}
fn observed(
    m: &mut SelectiveMemory,
    source: &str,
    action: &str,
    company: &str,
) -> Result<String, String> {
    let mut input = EncodeInput::observed(action, source);
    input.source = "genesis-transfer-structured-observation";
    input.attribution = Attribution::Internal;
    input.schema = Some("choice-pattern".into());
    input.permanence = 0.9;
    input.personal.scope_facets = vec![ContextFacet::new("company", company)];
    let mut sem = selmem::LexicalInterpreter::default().interpret_event(action)?;
    sem.schema = Some("choice-pattern".into());
    sem.stake_kind = StakeKind::Decision;
    sem.bearer = Bearer::Self_;
    sem.core.claim = action.into();
    input.semantics = Some(sem);
    let result = m.live_with(input);
    if !result.kept {
        return Err(format!("failed to encode {source}: {}", result.reason));
    }
    result
        .trace_id
        .ok_or_else(|| format!("no trace for {source}"))
}
fn seed_parent(
    m: &mut SelectiveMemory,
    action: &str,
    a: String,
    b: String,
) -> Result<String, String> {
    let mut input = EncodeInput::new("I have repeatedly chosen the same kind of setting.");
    input.attribution = Attribution::Internal;
    input.schema = Some("choice-pattern".into());
    input.personal = SelfMemory {
        role: SelfRole::Belief,
        context: "choice-pattern".into(),
        expectation: action.into(),
        certainty: 0.65,
        importance: 0.8,
        support_ids: vec![a, b],
        ..SelfMemory::default()
    };
    m.live_with(input)
        .trace_id
        .ok_or_else(|| "failed to seed broad belief".into())
}
fn history_path(dir: &Path, d: Domain, reversed: bool, on: bool) -> PathBuf {
    dir.join(format!(
        "{}-{}-{}.vault",
        d.id,
        if reversed { "reversed" } else { "normal" },
        if on { "on" } else { "off" }
    ))
}
fn build_life(
    dir: &Path,
    d: Domain,
    reversed: bool,
    on: bool,
    log: &mut BufWriter<File>,
    distractors: usize,
    seed: u64,
) -> Result<(), String> {
    let (a, b) = if reversed { (d.b, d.a) } else { (d.a, d.b) };
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let path = history_path(dir, d, reversed, on);
    let mut m = SelectiveMemory::open(&path, profile)
        .map_err(|e| e.to_string())?
        .with_scorer(Box::new(FixtureWitness { a: d.a, b: d.b }));
    m.store.contextual_scope.enabled = on;
    let x = observed(&mut m, "formation-1", a, "alone")?;
    let y = observed(&mut m, "formation-2", a, "alone")?;
    let parent = seed_parent(&mut m, a, x, y)?;
    observed(&mut m, "exception-1", b, "friends")?;
    observed(&mut m, "exception-2", b, "friends")?;
    consolidate_with_scorer(
        &mut m.store,
        &RuleNarrator,
        &HashEmbedder,
        &FixtureWitness { a: d.a, b: d.b },
    );
    let hidden = m
        .store
        .traces
        .values()
        .filter(|t| {
            t.personal.scope_parent.as_deref() == Some(parent.as_str())
                && t.personal.scope_status == ScopeStatus::Candidate
        })
        .count();
    if (on && hidden != 2) || (!on && hidden != 0) {
        return Err(format!("{} candidate gate failed", d.id));
    }
    observed(&mut m, "prospective-alone", a, "alone")?;
    observed(&mut m, "prospective-friends", b, "friends")?;
    consolidate_with_scorer(
        &mut m.store,
        &RuleNarrator,
        &HashEmbedder,
        &FixtureWitness { a: d.a, b: d.b },
    );
    let qualified = m
        .store
        .traces
        .values()
        .filter(|t| {
            t.personal.scope_parent.as_deref() == Some(parent.as_str())
                && t.personal.scope_status == ScopeStatus::Conditional
        })
        .count();
    if (on && qualified != 2) || (!on && qualified != 0) {
        return Err(format!("{} qualification failed ({qualified})", d.id));
    }
    let read = |value: &str| {
        view_scoped(
            &m.store,
            "choice-pattern",
            Some("choice-pattern"),
            &[ContextFacet::new("company", value)],
        )
        .into_iter()
        .map(|t| t.personal.expectation.as_str())
        .collect::<Vec<_>>()
    };
    let alone = read("alone");
    let friends = read("friends");
    if on && (!alone.contains(&a) || !friends.contains(&b)) {
        return Err(format!("{} contextual retrieval failed", d.id));
    }
    writeln!(
        log,
        "{}",
        obj([
            ("domain", text(d.id)),
            (
                "history",
                text(if reversed { "reversed" } else { "normal" })
            ),
            ("arm", text(if on { "on" } else { "off" })),
            ("candidates_before_validation", num(hidden as f64)),
            ("qualified", num(qualified as f64)),
            (
                "alone_readout",
                Json::Array(alone.into_iter().map(text).collect())
            ),
            (
                "friends_readout",
                Json::Array(friends.into_iter().map(text).collect())
            ),
        ])
        .encode()
    )
    .map_err(|e| e.to_string())?;
    // Interleave old, irrelevant experiences in the *actual* SelMem vault,
    // not just a fabricated actor prompt. Identical for ON and OFF.
    for i in 0..distractors {
        let themes = [
            "bicycle maintenance",
            "garden watering",
            "ticket sorting",
            "weather notes",
            "home repairs",
            "plant cataloguing",
            "parcel labeling",
            "kitchen inventory",
        ];
        let theme = themes
            [((rank(seed, &format!("{}:{}:{}", d.id, reversed, i)) >> 8) as usize) % themes.len()];
        let event =
            format!("Archive note {i}: I completed a routine task about {theme} during the week.");
        let mut input = EncodeInput::new(&event);
        input.source = "genesis-transfer-background";
        input.permanence = 0.6;
        input.attribution = Attribution::External;
        let result = m.live_with(input);
        if !result.kept {
            return Err(format!(
                "background observation {i} dropped: {}",
                result.reason
            ));
        }
    }
    m.save().map_err(|e| format!("save {}: {e}", d.id))
}
fn cur_domains() -> &'static [Domain] {
    &DOMAINS
}
include!("shared/contextual_trials.rs");

fn load(dir: &Path, d: Domain, reversed: bool, on: bool) -> Result<SelectiveMemory, String> {
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let mut m = SelectiveMemory::open(&history_path(dir, d, reversed, on), profile)
        .map_err(|e| format!("reading snapshot: {e}"))?;
    m.store.contextual_scope.enabled = on;
    Ok(m)
}
fn episodes(m: &SelectiveMemory) -> Vec<String> {
    let mut e = m
        .store
        .traces
        .values()
        .filter(|t| {
            t.source == "genesis-transfer-structured-observation"
                && t.personal.role == SelfRole::Episode
        })
        .collect::<Vec<_>>();
    e.sort_by(|a, b| a.observation_id.cmp(&b.observation_id));
    e.into_iter()
        .map(|t| {
            format!(
                "company={} | {}",
                t.personal
                    .scope_facets
                    .iter()
                    .find(|f| f.dimension == "company")
                    .map(|f| f.value.as_str())
                    .unwrap_or("unknown"),
                t.gist
            )
        })
        .collect()
}
// All readouts are capped by the SAME word budget. The budget is a maximum,
// not artificial filler to equalize content. No original six-line table leaks
// into ON/OFF readouts. Full episodes are available only to comparator methods.
fn cap_words(s: &str, budget: usize) -> String {
    s.split_whitespace()
        .take(budget)
        .collect::<Vec<_>>()
        .join(" ")
}
fn material(t: Trial, dir: &Path, budget: usize) -> Result<(String, Vec<String>, usize), String> {
    if t.arm == Arm::Bare {
        return Ok((String::new(), Vec::new(), 0));
    }
    let d = DOMAINS[t.domain];
    let m = load(dir, d, t.reversed, t.arm != Arm::Off)?;
    let facts = episodes(&m);
    if facts.len() != 6 {
        return Err(format!(
            "{} expected six events, found {}",
            d.id,
            facts.len()
        ));
    }
    let context = if t.company { "friends" } else { "alone" };
    let mut selected = Vec::new();
    let raw = match t.arm {
        Arm::On | Arm::Off => {
            let matching = view_scoped(
                &m.store,
                "choice-pattern",
                Some("choice-pattern"),
                &[ContextFacet::new("company", context)],
            );
            let relevant = matching
                .into_iter()
                .find(|s| s.personal.scope_status == ScopeStatus::Conditional)
                .or_else(|| {
                    view_scoped(
                        &m.store,
                        "choice-pattern",
                        Some("choice-pattern"),
                        &[ContextFacet::new("company", context)],
                    )
                    .into_iter()
                    .next()
                })
                .ok_or("no active self-belief")?;
            selected.push(relevant.id.clone());
            format!(
                "Historical self-model, fallible: {}. Prospective action record: {}",
                relevant.gist, relevant.personal.expectation
            )
        }
        Arm::Episodes => {
            // Unselected recency window: newest background events displace
            // the relevant six older decisions as the history grows.
            let mut events = m
                .store
                .traces
                .values()
                .filter(|r| r.source == "genesis-transfer-background")
                .collect::<Vec<_>>();
            events.sort_by(|x, y| y.id.cmp(&x.id));
            events
                .into_iter()
                .map(|r| r.gist.clone())
                .collect::<Vec<_>>()
                .join(" ")
        }
        Arm::Summary => {
            // Strong compression baseline: unbiased aggregate across ALL six
            // original decisions, rather than one preferred target outcome.
            let counts = |side: &str, act: &str| {
                facts
                    .iter()
                    .filter(|r| r.contains(&format!("company={side} |")) && r.contains(act))
                    .count()
            };
            format!("Aggregate of six recorded decisions (no self-model): alone: {} x [{}], {} x [{}]; friends: {} x [{}], {} x [{}].",
                counts("alone",d.a),d.a,counts("alone",d.b),d.b,
                counts("friends",d.a),d.a,counts("friends",d.b),d.b)
        }
        Arm::Retrieval => {
            // Strong task-aware episodic retrieval. This is intentionally a
            // competitive baseline, not a straw-man FIFO truncation.
            let relevant = facts
                .iter()
                .filter(|r| r.starts_with(&format!("company={context} |")))
                .cloned()
                .collect::<Vec<_>>();
            format!("Task-relevant recorded decisions: {}", relevant.join(" ; "))
        }
        Arm::Bare => unreachable!(),
    };
    let body = cap_words(&raw, budget);
    let words = body.split_whitespace().count();
    if words > budget {
        return Err("memory budget violated".into());
    }
    Ok((body, selected, words))
}

fn judge(c: &Config, dir: &Path, judge_key: &str) -> Result<(), String> {
    let file = File::create(c.out.join("judgements.jsonl")).map_err(|e| e.to_string())?;
    let mut out = BufWriter::new(file);
    let system="Independent memory audit. Judge only whether a tentative context-specific action prediction is supported by at least two independent observed actions in that context. Do not infer preferences, motives, feelings, causal mechanisms, or psychological traits. Reply exactly entail, contradict, or unknown.";
    for domain in 0..c.pairs {
        for reversed in [false, true] {
            for company in [false, true] {
                let d = DOMAINS[domain];
                let m = load(dir, d, reversed, true)?;
                let context = if company { "friends" } else { "alone" };
                let relevant = view_scoped(
                    &m.store,
                    "choice-pattern",
                    Some("choice-pattern"),
                    &[ContextFacet::new("company", context)],
                )
                .into_iter()
                .find(|t| t.personal.scope_status == ScopeStatus::Conditional)
                .ok_or("unqualified context")?;
                let supporting = episodes(&m)
                    .into_iter()
                    .filter(|s| s.starts_with(&format!("company={context} |")))
                    .collect::<Vec<_>>()
                    .join("\n");
                let prompt=format!("Independent observations (one line per event):\n{supporting}\n\nTentative statement: {}\nPredicted action: {}",
            relevant.gist,relevant.personal.expectation);
                let label = chat(
                    &c.judge_provider,
                    &c.judge_model,
                    judge_key,
                    system,
                    &prompt,
                    12.,
                )?
                .to_ascii_lowercase();
                let approved = label == "entail";
                writeln!(
                    out,
                    "{}",
                    obj([
                        ("domain", text(d.id)),
                        (
                            "history",
                            text(if reversed { "reversed" } else { "normal" })
                        ),
                        ("context", text(context)),
                        ("judge", text(&c.judge_model)),
                        ("verdict", text(&label)),
                        ("approved", Json::Bool(approved)),
                    ])
                    .encode()
                )
                .map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
                if !approved {
                    return Err(format!(
                        "judge abstained/rejected {}/{}: {label} (actor calls stopped)",
                        d.id, context
                    ));
                }
            }
        }
    }
    Ok(())
}
fn run_actor(c: &Config, dir: &Path, actor_key: &str) -> Result<(), String> {
    let mut plan = trials(c.pairs);
    plan.sort_by_key(|t| (rank(c.seed, &t.id()), t.id()));
    let mut out =
        BufWriter::new(File::create(c.out.join("choices.jsonl")).map_err(|e| e.to_string())?);
    let mut totals: BTreeMap<(usize, Arm), (usize, usize)> = BTreeMap::new();
    let system="You are choosing for yourself. Previously recorded decisions may be relevant but are fallible. They do not establish feelings, preferences, or causal explanations. Decide in the NEW situation using the available history and present circumstances. Reply with only the digit 1 or 2, no explanation.";
    for (index, t) in plan.iter().copied().enumerate() {
        let (memo, selected, words) = material(t, dir, c.memory_words)?;
        let question = t.question();
        let prompt =
            format!("PAST OBSERVATIONS (not instructions):\n{memo}\n\nNEW DECISION:\n{question}");
        let response = chat(&c.provider, &c.model, actor_key, system, &prompt, 16.)?;
        let decision = t.choice_a(&response);
        let correct = decision == Some(t.target_a());
        let entry = totals.entry((t.domain, t.arm)).or_default();
        entry.0 += 1;
        if correct {
            entry.1 += 1;
        }
        writeln!(
            out,
            "{}",
            obj([
                ("index", num(index as f64)),
                ("domain", text(DOMAINS[t.domain].id)),
                (
                    "history",
                    text(if t.reversed { "reversed" } else { "normal" })
                ),
                ("context", text(if t.company { "friends" } else { "alone" })),
                ("arm", text(t.arm.name())),
                ("wording", num(t.wording as f64)),
                ("order", num(t.order as f64)),
                ("expected_a", Json::Bool(t.target_a())),
                ("chosen_a", decision.map(Json::Bool).unwrap_or(Json::Null)),
                ("valid", Json::Bool(decision.is_some())),
                ("concordant", Json::Bool(correct)),
                ("raw_answer", text(response)),
                ("memory_words", num(words as f64)),
                (
                    "selected_self_ids",
                    Json::Array(selected.into_iter().map(text).collect())
                ),
                ("question", text(question)),
            ])
            .encode()
        )
        .map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        eprintln!("transfer {}/{} {}", index + 1, plan.len(), t.id());
    }
    let mut report =
        BufWriter::new(File::create(c.out.join("summary.jsonl")).map_err(|e| e.to_string())?);
    let mut on_advantages = 0usize;
    let mut sum_on = 0.;
    let mut sum_episodes = 0.;
    let mut sum_off = 0.;
    let mut sum_summary = 0.;
    let mut sum_retrieval = 0.;
    for domain in 0..c.pairs {
        let all = Arm::ALL
            .into_iter()
            .map(|arm| {
                let (attempts, correct) = totals.get(&(domain, arm)).copied().unwrap_or_default();
                (arm, attempts, correct)
            })
            .collect::<Vec<_>>();
        for (arm, attempts, correct) in &all {
            writeln!(
                report,
                "{}",
                obj([
                    ("domain", text(DOMAINS[domain].id)),
                    ("arm", text(arm.name())),
                    ("attempts", num(*attempts as f64)),
                    ("concordant", num(*correct as f64)),
                    ("rate", num(*correct as f64 / (*attempts).max(1) as f64)),
                ])
                .encode()
            )
            .map_err(|e| e.to_string())?;
        }
        let rate = |arm: Arm| {
            let (n, c) = totals[&(domain, arm)];
            c as f64 / n as f64
        };
        sum_on += rate(Arm::On);
        sum_episodes += rate(Arm::Episodes);
        sum_off += rate(Arm::Off);
        sum_summary += rate(Arm::Summary);
        sum_retrieval += rate(Arm::Retrieval);
        if rate(Arm::On) > rate(Arm::Episodes) {
            on_advantages += 1;
        }
    }
    let delta = (sum_on - sum_episodes) / c.pairs as f64;
    let sign_threshold = if c.pairs == 12 { 10 } else { c.pairs };
    let primary_pass = c.pairs == 12 && delta >= 0.15 && on_advantages >= sign_threshold;
    let final_summary = obj([
        ("kind", text("preregistered_primary")),
        ("effective_independent_domains", num(c.pairs as f64)),
        ("mirrored_lives", num((2 * c.pairs) as f64)),
        ("on_minus_episodes", num(delta)),
        ("on_minus_off", num((sum_on - sum_off) / c.pairs as f64)),
        (
            "on_minus_summary",
            num((sum_on - sum_summary) / c.pairs as f64),
        ),
        (
            "on_minus_retrieval",
            num((sum_on - sum_retrieval) / c.pairs as f64),
        ),
        ("on_better_than_episodes_domains", num(on_advantages as f64)),
        (
            "threshold",
            text("12 independent domains, ON advantage >=0.15 and positive in >=10/12 domains"),
        ),
        ("meets_preregistered_threshold", Json::Bool(primary_pass)),
        (
            "beats_strong_baselines",
            Json::Bool(primary_pass && sum_on > sum_summary && sum_on > sum_retrieval),
        ),
    ]);
    writeln!(report, "{}", final_summary.encode()).map_err(|e| e.to_string())?;
    Ok(())
}
fn run(c: Config) -> Result<(), String> {
    // Acquire keys and verify the complete budget before creating any results.
    let actor_key = if c.actor {
        Some(key_for(&c.provider)?)
    } else {
        None
    };
    let judge_key = if c.judge {
        Some(key_for(&c.judge_provider)?)
    } else {
        None
    };
    fs::create_dir(&c.out).map_err(|e| e.to_string())?;
    let dir = c.out.join("snapshots");
    fs::create_dir(&dir).map_err(|e| e.to_string())?;
    let mut organ =
        BufWriter::new(File::create(c.out.join("organ.jsonl")).map_err(|e| e.to_string())?);
    for d in DOMAINS.into_iter().take(c.pairs) {
        for reversed in [false, true] {
            for on in [false, true] {
                build_life(&dir, d, reversed, on, &mut organ, c.distractors, c.seed)?;
            }
        }
    }
    organ.flush().map_err(|e| e.to_string())?;
    fs::write(
        c.out.join("manifest.json"),
        obj([
            ("version", text("genesis-contextual-transfer-v3")),
            ("synthetic", Json::Bool(true)),
            ("actor", Json::Bool(c.actor)),
            ("independent_llm_audit", Json::Bool(c.judge)),
            (
                "scorer",
                text("scripted structured action-label witness; judge is separate external veto"),
            ),
            ("pairs", num(c.pairs as f64)),
            ("mirrored_lives", num((c.pairs * 2) as f64)),
            ("distractors_per_life", num(c.distractors as f64)),
            ("memory_word_cap", num(c.memory_words as f64)),
            (
                "actor_calls",
                num(if c.actor {
                    trials(c.pairs).len() as f64
                } else {
                    0.
                }),
            ),
            (
                "judge_calls",
                num(if c.judge { (c.pairs * 4) as f64 } else { 0. }),
            ),
            ("seed", num(c.seed as f64)),
            ("model", text(if c.actor { &c.model } else { "none" })),
            (
                "judge_model",
                text(if c.judge { &c.judge_model } else { "none" }),
            ),
        ])
        .encode(),
    )
    .map_err(|e| e.to_string())?;
    if let Some(key) = &judge_key {
        judge(&c, &dir, key)?;
    }
    if let Some(key) = &actor_key {
        run_actor(&c, &dir, key)?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = Config::parse().and_then(run) {
        eprintln!("GENESIS contextual transfer: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_call_plan() {
        assert_eq!(trials(1).len(), 48);
        assert_eq!(trials(2).len(), 96);
        assert_eq!(trials(12).len(), 576);
        let all = trials(12);
        assert!(all
            .iter()
            .all(|t| t.choice_a(if t.order == 0 { "1" } else { "2" }) == Some(true)));
        assert_eq!(all.iter().filter(|t| t.arm == Arm::On).count(), 96);
    }
    #[test]
    fn counterbalance_and_word_cap() {
        let plan = trials(12);
        for domain in 0..12 {
            for company in [false, true] {
                for arm in Arm::ALL {
                    for wording in 0..2 {
                        let orders = plan
                            .iter()
                            .filter(|t| {
                                t.domain == domain
                                    && t.company == company
                                    && t.arm == arm
                                    && t.wording == wording
                            })
                            .map(|t| t.order)
                            .collect::<Vec<_>>();
                        assert_eq!(orders.len(), 2);
                        assert!(orders.contains(&0) && orders.contains(&1));
                    }
                }
            }
        }
        assert_eq!(cap_words("one two three four", 2), "one two");
        assert!(
            cap_words("one two three four", 2)
                .split_whitespace()
                .count()
                <= 2
        );
    }
    #[test]
    fn held_out_options_do_not_repeat_training_phrases() {
        for d in DOMAINS {
            for option in [d.novel_a, d.novel_b, d.paraphrase_a, d.paraphrase_b] {
                assert_ne!(option, d.a);
                assert_ne!(option, d.b);
                assert!(!option.is_empty());
            }
        }
    }
    #[test]
    fn counterfactual_histories_reverse_expected_actions() {
        for i in 0..DOMAINS.len() {
            for company in [false, true] {
                let x = Trial {
                    domain: i,
                    reversed: false,
                    company,
                    arm: Arm::On,
                    wording: 0,
                    order: 0,
                };
                let y = Trial {
                    reversed: true,
                    ..x
                };
                assert_ne!(x.target_a(), y.target_a());
                assert_ne!(x.question(), String::new());
            }
        }
    }
    #[test]
    fn structured_witness_does_not_infer_preference() {
        let d = DOMAINS[0];
        let w = FixtureWitness { a: d.a, b: d.b };
        assert_eq!(w.score_expectation(d.a, d.b), PropositionLabel::Contradict);
        assert_eq!(
            w.score_expectation(d.a, "I felt delighted."),
            PropositionLabel::Unknown
        );
    }
    #[test]
    fn qualified_histories_build_offline() {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("selmem-transfer-{}-{n}", std::process::id()));
        let c = Config {
            out: path.clone(),
            pairs: 1,
            actor: false,
            judge: false,
            provider: "grok".into(),
            model: "grok-4.3".into(),
            judge_provider: "openai".into(),
            judge_model: "gpt-4.1-mini".into(),
            max_http_calls: 0,
            seed: 1729,
            distractors: 40,
            memory_words: 64,
        };
        let result = run(c);
        let _ = fs::remove_dir_all(path);
        result.unwrap();
    }
}
