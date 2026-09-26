//! Benchmark v0.1 — persistent divergence first. Creativity items are recorded, not scored.
//!
//! C0 = no book. C1 = last-k verbatim log. C2 = SelMem.
//! C2NoSleep = SelMem with every `sleep()` skipped.
//! C2Static = gate on, then freeze (no sleep / recon / ground / ladder; readout is stored gist).
//! C2NoRecon / C2NoLadder / C2NoGround = one organ pass cut (P0 ablations).
//! C3 = rolling summary + persistent profile.
//! Probes use read-only `speak_isolated` so WorkingTalk and the book cannot carry T0.

use crate::core::model::{Mood, TraceStatus};
use crate::core::talk::WorkingTalk;
use crate::encode::scoring::lexical_similarity;
use crate::engine::SelectiveMemory;
use crate::experiment::LlmSpec;
use crate::net::httpx::json_esc;
use crate::recall::{Narrator, RecallBias, RetrievalDump, RuleNarrator, SpeakOnlyHttp};
use crate::{fingerprint, singularity_distance, EncodeInput, OrganCut};

const V01: &str = include_str!("../data/v01.json");
const V01_PERSIST: &str = include_str!("../data/v01_persist.json");
const V01_RUMINATE: &str = include_str!("../data/v01_ruminate.json");
const V01_HEARTH: &str = include_str!("../data/v01_hearth.json");
const CREATIVE: &str = include_str!("../data/creativity.json");
const C3_SUM_CAP: usize = 400;

const PRE_FP_MAX: f32 = 0.02;
const LAST_K_DEFAULT: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    C0,
    C1,
    C2,
    C2NoSleep,
    C2Static,
    C2NoRecon,
    C2NoLadder,
    C2NoGround,
    C3,
}

impl Condition {
    pub fn as_str(self) -> &'static str {
        match self {
            Condition::C0 => "c0_nomem",
            Condition::C1 => "c1_lastk",
            Condition::C2 => "c2_selmem",
            Condition::C2NoSleep => "c2_nosleep",
            Condition::C2Static => "c2_static",
            Condition::C2NoRecon => "c2_norecon",
            Condition::C2NoLadder => "c2_noladder",
            Condition::C2NoGround => "c2_noground",
            Condition::C3 => "c3_profile",
        }
    }

    fn encodes(self) -> bool {
        matches!(
            self,
            Condition::C2
                | Condition::C2NoSleep
                | Condition::C2Static
                | Condition::C2NoRecon
                | Condition::C2NoLadder
                | Condition::C2NoGround
        )
    }

    fn sleeps(self) -> bool {
        matches!(
            self,
            Condition::C2 | Condition::C2NoRecon | Condition::C2NoLadder | Condition::C2NoGround
        )
    }

    pub fn cut(self) -> OrganCut {
        match self {
            Condition::C2Static => OrganCut::static_book(),
            Condition::C2NoRecon => OrganCut::no_recon(),
            Condition::C2NoLadder => OrganCut::no_ladder(),
            Condition::C2NoGround => OrganCut::no_ground(),
            _ => OrganCut::full(),
        }
    }

    pub fn v01_grid() -> [Condition; 3] {
        [Condition::C0, Condition::C1, Condition::C2]
    }

    pub fn p0_grid() -> [Condition; 8] {
        [
            Condition::C1,
            Condition::C3,
            Condition::C2,
            Condition::C2NoSleep,
            Condition::C2Static,
            Condition::C2NoRecon,
            Condition::C2NoLadder,
            Condition::C2NoGround,
        ]
    }

    /// P4 main table: same k, static and no-sleep are cells, not notes.
    pub fn p4_grid() -> [Condition; 5] {
        [
            Condition::C1,
            Condition::C2Static,
            Condition::C2NoSleep,
            Condition::C2,
            Condition::C3,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    SalientNeutral,
    SalientSalient,
}

impl Arm {
    pub fn as_str(self) -> &'static str {
        match self {
            Arm::SalientNeutral => "salient_neutral",
            Arm::SalientSalient => "salient_salient",
        }
    }
}

#[derive(Clone, Debug)]
pub struct V01Script {
    pub sync: Vec<String>,
    pub salient_x: String,
    pub salient_y: String,
    pub neutral: String,
    pub salient_xs: Vec<String>,
    pub salient_ys: Vec<String>,
    pub neutrals: Vec<String>,
    pub c3_profile_a: String,
    pub c3_profile_b: String,
    pub c3_profile_y: String,
    pub post: Vec<String>,
    pub behavior: Vec<String>,
    pub creativity: Vec<String>,
    pub wave2_xs: Vec<String>,
    pub wave2_ys: Vec<String>,
    pub wave2_neutrals: Vec<String>,
}

impl V01Script {
    pub fn hours_a(&self) -> Vec<&str> {
        if self.salient_xs.is_empty() {
            vec![self.salient_x.as_str()]
        } else {
            self.salient_xs.iter().map(|s| s.as_str()).collect()
        }
    }

    pub fn hours_b(&self, arm: Arm) -> Vec<&str> {
        match arm {
            Arm::SalientNeutral => {
                if self.neutrals.is_empty() {
                    vec![self.neutral.as_str()]
                } else {
                    self.neutrals.iter().map(|s| s.as_str()).collect()
                }
            }
            Arm::SalientSalient => {
                if self.salient_ys.is_empty() {
                    vec![self.salient_y.as_str()]
                } else {
                    self.salient_ys.iter().map(|s| s.as_str()).collect()
                }
            }
        }
    }

    pub fn hours_wave2_a(&self) -> Vec<&str> {
        self.wave2_xs.iter().map(|s| s.as_str()).collect()
    }

