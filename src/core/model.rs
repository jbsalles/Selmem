use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// Real unix seconds when the virtual clock was first read.
static CLOCK_ORIGIN_REAL: AtomicU64 = AtomicU64::new(0);
/// Extra virtual seconds jumped by `advance_hours` (UI night).
static CLOCK_JUMP: AtomicU64 = AtomicU64::new(0);
/// Wall-time multiplier 1..=200. UI default 24 (one sleep ≈ one day).
static CLOCK_SCALE: AtomicU32 = AtomicU32::new(24);

fn wall_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn origin_real() -> u64 {
    let o = CLOCK_ORIGIN_REAL.load(Ordering::Relaxed);
    if o == 0 {
        let w = wall_secs();
        match CLOCK_ORIGIN_REAL.compare_exchange(0, w, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => w,
            Err(cur) => cur,
        }
    } else {
        o
    }
}

/// One organ's virtual clock. Detached clocks do not share the process jump.
#[derive(Clone, Debug)]
pub struct MemoryClock {
    pub origin_real: u64,
    pub jump: u64,
    pub scale: u32,
    pub detached: bool,
}

impl Default for MemoryClock {
    fn default() -> Self {
        Self {
            origin_real: 0,
            jump: 0,
            scale: 24,
            detached: false,
        }
    }
}

impl MemoryClock {
    pub fn detach(mut self) -> Self {
        self.origin_real = origin_real();
        self.jump = CLOCK_JUMP.load(Ordering::Relaxed);
        self.scale = clock_scale();
        self.detached = true;
        self
    }

    pub fn now(&self) -> u64 {
        if !self.detached {
            return process_now();
        }
        let elapsed = wall_secs().saturating_sub(self.origin_real);
        self.origin_real
            .saturating_add(elapsed.saturating_mul(self.scale.max(1) as u64))
            .saturating_add(self.jump)
    }

    pub fn advance_hours(&mut self, hours: f32) {
        let secs = (hours.max(0.0) * 3600.0) as u64;
        if self.detached {
            self.jump = self.jump.saturating_add(secs);
        } else {
            CLOCK_JUMP.fetch_add(secs, Ordering::Relaxed);
        }
    }

    pub fn set_scale(&mut self, scale: u32) {
        let scale = scale.clamp(1, 200);
        self.scale = scale;
        if !self.detached {
            CLOCK_SCALE.store(scale, Ordering::Relaxed);
        }
    }
}

std::thread_local! {
    static ACTIVE_CLOCK: std::cell::RefCell<Option<MemoryClock>> = std::cell::RefCell::new(None);
}

pub struct ClockGuard {
    prev: Option<MemoryClock>,
}

impl ClockGuard {
    pub fn push(clock: MemoryClock) -> Self {
        let prev = ACTIVE_CLOCK.with(|c| c.borrow_mut().replace(clock));
        Self { prev }
    }
}

impl Drop for ClockGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        ACTIVE_CLOCK.with(|c| *c.borrow_mut() = prev);
    }
}

fn process_now() -> u64 {
    let origin = origin_real();
    let elapsed = wall_secs().saturating_sub(origin);
    let scale = CLOCK_SCALE.load(Ordering::Relaxed).clamp(1, 200) as u64;
    origin
        .saturating_add(elapsed.saturating_mul(scale))
        .saturating_add(CLOCK_JUMP.load(Ordering::Relaxed))
}

/// Virtual now. An organ guard wins when its clock is detached.
pub fn now_secs() -> u64 {
    ACTIVE_CLOCK.with(|c| match c.borrow().as_ref() {
        Some(clock) if clock.detached => clock.now(),
        _ => process_now(),
    })
}

pub fn clock_scale() -> u32 {
    CLOCK_SCALE.load(Ordering::Relaxed).clamp(1, 200)
}

pub fn set_clock_scale(scale: u32) {
    CLOCK_SCALE.store(scale.clamp(1, 200), Ordering::Relaxed);
}

