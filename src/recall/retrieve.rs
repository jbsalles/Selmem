//! Pick a few traces for a query, reconstruct them, then optionally
//! pull a drifted sentence back toward its semantic core.
//!
//! Live recall can write the book (rehearsal, grounding, reconsolidation).
//! Isolated probes use `RecallWrite::ReadOnly` and must not.

use crate::core::model::{now_secs, Mood, OrganCut, RecalledMemory, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::dream::drift::apply_reconsolidation;
use crate::encode::embed::Embedder;
use crate::encode::scoring::{recall_score_emb, refresh_access};
use crate::recall::narrator::Narrator;

/// Whether this recall may mutate traces, access, or mood-facing write-backs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecallWrite {
    Live,
    ReadOnly,
}

/// Probe-only ablation on the marked hour. Live `remember` stays `Observed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecallBias {
    #[default]
    Observed,
    /// Put the first usable marked trace at selected rank 1, even if its score is low.
    ForceMarked,
    /// Never select a marked trace.
    DropMarked,
    /// Drop the marked id, same-schema siblings, and (at speak) derived axioms.
    DropLineage,
}

#[derive(Clone, Debug)]
pub struct ScoredTrace {
    pub trace_id: String,
    pub score: f32,
    pub status: TraceStatus,
    pub base_score: f32,
    pub anchor: f32,
}