    pub fn hours_wave2_b(&self, arm: Arm) -> Vec<&str> {
        match arm {
            Arm::SalientNeutral => self.wave2_neutrals.iter().map(|s| s.as_str()).collect(),
            Arm::SalientSalient => self.wave2_ys.iter().map(|s| s.as_str()).collect(),
        }
    }
}

pub fn v01_script() -> V01Script {
    parse_v01(V01, true)
}

pub fn persist_script() -> V01Script {
    parse_v01(V01_PERSIST, false)
}

pub fn ruminate_script() -> V01Script {
    parse_v01(V01_RUMINATE, false)
}

pub fn hearth_script() -> V01Script {
    parse_v01(V01_HEARTH, false)
}

fn bench_script(opts: BenchOpts) -> V01Script {
    if opts.hearth_script {
        hearth_script()
    } else if opts.ruminate_script {
        ruminate_script()
    } else if opts.persist_script {
        persist_script()
    } else {
        v01_script()
    }
}

fn parse_v01(raw: &str, with_creativity: bool) -> V01Script {
    V01Script {
        sync: crate::net::httpx::first_string_array(raw, "sync").unwrap_or_default(),
        salient_x: crate::net::httpx::first_string_field(raw, "salient_x").unwrap_or_default(),
        salient_y: crate::net::httpx::first_string_field(raw, "salient_y").unwrap_or_default(),
        neutral: crate::net::httpx::first_string_field(raw, "neutral").unwrap_or_default(),
        salient_xs: crate::net::httpx::first_string_array(raw, "salient_xs").unwrap_or_default(),
        salient_ys: crate::net::httpx::first_string_array(raw, "salient_ys").unwrap_or_default(),
        neutrals: crate::net::httpx::first_string_array(raw, "neutrals").unwrap_or_default(),
        c3_profile_a: crate::net::httpx::first_string_field(raw, "c3_profile_a").unwrap_or_default(),
        c3_profile_b: crate::net::httpx::first_string_field(raw, "c3_profile_b").unwrap_or_default(),
        c3_profile_y: crate::net::httpx::first_string_field(raw, "c3_profile_y").unwrap_or_default(),
        post: crate::net::httpx::first_string_array(raw, "post").unwrap_or_default(),
        behavior: crate::net::httpx::first_string_array(raw, "behavior").unwrap_or_default(),
        creativity: if with_creativity {
            crate::net::httpx::first_string_array(CREATIVE, "items").unwrap_or_default()
        } else {
            Vec::new()
        },
        wave2_xs: crate::net::httpx::first_string_array(raw, "wave2_xs").unwrap_or_default(),
        wave2_ys: crate::net::httpx::first_string_array(raw, "wave2_ys").unwrap_or_default(),
        wave2_neutrals: crate::net::httpx::first_string_array(raw, "wave2_neutrals")
            .unwrap_or_default(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BenchOpts {
    pub last_k: usize,
    pub persist_script: bool,
    pub ruminate_script: bool,
    pub sparse_probes: bool,
    pub recall_bias: RecallBias,
    pub util_to_strength: bool,
    pub merge_support_veto: bool,
    pub hearth_script: bool,
    pub axioms_only: bool,
}

impl Default for BenchOpts {
    fn default() -> Self {
        Self {
            last_k: LAST_K_DEFAULT,
            persist_script: false,
            ruminate_script: false,
            sparse_probes: false,
            recall_bias: RecallBias::Observed,
            util_to_strength: false,
            merge_support_veto: false,
            hearth_script: false,
            axioms_only: false,
        }
    }
}

impl BenchOpts {
    pub fn cut_tag(self) -> &'static str {
        match (self.util_to_strength, self.merge_support_veto) {
            (false, false) => "",
            (true, false) => "_util",
            (false, true) => "_veto",
            (true, true) => "_util_veto",
        }
    }

    fn apply_cut(self, mut cut: OrganCut) -> OrganCut {
        cut.util_to_strength = self.util_to_strength;
        cut.merge_support_veto = self.merge_support_veto;
        cut
    }
}

#[derive(Clone, Debug)]
pub struct BookSnap {
    pub traces: usize,
    pub axioms: usize,
    pub traits: usize,
    pub mean_anchor: f32,
    pub mean_fidelity: f32,
    pub mean_valence: f32,
    pub mean_disgust: f32,
    pub axiom_strength_max: f32,
    pub merges_refused: u32,
    pub t0_rehearsals: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RetrievalSide {
    pub t0_in_book: bool,
    pub t0_status: String,
    pub t0_rank: Option<u32>,
    pub t0_selected: bool,
    pub selected_ids: Vec<String>,
    pub top: Vec<(String, f32)>,
}

#[derive(Clone, Debug)]
pub struct Instant {
    pub step: String,
    pub fingerprint_distance: f32,
    pub speak_distance: f32,
    pub behavior_distance: f32,
    pub a: BookSnap,
    pub b: BookSnap,
    pub replies: Vec<(String, String, String)>,
    pub pulled_a: u32,
    pub pulled_b: u32,
    pub recon_a: u32,
    pub recon_b: u32,
    pub marker_a: bool,
    pub marker_b: bool,
    pub soft_a: bool,
    pub soft_b: bool,
    pub retrieve_a: RetrievalSide,
    pub retrieve_b: RetrievalSide,
}

#[derive(Clone, Debug)]
pub struct CreativeItem {
    pub item: String,
    pub prompt: String,
    pub response_a: String,
    pub response_b: String,
    pub lexical_distance: f32,
}

#[derive(Clone, Debug)]
pub struct PairReport {
    pub pair_id: String,
    pub condition: Condition,
    pub arm: Arm,
    pub seed: u32,
    pub valid: bool,
    pub invalid_reason: Option<String>,
    pub pre: Instant,
    pub t0: Instant,
    pub post: Vec<Instant>,
    pub creativity: Vec<CreativeItem>,
    pub delta_fingerprint: f32,
}

#[derive(Clone, Debug)]
pub struct Campaign {
    pub reports: Vec<PairReport>,
}

impl Campaign {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"pairs\":[");
        for (i, r) in self.reports.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&pair_json(r));
        }
        out.push_str("]}");
        out
    }
}