/// Jump the organ clock forward. One UI sleep uses this so weather sees days.
pub fn advance_hours(hours: f32) {
    let secs = (hours.max(0.0) * 3600.0) as u64;
    CLOCK_JUMP.fetch_add(secs, Ordering::Relaxed);
}

#[cfg(test)]
pub fn reset_clock_for_tests() {
    CLOCK_ORIGIN_REAL.store(0, Ordering::Relaxed);
    CLOCK_JUMP.store(0, Ordering::Relaxed);
    CLOCK_SCALE.store(1, Ordering::Relaxed);
}

pub fn new_id(prefix: &str) -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() & 0xffff;
    format!("{prefix}_{pid:04x}_{n:08x}")
}

pub fn set_next_id(n: u64) {
    NEXT_ID.store(n.max(1), Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Selfhood,
    World,
    /// Tool / journal lines. Always kept. No reconstruct, weather, or merge.
    Log,
}

impl Channel {
    /// Do not warp this record. World facts and tool logs.
    pub fn verbatim(self) -> bool {
        matches!(self, Self::World | Self::Log)
    }
}

/// Who the hour treats as the agent. Default `None` keeps night policy unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Attribution {
    #[default]
    None,
    External,
    Internal,
}

impl Attribution {
    pub fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::External => "external",
            Self::Internal => "internal",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "external" => Self::External,
            "internal" => Self::Internal,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceStatus {
    Active,
    Cold,
    Myth,
    /// Scene gone; affect / schema still color the next hour.
    Latent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriftKind {
    Embellish,
    AmplifyDisgust,
    Fade,
    Merge,
    Weather,
    Rewrite,
    Reinterpret,
    Ground,
    /// Same event, other speech act. Mouth only; gist and core stay.
    Color,
    /// Fill a hole. Not gild: the detail was gone, the fill is new.
    Confabulate,
    /// Directed forgetting. Not weather, not release.
    Suppress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxiomLayer {
    Motif,
    Belief,
    Trait,
}

impl AxiomLayer {
    /// A motif cannot become a law. Only a trait may sit at 1.0.
    pub fn strength_cap(self) -> f32 {
        match self {
            AxiomLayer::Motif => 0.42,
            AxiomLayer::Belief => 0.70,
            AxiomLayer::Trait => 1.0,
        }
    }

    pub fn strength_floor(self) -> f32 {
        match self {
            AxiomLayer::Motif => 0.18,
            AxiomLayer::Belief => 0.28,
            AxiomLayer::Trait => 0.40,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DriftEvent {
    pub kind: DriftKind,
    pub at: u64,
    pub note: String,
    pub fidelity_delta: f32,
    pub valence_delta: f32,
    pub disgust_delta: f32,
}

/// Where a clause claims to come from. Confabulation is not event evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceOrigin {
    Event,
    Reconstruction,
    Confabulation,
    Axiom,
    SchemaCenter,
}

impl EvidenceOrigin {
    pub fn token(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Reconstruction => "reconstruction",
            Self::Confabulation => "confabulation",
            Self::Axiom => "axiom",
            Self::SchemaCenter => "center",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "reconstruction" => Self::Reconstruction,
            "confabulation" => Self::Confabulation,
            "axiom" => Self::Axiom,
            "center" => Self::SchemaCenter,
            _ => Self::Event,
        }
    }
}

/// One transformation of an hour. The genealogy the mouth does not read.
#[derive(Clone, Debug)]
pub struct MemoryOperation {
    pub kind: String,
    pub at: u64,
    pub source_trace_ids: Vec<String>,
    pub source_axiom_ids: Vec<String>,
    pub source_center: Option<String>,
    pub before: String,
    pub after: String,
    pub confidence: f32,
    pub origin: EvidenceOrigin,
}

/// Claim plus the pieces a lexical core cannot name. The string core stays the freeze.
#[derive(Clone, Debug)]
pub struct SemanticCore {
    pub claim: String,
    pub entities: Vec<String>,
    pub actions: Vec<String>,
    pub polarity: f32,
    pub confidence: f32,
}

impl Default for SemanticCore {
    fn default() -> Self {
        Self {
            claim: String::new(),
            entities: Vec::new(),
            actions: Vec::new(),
            polarity: 0.0,
            confidence: 0.0,
        }
    }
}

impl SemanticCore {
    pub fn from_event(event: &str, claim: &str, valence: f32) -> Self {
        let entities = extract_entities(event);
        let actions = extract_actions(event);
        Self {
            claim: claim.to_string(),
            entities,
            actions,
            polarity: valence.clamp(-1.0, 1.0),
            confidence: 0.55,
        }
    }
}

fn extract_entities(event: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in event.split_whitespace() {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if w.len() < 2 {
            continue;
        }
        let low = w.to_lowercase();
        let pronoun = matches!(low.as_str(), "i" | "me" | "she" | "he" | "they" | "we" | "him" | "her");
        let named = w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        if pronoun || named {
            if !out.iter().any(|e: &String| e.eq_ignore_ascii_case(w)) {
                out.push(w.to_string());
            }
        }
    }
    out.truncate(8);
    out
}

fn extract_actions(event: &str) -> Vec<String> {
    const VERBS: &[&str] = &[
        "left", "said", "walked", "abandoned", "told", "asked", "stayed", "opened", "closed",
        "went", "came", "took", "gave", "kept", "broke", "loved", "hated", "waited", "lied",
        "withdrawn", "renewed",
    ];
    let mut out = Vec::new();
    for raw in event.split_whitespace() {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if VERBS.contains(&w.as_str()) || (w.len() > 4 && w.ends_with("ed")) {
            if !out.contains(&w) {
                out.push(w);
            }
        }
    }
    out.truncate(6);
    out
}

/// What is still checkable. Distinct from the belief core, which may be revised.
#[derive(Clone, Debug)]
pub struct RealityAnchor {
    pub observation_id: Option<String>,
    pub claim: String,
    pub verifiable: bool,
}

impl Default for RealityAnchor {
    fn default() -> Self {
        Self {
            observation_id: None,
            claim: String::new(),
            verifiable: false,
        }
    }
}

/// What was concluded at encode. The verbatim observation stays in the archive.
#[derive(Clone, Debug)]
pub struct InterpretationStamp {
    pub statement: String,
    pub valence: f32,
    pub confidence: f32,
}

/// What is at stake. Distinct from the sign of the hour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StakeKind {
    #[default]
    None,
    Decision,
    Presence,
    Rule,
    Promise,
    Limit,
    Mood,
    Absence,
}

impl StakeKind {
    pub fn token(self) -> &'static str {
        match self {
            StakeKind::None => "none",
            StakeKind::Decision => "decision",
            StakeKind::Presence => "presence",
            StakeKind::Rule => "rule",
            StakeKind::Promise => "promise",
            StakeKind::Limit => "limit",
            StakeKind::Mood => "mood",
            StakeKind::Absence => "absence",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "decision" => StakeKind::Decision,
            "presence" => StakeKind::Presence,
            "rule" => StakeKind::Rule,
            "promise" => StakeKind::Promise,
            "limit" => StakeKind::Limit,
            "mood" => StakeKind::Mood,
            "absence" => StakeKind::Absence,
            _ => StakeKind::None,
        }
    }

    /// A limit outlasts a mood. The sign is not the reason.
    pub fn survival(self) -> f32 {
        match self {
            StakeKind::Limit | StakeKind::Promise | StakeKind::Rule => 0.55,
            StakeKind::Absence => 0.70,
            StakeKind::Decision | StakeKind::Presence => 0.85,
            StakeKind::Mood | StakeKind::None => 1.15,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Bearer {
    #[default]
    World,
    Self_,
    Other,
}

impl Bearer {
    pub fn token(self) -> &'static str {
        match self {
            Bearer::World => "world",
            Bearer::Self_ => "self",
            Bearer::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "self" => Bearer::Self_,
            "other" => Bearer::Other,
            _ => Bearer::World,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LossKind {
    #[default]
    None,
    Status,
    Access,
    Coherence,
    Time,
}

impl LossKind {
    pub fn token(self) -> &'static str {
        match self {
            LossKind::None => "none",
            LossKind::Status => "status",
            LossKind::Access => "access",
            LossKind::Coherence => "coherence",
            LossKind::Time => "time",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "status" => LossKind::Status,
            "access" => LossKind::Access,
            "coherence" => LossKind::Coherence,
            "time" => LossKind::Time,
            _ => LossKind::None,
        }
    }
}

/// What did not happen. Not a negative event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsenceKind {
    Unmet,
    Unanswered,
    Missed,
}

impl AbsenceKind {
    pub fn token(self) -> &'static str {
        match self {
            AbsenceKind::Unmet => "unmet",
            AbsenceKind::Unanswered => "unanswered",
            AbsenceKind::Missed => "missed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "unmet" => Some(AbsenceKind::Unmet),
            "unanswered" => Some(AbsenceKind::Unanswered),
            "missed" => Some(AbsenceKind::Missed),
            _ => None,
        }
    }
}

/// Stake read off the hour, before affect colors it.
pub fn derive_stake(event: &str) -> (StakeKind, Bearer, LossKind, String, Option<AbsenceKind>) {
    let low = event.to_lowercase();
    let absence = if low.contains("no reply") || low.contains("unanswered") {
        Some(AbsenceKind::Unanswered)
    } else if low.contains("didn't come") || low.contains("did not come") || low.contains("missed") {
        Some(AbsenceKind::Missed)
    } else if low.contains("never arrived") || low.contains("unmet") {
        Some(AbsenceKind::Unmet)
    } else {
        None
    };
    let kind = if absence.is_some() {
        StakeKind::Absence
    } else if low.contains("promise") || low.contains("vow") {
        StakeKind::Promise
    } else if low.contains("rule") || low.contains("must") {
        StakeKind::Rule
    } else if low.contains("limit") || low.contains("boundary") || low.contains("withdrawn") || low.contains("cancelled") {
        StakeKind::Limit
    } else if low.contains("decision") || low.contains("choose") {
        StakeKind::Decision
    } else if low.contains("present") || low.contains("stayed") {
        StakeKind::Presence
    } else if low.contains("felt") || low.contains("mood") {
        StakeKind::Mood
    } else {
        StakeKind::None
    };
    let bearer = if low.contains(" i ") || low.starts_with("i ") {
        Bearer::Self_
    } else if low.contains("she ") || low.contains("he ") || low.contains("they ") {
        Bearer::Other
    } else {
        Bearer::World
    };
    let loss = if kind == StakeKind::Limit {
        LossKind::Status
    } else if kind == StakeKind::Absence {
        LossKind::Access
    } else if kind == StakeKind::Promise || kind == StakeKind::Rule {
        LossKind::Coherence
    } else if kind == StakeKind::Presence {
        LossKind::Time
    } else {
        LossKind::None
    };
    let mark = stake_mark(&low);
    (kind, bearer, loss, mark, absence)
}

fn stake_mark(low: &str) -> String {
    const SKIP: &[&str] = &[
        "that", "this", "with", "from", "after", "before", "into", "your", "their", "been",
        "were", "was", "have", "has", "had", "them", "they", "what", "the", "and", "for",
    ];
    let tokens: Vec<&str> = low
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 4 && !SKIP.contains(w))
        .collect();
    const PREFER: &[&str] = &["withdrawn", "cancelled", "extended", "renewed"];
    if let Some(act) = tokens.iter().find(|w| PREFER.contains(w)) {
        return (*act).to_string();
    }
    tokens.into_iter().max_by_key(|w| w.len()).unwrap_or("").to_string()
}

/// One lived episode. This is what the entity *uses*.
/// The sealed archive (verbatim) lives in `ArchiveRecord`, pointed at by `archive_id`.
#[derive(Clone, Debug)]
pub struct MemoryTrace {
    pub id: String,
    /// Reconstructable story the narrator may tell. This can drift.
    pub gist: String,
    pub cues: Vec<String>,
    pub valence: f32,
    pub arousal: f32,
    pub disgust: f32,
    pub self_relevance: f32,
    /// 0 = does not describe the living self. 1 = sits in it. Snapshot at encode.
    pub self_congruence: f32,
    /// Agent of the hour. `None` until a seed pins it.
    pub attribution: Attribution,
    /// Topic bucket used to cluster traces into motifs / beliefs.
    pub schema: Option<String>,
    pub channel: Channel,
    /// Pointer into the sealed book. Never handed to the narrator.
    pub archive_id: Option<String>,
    pub created_at: u64,
    pub last_recalled_at: Option<u64>,
    pub last_consolidated_at: Option<u64>,
    /// How precise the current gist still is (0 = blur, 1 = sharp).
    pub fidelity: f32,
    /// How long this episode is meant to last. High → resists the encode gate drop.
    pub permanence: f32,
    pub rehearsals: u32,
    /// How easy this episode is to find again. Falls with disuse.
    pub access: f32,
    /// How sure the current gist still is. Not access. A fluent lie can be easy to find.
    pub confidence: f32,
    /// Directed forgetting. The hour stays; default recall will not pick it.
    pub suppressed: bool,
    pub status: TraceStatus,
    pub drifts: Vec<DriftEvent>,
    pub salience_at_encode: f32,
    pub embedding: Vec<f32>,
    /// Stable semantic reference frozen at encode. Grounding compares against this.
    pub core: String,
    /// 0..1 resistance to decay and rewrite (trauma, triumph, vow).
    pub anchor: f32,
    /// Consecutive spoken sentences that left the core. Reset after a pull-back.
    pub detach_strikes: u32,
    /// Archive id of the observation. The verbatim is not copied into the live core.
    pub observation_id: Option<String>,
    /// What SelMem concluded at encode. Distinct from the frozen core and the drifted gist.
    pub interpretation: InterpretationStamp,
    /// Genealogy of later transformations. Empty on old vaults.
    pub operations: Vec<MemoryOperation>,
    /// Structured claim. The string `core` remains the lexical freeze.
    pub semantic: SemanticCore,
    /// Verifiable claim frozen at encode. Reinterpretation does not rewrite it.
    pub reality: RealityAnchor,
    pub stake_kind: StakeKind,
    pub bearer: Bearer,
    pub loss_kind: LossKind,
    pub stake_mark: String,
    pub absence: Option<AbsenceKind>,
}

impl MemoryTrace {
    pub fn record_operation(&mut self, op: MemoryOperation) {
        self.operations.push(op);
    }

    pub fn clamp(&mut self) {
        self.valence = self.valence.clamp(-1.0, 1.0);
        self.arousal = self.arousal.clamp(0.0, 1.0);
        self.disgust = self.disgust.clamp(0.0, 1.0);
        self.self_relevance = self.self_relevance.clamp(0.0, 1.0);
        self.self_congruence = self.self_congruence.clamp(0.0, 1.0);
        self.fidelity = self.fidelity.clamp(0.0, 1.0);
        self.permanence = self.permanence.clamp(0.0, 1.0);
        self.access = self.access.clamp(0.0, 1.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self.anchor = self.anchor.clamp(0.0, 1.0);
    }

    /// Certainty of the current gist. Access is only how easy the hour is to find.
    pub fn recompute_confidence(&mut self) {
        let mut penalty = 0.0f32;
        let mut confab = false;
        let mut polish = 0u32;
        for d in &self.drifts {
            match d.kind {
                DriftKind::Confabulate => confab = true,
                DriftKind::Embellish | DriftKind::Rewrite => polish += 1,
                _ => {}
            }
        }
        if confab {
            penalty += 0.18;
        }
        penalty += 0.06 * polish.min(4) as f32;
        penalty += 0.05 * self.detach_strikes.min(3) as f32;
        self.confidence = (self.fidelity - penalty).clamp(0.08, 1.0);
    }
}

#[derive(Clone, Debug)]
pub struct ArchiveRecord {
    pub id: String,
    pub verbatim: String,
    pub source: String,
    pub created_at: u64,
    /// Living hour that left the book. Empty while the hour is still there.
    pub released_from: Option<String>,
    pub released_at: Option<u64>,
    /// Binding fact at release. Not the gist. The mouth never reads this.
    pub core: String,
}

#[derive(Clone, Debug)]
pub struct IdentityAxiom {
    pub id: String,
    pub statement: String,
    pub support_trace_ids: Vec<String>,
    pub valence: f32,
    pub strength: f32,
    pub created_at: u64,
    pub superseded_by: Option<String>,
    pub schema: Option<String>,
    pub layer: AxiomLayer,
    pub stake_kind: StakeKind,
    pub bearer: Bearer,
    pub loss_kind: LossKind,
    pub stake_mark: String,
}

/// Prototype of a schema. Peripheral hours of that schema fall toward it.
#[derive(Clone, Debug)]
pub struct SchemaCenter {
    pub schema: String,
    pub core: String,
    pub valence: f32,
    pub weight: f32,
    pub hub_id: Option<String>,
    pub axiom_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Mood {
    pub valence: f32,
    pub arousal: f32,
    pub disgust: f32,
}

impl Default for Mood {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.2,
            disgust: 0.0,
        }
    }
}

impl Mood {
    pub fn blend(&mut self, other: &Mood, weight: f32) {
        let w = weight.clamp(0.0, 1.0);
        self.valence = (1.0 - w) * self.valence + w * other.valence;
        self.arousal = (1.0 - w) * self.arousal + w * other.arousal;
        self.disgust = (1.0 - w) * self.disgust + w * other.disgust;
    }
}

/// Runtime cuts for P0 / P2 ablations. Not persisted. Default is the full organ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrganCut {
    pub reconsolidate: bool,
    pub ground: bool,
    pub ladder: bool,
    /// When false, readout is the stored gist. No reconstruct / write-back.
    pub reconstruct: bool,
    /// Live speak (or `note_spoken`) raises strength on axioms that list the hour.
    pub util_to_strength: bool,
    /// Night merge refuses a pair that supports distinct living axioms, or a pin.
    pub merge_support_veto: bool,
}

impl Default for OrganCut {
    fn default() -> Self {
        Self::full()
    }
}

impl OrganCut {
    pub fn full() -> Self {
        Self {
            reconsolidate: true,
            ground: true,
            ladder: true,
            reconstruct: true,
            util_to_strength: false,
            merge_support_veto: false,
        }
    }

    pub fn static_book() -> Self {
        Self {
            reconsolidate: false,
            ground: false,
            ladder: false,
            reconstruct: false,
            util_to_strength: false,
            merge_support_veto: false,
        }
    }

    pub fn no_recon() -> Self {
        Self {
            reconsolidate: false,
            ..Self::full()
        }
    }

    pub fn no_ground() -> Self {
        Self {
            ground: false,
            ..Self::full()
        }
    }

    pub fn no_ladder() -> Self {
        Self {
            ladder: false,
            ..Self::full()
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RecallTally {
    pub n: u32,
    pub pulled: u32,
    pub reconsolidated: u32,
}

#[derive(Clone, Debug)]
pub struct RecalledMemory {
    pub trace_id: String,
    pub narrative: String,
    pub fidelity: f32,
    pub schema: Option<String>,
    pub channel: Channel,
    pub disclaimer: String,
    pub pulled_toward_core: bool,
    pub reconsolidated: bool,
}