impl ScoredTrace {
    pub fn is_eligible(&self) -> bool {
        eligible(self)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RetrievalDump {
    pub candidates: Vec<ScoredTrace>,
    pub selected: Vec<String>,
    pub pulled: u32,
    pub reconsolidated: u32,
    /// Name from the scorer. `null` when none is attached.
    pub scorer: String,
}

impl RetrievalDump {
    pub fn rank_of(&self, id: &str) -> Option<u32> {
        self.candidates
            .iter()
            .position(|c| c.trace_id == id)
            .map(|i| (i + 1) as u32)
    }
}

pub struct RecallOutcome {
    pub memories: Vec<RecalledMemory>,
    pub dump: RetrievalDump,
}

pub fn recall(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    query: &str,
    mood: &Mood,
) -> Vec<RecalledMemory> {
    recall_cut(
        store,
        profile,
        narrator,
        embedder,
        query,
        mood,
        OrganCut::full(),
    )
}

pub fn recall_cut(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    query: &str,
    mood: &Mood,
    cut: OrganCut,
) -> Vec<RecalledMemory> {
    recall_with(
        store,
        profile,
        narrator,
        embedder,
        query,
        mood,
        cut,
        RecallWrite::Live,
        RecallBias::Observed,
        &[],
        &crate::recall::NullScorer,
    )
    .memories
}

pub fn recall_with(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    query: &str,
    mood: &Mood,
    cut: OrganCut,
    write: RecallWrite,
    bias: RecallBias,
    marked: &[String],
    scorer: &dyn crate::recall::PropositionScorer,
) -> RecallOutcome {
    let query_embedding = embedder.embed(query);
    let live = write == RecallWrite::Live;
    let cloud = context_cloud(store, query, &query_embedding);
    let ids: Vec<String> = store.active_ids();
    let mut ranked: Vec<ScoredTrace> = ids
        .into_iter()
        .filter_map(|trace_id| {
            let trace = store.traces.get(&trace_id)?;
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
            let anchor = if trace.attribution == crate::core::model::Attribution::External
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
            // Bonuses require topic evidence independent of the context cloud.
            // A singleton motif cannot manufacture eligibility.
            if anchor > 0.0 && relevance > 0.0 {
                let strength = store
                    .axioms
                    .values()
                    .filter(|a| {
                        a.superseded_by.is_none()
                            && a.layer != crate::core::model::AxiomLayer::Motif
                            && a.support_trace_ids.contains(&trace_id)
                    })
                    .filter(|a| {
                        let observations: std::collections::HashSet<_> = a
                            .support_trace_ids
                            .iter()
                            .filter_map(|id| store.traces.get(id))
                            .map(|t| t.observation_id.as_deref().unwrap_or(&t.id))
                            .collect();
                        observations.len() >= 2
                    })
                    .map(|a| a.strength.clamp(0.0, 1.0))
                    .fold(0.0_f32, f32::max);
                score += 0.12 * strength * relevance * anchor;
                if episode_ask(query) {
                    if trace.channel.verbatim() {
                        score *= 0.32;
                    } else {
                        score += 0.20 * relevance;
                    }
                }
            }
            // Schema names are metadata, not query evidence.
            Some(ScoredTrace {
                trace_id,
                score,
                status: trace.status,
                base_score,
                anchor,
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

    let eligible: Vec<String> = ranked
        .iter()
        .filter(|c| eligible(c))
        .map(|c| c.trace_id.clone())
        .collect();

    let chosen_ids = apply_bias(&eligible, store, bias, marked, profile.max_recall);

    let mut recalled = Vec::new();
    let mut pulled_n = 0u32;
    let mut recon_n = 0u32;
    for trace_id in &chosen_ids {
        let (channel, gist, schema) = {
            let trace = store.traces.get(trace_id).unwrap();
            (trace.channel, trace.gist.clone(), trace.schema.clone())
        };

        let (narrative, disclaimer, fidelity, pulled, reconsolidated) = if channel.verbatim() {
            (
                gist,
                "verbatim record, not distorted".to_string(),
                store.traces[trace_id].fidelity,
                false,
                false,
            )
        } else {
            speak_self(
                store, profile, narrator, query, mood, cut, write, trace_id, scorer,
            )
        };

        if live {
            let operational = store
                .traces
                .get(trace_id)
                .map(|t| t.channel.verbatim() && t.source != "talk")
                .unwrap_or(false);
            if let Some(trace) = store.traces.get_mut(trace_id) {
                if operational {
                    trace.access = 1.0;
                } else {
                    refresh_access(trace, profile);
                }
                trace.last_recalled_at = Some(now_secs());
            }
        }
        if pulled {
            pulled_n += 1;
        }
        if reconsolidated {
            recon_n += 1;
        }
        recalled.push(RecalledMemory {
            trace_id: trace_id.clone(),
            narrative,
            fidelity,
            schema,
            channel,
            disclaimer,
            pulled_toward_core: pulled,
            reconsolidated,
        });
    }
    RecallOutcome {
        memories: recalled,
        dump: RetrievalDump {
            candidates: ranked,
            selected: chosen_ids,
            pulled: pulled_n,
            reconsolidated: recon_n,
            scorer: scorer.name().to_string(),
        },
    }
}

fn eligible(c: &ScoredTrace) -> bool {
    if matches!(c.status, TraceStatus::Latent) {
        return false;
    }
    if matches!(c.status, TraceStatus::Cold) && c.score < 0.12 {
        return false;
    }
    c.score >= 0.08
}

fn apply_bias(
    eligible: &[String],
    store: &MemoryStore,
    bias: RecallBias,
    marked: &[String],
    cap: usize,
) -> Vec<String> {
    if cap == 0 {
        return Vec::new();
    }
    match bias {
        RecallBias::Observed => eligible.iter().take(cap).cloned().collect(),
        RecallBias::DropMarked => eligible
            .iter()
            .filter(|id| !marked.iter().any(|m| m == *id))
            .take(cap)
            .cloned()
            .collect(),
        RecallBias::DropLineage => {
            let banned = lineage_of(store, marked);
            eligible
                .iter()
                .filter(|id| !banned.iter().any(|b| b == *id))
                .take(cap)
                .cloned()
                .collect()
        }
        RecallBias::ForceMarked => {
            let forced = marked.iter().find(|id| can_force(store, id)).cloned();
            let mut out = Vec::new();
            if let Some(id) = forced {
                out.push(id);
            }
            for id in eligible {
                if out.len() >= cap {
                    break;
                }
                if out.iter().any(|x| x == id) {
                    continue;
                }
                out.push(id.clone());
            }
            out
        }
    }
}

fn can_force(store: &MemoryStore, id: &str) -> bool {
    match store.traces.get(id) {
        Some(t) if !matches!(t.status, TraceStatus::Latent) => true,
        _ => false,
    }
}

/// Marked ids, same-schema siblings, merge edges, and axiom supports that touch the set.
pub fn lineage_of(store: &MemoryStore, marked: &[String]) -> Vec<String> {
    store.lineage(marked)
}

pub fn axiom_supported_by_lineage(
    support: &[String],
    schema: Option<&str>,
    lineage: &[String],
    lineage_schemas: &[String],
) -> bool {
    if support.iter().any(|id| lineage.iter().any(|f| f == id)) {
        return true;
    }
    schema
        .map(|s| lineage_schemas.iter().any(|x| x == s))
        .unwrap_or(false)
}

fn speak_self(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    query: &str,
    mood: &Mood,
    cut: OrganCut,
    write: RecallWrite,
    trace_id: &str,
    scorer: &dyn crate::recall::PropositionScorer,
) -> (String, String, f32, bool, bool) {
    let live = write == RecallWrite::Live;
    if !cut.reconstruct {
        let t = store.traces.get(trace_id).unwrap();
        return (
            t.gist.clone(),
            format!("stored gist (fidelity {:.2})", t.fidelity),
            t.fidelity,
            false,
            false,
        );
    }

    let generated = {
        let trace = store.traces.get(trace_id).unwrap();
        narrator.reconstruct(trace, mood, query)
    };

    if !live {
        let fidelity = store.traces[trace_id].fidelity;
        return (
            generated,
            format!("lived account (fidelity {fidelity:.2})"),
            fidelity,
            false,
            false,
        );
    }

    let core = store.traces.get(trace_id).unwrap().core.clone();

    if !cut.ground {
        let reconsolidated = if cut.reconsolidate {
            if let Some(trace) = store.traces.get_mut(trace_id) {
                apply_reconsolidation(trace, &generated, profile, mood.valence);
            }
            true
        } else {
            false
        };
        let fidelity = store.traces[trace_id].fidelity;
        return (
            generated,
            format!("lived account (fidelity {fidelity:.2})"),
            fidelity,
            false,
            reconsolidated,
        );
    }

    let narrator_rewrite = {
        let trace = store.traces.get(trace_id).unwrap();
        if crate::recall::ground::should_force_core_rewrite(
            trace, profile, &generated, &core, scorer,
        ) {
            Some(narrator.recontextualize(trace, &core, profile))
        } else {
            None
        }
    };
    let outcome = {
        let trace = store.traces.get_mut(trace_id).unwrap();
        crate::recall::ground::apply_grounding(
            trace,
            profile,
            &generated,
            &core,
            narrator_rewrite,
            scorer,
        )
    };
    let reconsolidated = if !outcome.pulled_toward_core && !outcome.unjudged && cut.reconsolidate {
        if let Some(trace) = store.traces.get_mut(trace_id) {
            apply_reconsolidation(trace, &outcome.spoken_text, profile, mood.valence);
        }
        true
    } else {
        false
    };
    let trace = store.traces.get(trace_id).unwrap();
    let disclaimer = if outcome.pulled_toward_core {
        "pulled back toward the core".to_string()
    } else {
        format!("lived account (fidelity {:.2})", trace.fidelity)
    };
    (
        outcome.spoken_text,
        disclaimer,
        trace.fidelity,
        outcome.pulled_toward_core,
        reconsolidated,
    )
}

fn episode_ask(query: &str) -> bool {
    let q = query.to_ascii_lowercase();
    q.contains("what happened")
        || q.contains("what effect")
        || q.contains("how did")
        || q.contains("that day")
        || q.contains("this day")
        || q.contains("how did you feel")
        || q.contains("how sure")
        || q.contains("do you remember")
        || q.contains("raconte")
        || q.contains("ce jour")
}

/// Query seeds the cloud. Book links add hours: merge edges and axiom co-supports.
/// A same-schema label does not. One shared token is not an anchor.
pub fn context_cloud_pub(
    store: &crate::core::store::MemoryStore,
    query: &str,
) -> std::collections::HashMap<String, f32> {
    context_cloud(store, query, &[])
}

fn context_cloud(
    store: &crate::core::store::MemoryStore,
    query: &str,
    query_emb: &[f32],
) -> std::collections::HashMap<String, f32> {
    use crate::encode::scoring::{behavior_weight, lexical_similarity, token_set};
    let mut weights: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    let mut ids = store.active_ids();
    ids.sort();
    let add = |weights: &mut std::collections::HashMap<String, f32>, text: &str, w: f32| {
        if w <= 0.0 {
            return;
        }
        for tok in token_set(text) {
            *weights.entry(tok).or_insert(0.0) += w;
        }
    };
    let mut seeds: Vec<String> = Vec::new();
    for id in &ids {
        let Some(t) = store.traces.get(id) else {
            continue;
        };
        if t.suppressed || t.status == TraceStatus::Latent {
            continue;
        }
        let mut sim = lexical_similarity(query, &t.gist);
        sim = sim.max(lexical_similarity(query, &t.core));
        for cue in &t.cues {
            sim = sim.max(lexical_similarity(query, cue));
        }
        if !query_emb.is_empty() && !t.embedding.is_empty() {
            sim = sim.max(crate::encode::embed::cosine(query_emb, &t.embedding));
        }
        if sim <= 0.0 {
            continue;
        }
        seeds.push(id.clone());
        let w = sim * behavior_weight(t) * t.self_relevance.max(0.05) * t.access.max(0.05);
        add(&mut weights, &t.gist, w);
        add(&mut weights, &t.core, w * 0.5);
    }
    let mut linked: Vec<String> = Vec::new();
    for id in &seeds {
        let seed_schema = store.traces.get(id).and_then(|t| t.schema.clone());
        if let Some(neigh) = store.edges.get(id) {
            for n in neigh {
                let same = seed_schema.as_ref().is_some()
                    && store.traces.get(n).and_then(|t| t.schema.as_ref()) == seed_schema.as_ref();
                if !same {
                    linked.push(n.clone());
                }
            }
        }
        for a in store.axioms.values() {
            if a.superseded_by.is_some() {
                continue;
            }
            if a.support_trace_ids.iter().any(|s| s == id) {
                linked.extend(a.support_trace_ids.iter().cloned());
            }
        }
    }
    linked.sort();
    linked.dedup();
    for id in linked {
        if seeds.iter().any(|s| s == &id) {
            continue;
        }
        let Some(t) = store.traces.get(&id) else {
            continue;
        };
        if t.suppressed || t.status == TraceStatus::Latent {
            continue;
        }
        let w = 0.5 * behavior_weight(t) * t.self_relevance.max(0.05) * t.access.max(0.05);
        add(&mut weights, &t.gist, w);
        add(&mut weights, &t.core, w * 0.5);
    }
    weights
}

/// Weighted share of the statement's own tokens. No half-cut, no minimum count.
pub fn statement_share(cloud: &std::collections::HashMap<String, f32>, statement: &str) -> f32 {
    token_coverage(cloud, &crate::encode::scoring::token_set(statement))
}

pub fn statement_anchored(cloud: &std::collections::HashMap<String, f32>, statement: &str) -> bool {
    statement_share(cloud, statement) > 0.0
}

fn token_coverage(cloud: &std::collections::HashMap<String, f32>, tokens: &[String]) -> f32 {
    if cloud.is_empty() || tokens.is_empty() {
        return 0.0;
    }
    let scale = cloud.values().copied().fold(0.0_f32, f32::max);
    if scale <= 0.0 {
        return 0.0;
    }
    let hit: f32 = tokens
        .iter()
        .map(|t| cloud.get(t).copied().unwrap_or(0.0))
        .sum();
    (hit / (tokens.len() as f32 * scale)).clamp(0.0, 1.0)
}

fn cloud_anchor(
    cloud: &std::collections::HashMap<String, f32>,
    trace: &crate::core::model::MemoryTrace,
) -> f32 {
    let mut tokens = crate::encode::scoring::token_set(&trace.gist);
    tokens.extend(crate::encode::scoring::token_set(&trace.core));
    tokens.sort();
    tokens.dedup();
    token_coverage(cloud, &tokens)
}

/// Query-topic overlap, excluding task boilerplate and the observed speaker.
/// This lexical check deliberately does not trust hash collisions.
pub fn topic_relevance(query: &str, core: &str) -> f32 {
    let subject = core.split_once(" said:").map(|(s, _)| s.to_lowercase());
    let tokens = |text: &str| {
        crate::encode::scoring::token_set(text)
            .into_iter()
            .filter(|w| {
                !matches!(
                    w.as_str(),
                    "the"
                        | "and"
                        | "that"
                        | "this"
                        | "with"
                        | "have"
                        | "has"
                        | "had"
                        | "was"
                        | "were"
                        | "been"
                        | "said"
                        | "what"
                        | "how"
                        | "who"
                        | "when"
                        | "where"
                        | "why"
                        | "did"
                        | "does"
                        | "from"
                        | "for"
                        | "her"
                        | "his"
                        | "she"
                        | "him"
                        | "they"
                        | "them"
                        | "their"
                        | "you"
                        | "your"
                        | "our"
                        | "are"
                        | "but"
                        | "not"
                        | "will"
                        | "would"
                        | "should"
                        | "can"
                        | "could"
                        | "one"
                        | "more"
                        | "any"
                        | "some"
                        | "into"
                        | "according"
                        | "observed"
                        | "conversation"
                        | "conversations"
                        | "answer"
                        | "words"
                        | "say"
                        | "know"
                        | "please"
                        | "recommend"
                        | "concrete"
                        | "step"
                        | "safeguard"
                        | "fallback"
                        | "state"
                        | "benefit"
                ) && subject
                    .as_ref()
                    .map_or(true, |s| !s.split_whitespace().any(|p| p == w))
            })
            .collect::<Vec<_>>()
    };
    let q = tokens(query);
    let c = tokens(core);
    if q.is_empty() || c.is_empty() {
        return 0.0;
    }
    let hits = q.iter().filter(|t| c.contains(t)).count();
    hits as f32 / q.len().min(c.len()) as f32
}