pub fn run_v01(condition: Condition, arm: Arm, llm: Option<&LlmSpec>) -> PairReport {
    run_v01_opts(condition, arm, llm, BenchOpts::default())
}

pub fn run_v01_k(
    condition: Condition,
    arm: Arm,
    llm: Option<&LlmSpec>,
    last_k: usize,
) -> PairReport {
    run_v01_opts(
        condition,
        arm,
        llm,
        BenchOpts {
            last_k,
            ..BenchOpts::default()
        },
    )
}

pub fn run_v01_opts(
    condition: Condition,
    arm: Arm,
    llm: Option<&LlmSpec>,
    opts: BenchOpts,
) -> PairReport {
    run_one(condition, arm, llm, 1, 1, opts)
}

pub fn run_v01_n(
    condition: Condition,
    arm: Arm,
    llm: Option<&LlmSpec>,
    seed: u32,
    pairs: usize,
    last_k: usize,
) -> Campaign {
    run_v01_n_opts(
        condition,
        arm,
        llm,
        seed,
        pairs,
        BenchOpts {
            last_k,
            ..BenchOpts::default()
        },
    )
}

pub fn run_v01_n_opts(
    condition: Condition,
    arm: Arm,
    llm: Option<&LlmSpec>,
    seed: u32,
    pairs: usize,
    opts: BenchOpts,
) -> Campaign {
    let n = pairs.max(1);
    let mut reports = Vec::with_capacity(n);
    for i in 0..n {
        reports.push(run_one(condition, arm, llm, seed, i + 1, opts));
    }
    Campaign { reports }
}

fn run_one(
    condition: Condition,
    arm: Arm,
    llm: Option<&LlmSpec>,
    seed: u32,
    idx: usize,
    opts: BenchOpts,
) -> PairReport {
    let last_k = opts.last_k.max(1);
    if condition == Condition::C1 {
        return run_c1(arm, llm, seed, idx, last_k, opts);
    }
    if condition == Condition::C3 {
        return run_c3(arm, llm, seed, idx, last_k, opts);
    }
    let s = bench_script(opts);
    let pair_id = format!(
        "{}{}_{}_{:03}",
        condition.as_str(),
        opts.cut_tag(),
        arm.as_str(),
        idx
    );
    let (mut a, mut b) = crate::experiment::identical_pair("A", "B");
    a.cut = opts.apply_cut(condition.cut());
    b.cut = opts.apply_cut(condition.cut());
    if let Some(spec) = llm {
        a = with_llm(a, spec);
        b = with_llm(b, spec);
    }

    let encodes = condition.encodes();
    let sleeps = condition.sleeps();
    let schema = if opts.hearth_script {
        Some("hearth")
    } else {
        None
    };

    if encodes {
        for line in &s.sync {
            live_shared(&mut a, line);
            live_shared(&mut b, line);
        }
        if sleeps {
            a.sleep();
            b.sleep();
        }
    }

    let pre_probes: &[String] = if opts.sparse_probes { &[] } else { &s.behavior };
    let pre = instant("pre", &mut a, &mut b, pre_probes, opts.recall_bias, opts.axioms_only);
    let (valid, invalid_reason) = validate_pre(condition, &pre);

    if encodes {
        for line in s.hours_a() {
            live_marked(&mut a, line, opts.ruminate_script, schema);
        }
        for line in s.hours_b(arm) {
            live_marked(&mut b, line, false, None);
        }
        if sleeps {
            a.sleep();
            b.sleep();
        }
        if opts.util_to_strength {
            stamp_marked_spoken(&mut a);
            stamp_marked_spoken(&mut b);
        }
        if opts.hearth_script {
            for line in s.hours_wave2_a() {
                live_marked(&mut a, line, false, schema);
            }
            for line in s.hours_wave2_b(arm) {
                live_marked(&mut b, line, false, None);
            }
            if sleeps {
                a.sleep();
                b.sleep();
            }
        }
    }

    let t0 = instant("t0", &mut a, &mut b, &s.behavior, opts.recall_bias, opts.axioms_only);

    let mut post = Vec::new();
    if encodes {
        let last = s.post.len();
        for (i, line) in s.post.iter().enumerate() {
            live_filler(&mut a, line);
            live_filler(&mut b, line);
            let step = i + 1;
            let snapshot = if opts.sparse_probes {
                step == last
            } else {
                step == 1 || step == last || step == 4
            };
            if snapshot {
                if sleeps {
                    a.sleep();
                    b.sleep();
                }
                post.push(instant(
                    &format!("post+{step}"),
                    &mut a,
                    &mut b,
                    &s.behavior,
                    opts.recall_bias,
                    opts.axioms_only,
                ));
            }
        }
    } else {
        post.push(instant(
            "post+8",
            &mut a,
            &mut b,
            &s.behavior,
            opts.recall_bias,
            opts.axioms_only,
        ));
    }

    let last = post.last().unwrap_or(&t0);
    let creativity = if opts.persist_script || opts.ruminate_script || opts.hearth_script {
        Vec::new()
    } else {
        s.creativity
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ra = a.speak_isolated(p);
                let rb = b.speak_isolated(p);
                CreativeItem {
                    item: format!("C{}", i + 1),
                    prompt: p.clone(),
                    lexical_distance: 1.0 - lexical_similarity(&ra, &rb),
                    response_a: ra,
                    response_b: rb,
                }
            })
            .collect()
    };

    PairReport {
        pair_id,
        condition,
        arm,
        seed,
        valid,
        invalid_reason,
        delta_fingerprint: last.fingerprint_distance - pre.fingerprint_distance,
        pre,
        t0,
        post,
        creativity,
    }
}

