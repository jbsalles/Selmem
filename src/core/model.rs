use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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
}

impl MemoryTrace {
    pub fn clamp(&mut self) {
        self.valence = self.valence.clamp(-1.0, 1.0);
        self.arousal = self.arousal.clamp(0.0, 1.0);
        self.disgust = self.disgust.clamp(0.0, 1.0);
        self.self_relevance = self.self_relevance.clamp(0.0, 1.0);
        self.fidelity = self.fidelity.clamp(0.0, 1.0);
        self.permanence = self.permanence.clamp(0.0, 1.0);
        self.access = self.access.clamp(0.0, 1.0);
        self.anchor = self.anchor.clamp(0.0, 1.0);
    }
}

#[derive(Clone, Debug)]
pub struct ArchiveRecord {
    pub id: String,
    pub verbatim: String,
    pub source: String,
    pub created_at: u64,
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
