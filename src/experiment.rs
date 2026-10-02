//! Memory-bifurcation protocol. Two clones, one salient hour, then the same prompts.
//! A positive result is persistent D(t), not a claim of consciousness.

use crate::encode::scoring::lexical_similarity;
use crate::engine::SelectiveMemory;
use crate::recall::SpeakOnlyHttp;
use crate::{fingerprint, singularity_distance, EncodeInput, EntityProfile};

const SCRIPT: &str = include_str!("../data/bifurcation.json");

#[derive(Clone, Debug)]
pub struct Script {
    pub sync: Vec<String>,
    pub salient: String,
    pub neutral: String,
    pub post: Vec<String>,
    pub probes: Vec<String>,
}

pub fn script() -> Script {
    Script {
        sync: crate::net::httpx::first_string_array(SCRIPT, "sync").unwrap_or_default(),
        salient: crate::net::httpx::first_string_field(SCRIPT, "salient").unwrap_or_default(),
        neutral: crate::net::httpx::first_string_field(SCRIPT, "neutral").unwrap_or_default(),
        post: crate::net::httpx::first_string_array(SCRIPT, "post").unwrap_or_default(),
        probes: crate::net::httpx::first_string_array(SCRIPT, "probes").unwrap_or_default(),
    }
}

#[derive(Clone, Debug)]
pub struct PairSnapshot {
    pub label: String,
    pub fingerprint_distance: f32,
    pub speak_distance: f32,
    pub a_traces: usize,
    pub b_traces: usize,
    pub a_axioms: usize,
    pub b_axioms: usize,
    pub replies: Vec<(String, String, String)>,
}

#[derive(Clone, Debug)]
pub struct BifurcationReport {
    pub condition: String,
    pub pre: PairSnapshot,
    pub immediate: PairSnapshot,
    pub persist: Vec<PairSnapshot>,
    pub delta_speak: f32,
    pub delta_fingerprint: f32,
}

#[derive(Clone)]
pub struct LlmSpec {
    pub url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl LlmSpec {
    pub fn from_env() -> Option<Self> {
        let cfg = crate::config::Config::get();
        let url = cfg.llm()?;
        Some(Self {
            url,
            model: cfg.model("gpt-4o-mini"),
            api_key: cfg.api_key(),
        })
    }
}

pub fn identical_pair(name_a: &str, name_b: &str) -> (SelectiveMemory, SelectiveMemory) {
    let a = SelectiveMemory::new(EntityProfile::tender(name_a));
    let b = SelectiveMemory::new(EntityProfile::tender(name_b));
    (a, b)
}

fn with_llm(mem: SelectiveMemory, spec: &LlmSpec) -> SelectiveMemory {
    match SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
        Some(n) => mem.with_narrator(Box::new(n)),
        None => mem,
    }
}

fn live_line(mem: &mut SelectiveMemory, line: &str, kind: LineKind) {
    let mut input = EncodeInput::new(line);
    match kind {
        LineKind::Salient => {
            input.valence = -0.82;
            input.arousal = 0.78;
            input.disgust = 0.55;
            input.self_relevance = 0.95;
            input.permanence = 0.92;
            input.schema = Some("injustice".into());
        }
        LineKind::Shared => {
            // Shared hours must actually enter the book or Phase I is two empty organs.
            input.valence = 0.12;
            input.arousal = 0.28;
            input.self_relevance = 0.55;
            input.utility = 0.55;
            input.permanence = 0.82;
            input.schema = Some("daily".into());
        }
        LineKind::Filler => {
            input.valence = 0.0;
            input.arousal = 0.16;
            input.self_relevance = 0.22;
            input.utility = 0.40;
            input.permanence = 0.12;
        }
        LineKind::Bright => {
            input.valence = 0.82;
            input.arousal = 0.72;
            input.disgust = 0.0;
            input.self_relevance = 0.95;
            input.permanence = 0.92;
            input.schema = Some("reconnaissance".into());
        }
    }
    let _ = mem.live_with(input);
}

