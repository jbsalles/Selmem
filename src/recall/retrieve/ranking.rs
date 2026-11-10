//! Read-only candidate ranking and precomputed axiom support.
use super::*;

// Each axiom's independent observation count is checked once per recall,
// rather than being recomputed for every candidate episode.
fn axiom_support_strengths(store: &MemoryStore) -> std::collections::HashMap<&str, f32> {
    use crate::core::model::AxiomLayer;
    use std::collections::{HashMap, HashSet};
    let mut strengths = HashMap::<&str, f32>::new();
    for axiom in store
        .axioms
        .values()
        .filter(|a| store.axiom_available(a) && a.layer != AxiomLayer::Motif)
    {
        let observations: HashSet<_> = axiom
            .support_trace_ids
            .iter()
            .filter_map(|id| store.traces.get(id))
            .map(|trace| trace.observation_id.as_deref().unwrap_or(&trace.id))
            .collect();
        if observations.len() < 2 {
            continue;
        }
        let strength = axiom.strength.clamp(0.0, 1.0);
        for trace_id in &axiom.support_trace_ids {
            let current = strengths.entry(trace_id.as_str()).or_insert(0.0);
            *current = current.max(strength);
        }
    }
    strengths
}

pub(super) fn rank_candidates(
    store: &MemoryStore,
    profile: &EntityProfile,
    embedder: &dyn Embedder,
    query: &str,
    mood: &Mood,
    cut: OrganCut,
    bias: RecallBias,
) -> Vec<ScoredTrace> {
    let query_embedding = if store.embedding_space.as_deref() == Some(embedder.space_id().as_str())
    {
        embedder.embed(query)
    } else {
        Vec::new()
    };
    let transfer_query = transfer_terms(query);
    let is_episode_query = episode_ask(query);
    let axiom_strength = axiom_support_strengths(store);
    let cloud = context_cloud(store, query, &query_embedding);
    let ids: Vec<String> = store.active_ids();
    let mut ranked: Vec<ScoredTrace> = ids
        .into_iter()
        .filter_map(|trace_id| {
            let trace = store.traces.get(&trace_id)?;
            if trace.personal.role != crate::SelfRole::Episode {
                return None;
            }
            if trace.suppressed && !matches!(bias, RecallBias::ForceMarked) {
                return None;
            }
            let talk = trace.source == "talk";
            let access = crate::encode::scoring::access_value(trace, profile);
            if talk && access < 0.10 {
                return None;
            }
            let mut score = recall_score_emb(trace, query, Some(&query_embedding), mood, profile);
            // Operational facts have no identity/affect/recency dependency.
            // Require lexical relevance; never give unrelated World records a floor.
            if trace.channel.verbatim() && !talk {
                let sim = crate::encode::scoring::lexical_similarity(query, &trace.gist).max(
                    crate::encode::scoring::lexical_similarity(query, &trace.core),
                );
                if sim <= 0.0 {
                    return None;
                }
                score = 0.75 * sim + 0.25;
            }
            let relevance = topic_relevance(query, &trace.core);
            let base_score = score;
            let transfer = if crate::recall::interpretation::subject_matches(store, trace, query) {
                crate::recall::interpretation::for_query(trace, query)
            } else {
                None
            };
            let mut anchor = if trace.attribution == crate::core::model::Attribution::External
                && relevance <= 0.0
            {
                0.0
            } else if trace.channel.verbatim() && !talk {
                1.0
            } else {
                cloud_anchor(&cloud, trace)
            };
            if anchor <= 0.0 {
                score = 0.0;
            } else {
                score *= anchor;
            }
            // Reconstructive support signal: modest score boost from distinct
            // observation counts. Lesionable via OrganCut::support_signal.
            // Never rewrites core. Zero when signal is off or weight is zero.
            if cut.support_signal && anchor > 0.0 {
                let w = store.observation_evidence.support_weight(store, &trace_id);
                if w > 0.0 {
                    score *= 1.0 + 0.22 * w;
                }
            }
            // An explicit cue can evoke a deliberately retained own episode
            // even after a long unused interval. Recency still ranks diffuse
            // associations; it must not erase a precise, accessible match.
            let (cue_hits, _) = topic_overlap(query, &trace.core);
            if anchor > 0.0
                && trace.permanence >= 0.8
                && trace.anchor >= 0.8
                && trace.attribution != crate::core::model::Attribution::External
                && cue_hits >= 2
                && relevance >= 0.5
            {
                // Access already includes fidelity. Multiplying it again
                // would penalize detail loss twice, although the core is frozen.
                score = score.max(0.5 * relevance * access);
            }
            if let Some(i) = &transfer {
                // Evidence confidence and retained accessibility determine the
                // score. Keep the normal .08/.12 gates; no eligibility floor.
                score = score.max(0.65 * i.confidence * trace.fidelity * access);
            }
            // Bonuses require topic evidence independent of the context cloud.
            // A singleton motif cannot manufacture eligibility.
            if anchor > 0.0 && relevance > 0.0 {
                let strength = axiom_strength
                    .get(trace_id.as_str())
                    .copied()
                    .unwrap_or(0.0);
                score += 0.12 * strength * relevance * anchor;
                if is_episode_query {
                    if trace.channel.verbatim() {
                        score *= 0.32;
                    } else {
                        score += 0.20 * relevance;
                    }
                }
            }
            if bias == RecallBias::Transfer {
                // Analogy needs topical content, not the identity of the old speaker.
                // Hash collisions, timestamps and affect alone cannot license a source.
                let content = crate::encode::scoring::event_content;
                let relevant =
                    topic_relevance(&transfer_query, &transfer_terms(content(&trace.core))).max(
                        topic_relevance(&transfer_query, &transfer_terms(content(&trace.gist))),
                    );
                anchor = relevant;
                score = if relevant > 0.0 {
                    (0.75 * relevant + 0.25 * base_score.clamp(0.0, 1.0)) * access
                } else {
                    0.0
                };
            }
            // Schema names are metadata, not query evidence.
            if bias != RecallBias::Transfer
                && trace.attribution == crate::core::model::Attribution::External
                && (!crate::recall::interpretation::subject_matches(store, trace, query)
                    || !crate::recall::interpretation::factual_relation_matches(query, &trace.core))
            {
                score = 0.0;
            }
            Some(ScoredTrace {
                trace_id,
                score,
                status: trace.status,
                base_score,
                anchor: transfer
                    .as_ref()
                    .map_or(anchor, |i| anchor.max(i.confidence)),
            })
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ta = &store.traces[&a.trace_id];
                let tb = &store.traces[&b.trace_id];
                ta.core
                    .cmp(&tb.core)
                    .then(ta.gist.cmp(&tb.gist))
                    .then_with(|| ta.valence.total_cmp(&tb.valence))
                    .then(ta.created_at.cmp(&tb.created_at))
                    .then(a.trace_id.cmp(&b.trace_id))
            })
    });

    ranked
}