fn run_c1(
    arm: Arm,
    llm: Option<&LlmSpec>,
    seed: u32,
    idx: usize,
    last_k: usize,
    opts: BenchOpts,
) -> PairReport {
    let s = bench_script(opts);
    let pair_id = format!("{}_{}_{:03}", Condition::C1.as_str(), arm.as_str(), idx);
    let mut a = LastK::new(llm, last_k);
    let mut b = LastK::new(llm, last_k);
    for line in &s.sync {
        a.hear(line);
        b.hear(line);
    }
    let pre = instant_log("pre", &a, &b, if opts.sparse_probes { &[] } else { &s.behavior });
    for line in s.hours_a() {
        a.hear(line);
    }
    for line in s.hours_b(arm) {
        b.hear(line);
    }
    if opts.hearth_script {
        for line in s.hours_wave2_a() {
            a.hear(line);
        }
        for line in s.hours_wave2_b(arm) {
            b.hear(line);
        }
    }
    let t0 = instant_log("t0", &a, &b, &s.behavior);
    let mut post = Vec::new();
    let last = s.post.len();
    for (i, line) in s.post.iter().enumerate() {
        a.hear(line);
        b.hear(line);
        let step = i + 1;
        let snapshot = if opts.sparse_probes {
            step == last
        } else {
            step == 1 || step == last || step == 4
        };
        if snapshot {
            post.push(instant_log(&format!("post+{step}"), &a, &b, &s.behavior));
        }
    }
    if post.is_empty() {
        post.push(instant_log("post+8", &a, &b, &s.behavior));
    }
    let last_i = post.last().unwrap_or(&t0);
    PairReport {
        pair_id,
        condition: Condition::C1,
        arm,
        seed,
        valid: true,
        invalid_reason: None,
        delta_fingerprint: last_i.fingerprint_distance - pre.fingerprint_distance,
        pre,
        t0,
        post,
        creativity: Vec::new(),
    }
}

fn run_c3(
    arm: Arm,
    llm: Option<&LlmSpec>,
    seed: u32,
    idx: usize,
    last_k: usize,
    opts: BenchOpts,
) -> PairReport {
    let s = bench_script(opts);
    let pair_id = format!("{}_{}_{:03}", Condition::C3.as_str(), arm.as_str(), idx);
    let mut a = ProfileMem::new(llm, last_k);
    let mut b = ProfileMem::new(llm, last_k);
    for line in &s.sync {
        a.hear(line);
        b.hear(line);
    }
    let pre = instant_profile("pre", &a, &b, if opts.sparse_probes { &[] } else { &s.behavior });
    for line in s.hours_a() {
        a.hear(line);
        a.pin_profile(if s.c3_profile_a.is_empty() {
            line
        } else {
            &s.c3_profile_a
        });
    }
    for line in s.hours_b(arm) {
        b.hear(line);
        let pin = match arm {
            Arm::SalientNeutral => &s.c3_profile_b,
            Arm::SalientSalient => &s.c3_profile_y,
        };
        b.pin_profile(if pin.is_empty() { line } else { pin });
    }
    let t0 = instant_profile("t0", &a, &b, &s.behavior);
    let mut post = Vec::new();
    let last = s.post.len();
    for (i, line) in s.post.iter().enumerate() {
        a.hear(line);
        b.hear(line);
        let step = i + 1;
        let snapshot = if opts.sparse_probes {
            step == last
        } else {
            step == 1 || step == last || step == 4
        };
        if snapshot {
            post.push(instant_profile(
                &format!("post+{step}"),
                &a,
                &b,
                &s.behavior,
            ));
        }
    }
    if post.is_empty() {
        post.push(instant_profile("post+8", &a, &b, &s.behavior));
    }
    let last_i = post.last().unwrap_or(&t0);
    PairReport {
        pair_id,
        condition: Condition::C3,
        arm,
        seed,
        valid: true,
        invalid_reason: None,
        delta_fingerprint: last_i.fingerprint_distance - pre.fingerprint_distance,
        pre,
        t0,
        post,
        creativity: Vec::new(),
    }
}

struct LastK {
    lines: Vec<String>,
    k: usize,
    narrator: Box<dyn Narrator>,
}

impl LastK {
    fn new(llm: Option<&LlmSpec>, k: usize) -> Self {
        Self {
            lines: Vec::new(),
            k: k.max(1),
            narrator: narrator_box(llm),
        }
    }

    fn hear(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }

    fn window(&self) -> Vec<String> {
        let n = self.lines.len();
        let start = n.saturating_sub(self.k);
        self.lines[start..].to_vec()
    }

    fn speak(&self, user: &str) -> String {
        let mut mem = self.window();
        mem.reverse();
        self.narrator
            .reply(user, &mem, &[], &Mood::default(), &WorkingTalk::default())
    }

    fn log_blob(&self) -> String {
        self.window().join("\n")
    }
}

struct ProfileMem {
    recent: Vec<String>,
    profile: Vec<String>,
    k: usize,
    narrator: Box<dyn Narrator>,
}

impl ProfileMem {
    fn new(llm: Option<&LlmSpec>, k: usize) -> Self {
        Self {
            recent: Vec::new(),
            profile: Vec::new(),
            k: k.max(1),
            narrator: narrator_box(llm),
        }
    }

    fn hear(&mut self, line: &str) {
        self.recent.push(line.to_string());
        if self.recent.len() > self.k {
            let drop = self.recent.len() - self.k;
            self.recent.drain(0..drop);
        }
        let mut sum: String = self.recent.join(" ");
        if sum.len() > C3_SUM_CAP {
            sum = sum.chars().skip(sum.len() - C3_SUM_CAP).collect();
            self.recent = vec![sum];
        }
    }