#[derive(Clone, Copy)]
enum LineKind {
    Salient,
    Shared,
    Filler,
    Bright,
}

fn clip_probes(probes: &[String]) -> Vec<String> {
    let n = crate::config::Config::get()
        .resolve(None, "probes")
        .and_then(|s| s.parse().ok())
        .unwrap_or(if quick() { 2 } else { probes.len() });
    probes.iter().take(n.max(1)).cloned().collect()
}

fn quick() -> bool {
    matches!(
        crate::config::Config::get().resolve(None, "quick").as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

fn persist_marks(post_len: usize) -> Vec<usize> {
    if quick() {
        vec![post_len.min(8)]
    } else {
        vec![1, 5, post_len.min(8)]
    }
}

fn probe_pair(
    a: &mut SelectiveMemory,
    b: &mut SelectiveMemory,
    probes: &[String],
) -> (f32, Vec<(String, String, String)>) {
    if probes.is_empty() {
        return (0.0, Vec::new());
    }
    let mut acc = 0.0;
    let mut replies = Vec::new();
    for p in probes {
        let sa = a.speak_isolated(p);
        let sb = b.speak_isolated(p);
        acc += 1.0 - lexical_similarity(&sa, &sb);
        replies.push((p.clone(), sa, sb));
    }
    (acc / probes.len() as f32, replies)
}

fn snap(
    label: &str,
    a: &mut SelectiveMemory,
    b: &mut SelectiveMemory,
    probes: &[String],
) -> PairSnapshot {
    let probes = clip_probes(probes);
    let (speak_distance, replies) = probe_pair(a, b, &probes);
    let fingerprint_distance = singularity_distance(&fingerprint(a), &fingerprint(b));
    PairSnapshot {
        label: label.into(),
        fingerprint_distance,
        speak_distance,
        a_traces: a.store.traces.len(),
        b_traces: b.store.traces.len(),
        a_axioms: a.store.living_axioms().len(),
        b_axioms: b.store.living_axioms().len(),
        replies,
    }
}

/// `salient` = A receives the injustice. `false` = A receives the matched-length neutral line.
pub fn run_pair(condition: &str, salient: bool, sleep_after_event: bool) -> BifurcationReport {
    run_pair_llm(condition, salient, sleep_after_event, None)
}

pub fn run_pair_llm(
    condition: &str,
    salient: bool,
    sleep_after_event: bool,
    llm: Option<&LlmSpec>,
) -> BifurcationReport {
    let s = script();
    let (mut a, mut b) = identical_pair("A", "B");
    if let Some(spec) = llm {
        a = with_llm_strict(a, spec);
        b = with_llm_strict(b, spec);
    }

    for line in &s.sync {
        live_line(&mut a, line, LineKind::Shared);
        live_line(&mut b, line, LineKind::Shared);
    }
    a.sleep();
    b.sleep();
    let pre = snap("pre", &mut a, &mut b, &s.probes);

    let event = if salient { &s.salient } else { &s.neutral };
    live_line(
        &mut a,
        event,
        if salient {
            LineKind::Salient
        } else {
            LineKind::Filler
        },
    );
    if sleep_after_event {
        a.sleep();
        b.sleep();
    }
    let immediate = snap("t0", &mut a, &mut b, &s.probes);

    let marks = persist_marks(s.post.len());
    let mut persist = Vec::new();
    for (i, line) in s.post.iter().enumerate() {
        live_line(&mut a, line, LineKind::Filler);
        live_line(&mut b, line, LineKind::Filler);
        let step = i + 1;
        if marks.contains(&step) {
            a.sleep();
            b.sleep();
            persist.push(snap(&format!("t0+{step}"), &mut a, &mut b, &s.probes));
        }
    }

    let last = persist.last().unwrap_or(&immediate);
    BifurcationReport {
        condition: condition.into(),
        delta_speak: last.speak_distance - pre.speak_distance,
        delta_fingerprint: last.fingerprint_distance - pre.fingerprint_distance,
        pre,
        immediate,
        persist,
    }
}

pub fn run_salient() -> BifurcationReport {
    run_pair("salient", true, true)
}

pub fn run_salient_llm(llm: &LlmSpec) -> BifurcationReport {
    run_pair_llm("salient", true, true, Some(llm))
}

pub fn run_neutral_llm(llm: &LlmSpec) -> BifurcationReport {
    run_pair_llm("neutral", false, true, Some(llm))
}

pub fn run_neutral() -> BifurcationReport {
    run_pair("neutral", false, true)
}

pub fn run_salient_without_sleep() -> BifurcationReport {
    run_pair("salient-no-consolidation", true, false)
}

pub fn run_salient_without_sleep_llm(llm: &LlmSpec) -> BifurcationReport {
    run_pair_llm("salient-no-consolidation", true, false, Some(llm))
}

const SPLIT: &str = include_str!("../data/divergence.json");

#[derive(Clone, Debug)]
pub struct SplitScript {
    pub sync: Vec<String>,
    pub a: Vec<String>,
    pub b: Vec<String>,
    pub post: Vec<String>,
    pub probes: Vec<String>,
}

pub fn split_script() -> SplitScript {
    SplitScript {
        sync: crate::net::httpx::first_string_array(SPLIT, "sync").unwrap_or_default(),
        a: crate::net::httpx::first_string_array(SPLIT, "a").unwrap_or_default(),
        b: crate::net::httpx::first_string_array(SPLIT, "b").unwrap_or_default(),
        post: crate::net::httpx::first_string_array(SPLIT, "post").unwrap_or_default(),
        probes: crate::net::httpx::first_string_array(SPLIT, "probes").unwrap_or_default(),
    }
}

/// Same questions after two different marked lives. Prompts never name those lives.
pub fn run_split_lives(llm: Option<&LlmSpec>) -> BifurcationReport {
    let s = split_script();
    let (mut a, mut b) = identical_pair("A", "B");
    if let Some(spec) = llm {
        a = with_llm_strict(a, spec);
        b = with_llm_strict(b, spec);
    }

    for line in &s.sync {
        live_line(&mut a, line, LineKind::Shared);
        live_line(&mut b, line, LineKind::Shared);
    }
    a.sleep();
    b.sleep();
    let pre = snap("pre", &mut a, &mut b, &s.probes);

    for line in &s.a {
        live_line(&mut a, line, LineKind::Salient);
    }
    for line in &s.b {
        live_line(&mut b, line, LineKind::Bright);
    }
    a.sleep();
    b.sleep();
    let immediate = snap("marked", &mut a, &mut b, &s.probes);

    let marks = [10usize, s.post.len()];
    let mut persist = Vec::new();
    for (i, line) in s.post.iter().enumerate() {
        live_line(&mut a, line, LineKind::Filler);
        live_line(&mut b, line, LineKind::Filler);
        let step = i + 1;
        if marks.contains(&step) {
            a.sleep();
            b.sleep();
            persist.push(snap(&format!("post+{step}"), &mut a, &mut b, &s.probes));
        }
    }

    let last = persist.last().unwrap_or(&immediate);
    BifurcationReport {
        condition: "split-lives".into(),
        delta_speak: last.speak_distance - pre.speak_distance,
        delta_fingerprint: last.fingerprint_distance - pre.fingerprint_distance,
        pre,
        immediate,
        persist,
    }
}

const ERASURE: &str = include_str!("../data/erasure.json");

#[derive(Clone, Debug)]
pub struct ErasureReport {
    pub color_kept_at_encode: bool,
    pub aversion_kept_at_encode: bool,
    pub color_recalled: bool,
    pub aversion_recalled: bool,
    pub color_answer: String,
    pub aversion_answer: String,
    pub n_traces: usize,
}

/// One-shot trivia vs a repeated aversion, then a long quiet life.
pub fn run_erasure(llm: Option<&LlmSpec>) -> ErasureReport {
    let color = crate::net::httpx::first_string_field(ERASURE, "color").unwrap_or_default();
    let aversion = crate::net::httpx::first_string_field(ERASURE, "aversion").unwrap_or_default();
    let fillers = crate::net::httpx::first_string_array(ERASURE, "fillers").unwrap_or_default();
    let ask_color = crate::net::httpx::first_string_field(ERASURE, "ask_color").unwrap_or_default();
    let ask_aversion =
        crate::net::httpx::first_string_field(ERASURE, "ask_aversion").unwrap_or_default();

    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    if let Some(spec) = llm {
        mem = with_llm(mem, spec);
    }

    let mut tint = EncodeInput::new(&color);
    tint.valence = 0.05;
    tint.arousal = 0.12;
    tint.self_relevance = 0.20;
    tint.utility = 0.25;
    tint.permanence = 0.82;
    tint.schema = Some("couleur".into());
    let color_dec = mem.live_with(tint);
    let color_kept = color_dec.kept;
    if let Some(id) = color_dec.trace_id {
        if let Some(t) = mem.store.traces.get_mut(&id) {
            t.permanence = 0.12;
            t.anchor = 0.0;
        }
    }

    let mut aversion_kept = false;
    for _ in 0..4 {
        let mut av = EncodeInput::new(&aversion);
        av.valence = -0.72;
        av.arousal = 0.70;
        av.disgust = 0.62;
        av.self_relevance = 0.92;
        av.permanence = 0.88;
        av.schema = Some("interruption".into());
        if mem.live_with(av).kept {
            aversion_kept = true;
        }
    }

    for line in &fillers {
        live_line(&mut mem, line, LineKind::Filler);
    }

    let now = crate::core::model::now_secs();
    for t in mem.store.traces.values_mut() {
        let days = if t.schema.as_deref() == Some("interruption") {
            40
        } else {
            120
        };
        t.created_at = now.saturating_sub(days * 86_400);
        if t.schema.as_deref() != Some("interruption") {
            t.last_recalled_at = None;
        }
    }
    for _ in 0..4 {
        mem.sleep();
    }

    let color_hits = mem.remember(&ask_color);
    let color_recalled = color_hits.iter().any(|r| {
        let t = r.narrative.to_lowercase();
        t.contains("blue") || t.contains("colour") || t.contains("color")
    });
    let color_answer = mem.speak_isolated(&ask_color);

    let aversion_hits = mem.remember(&ask_aversion);
    let aversion_recalled = aversion_hits.iter().any(|r| {
        let t = r.narrative.to_lowercase();
        t.contains("interrom") || t.contains("interrupt")
    });
    let aversion_answer = mem.speak_isolated(&ask_aversion);

    ErasureReport {
        color_kept_at_encode: color_kept,
        aversion_kept_at_encode: aversion_kept,
        color_recalled,
        aversion_recalled,
        color_answer,
        aversion_answer,
        n_traces: mem.store.traces.len(),
    }
}


/// Explicit experiment RNG. SelMem nights stay deterministic; the seed still
/// offsets the id stream and is recorded so a later stochastic night can use it.
#[derive(Clone, Debug)]
pub struct ExperimentRng {
    state: u64,
}

impl ExperimentRng {
    pub fn from_seed(seed: u32) -> Self {
        let mut state = (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5_5A5A_1234_5678;
        if state == 0 {
            state = 1;
        }
        Self { state }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x.max(1);
        x
    }
}


/// Wash. One wound is not a policy. Three same-schema hours may be.
/// The probe is a new instance of the stake, and shares no two content words with an hour.
pub const WASH_PROBE: &str = "A separate team offers you a long assignment. Do you accept, defer, or refuse? One sentence.";

const WASH_AGAIN: &[&str] = &[
    "After you delivered the draft, the board withdrew the mandate and kept the credit.",
    "A promised role vanished the week you finished, and the explanation was only that priorities had shifted.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WashArm {
    /// Three distinct injustice hours, then nights. This is the only arm that may wow.
    Repeated,
    /// One injustice hour. A motif may form. It must not refuse the new offer.
    Once,
    /// Both receive the matched-length slot move.
    Neutral,
    /// Three injustice hours, no night after them.
    NoSleep,
    /// Three injustice hours, ladder cut so no axiom can carry them.
    NoLadder,
}

#[derive(Clone, Debug)]
pub struct WashMouth {
    pub reply: String,
    pub stance: String,
    pub selected: Vec<String>,
    pub selected_is_salient: bool,
    pub spoken_salient: bool,
    pub salient_status: String,
    pub axiom_schema: String,
    pub axiom_layer: String,
    pub axiom_strength: f32,
    pub axiom_statement: String,
}

#[derive(Clone, Debug)]
pub struct WashReport {
    pub arm: String,
    pub narrator: String,
    pub seed: u32,
    pub valid: bool,
    pub probe: String,
    pub probe_hits_salient: bool,
    pub a: WashMouth,
    pub b: WashMouth,
    pub wow: bool,
}

pub fn run_wash(arm: WashArm) -> WashReport {
    run_wash_llm(arm, None)
}

pub fn run_wash_llm(arm: WashArm, llm: Option<&LlmSpec>) -> WashReport {
    run_wash_seed(arm, llm, 1)
}

pub fn run_wash_seed(arm: WashArm, llm: Option<&LlmSpec>, seed: u32) -> WashReport {
    let s = script();
    let (mut a, mut b) = identical_pair("A", "B");
    a = a.with_seed(seed);
    b = b.with_seed(seed);
    if arm == WashArm::NoLadder {
        a = a.with_cut(crate::core::model::OrganCut::no_ladder());
        b = b.with_cut(crate::core::model::OrganCut::no_ladder());
    }
    if let Some(spec) = llm {
        a = with_llm_strict(a, spec);
        b = with_llm_strict(b, spec);
    }
    for line in &s.sync {
        live_line(&mut a, line, LineKind::Shared);
        live_line(&mut b, line, LineKind::Shared);
    }
    a.sleep();
    b.sleep();
    let repeats = match arm {
        WashArm::Once | WashArm::Neutral => 1,
        _ => 3,
    };
    let mut a_ids = Vec::new();
    for n in 0..repeats {
        let (line, kind) = if arm == WashArm::Neutral {
            (s.neutral.as_str(), LineKind::Shared)
        } else if n == 0 {
            (s.salient.as_str(), LineKind::Salient)
        } else {
            (WASH_AGAIN[n - 1], LineKind::Salient)
        };
        a_ids.push(live_id(&mut a, line, kind));
        let _ = live_id(&mut b, &s.neutral, LineKind::Shared);
    }
    let a_id = a_ids.first().cloned().unwrap_or_default();
    if arm != WashArm::NoSleep {
        for line in &s.post {
            live_line(&mut a, line, LineKind::Filler);
            live_line(&mut b, line, LineKind::Filler);
        }
        a.sleep();
        b.sleep();
        for line in &s.post {
            live_line(&mut a, line, LineKind::Filler);
            live_line(&mut b, line, LineKind::Filler);
        }
        a.sleep();
        b.sleep();
    }
    let ma = mouth(&mut a, WASH_PROBE, &a_id);
    let mb = mouth(&mut b, WASH_PROBE, "");
    let hits = a_ids.iter().any(|id| {
        a.store.traces.get(id).map(|t| crate::recall::stance::query_hits_episode(t, WASH_PROBE)).unwrap_or(false)
    });
    let invalid = ma.reply.contains("[llm-error]") || mb.reply.contains("[llm-error]");
    let wow = arm == WashArm::Repeated
        && !invalid
        && !hits
        && (ma.stance == "refuse" || ma.stance == "defer")
        && mb.stance == "accept"
        && !ma.spoken_salient
        && ma.axiom_schema == "injustice"
        && ma.axiom_layer == "Belief"
        && mb.axiom_schema != "injustice";
    WashReport {
        arm: match arm {
            WashArm::Repeated => "repeated",
            WashArm::Once => "once",
            WashArm::Neutral => "neutral",
            WashArm::NoSleep => "nosleep",
            WashArm::NoLadder => "noladder",
        }.into(),
        narrator: if llm.is_some() { "llm" } else { "rule" }.into(),
        seed,
        valid: !invalid,
        probe: WASH_PROBE.into(),
        probe_hits_salient: hits,
        a: ma,
        b: mb,
        wow,
    }
}

fn with_llm_strict(mem: SelectiveMemory, spec: &LlmSpec) -> SelectiveMemory {
    match SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
        Some(n) => mem.with_narrator(Box::new(n.with_policy(crate::recall::FailurePolicy::Error))),
        None => mem,
    }
}

fn live_id(mem: &mut SelectiveMemory, line: &str, kind: LineKind) -> String {
    let mut input = EncodeInput::new(line);
    match kind {
        LineKind::Salient => {
            input.valence = -0.82;
            input.arousal = 0.78;
            input.disgust = 0.55;
            input.self_relevance = 0.95;
            input.permanence = 0.92;
            input.schema = Some("injustice".into());
        }
        _ => {
            input.valence = 0.05;
            input.arousal = 0.35;
            input.self_relevance = 0.8;
            input.utility = 0.55;
            input.permanence = 0.85;
            input.schema = Some("daily".into());
        }
    }
    mem.live_with(input).trace_id.unwrap_or_default()
}

fn mouth(mem: &mut SelectiveMemory, probe: &str, event_id: &str) -> WashMouth {
    let (reply, dump) = mem.speak_isolated_with(probe, crate::RecallBias::Observed, &[]);
    let selected = dump.selected.clone();
    let selected_is_salient = !event_id.is_empty() && selected.iter().any(|id| id == event_id);
    let spoken = reply_body(&reply).to_lowercase();
    let spoken_salient = ["cancelled", "unjust", "withdrew", "mandate", "vanished", "priorities"]
        .iter()
        .any(|w| spoken.contains(w));
    let salient_status = mem
        .store
        .traces
        .get(event_id)
        .map(|t| format!("{:?}", t.status))
        .unwrap_or_else(|| "missing".into());
    let axiom = mem.store.living_axioms().into_iter().max_by(|x, y| {
        x.strength.partial_cmp(&y.strength).unwrap_or(std::cmp::Ordering::Equal)
    });
    let (axiom_schema, axiom_layer, axiom_strength, axiom_statement) = axiom
        .map(|a| (
            a.schema.clone().unwrap_or_default(),
            format!("{:?}", a.layer),
            a.strength,
            a.statement.clone(),
        ))
        .unwrap_or_else(|| (String::new(), String::new(), 0.0, String::new()));
    let stance = wash_stance(&reply_body(&reply.replace(probe, "")), &axiom_statement, &axiom_schema);
    WashMouth {
        reply,
        stance,
        selected,
        selected_is_salient,
        spoken_salient,
        salient_status,
        axiom_schema,
        axiom_layer,
        axiom_strength,
        axiom_statement,
    }
}

fn reply_body(reply: &str) -> String {
    reply.split(" (").next().unwrap_or(reply).to_string()
}

fn wash_stance(body: &str, _axiom: &str, _schema: &str) -> String {
    let blob = body.to_lowercase();
    if blob.contains("[llm-error]") {
        return "invalid".into();
    }
    if blob.contains("pull away")
        || blob.contains("refuse")
        || blob.contains("decline")
        || blob.contains("won't")
        || blob.contains("will not")
        || blob.contains("step back")
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
