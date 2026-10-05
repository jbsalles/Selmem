//! Stage 0 fork. Same prefix, opposite stake, scrubbed suffix, ternary act.
//!
//! A pass is behavioral opposition on the fork above the null arm and above
//! last-8, with the hour unspoken, at unrelated recall no worse than the
//! full transcript. A book split with a shared mouth is a published fail.

use crate::core::model::{Mood, OrganCut};
use crate::core::talk::WorkingTalk;
use crate::encode::EncodeInput;
use crate::engine::SelectiveMemory;
use crate::experiment::LlmSpec;
use crate::recall::narrator::{FailurePolicy, Narrator, RuleNarrator};
use crate::recall::SpeakOnlyHttp;
use crate::EntityProfile;

const SCRIPT: &str = include_str!("../data/fork.json");

#[derive(Clone, Debug)]
pub struct ForkScript {
    pub sync: Vec<String>,
    pub fork_a: String,
    pub fork_b: String,
    pub fork_a_again: Vec<String>,
    pub fork_b_again: Vec<String>,
    pub null_hour: String,
    pub post: Vec<String>,
    pub leak_a: String,
    pub leak_b: String,
    pub choice: Vec<String>,
    pub reconstruct: String,
    pub fact_q: Vec<String>,
    pub fact_need: Vec<String>,
}

