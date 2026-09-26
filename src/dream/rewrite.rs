//! Second night pass: neighbor retell. A miss vs core is pulled, not written.

use crate::core::model::{now_secs, Channel, DriftEvent, DriftKind, TraceStatus};
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
        let (channel, anchor, schema, embedding, status) = {
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
        if skip_rewrite(store, &id, anchor, schema.as_deref()) {
            continue;
        }
        let neighbors: Vec<crate::core::model::MemoryTrace> = store
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
            .take(3)
            .collect();
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
            if let Some(tr) = store.traces.get_mut(&id) {
                crate::recall::ground::apply_grounding(tr, profile, &text, &core, Some(rewrite));
            }
            continue;
        }
        if let Some(t) = store.traces.get_mut(&id) {
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
            t.clamp();
            rewritten += 1;
            budget -= 1;
        }
    }
    rewritten
}

fn skip_rewrite(
    store: &crate::core::store::MemoryStore,
    id: &str,
    anchor: f32,
    schema: Option<&str>,
) -> bool {
    if anchor >= 0.80 {
        return true;
    }
    if !store.living_axiom_ids_for(id).is_empty() {
        return true;
    }
    let Some(schema) = schema else {
        return false;
    };
    let charged = store.traces.values().filter(|t| {
        t.schema.as_deref() == Some(schema)
            && t.channel == Channel::Selfhood
            && t.self_relevance >= 0.80
            && t.valence.abs() >= 0.40
    }).count();
    charged >= 2
}
