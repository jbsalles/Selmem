//! One field list for the lived book. File and SQLite only choose a container.
//!
//! Adding a field on `MemoryTrace` means editing `assemble_trace` here.
//! Layout of SELMEM1 and the SQL schema stay the backends' problem.

use crate::core::model::{
    ArchiveRecord, AxiomLayer, Channel, DriftEvent, DriftKind, IdentityAxiom, MemoryTrace, Mood,
    TraceStatus,
};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;

pub struct Snapshot {
    pub profile: EntityProfile,
    pub mood: Mood,
    pub store: MemoryStore,
}

pub fn assemble_trace(
    id: String,
    gist: String,
    core: String,
    valence: f32,
    arousal: f32,
    disgust: f32,
    self_relevance: f32,
    schema: Option<String>,
    channel: Channel,
    archive_id: Option<String>,
    created_at: u64,
    last_recalled_at: Option<u64>,
    last_consolidated_at: Option<u64>,
    fidelity: f32,
    permanence: f32,
    rehearsals: u32,
    access: f32,
    status: TraceStatus,
    salience_at_encode: f32,
    embedding: Vec<f32>,
    anchor: f32,
    detach_strikes: u32,
    cues: Vec<String>,
    drifts: Vec<DriftEvent>,
) -> MemoryTrace {
    MemoryTrace {
        id,
        gist,
        core,
        valence,
        arousal,
        disgust,
        self_relevance,
        schema,
        channel,
        archive_id,
        created_at,
        last_recalled_at,
        last_consolidated_at,
        fidelity,
        permanence,
        rehearsals,
        access,
        status,
        salience_at_encode,
        embedding,
        anchor,
        detach_strikes,
        cues,
        drifts,
    }
}

pub fn assemble_axiom(
    id: String,
    statement: String,
    valence: f32,
    strength: f32,
    created_at: u64,
    superseded_by: Option<String>,
    schema: Option<String>,
    layer: AxiomLayer,
    support_trace_ids: Vec<String>,
) -> IdentityAxiom {
    IdentityAxiom {
        id,
        statement,
        valence,
        strength,
        created_at,
        superseded_by,
        schema,
        layer,
        support_trace_ids,
    }
}

pub fn assemble_archive(
    id: String,
    created_at: u64,
    source: String,
    verbatim: String,
) -> ArchiveRecord {
    ArchiveRecord {
        id,
        created_at,
        source,
        verbatim,
    }
}

pub fn assemble_drift(
    kind: DriftKind,
    at: u64,
    note: String,
    fidelity_delta: f32,
    valence_delta: f32,
    disgust_delta: f32,
) -> DriftEvent {
    DriftEvent {
        kind,
        at,
        note,
        fidelity_delta,
        valence_delta,
        disgust_delta,
    }
}

pub fn assemble_mood(valence: f32, arousal: f32, disgust: f32) -> Mood {
    Mood {
        valence,
        arousal,
        disgust,
    }
}

pub fn profile_params_line(p: &EntityProfile) -> String {
    format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        p.encode_threshold,
        p.w_arousal,
        p.w_novelty,
        p.w_self,
        p.w_utility,
        p.w_goal,
        p.w_redundancy,
        p.decay_lambda,
        p.rehearsal_boost,
        p.embellish_gain,
        p.disgust_gain,
        p.disgust_cap,
        p.fidelity_loss_on_recall,
        p.reconsolidation_eta,
        p.mood_blend,
        p.cold_access,
        p.myth_access,
        p.max_recall,
        p.extinction_rate,
        p.merge_similarity,
        p.ground_min_overlap,
        p.ground_strikes,
        p.narrator_firmness,
        p.deep_min_hours,
        p.deep_min_charge
    )
}

/// Shared defaults when an older vault is missing trailing knobs.
pub fn profile_from_params(name: impl Into<String>, n: &[f32]) -> Option<EntityProfile> {
    if n.len() < 18 {
        return None;
    }
    Some(EntityProfile {
        name: name.into(),
        encode_threshold: n[0],
        w_arousal: n[1],
        w_novelty: n[2],
        w_self: n[3],
        w_utility: n[4],
        w_goal: n[5],
        w_redundancy: n[6],
        decay_lambda: n[7],
        rehearsal_boost: n[8],
        embellish_gain: n[9],
        disgust_gain: n[10],
        disgust_cap: n[11],
        fidelity_loss_on_recall: n[12],
        reconsolidation_eta: n[13],
        mood_blend: n[14],
        cold_access: n[15],
        myth_access: n[16],
        max_recall: n[17] as usize,
        extinction_rate: n.get(18).copied().unwrap_or(0.06),
        merge_similarity: n.get(19).copied().unwrap_or(0.32),
        ground_min_overlap: n.get(20).copied().unwrap_or(0.18),
        ground_strikes: n.get(21).copied().unwrap_or(3.0) as usize,
        narrator_firmness: n.get(22).copied().unwrap_or(0.55),
        deep_min_hours: n.get(23).copied().unwrap_or(1.0) as u32,
        deep_min_charge: n.get(24).copied().unwrap_or(9.0),
        voice: crate::core::profile::Voice::from_gains(n[9], n[10]),
    })
}

pub fn channel_token(c: Channel) -> &'static str {
    match c {
        Channel::Selfhood => "self",
        Channel::World => "world",
        Channel::Log => "log",
    }
}

pub fn status_token(s: TraceStatus) -> &'static str {
    match s {
        TraceStatus::Active => "active",
        TraceStatus::Cold => "cold",
        TraceStatus::Myth => "myth",
        TraceStatus::Latent => "latent",
    }
}

pub fn drift_token(k: DriftKind) -> &'static str {
    match k {
        DriftKind::Embellish => "embellish",
        DriftKind::AmplifyDisgust => "disgust",
        DriftKind::Fade => "fade",
        DriftKind::Merge => "merge",
        DriftKind::Weather => "weather",
        DriftKind::Rewrite => "rewrite",
        DriftKind::Reinterpret => "reinterpret",
        DriftKind::Ground => "ground",
        DriftKind::Color => "color",
    }
}

pub fn layer_token(l: AxiomLayer) -> &'static str {
    match l {
        AxiomLayer::Motif => "motif",
        AxiomLayer::Belief => "belief",
        AxiomLayer::Trait => "trait",
    }
}

pub fn parse_layer_token(s: &str) -> AxiomLayer {
    match s {
        "motif" => AxiomLayer::Motif,
        "trait" => AxiomLayer::Trait,
        _ => AxiomLayer::Belief,
    }
}
