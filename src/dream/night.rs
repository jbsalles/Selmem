//! One night: the five passes, in this order.
//! No LLM is required. A narrator, if present, only rewrites gists.

use crate::core::model::{now_secs, DriftEvent, IdentityAxiom, OrganCut, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::dream::{ladder, merge, release, rewrite, singularite, weather};
use crate::encode::embed::Embedder;
use crate::recall::narrator::Narrator;

/// Scientific order for a deep night. A swap changes the book; unit tests on traces will not see it.
pub const NIGHT_PASSES: &[&str] = &["weather", "rewrite", "merge", "ladder", "release"];

/// Shallow night: weather and release only. No rewrite, merge, or ladder.
pub const SHALLOW_PASSES: &[&str] = &["weather", "release"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NightKind {
    Shallow,
    Deep,
}

impl NightKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shallow => "shallow",
            Self::Deep => "deep",
        }
    }
}

pub struct NightBudget {
    pub new_hours: u32,
    pub charge: f32,
    pub kind: NightKind,
}

pub fn evaluate_budget(store: &MemoryStore, profile: &EntityProfile) -> NightBudget {
    let new_hours = store.hours_since_deep().len() as u32;
    let charge = store.night_charge();
    let kind = if new_hours >= profile.deep_min_hours || charge >= profile.deep_min_charge {
        NightKind::Deep
    } else {
        NightKind::Shallow
    };
    NightBudget {
        new_hours,
        charge,
        kind,
    }
}

pub struct DreamReport {
    pub faded: u32,
    pub cold: u32,
    pub myth: u32,
    pub merged: u32,
    pub extinguished: u32,
    pub weathered: u32,
    pub rewritten: u32,
    pub released: u32,
    pub sculpted: Vec<DriftEvent>,
    pub axioms: Vec<IdentityAxiom>,
    pub kind: NightKind,
    pub new_hours: u32,
    pub charge: f32,
}

pub fn dream(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
) -> DreamReport {
    dream_cut(store, profile, narrator, embedder, OrganCut::full())
}

pub fn dream_cut(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    cut: OrganCut,
) -> DreamReport {
    dream_kind(store, profile, narrator, embedder, cut, NightKind::Deep)
}

pub fn dream_budget(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    cut: OrganCut,
) -> DreamReport {
    let kind = evaluate_budget(store, profile).kind;
    dream_kind(store, profile, narrator, embedder, cut, kind)
}

pub fn dream_kind(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    cut: OrganCut,
    kind: NightKind,
) -> DreamReport {
    let budget = evaluate_budget(store, profile);
    let previously_latent: std::collections::HashSet<String> = store
        .traces
        .values()
        .filter(|t| t.status == TraceStatus::Latent)
        .map(|t| t.id.clone())
        .collect();

    singularite::apply_anchors(store);
    let w = weather::run(store, profile);
    let (rewritten, merged, axioms) = if kind == NightKind::Deep {
        let rewritten = rewrite::run(store, profile, narrator, embedder, cut.ground);
        let merged = merge::run(store, profile, cut.merge_support_veto);
        let axioms = if cut.ladder {
            ladder::run(store, narrator)
        } else {
            Vec::new()
        };
        (rewritten, merged, axioms)
    } else {
        (0, 0, Vec::new())
    };
    let released = release::run(store, &previously_latent);
    singularite::apply_anchors(store);
    if kind == NightKind::Deep {
        store.last_deep_at = Some(now_secs());
        store.pending_night.clear();
    }

    DreamReport {
        faded: w.faded,
        cold: w.cold,
        myth: w.myth,
        merged,
        extinguished: w.extinguished,
        weathered: w.weathered,
        rewritten,
        released,
        sculpted: w.sculpted,
        axioms,
        kind,
        new_hours: budget.new_hours,
        charge: budget.charge,
    }
}
