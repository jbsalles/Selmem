//! Second night pass: neighbor retell. A miss vs core is pulled, not written.

use crate::core::model::{
    now_secs, Attribution, Channel, DriftEvent, DriftKind, MemoryTrace, TraceStatus,
};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::encode::embed::Embedder;
use crate::recall::narrator::Narrator;

pub fn run(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    ground: bool,
) -> u32 {
    let ids = store.active_ids();
    let mut rewritten = 0u32;
    let mut budget = 6u32;
    for id in ids {
        if budget == 0 {
            break;
        }
        let (channel, _anchor, schema, embedding, status) = {
            let Some(t) = store.traces.get(&id) else { continue };
            (
                t.channel,
                t.anchor,
                t.schema.clone(),
                t.embedding.clone(),
                t.status,
            )
        };
        if channel.verbatim() || status == TraceStatus::Latent {
            continue;
        }
        if skip_rewrite(store, &id) {
            continue;
        }
        let mut neighbors: Vec<crate::core::model::MemoryTrace> = store
            .traces
            .values()
            .filter(|o| o.id != id && o.channel == Channel::Selfhood)
            .filter(|o| {
                if let (Some(a), Some(b)) = (schema.as_ref(), o.schema.as_ref()) {
                    if a == b {
                        return true;
                    }
                }
                !embedding.is_empty()
                    && !o.embedding.is_empty()
                    && crate::encode::embed::cosine(&embedding, &o.embedding) >= profile.merge_similarity
            })
            .cloned()
            .collect();
        neighbors.sort_by(|a, b| a.id.cmp(&b.id));
        neighbors.truncate(3);
        if let Some(s) = schema.as_ref() {
            if let Some(c) = store.centers.get(s) {
                if let Some(hub) = c
                    .hub_id
                    .as_ref()
                    .and_then(|hid| store.traces.get(hid))
                    .cloned()
                {
                    if hub.id != id && !neighbors.iter().any(|n| n.id == hub.id) {
                        neighbors.insert(0, hub);
                    }
                }
            }
        }
        let neighbor_refs: Vec<&crate::core::model::MemoryTrace> = neighbors.iter().collect();
        let Some(t) = store.traces.get(&id) else { continue };
        let Some(text) = narrator.rewrite(t, &neighbor_refs, profile) else { continue };
        if text.trim().is_empty() || text == t.gist {
            continue;
        }
        let core = t.core.clone();
        if ground
            && crate::recall::ground::is_grounding_miss(&text, &core, profile.ground_min_overlap)
        {
            let rewrite = narrator.recontextualize(t, &core, profile);
            let before = t.gist.clone();
            if let Some(tr) = store.traces.get_mut(&id) {
                let outcome = crate::recall::ground::apply_grounding(
                    tr,
                    profile,
                    &text,
                    &core,
                    Some(rewrite),
                );
                if outcome.pulled_toward_core && tr.gist != before {
                    record_rewrite(tr, &neighbors, before);
                    rewritten += 1;
                    budget -= 1;
                }
            }
            continue;
        }
        if let Some(t) = store.traces.get_mut(&id) {
            let before = t.gist.clone();
            t.gist = text.chars().take(280).collect();
            t.embedding = embedder.embed(&t.gist);
            t.drifts.push(DriftEvent {
                kind: DriftKind::Rewrite,
                at: now_secs(),
                note: "consolidation narrative".into(),
                fidelity_delta: -0.02 * (1.0 - t.anchor),
                valence_delta: 0.0,
                disgust_delta: 0.0,
            });
            t.fidelity = (t.fidelity - 0.02 * (1.0 - t.anchor)).max(0.15);
            record_rewrite(t, &neighbors, before);
            t.recompute_confidence();
            t.clamp();
            rewritten += 1;
            budget -= 1;
        }
    }
    rewritten
}

fn record_rewrite(t: &mut MemoryTrace, neighbors: &[MemoryTrace], before: String) {
    t.record_operation(crate::core::model::MemoryOperation {
        kind: "rewrite".into(),
        at: now_secs(),
        source_trace_ids: neighbors.iter().map(|n| n.id.clone()).collect(),
        source_axiom_ids: Vec::new(),
        source_center: t.schema.clone(),
        before,
        after: t.gist.clone(),
        confidence: t.confidence,
        origin: crate::core::model::EvidenceOrigin::Reconstruction,
    });
}

pub const CONFLICT_CONGRUENCE: f32 = 0.40;

/// Internal hour that cannot sit in living identity without a rewrite.
pub fn is_conflict(store: &MemoryStore, trace: &MemoryTrace) -> bool {
    if trace.attribution != Attribution::Internal {
        return false;
    }
    crate::encode::measure_congruence(store, trace.schema.as_deref(), trace.valence)
        < CONFLICT_CONGRUENCE
}

/// Weather may drop unused detail. `true` = do not fade the gist wording.
pub fn hold_gist_text(trace: &MemoryTrace, conflict: bool) -> bool {
    match trace.attribution {
        Attribution::External => true,
        Attribution::Internal => !conflict,
        Attribution::None => false,
    }
}

/// Sculpt retell / token substitution. External preserves. None only fades.
pub fn allow_retell(trace: &MemoryTrace, conflict: bool) -> bool {
    matches!(trace.attribution, Attribution::Internal) && conflict
}

pub fn skip_rewrite(store: &MemoryStore, id: &str) -> bool {
    let Some(t) = store.traces.get(id) else {
        return true;
    };
    if t.channel.verbatim() || t.status == TraceStatus::Latent {
        return true;
    }
    match t.attribution {
        Attribution::External => true,
        Attribution::Internal => !is_conflict(store, t),
        Attribution::None => skip_rewrite_legacy(store, t),
    }
}

fn skip_rewrite_legacy(store: &MemoryStore, t: &MemoryTrace) -> bool {
    let charged = t.self_relevance >= 0.80 && t.valence.abs() >= 0.40;
    if charged || t.permanence >= 0.92 || t.anchor >= 0.80 {
        return true;
    }
    if !store.living_axiom_ids_for(&t.id).is_empty() {
        return true;
    }
    let Some(schema) = t.schema.as_deref() else {
        return false;
    };
    let charged_n = store
        .traces
        .values()
        .filter(|o| {
            o.schema.as_deref() == Some(schema)
                && o.channel == Channel::Selfhood
                && o.self_relevance >= 0.80
                && o.valence.abs() >= 0.40
        })
        .count();
    charged_n >= 2
}
