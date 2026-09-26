//! Third night pass: fuse close Selfhood episodes that share a schema.

use std::collections::HashMap;

use crate::core::model::{now_secs, Channel, DriftEvent, DriftKind, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::encode::scoring::lexical_similarity;

pub fn run(store: &mut MemoryStore, profile: &EntityProfile, veto: bool) -> u32 {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for t in store.traces.values() {
        if t.channel != Channel::Selfhood {
            continue;
        }
        if let Some(s) = &t.schema {
            groups.entry(s.clone()).or_default().push(t.id.clone());
        }
    }

    let mut merged = 0u32;
    for (_schema, mut ids) in groups {
        if ids.len() < 2 {
            continue;
        }
        ids.sort();
        let Some(keep) = ids
            .iter()
            .filter_map(|id| store.traces.get(id).map(|t| (id.clone(), merge_weight(t))))
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(id, _)| id)
        else {
            continue;
        };
        for other in ids.iter().filter(|id| *id != &keep) {
            let similar = {
                let a = store.traces.get(&keep);
                let b = store.traces.get(other);
                match (a, b) {
                    (Some(a), Some(b)) => {
                        if b.status == TraceStatus::Myth || (a.anchor >= 0.8 && b.anchor >= 0.8) {
                            false
                        } else if !a.embedding.is_empty() && !b.embedding.is_empty() {
                            crate::encode::embed::cosine(&a.embedding, &b.embedding)
                                >= profile.merge_similarity
                        } else {
                            lexical_similarity(&a.gist, &b.gist) >= profile.merge_similarity
                        }
                    }
                    _ => false,
                }
            };
            if !similar {
                continue;
            }
            if veto && merge_vetoed(store, &keep, other) {
                store.merges_refused = store.merges_refused.saturating_add(1);
                continue;
            }
            let other_clone = store.traces.get(other).cloned();
            let Some(src) = other_clone else { continue };
            if let Some(dst) = store.traces.get_mut(&keep) {
                dst.gist = fuse_gist(&dst.gist, &src.gist);
                dst.core = fuse_core(&dst.core, &src.core);
                dst.valence = (dst.valence + src.valence) / 2.0;
                dst.disgust = dst.disgust.max(src.disgust);
                dst.arousal = dst.arousal.max(src.arousal);
                dst.anchor = dst.anchor.max(src.anchor);
                dst.permanence = dst.permanence.max(src.permanence);
                dst.fidelity = (dst.fidelity.min(src.fidelity) * 0.85).max(0.15);
                dst.self_relevance = dst.self_relevance.max(src.self_relevance);
                for c in src.cues {
                    if !dst.cues.iter().any(|x| x == &c) {
                        dst.cues.push(c);
                    }
                }
                dst.drifts.push(DriftEvent {
                    kind: DriftKind::Merge,
                    at: now_secs(),
                    note: format!("merge of {}", src.id),
                    fidelity_delta: -0.05,
                    valence_delta: 0.0,
                    disgust_delta: 0.0,
                });
                dst.clamp();
            }
            if let Some(src_mut) = store.traces.get_mut(other) {
                src_mut.status = TraceStatus::Myth;
            }
            store.link(&keep, other);
            for axiom in store.axioms.values_mut() {
                let touches = axiom.support_trace_ids.iter().any(|id| id == other);
                if touches && !axiom.support_trace_ids.iter().any(|id| id == &keep) {
                    axiom.support_trace_ids.push(keep.clone());
                }
            }
            merged += 1;
        }
    }
    merged
}

fn merge_vetoed(store: &MemoryStore, keep: &str, other: &str) -> bool {
    let pinned = |id: &str| {
        store
            .traces
            .get(id)
            .map(|t| t.anchor >= 0.85)
            .unwrap_or(false)
    };
    if pinned(keep) || pinned(other) {
        return true;
    }
    let a = store.living_axiom_ids_for(keep);
    let b = store.living_axiom_ids_for(other);
    // Empty vs supported is distinct: a new hour must not fold into a minted family.
    a != b
}

fn merge_weight(trace: &crate::core::model::MemoryTrace) -> (i32, i32, i32, String) {
    // Higher numeric key wins. Text is a content tie-break so two clones
    // that lived the same hours keep the same survivor, not the smaller id.
    let status_penalty = match trace.status {
        TraceStatus::Active => 0,
        TraceStatus::Cold => -1,
        TraceStatus::Latent => -3,
        TraceStatus::Myth => -8,
    };
    let text = if trace.core.is_empty() {
        trace.gist.clone()
    } else {
        trace.core.clone()
    };
    (
        (trace.anchor * 1000.0) as i32 + status_penalty * 1000,
        (trace.permanence * 1000.0) as i32,
        (trace.fidelity * 1000.0) as i32,
        text,
    )
}

fn fuse_core(keeper: &str, absorbed: &str) -> String {
    let keeper = keeper.trim();
    let absorbed = absorbed.trim();
    if absorbed.is_empty() || keeper.contains(absorbed) {
        return keeper.to_string();
    }
    if keeper.is_empty() || absorbed.contains(keeper) {
        return absorbed.to_string();
    }
    // Keeper remains the semantic reference; distinctive words from the
    // absorbed episode stay available for later grounding.
    let extra: Vec<&str> = absorbed
        .split_whitespace()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.chars().count() > 2 && !keeper.to_lowercase().contains(&w.to_lowercase())
        })
        .take(4)
        .collect();
    if extra.is_empty() {
        keeper.to_string()
    } else {
        format!("{keeper} {}", extra.join(" "))
            .chars()
            .take(180)
            .collect()
    }
}

fn fuse_gist(a: &str, b: &str) -> String {
    let left = a.split(" / ").next().unwrap_or(a).trim();
    let right = b.split(" / ").next().unwrap_or(b).trim();
    if left == right {
        format!("{left}, devenu un mythe.")
    } else {
        format!("{left} / {right}")
    }
}