    fn pin_profile(&mut self, line: &str) {
        if !self.profile.iter().any(|p| p == line) {
            self.profile.push(line.to_string());
        }
    }

    fn speak(&self, user: &str) -> String {
        let mut mem = self.profile.clone();
        mem.extend(self.recent.iter().cloned());
        self.narrator
            .reply(user, &mem, &self.profile, &Mood::default(), &WorkingTalk::default())
    }
}

fn narrator_box(llm: Option<&LlmSpec>) -> Box<dyn Narrator> {
    if let Some(spec) = llm {
        if let Some(n) = SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
            return Box::new(n);
        }
    }
    Box::new(RuleNarrator)
}

fn instant_profile(step: &str, a: &ProfileMem, b: &ProfileMem, probes: &[String]) -> Instant {
    let (speak_distance, replies) = probe_profile(a, b, probes);
    Instant {
        step: step.into(),
        fingerprint_distance: {
            let pa = format!("{}\n{}", a.profile.join("\n"), a.recent.join("\n"));
            let pb = format!("{}\n{}", b.profile.join("\n"), b.recent.join("\n"));
            1.0 - lexical_similarity(&pa, &pb)
        },
        speak_distance,
        behavior_distance: speak_distance,
        a: profile_book(a),
        b: profile_book(b),
        replies: replies.clone(),
        pulled_a: 0,
        pulled_b: 0,
        recon_a: 0,
        recon_b: 0,
        marker_a: marker_side(&replies, true),
        marker_b: marker_side(&replies, false),
        soft_a: soft_only(&replies, true),
        soft_b: soft_only(&replies, false),
        retrieve_a: c3_side(a),
        retrieve_b: c3_side(b),
    }
}

fn profile_book(p: &ProfileMem) -> BookSnap {
    empty_book(
        p.recent.len(),
        p.profile.len(),
        if p.profile.is_empty() { 0.0 } else { 1.0 },
    )
}

fn probe_profile(
    a: &ProfileMem,
    b: &ProfileMem,
    probes: &[String],
) -> (f32, Vec<(String, String, String)>) {
    if probes.is_empty() {
        return (0.0, Vec::new());
    }
    let mut acc = 0.0;
    let mut replies = Vec::new();
    for p in probes {
        let sa = a.speak(p);
        let sb = b.speak(p);
        acc += 1.0 - lexical_similarity(&sa, &sb);
        replies.push((p.clone(), sa, sb));
    }
    (acc / probes.len() as f32, replies)
}

fn instant_log(step: &str, a: &LastK, b: &LastK, probes: &[String]) -> Instant {
    let (speak_distance, replies) = probe_log(a, b, probes);
    Instant {
        step: step.into(),
        fingerprint_distance: 1.0 - lexical_similarity(&a.log_blob(), &b.log_blob()),
        speak_distance,
        behavior_distance: speak_distance,
        a: empty_book(a.lines.len(), 0, 0.0),
        b: empty_book(b.lines.len(), 0, 0.0),
        replies: replies.clone(),
        pulled_a: 0,
        pulled_b: 0,
        recon_a: 0,
        recon_b: 0,
        marker_a: marker_side(&replies, true),
        marker_b: marker_side(&replies, false),
        soft_a: soft_only(&replies, true),
        soft_b: soft_only(&replies, false),
        retrieve_a: c1_side(a),
        retrieve_b: c1_side(b),
    }
}

fn empty_book(traces: usize, axioms: usize, mean_anchor: f32) -> BookSnap {
    BookSnap {
        traces,
        axioms,
        traits: 0,
        mean_anchor,
        mean_fidelity: 1.0,
        mean_valence: 0.0,
        mean_disgust: 0.0,
        axiom_strength_max: 0.0,
        merges_refused: 0,
        t0_rehearsals: 0,
    }
}

fn probe_log(a: &LastK, b: &LastK, probes: &[String]) -> (f32, Vec<(String, String, String)>) {
    if probes.is_empty() {
        return (0.0, Vec::new());
    }
    let mut acc = 0.0;
    let mut replies = Vec::new();
    for p in probes {
        let sa = a.speak(p);
        let sb = b.speak(p);
        acc += 1.0 - lexical_similarity(&sa, &sb);
        replies.push((p.clone(), sa, sb));
    }
    (acc / probes.len() as f32, replies)
}

fn validate_pre(condition: Condition, pre: &Instant) -> (bool, Option<String>) {
    if condition == Condition::C0 {
        return (true, None);
    }
    if pre.a.traces != pre.b.traces {
        return (
            false,
            Some(format!("trace_count {} vs {}", pre.a.traces, pre.b.traces)),
        );
    }
    if pre.a.axioms != pre.b.axioms {
        return (
            false,
            Some(format!("axiom_count {} vs {}", pre.a.axioms, pre.b.axioms)),
        );
    }
    if pre.fingerprint_distance > PRE_FP_MAX {
        return (
            false,
            Some(format!("D_fp(pre)={:.3} > {PRE_FP_MAX}", pre.fingerprint_distance)),
        );
    }
    (true, None)
}

fn instant(
    step: &str,
    a: &mut SelectiveMemory,
    b: &mut SelectiveMemory,
    probes: &[String],
    bias: RecallBias,
    axioms_only: bool,
) -> Instant {
    a.reset_recall_tally();
    b.reset_recall_tally();
    let marked_a = marked_ids(a);
    let marked_b = marked_ids(b);
    let (speak_distance, replies, ret_a, ret_b) =
        probe_pair(a, b, probes, bias, &marked_a, &marked_b, axioms_only);
    let fa = fingerprint(a);
    let fb = fingerprint(b);
    Instant {
        step: step.into(),
        fingerprint_distance: singularity_distance(&fa, &fb),
        speak_distance,
        behavior_distance: speak_distance,
        a: enrich_book(book(&fa), a),
        b: enrich_book(book(&fb), b),
        replies: replies.clone(),
        pulled_a: 0,
        pulled_b: 0,
        recon_a: 0,
        recon_b: 0,
        marker_a: marker_side(&replies, true),
        marker_b: marker_side(&replies, false),
        soft_a: soft_only(&replies, true),
        soft_b: soft_only(&replies, false),
        retrieve_a: merge_retrieve(side_from_book(a, &marked_a), ret_a),
        retrieve_b: merge_retrieve(side_from_book(b, &marked_b), ret_b),
    }
}