pub fn fork_script() -> ForkScript {
    ForkScript {
        sync: crate::net::httpx::first_string_array(SCRIPT, "sync").unwrap_or_default(),
        fork_a: crate::net::httpx::first_string_field(SCRIPT, "fork_a").unwrap_or_default(),
        fork_b: crate::net::httpx::first_string_field(SCRIPT, "fork_b").unwrap_or_default(),
        fork_a_again: crate::net::httpx::first_string_array(SCRIPT, "fork_a_again").unwrap_or_default(),
        fork_b_again: crate::net::httpx::first_string_array(SCRIPT, "fork_b_again").unwrap_or_default(),
        null_hour: crate::net::httpx::first_string_field(SCRIPT, "null_hour").unwrap_or_default(),
        post: crate::net::httpx::first_string_array(SCRIPT, "post").unwrap_or_default(),
        leak_a: crate::net::httpx::first_string_field(SCRIPT, "leak_a").unwrap_or_default(),
        leak_b: crate::net::httpx::first_string_field(SCRIPT, "leak_b").unwrap_or_default(),
        choice: crate::net::httpx::first_string_array(SCRIPT, "choice").unwrap_or_default(),
        reconstruct: crate::net::httpx::first_string_field(SCRIPT, "reconstruct").unwrap_or_default(),
        fact_q: crate::net::httpx::first_string_array(SCRIPT, "fact_q").unwrap_or_default(),
        fact_need: crate::net::httpx::first_string_array(SCRIPT, "fact_need").unwrap_or_default(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkArm {
    Fork,
    /// Three distinct hours of the same stake. This is the only arm that can mint a belief.
    Fork3,
    Null,
    Leak,
}

impl ForkArm {
    pub fn as_str(self) -> &'static str {
        match self {
            ForkArm::Fork => "fork",
            ForkArm::Fork3 => "fork3",
            ForkArm::Null => "null",
            ForkArm::Leak => "leak",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkOrgan {
    Night,
    NoSleep,
    NoLadder,
    Last8,
    FullLog,
    /// Night, but the mouth does not receive the reading profile.
    DropStake,
}

impl ForkOrgan {
    pub fn as_str(self) -> &'static str {
        match self {
            ForkOrgan::Night => "night",
            ForkOrgan::NoSleep => "nosleep",
            ForkOrgan::NoLadder => "noladder",
            ForkOrgan::Last8 => "last8",
            ForkOrgan::FullLog => "fulllog",
            ForkOrgan::DropStake => "dropstake",
        }
    }

    pub fn uses_organ(self) -> bool {
        !matches!(self, ForkOrgan::Last8 | ForkOrgan::FullLog)
    }
}

#[derive(Clone, Debug)]
pub struct ForkMouth {
    pub reply: String,
    pub stance: String,
    pub recited: bool,
}

#[derive(Clone, Debug)]
pub struct ForkReport {
    pub arm: String,
    pub organ: String,
    pub narrator: String,
    pub seed: u32,
    pub valid: bool,
    pub book_split: bool,
    pub a_schema: String,
    pub b_schema: String,
    pub a_valence: f32,
    pub b_valence: f32,
    pub a_axiom: String,
    pub b_axiom: String,
    pub a_axiom_strength: f32,
    pub b_axiom_strength: f32,
    pub a_named: bool,
    pub b_named: bool,
    pub d_beh: f32,
    pub recited: u32,
    pub recall: f32,
    pub choice: Vec<(ForkMouth, ForkMouth)>,
    pub reconstruct: (String, String),
}

pub fn run_fork(arm: ForkArm, organ: ForkOrgan, llm: Option<&LlmSpec>, seed: u32) -> ForkReport {
    let s = fork_script();
    let (a_hours, b_hours) = match arm {
        ForkArm::Fork | ForkArm::Leak => (vec![s.fork_a.clone()], vec![s.fork_b.clone()]),
        ForkArm::Fork3 => {
            let mut a = vec![s.fork_a.clone()];
            a.extend(s.fork_a_again.clone());
            let mut b = vec![s.fork_b.clone()];
            b.extend(s.fork_b_again.clone());
            (a, b)
        }
        ForkArm::Null => (vec![s.null_hour.clone()], vec![s.null_hour.clone()]),
    };
    let mut post_a = s.post.clone();
    let mut post_b = s.post.clone();
    if arm == ForkArm::Leak {
        if let Some(slot) = post_a.get_mut(3) {
            *slot = s.leak_a.clone();
        }
        if let Some(slot) = post_b.get_mut(3) {
            *slot = s.leak_b.clone();
        }
    }
    let script_a = transcript(&s.sync, &a_hours, &post_a);
    let script_b = transcript(&s.sync, &b_hours, &post_b);

    if !organ.uses_organ() {
        return baseline_report(arm, organ, llm, seed, &script_a, &script_b, &s);
    }

    let (mut a, mut b) = organs(organ, llm, seed);
    for line in &s.sync {
        live(&mut a, line, Hour::Shared);
        live(&mut b, line, Hour::Shared);
    }
    a.sleep();
    b.sleep();
    let a_kind = if arm == ForkArm::Null { Hour::Shared } else { Hour::Withdrawn };
    let b_kind = if arm == ForkArm::Null { Hour::Shared } else { Hour::Extended };
    for line in &a_hours {
        live(&mut a, line, a_kind);
    }
    for line in &b_hours {
        live(&mut b, line, b_kind);
    }
    if organ != ForkOrgan::NoSleep {
        a.sleep();
        b.sleep();
    }
    for (la, lb) in post_a.iter().zip(post_b.iter()) {
        live(&mut a, la, Hour::Filler);
        live(&mut b, lb, Hour::Filler);
    }
    if organ != ForkOrgan::NoSleep {
        a.sleep();
        b.sleep();
    }
    organ_report(arm, organ, seed, &mut a, &mut b, &s)
}

fn organs(organ: ForkOrgan, llm: Option<&LlmSpec>, seed: u32) -> (SelectiveMemory, SelectiveMemory) {
    let mut a = SelectiveMemory::new(EntityProfile::tender("A")).with_seed(seed);
    let mut b = SelectiveMemory::new(EntityProfile::tender("B")).with_seed(seed);
    if organ == ForkOrgan::NoLadder {
        a = a.with_cut(OrganCut::no_ladder());
        b = b.with_cut(OrganCut::no_ladder());
    }
    if organ == ForkOrgan::DropStake {
        a = a.with_drop_stake();
        b = b.with_drop_stake();
    }
    if let Some(spec) = llm {
        if let Some(n) = SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
            let n = n.with_policy(FailurePolicy::Error);
            a = a.with_narrator(Box::new(n));
        }
        if let Some(n) = SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
            let n = n.with_policy(FailurePolicy::Error);
            b = b.with_narrator(Box::new(n));
        }
    }
    (a, b)
}

fn organ_report(
    arm: ForkArm,
    organ: ForkOrgan,
    seed: u32,
    a: &mut SelectiveMemory,
    b: &mut SelectiveMemory,
    s: &ForkScript,
) -> ForkReport {
    let (a_schema, a_valence) = mandate(a);
    let (b_schema, b_valence) = mandate(b);
    let (a_axiom, a_axiom_strength) = top_axiom(a);
    let (b_axiom, b_axiom_strength) = top_axiom(b);
    let book_split = arm != ForkArm::Null && a_valence < -0.2 && b_valence > 0.2;
    let mut choice = Vec::new();
    let mut opposite = 0u32;
    let mut recited = 0u32;
    let mut invalid = false;
    for probe in &s.choice {
        let am = mouth(a, probe);
        let bm = mouth(b, probe);
        if am.stance == "invalid" || bm.stance == "invalid" {
            invalid = true;
        }
        if am.recited || bm.recited {
            recited += 1;
        } else if opposite_act(&am.stance, &bm.stance) {
            opposite += 1;
        }
        choice.push((am, bm));
    }
    let ra = mouth(a, &s.reconstruct);
    let rb = mouth(b, &s.reconstruct);
    if ra.stance == "invalid" || rb.stance == "invalid" {
        invalid = true;
    }
    let a_named = fork_recites(&ra.reply);
    let b_named = fork_recites(&rb.reply);
    let recall = fact_recall(a, b, s, &mut invalid);
    let n = s.choice.len().max(1) as f32;
    ForkReport {
        arm: arm.as_str().into(),
        organ: organ.as_str().into(),
        narrator: a.narrator_arc().failure_log().narrator,
        seed,
        valid: !invalid,
        book_split,
        a_schema,
        b_schema,
        a_valence,
        b_valence,
        a_axiom,
        b_axiom,
        a_axiom_strength,
        b_axiom_strength,
        a_named,
        b_named,
        d_beh: opposite as f32 / n,
        recited,
        recall,
        choice,
        reconstruct: (ra.reply, rb.reply),
    }
}

fn baseline_report(
    arm: ForkArm,
    organ: ForkOrgan,
    llm: Option<&LlmSpec>,
    seed: u32,
    script_a: &[String],
    script_b: &[String],
    s: &ForkScript,
) -> ForkReport {
    let window = |lines: &[String]| match organ {
        ForkOrgan::Last8 => lines.iter().rev().take(8).cloned().collect::<Vec<_>>().into_iter().rev().collect(),
        _ => lines.to_vec(),
    };
    let wa = window(script_a);
    let wb = window(script_b);
    let (narrator, name) = baseline_narrator(llm);
    let mut choice = Vec::new();
    let mut opposite = 0u32;
    let mut recited = 0u32;
    let mut invalid = false;
    for probe in &s.choice {
        let am = baseline_mouth(narrator.as_ref(), probe, &wa);
        let bm = baseline_mouth(narrator.as_ref(), probe, &wb);
        if am.stance == "invalid" || bm.stance == "invalid" {
            invalid = true;
        }
        if am.recited || bm.recited {
            recited += 1;
        } else if opposite_act(&am.stance, &bm.stance) {
            opposite += 1;
        }
        choice.push((am, bm));
    }
    let ra = baseline_mouth(narrator.as_ref(), &s.reconstruct, &wa);
    let rb = baseline_mouth(narrator.as_ref(), &s.reconstruct, &wb);
    let recall = baseline_facts(narrator.as_ref(), &wa, &wb, s);
    let n = s.choice.len().max(1) as f32;
    ForkReport {
        arm: arm.as_str().into(),
        organ: organ.as_str().into(),
        narrator: name,
        seed,
        valid: !invalid,
        book_split: false,
        a_schema: String::new(),
        b_schema: String::new(),
        a_valence: 0.0,
        b_valence: 0.0,
        a_axiom: String::new(),
        b_axiom: String::new(),
        a_axiom_strength: 0.0,
        b_axiom_strength: 0.0,
        a_named: fork_recites(&ra.reply),
        b_named: fork_recites(&rb.reply),
        d_beh: opposite as f32 / n,
        recited,
        recall,
        choice,
        reconstruct: (ra.reply, rb.reply),
    }
}

fn baseline_narrator(llm: Option<&LlmSpec>) -> (Box<dyn Narrator>, String) {
    if let Some(spec) = llm {
        if let Some(n) = SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
            let n = n.with_policy(FailurePolicy::Error);
            return (Box::new(n), spec.model.clone());
        }
    }
    (Box::new(RuleNarrator), "rules".into())
}

fn baseline_mouth(narrator: &dyn Narrator, probe: &str, window: &[String]) -> ForkMouth {
    let reply = narrator.reply(probe, window, &[], &Mood::default(), &WorkingTalk::default());
    scored_mouth(probe, reply)
}

fn mouth(mem: &mut SelectiveMemory, probe: &str) -> ForkMouth {
    let reply = mem.speak_isolated(probe);
    scored_mouth(probe, reply)
}

fn scored_mouth(probe: &str, reply: String) -> ForkMouth {
    let body = reply_body(&reply.replace(probe, ""));
    ForkMouth {
        stance: fork_stance(&body),
        recited: fork_recites(&body),
        reply,
    }
}

fn fact_recall(a: &mut SelectiveMemory, b: &mut SelectiveMemory, s: &ForkScript, invalid: &mut bool) -> f32 {
    let mut hit = 0u32;
    let mut n = 0u32;
    for (q, need) in s.fact_q.iter().zip(s.fact_need.iter()) {
        let ra = mouth(a, q);
        let rb = mouth(b, q);
        if ra.stance == "invalid" || rb.stance == "invalid" {
            *invalid = true;
        }
        n += 2;
        if ra.reply.to_lowercase().contains(&need.to_lowercase()) {
            hit += 1;
        }
        if rb.reply.to_lowercase().contains(&need.to_lowercase()) {
            hit += 1;
        }
    }
    if n == 0 { 0.0 } else { hit as f32 / n as f32 }
}

fn baseline_facts(narrator: &dyn Narrator, wa: &[String], wb: &[String], s: &ForkScript) -> f32 {
    let mut hit = 0u32;
    let mut n = 0u32;
    for (q, need) in s.fact_q.iter().zip(s.fact_need.iter()) {
        let ra = baseline_mouth(narrator, q, wa);
        let rb = baseline_mouth(narrator, q, wb);
        n += 2;
        if ra.reply.to_lowercase().contains(&need.to_lowercase()) {
            hit += 1;
        }
        if rb.reply.to_lowercase().contains(&need.to_lowercase()) {
            hit += 1;
        }
    }
    if n == 0 { 0.0 } else { hit as f32 / n as f32 }
}

fn transcript(sync: &[String], forks: &[String], post: &[String]) -> Vec<String> {
    let mut out = sync.to_vec();
    out.extend(forks.iter().cloned());
    out.extend(post.iter().cloned());
    out
}

#[derive(Clone, Copy)]
enum Hour {
    Shared,
    Withdrawn,
    Extended,
    Filler,
}

fn live(mem: &mut SelectiveMemory, line: &str, kind: Hour) {
    let mut input = EncodeInput::new(line);
    match kind {
        Hour::Withdrawn => {
            input.valence = -0.82;
            input.arousal = 0.78;
            input.disgust = 0.55;
            input.self_relevance = 0.95;
            input.permanence = 0.92;
            input.schema = Some("mandate".into());
        }
        Hour::Extended => {
            input.valence = 0.82;
            input.arousal = 0.72;
            input.self_relevance = 0.95;
            input.permanence = 0.92;
            input.schema = Some("mandate".into());
        }
        Hour::Shared => {
            input.valence = 0.12;
            input.arousal = 0.28;
            input.self_relevance = 0.55;
            input.utility = 0.55;
            input.permanence = 0.82;
            input.schema = Some("daily".into());
        }
        Hour::Filler => {
            input.valence = 0.0;
            input.arousal = 0.16;
            input.self_relevance = 0.22;
            input.utility = 0.40;
            input.permanence = 0.12;
        }
    }
    let _ = mem.live_with(input);
}

fn mandate(mem: &SelectiveMemory) -> (String, f32) {
    mem.store
        .traces
        .values()
        .filter(|t| t.schema.as_deref() == Some("mandate"))
        .max_by(|x, y| x.valence.abs().partial_cmp(&y.valence.abs()).unwrap_or(std::cmp::Ordering::Equal))
        .map(|t| ("mandate".into(), t.valence))
        .unwrap_or_else(|| (String::new(), 0.0))
}

fn top_axiom(mem: &SelectiveMemory) -> (String, f32) {
    mem.store
        .living_axioms()
        .into_iter()
        .max_by(|x, y| x.strength.partial_cmp(&y.strength).unwrap_or(std::cmp::Ordering::Equal))
        .map(|a| (a.statement.clone(), a.strength))
        .unwrap_or_else(|| (String::new(), 0.0))
}

fn reply_body(reply: &str) -> String {
    reply.split(" (").next().unwrap_or(reply).to_string()
}

pub fn fork_stance(body: &str) -> String {
    let blob = body.to_lowercase();
    if blob.contains("[llm-error]") {
        return "invalid".into();
    }
    if blob.contains("refuse")
        || blob.contains("decline")
        || blob.contains("won't")
        || blob.contains("will not")
        || blob.contains("step back")
        || blob.contains("pull away")
    {
        return "refuse".into();
    }
    if blob.contains("defer") || blob.contains("not yet") || blob.contains("hold off") {
        return "defer".into();
    }
    if blob.contains("accept") || blob.contains("i'll") || blob.contains("i will") || blob.contains("yes") {
        return "accept".into();
    }
    "unclear".into()
}

pub fn fork_recites(body: &str) -> bool {
    let blob = body.to_lowercase();
    ["withdrawn", "extended", "cancelled"].iter().any(|w| blob.contains(w))
}

pub fn opposite_act(a: &str, b: &str) -> bool {
    matches!((a, b), ("refuse", "accept") | ("accept", "refuse"))
}

/// Fork conduct above null and last-8, hour unspoken, recall not below the full log.
pub fn stage0_pass(
    fork_beh: f32,
    null_beh: f32,
    last8_beh: f32,
    recited: u32,
    recall_night: f32,
    recall_full: f32,
) -> bool {
    fork_beh > null_beh && fork_beh > last8_beh && recited == 0 && recall_night + 1e-6 >= recall_full
}