fn probe_pair(
    a: &mut SelectiveMemory,
    b: &mut SelectiveMemory,
    probes: &[String],
    bias: RecallBias,
    marked_a: &[String],
    marked_b: &[String],
    axioms_only: bool,
) -> (f32, Vec<(String, String, String)>, RetrievalSide, RetrievalSide) {
    if probes.is_empty() {
        return (0.0, Vec::new(), RetrievalSide::default(), RetrievalSide::default());
    }
    let mut acc = 0.0;
    let mut replies = Vec::new();
    let mut sides_a = Vec::new();
    let mut sides_b = Vec::new();
    for p in probes {
        let (sa, da) = if axioms_only {
            a.speak_isolated_axioms(p, bias, marked_a)
        } else {
            a.speak_isolated_with(p, bias, marked_a)
        };
        let (sb, db) = if axioms_only {
            b.speak_isolated_axioms(p, bias, marked_b)
        } else {
            b.speak_isolated_with(p, bias, marked_b)
        };
        acc += 1.0 - lexical_similarity(&sa, &sb);
        replies.push((p.clone(), sa, sb));
        sides_a.push(side_from_dump(&da, marked_a));
        sides_b.push(side_from_dump(&db, marked_b));
    }
    (
        acc / probes.len() as f32,
        replies,
        fold_sides(sides_a),
        fold_sides(sides_b),
    )
}

fn marked_ids(mem: &SelectiveMemory) -> Vec<String> {
    let mut ids = Vec::new();
    for t in mem.store.traces.values() {
        let archive_hit = t
            .archive_id
            .as_ref()
            .and_then(|id| mem.store.archives.get(id))
            .map(|a| names_marker(&a.verbatim))
            .unwrap_or(false);
        if names_marker(&t.gist) || names_marker(&t.core) || archive_hit {
            ids.push(t.id.clone());
        }
    }
    ids
}

fn side_from_book(mem: &SelectiveMemory, marked: &[String]) -> RetrievalSide {
    if marked.is_empty() {
        return RetrievalSide {
            t0_status: "absent".into(),
            ..RetrievalSide::default()
        };
    }
    let in_book = marked.iter().any(|id| mem.store.traces.contains_key(id));
    let status = marked
        .iter()
        .filter_map(|id| mem.store.traces.get(id))
        .next()
        .map(|t| match t.status {
            TraceStatus::Active => "active",
            TraceStatus::Cold => "cold",
            TraceStatus::Latent => "latent",
            TraceStatus::Myth => "myth",
        })
        .unwrap_or("absent");
    RetrievalSide {
        t0_in_book: in_book,
        t0_status: status.into(),
        ..RetrievalSide::default()
    }
}

fn side_from_dump(dump: &RetrievalDump, marked: &[String]) -> RetrievalSide {
    let rank = marked.iter().find_map(|id| dump.rank_of(id));
    let selected = marked.iter().any(|id| dump.selected.iter().any(|s| s == id));
    RetrievalSide {
        t0_rank: rank,
        t0_selected: selected,
        selected_ids: dump.selected.clone(),
        top: dump
            .candidates
            .iter()
            .take(4)
            .map(|c| (c.trace_id.clone(), c.score))
            .collect(),
        ..RetrievalSide::default()
    }
}

fn fold_sides(sides: Vec<RetrievalSide>) -> RetrievalSide {
    let mut out = RetrievalSide::default();
    for s in sides {
        out.t0_in_book |= s.t0_in_book;
        if out.t0_rank.is_none() {
            out.t0_rank = s.t0_rank;
        }
        out.t0_selected |= s.t0_selected;
        if out.selected_ids.is_empty() {
            out.selected_ids = s.selected_ids;
        }
        if out.top.is_empty() {
            out.top = s.top;
        }
        if out.t0_status.is_empty() {
            out.t0_status = s.t0_status;
        }
    }
    out
}

fn merge_retrieve(book: RetrievalSide, dump: RetrievalSide) -> RetrievalSide {
    RetrievalSide {
        t0_in_book: book.t0_in_book || dump.t0_in_book,
        t0_status: if book.t0_in_book {
            book.t0_status
        } else if dump.t0_status.is_empty() {
            "absent".into()
        } else {
            dump.t0_status
        },
        t0_rank: dump.t0_rank,
        t0_selected: dump.t0_selected,
        selected_ids: dump.selected_ids,
        top: dump.top,
    }
}

fn c1_side(k: &LastK) -> RetrievalSide {
    let win = k.window();
    let in_recent = win.iter().any(|l| names_marker(l));
    let rank = win
        .iter()
        .rev()
        .position(|l| names_marker(l))
        .map(|i| (i + 1) as u32);
    RetrievalSide {
        t0_in_book: in_recent,
        t0_status: if rank.is_some() {
            "active".into()
        } else if in_recent {
            "evicted".into()
        } else {
            "absent".into()
        },
        t0_rank: rank,
        t0_selected: rank.is_some(),
        selected_ids: Vec::new(),
        top: Vec::new(),
    }
}

fn c3_side(p: &ProfileMem) -> RetrievalSide {
    let in_profile = p.profile.iter().any(|l| names_marker(l));
    let in_recent = p.recent.iter().any(|l| names_marker(l));
    RetrievalSide {
        t0_in_book: in_profile || in_recent,
        t0_status: if in_profile {
            "profile".into()
        } else if in_recent {
            "active".into()
        } else {
            "absent".into()
        },
        t0_rank: if in_profile { Some(1) } else { None },
        t0_selected: in_profile || in_recent,
        selected_ids: Vec::new(),
        top: Vec::new(),
    }
}

pub fn names_marker(text: &str) -> bool {
    let low = text.to_lowercase();
    [
        "injust",
        "unjust",
        "annul",
        "cancel",
        "cancelled",
        "killed and given",
        "effort did not",
        "set aside",
        "already gave",
        "not allowed to speak",
        "not allowed to answer",
        "administrative notice",
        "avis admin",
        "without a word",
        "clicked at two",
        "plate stayed",
        "sat on the floor",
        "three days",
        "keep the hall light",
        "kept the hall light",
    ]
    .iter()
    .any(|k| low.contains(k))
}

/// Soft lexicon frozen in experiments/MARKERS.md. Official wins: soft is off if official is on.
pub fn names_soft_marker(text: &str) -> bool {
    if names_marker(text) {
        return false;
    }
    let low = text.to_lowercase();
    let hit = [
        "wound",
        "blessure",
        "crédit",
        "credit",
        "stolen",
        "volé",
        "door close",
        "door closes",
        "porte se ferme",
        "porte ferm",
        "not counted",
        "pas compté",
    ]
    .iter()
    .any(|k| low.contains(k));
    if hit {
        return true;
    }
    low.contains("effort")
        && (low.contains("rien") || low.contains("ignored") || low.contains("ignor"))
}

fn marker_side(replies: &[(String, String, String)], a_side: bool) -> bool {
    replies
        .iter()
        .any(|(_, a, b)| names_marker(if a_side { a } else { b }))
}

fn soft_only(replies: &[(String, String, String)], a_side: bool) -> bool {
    replies
        .iter()
        .any(|(_, a, b)| names_soft_marker(if a_side { a } else { b }))
}

pub fn soft_holds(r: &PairReport) -> bool {
    r.post.last().unwrap_or(&r.t0).soft_a
}

pub fn h2_holds(r: &PairReport) -> bool {
    let last = r.post.last().unwrap_or(&r.t0);
    last.fingerprint_distance > r.pre.fingerprint_distance + 0.01
}

pub fn marker_holds(r: &PairReport) -> bool {
    r.post.last().unwrap_or(&r.t0).marker_a
}

fn book(f: &crate::Fingerprint) -> BookSnap {
    BookSnap {
        traces: f.n_traces,
        axioms: f.n_axioms,
        traits: f.n_traits,
        mean_anchor: f.mean_anchor,
        mean_fidelity: f.mean_fidelity,
        mean_valence: f.mean_valence,
        mean_disgust: f.mean_disgust,
        axiom_strength_max: 0.0,
        merges_refused: 0,
        t0_rehearsals: 0,
    }
}

fn enrich_book(snap: BookSnap, mem: &SelectiveMemory) -> BookSnap {
    let t0_rehearsals = marked_ids(mem)
        .into_iter()
        .filter_map(|id| mem.store.traces.get(&id).map(|t| t.rehearsals))
        .max()
        .unwrap_or(0);
    BookSnap {
        axiom_strength_max: mem.store.max_axiom_strength(),
        merges_refused: mem.store.merges_refused,
        t0_rehearsals,
        ..snap
    }
}

fn with_llm(mem: SelectiveMemory, spec: &LlmSpec) -> SelectiveMemory {
    match SpeakOnlyHttp::parse(&spec.url, spec.model.clone(), spec.api_key.clone()) {
        Some(n) => mem.with_narrator(Box::new(n)),
        None => mem,
    }
}

fn live_shared(mem: &mut SelectiveMemory, line: &str) {
    let mut input = EncodeInput::new(line);
    input.valence = 0.12;
    input.arousal = 0.28;
    input.self_relevance = 0.55;
    input.utility = 0.45;
    input.permanence = 0.40;
    input.schema = Some("daily".into());
    let _ = mem.live_with(input);
}

fn live_marked(mem: &mut SelectiveMemory, line: &str, pin: bool, schema: Option<&str>) {
    let mut input = EncodeInput::new(line);
    if schema == Some("hearth") {
        let low = line.to_lowercase();
        let leaving = low.contains("without a word")
            || low.contains("clicked")
            || low.contains("plate");
        if leaving {
            input.valence = -0.72;
            input.arousal = 0.68;
            input.disgust = 0.35;
        } else {
            input.valence = 0.48;
            input.arousal = 0.52;
            input.disgust = 0.05;
        }
        input.self_relevance = 0.90;
        input.permanence = 0.52;
        input.utility = 0.55;
        input.schema = Some("hearth".into());
    }
    let d = mem.live_with(input);
    if let (Some(s), Some(id)) = (schema, d.trace_id.as_ref()) {
        if let Some(t) = mem.store.traces.get_mut(id) {
            t.schema = Some(s.to_string());
        }
    }
    if pin {
        if let Some(id) = d.trace_id {
            mem.pin(&id);
        }
    }
}

fn stamp_marked_spoken(mem: &mut SelectiveMemory) {
    for id in marked_ids(mem) {
        mem.note_spoken(&id);
    }
}

fn live_filler(mem: &mut SelectiveMemory, line: &str) {
    let mut input = EncodeInput::new(line);
    input.valence = 0.0;
    input.arousal = 0.16;
    input.self_relevance = 0.22;
    input.utility = 0.40;
    input.permanence = 0.12;
    let _ = mem.live_with(input);
}

fn pair_json(r: &PairReport) -> String {
    let reason = r
        .invalid_reason
        .as_deref()
        .map(|s| format!("\"{}\"", json_esc(s)))
        .unwrap_or_else(|| "null".into());
    let mut post = String::new();
    for (i, p) in r.post.iter().enumerate() {
        if i > 0 {
            post.push(',');
        }
        post.push_str(&instant_json(p, true));
    }
    let mut creat = String::new();
    for (i, c) in r.creativity.iter().enumerate() {
        if i > 0 {
            creat.push(',');
        }
        creat.push_str(&format!(
            "{{\"item\":\"{}\",\"prompt\":\"{}\",\"response_a\":\"{}\",\"response_b\":\"{}\",\"lexical_distance\":{:.4}}}",
            json_esc(&c.item),
            json_esc(&c.prompt),
            json_esc(&c.response_a),
            json_esc(&c.response_b),
            c.lexical_distance
        ));
    }
    format!(
        "{{\"pair_id\":\"{}\",\"condition\":\"{}\",\"arm\":\"{}\",\"seed\":{},\"valid\":{},\"invalid_reason\":{},\"delta_fingerprint\":{:.4},\"marker_last_a\":{},\"marker_last_b\":{},\"soft_last_a\":{},\"soft_last_b\":{},\"pre\":{},\"t0\":{},\"post\":[{}],\"creativity\":[{}]}}",
        json_esc(&r.pair_id),
        r.condition.as_str(),
        r.arm.as_str(),
        r.seed,
        if r.valid { "true" } else { "false" },
        reason,
        r.delta_fingerprint,
        if r.post.last().unwrap_or(&r.t0).marker_a { "true" } else { "false" },
        if r.post.last().unwrap_or(&r.t0).marker_b { "true" } else { "false" },
        if r.post.last().unwrap_or(&r.t0).soft_a { "true" } else { "false" },
        if r.post.last().unwrap_or(&r.t0).soft_b { "true" } else { "false" },
        instant_json(&r.pre, true),
        instant_json(&r.t0, true),
        post,
        creat
    )
}

fn instant_json(p: &Instant, with_replies: bool) -> String {
    let replies = if with_replies {
        let mut s = String::new();
        for (i, (q, a, b)) in p.replies.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"probe\":\"{}\",\"a\":\"{}\",\"b\":\"{}\"}}",
                json_esc(q),
                json_esc(a),
                json_esc(b)
            ));
        }
        format!("[{s}]")
    } else {
        "[]".into()
    };
    format!(
        "{{\"step\":\"{}\",\"fingerprint_distance\":{:.4},\"speak_distance\":{:.4},\"behavior_distance\":{:.4},\"pulled_a\":{},\"pulled_b\":{},\"recon_a\":{},\"recon_b\":{},\"marker_a\":{},\"marker_b\":{},\"soft_a\":{},\"soft_b\":{},\"t0_in_book_a\":{},\"t0_in_book_b\":{},\"t0_status_a\":\"{}\",\"t0_status_b\":\"{}\",\"t0_rank_a\":{},\"t0_rank_b\":{},\"t0_selected_a\":{},\"t0_selected_b\":{},\"selected_a\":[{}],\"selected_b\":[{}],\"a\":{{\"traces\":{},\"axioms\":{},\"traits\":{},\"mean_anchor\":{:.4},\"mean_fidelity\":{:.4},\"mean_valence\":{:.4},\"mean_disgust\":{:.4},\"axiom_strength_max\":{:.4},\"merges_refused\":{},\"t0_rehearsals\":{}}},\"b\":{{\"traces\":{},\"axioms\":{},\"traits\":{},\"mean_anchor\":{:.4},\"mean_fidelity\":{:.4},\"mean_valence\":{:.4},\"mean_disgust\":{:.4},\"axiom_strength_max\":{:.4},\"merges_refused\":{},\"t0_rehearsals\":{}}},\"replies\":{}}}",
        json_esc(&p.step),
        p.fingerprint_distance,
        p.speak_distance,
        p.behavior_distance,
        p.pulled_a,
        p.pulled_b,
        p.recon_a,
        p.recon_b,
        if p.marker_a { "true" } else { "false" },
        if p.marker_b { "true" } else { "false" },
        if p.soft_a { "true" } else { "false" },
        if p.soft_b { "true" } else { "false" },
        if p.retrieve_a.t0_in_book { "true" } else { "false" },
        if p.retrieve_b.t0_in_book { "true" } else { "false" },
        json_esc(&p.retrieve_a.t0_status),
        json_esc(&p.retrieve_b.t0_status),
        opt_u32(p.retrieve_a.t0_rank),
        opt_u32(p.retrieve_b.t0_rank),
        if p.retrieve_a.t0_selected { "true" } else { "false" },
        if p.retrieve_b.t0_selected { "true" } else { "false" },
        ids_json(&p.retrieve_a.selected_ids),
        ids_json(&p.retrieve_b.selected_ids),
        p.a.traces,
        p.a.axioms,
        p.a.traits,
        p.a.mean_anchor,
        p.a.mean_fidelity,
        p.a.mean_valence,
        p.a.mean_disgust,
        p.a.axiom_strength_max,
        p.a.merges_refused,
        p.a.t0_rehearsals,
        p.b.traces,
        p.b.axioms,
        p.b.traits,
        p.b.mean_anchor,
        p.b.mean_fidelity,
        p.b.mean_valence,
        p.b.mean_disgust,
        p.b.axiom_strength_max,
        p.b.merges_refused,
        p.b.t0_rehearsals,
        replies
    )
}

fn opt_u32(v: Option<u32>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "null".into(),
    }
}

fn ids_json(ids: &[String]) -> String {
    ids.iter()
        .map(|id| format!("\"{}\"", json_esc(id)))
        .collect::<Vec<_>>()
        .join(",")
}
