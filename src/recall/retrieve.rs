//! Pick a few traces for a query, reconstruct them, then optionally
//! pull a drifted sentence back toward its semantic core.
//!
//! Live recall can write the book (rehearsal, grounding, reconsolidation).
//! Isolated probes use `RecallWrite::ReadOnly` and must not.

use crate::core::association::{AssociationChange, AssociativeCue};
use crate::core::model::{now_secs, Mood, OrganCut, RecalledMemory, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::dream::drift::apply_reconsolidation;
use crate::encode::embed::Embedder;
use crate::encode::scoring::{recall_score_emb, refresh_access};
use crate::recall::narrator::Narrator;

#[cfg(test)]
#[path = "dependency_tests.rs"]
mod dependency_selection_tests;

#[derive(Clone, Debug)]
pub struct AssociationUse {
    pub target: String,
    pub cues: Vec<AssociativeCue>,
    pub accepted: bool,
}

/// Whether this recall may mutate traces, access, or mood-facing write-backs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecallWrite {
    Live,
    ReadOnly,
}

/// Explicit retrieval intent and probe ablations. Ordinary recall stays `Observed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecallBias {
    #[default]
    Observed,
    /// Retrieve topical source experiences for analogy, not claims about a named subject.
    /// This explicit intent never forces an unrelated trace above the recall gate.
    Transfer,
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
    pub associations_used: Vec<AssociationUse>,
    pub associations_reinforced: Vec<AssociationChange>,
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

mod pipeline;
mod ranking;
mod selection;
pub use pipeline::recall_with;
use ranking::rank_candidates;
use selection::choose_trace_ids;

fn eligible(c: &ScoredTrace) -> bool {
    if matches!(c.status, TraceStatus::Latent) {
        return false;
    }
    if matches!(c.status, TraceStatus::Cold) && c.score < 0.12 {
        return false;
    }
    c.score >= 0.08
}

/// Task-writing instructions and conversational encouragement are not source topics.
fn transfer_terms(text: &str) -> String {
    crate::encode::scoring::token_set(text)
        .into_iter()
        .filter(|word| {
            topic_relevance(word, word) > 0.0
                && !matches!(
                    word.as_str(),
                    "design"
                        | "original"
                        | "propose"
                        | "describe"
                        | "explain"
                        | "include"
                        | "maximum"
                        | "must"
                        | "within"
                        | "require"
                        | "requires"
                        | "using"
                        | "use"
                        | "used"
                        | "without"
                        | "every"
                        | "each"
                        | "all"
                        | "only"
                        | "help"
                        | "helps"
                        | "make"
                        | "makes"
                        | "made"
                        | "great"
                        | "good"
                        | "thanks"
                        | "thank"
                        | "yeah"
                        | "really"
                        | "much"
                        | "lot"
                        | "going"
                        | "get"
                        | "got"
                        | "way"
                        | "well"
                        | "sure"
                        | "something"
                        | "anything"
                        | "success"
                        | "successful"
                        | "work"
                        | "working"
                        | "works"
                        | "want"
                        | "wants"
                        | "need"
                        | "needs"
                        | "needed"
                        | "new"
                        | "two"
                        | "three"
                        | "six"
                        | "twelve"
                        | "twenty"
                        | "one"
                        | "its"
                )
                && !word.chars().all(|c| c.is_numeric())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Marginal relevance penalizes redundant content only among eligible traces.
fn select_transfer(ranked: &[ScoredTrace], store: &MemoryStore, cap: usize) -> Vec<String> {
    let mut chosen: Vec<String> = Vec::new();
    while chosen.len() < cap {
        let best = ranked
            .iter()
            .filter(|c| c.is_eligible() && !chosen.contains(&c.trace_id))
            .map(|c| {
                let content = crate::encode::scoring::event_content;
                let text = content(&store.traces[&c.trace_id].core);
                let redundancy = chosen
                    .iter()
                    .map(|id| {
                        crate::encode::scoring::lexical_similarity(
                            text,
                            content(&store.traces[id].core),
                        )
                    })
                    .fold(0.0_f32, f32::max);
                (c, c.score * (1.0 - 0.5 * redundancy))
            })
            .max_by(|(a, x), (b, y)| x.total_cmp(y).then_with(|| b.trace_id.cmp(&a.trace_id)));
        let Some((next, _)) = best else { break };
        chosen.push(next.trace_id.clone());
    }
    chosen
}

/// Penalize repeated content from the same source without deleting evidence.
/// Eligibility and ablation filters are applied first. Scores in the audit remain
/// the original relevance scores; selected IDs record the subsequent selection.
#[cfg(test)]
fn select_diverse(
    ranked: &[ScoredTrace],
    allowed: &[String],
    store: &MemoryStore,
    cap: usize,
) -> Vec<String> {
    select_diverse_for_query(ranked, allowed, store, cap, "")
}

fn select_diverse_for_query(
    ranked: &[ScoredTrace],
    allowed: &[String],
    store: &MemoryStore,
    cap: usize,
    query: &str,
) -> Vec<String> {
    use super::dependency::{conflicts, record, same_provenance};
    let allowed: std::collections::HashSet<&str> = allowed.iter().map(String::as_str).collect();
    let records: std::collections::HashMap<_, _> = ranked
        .iter()
        .filter_map(|c| record(&store.traces[&c.trace_id]).map(|r| (c.trace_id.as_str(), r)))
        .collect();
    let mut chosen: Vec<String> = Vec::new();
    while chosen.len() < cap {
        let mut best: Option<(&ScoredTrace, f32, Option<String>)> = None;
        for candidate in ranked {
            if !allowed.contains(candidate.trace_id.as_str())
                || chosen.contains(&candidate.trace_id)
            {
                continue;
            }
            let trace = &store.traces[&candidate.trace_id];
            let mut parent_id = None;
            if let Some(record) = records.get(candidate.trace_id.as_str()) {
                if conflicts(record, query) {
                    continue;
                }
                if let Some(parent_version) = &record.parent {
                    let parents: Vec<_> = records
                        .iter()
                        .filter(|(id, other)| {
                            other.version == *parent_version
                                && other.issuer == record.issuer
                                && other.scope == record.scope
                                && other.parent.is_none()
                                && same_provenance(trace, &store.traces[**id])
                                && store.traces[**id].created_at <= trace.created_at
                        })
                        .map(|(id, _)| *id)
                        .collect();
                    // Missing, ambiguous, or ablated parents cannot be supplied
                    // by bypassing the eligibility gate. Nested chains abstain.
                    if parents.len() != 1 || !allowed.contains(parents[0]) {
                        continue;
                    }
                    if !chosen.iter().any(|id| id == parents[0]) {
                        if chosen.len() + 2 > cap {
                            continue;
                        }
                        parent_id = Some(parents[0].to_string());
                    }
                }
            }
            let redundancy = chosen
                .iter()
                .map(|id| {
                    let previous = &store.traces[id];
                    if trace.source != previous.source
                        || trace.attribution != previous.attribution
                        || trace.channel != previous.channel
                    {
                        return 0.0;
                    }
                    let similarity = crate::encode::scoring::lexical_similarity;
                    // Both representations must overlap; a short shared core alone
                    // must not erase a distinction retained in the lived narrative.
                    similarity(&trace.gist, &previous.gist)
                        .min(similarity(&trace.core, &previous.core))
                })
                .fold(0.0_f32, f32::max);
            let adjusted = candidate.score * (1.0 - redundancy);
            // Keep the existing deterministic ranking as the tie breaker.
            if best
                .as_ref()
                .map_or(true, |(_, score, _)| adjusted > *score)
            {
                best = Some((candidate, adjusted, parent_id));
            }
        }
        let Some((candidate, _, parent)) = best else {
            break;
        };
        if let Some(parent) = parent {
            chosen.push(parent);
        }
        chosen.push(candidate.trace_id.clone());
    }
    chosen
}

#[cfg(test)]
mod diversity_tests {
    use super::*;

    fn fixture() -> (MemoryStore, Vec<ScoredTrace>, Vec<String>) {
        let mut memory = crate::SelectiveMemory::new(EntityProfile::new("observer"));
        let mut input = crate::EncodeInput::new("Workshop volunteers repair broken tools.");
        input.permanence = 1.0;
        let id = memory.live_with(input).trace_id.unwrap();
        let template = memory.store.traces[&id].clone();
        let mut store = MemoryStore::new();
        let mut ranked = Vec::new();
        for (index, (text, score)) in [
            ("Workshop volunteers repair broken tools.", 0.9),
            ("Workshop volunteers repair broken tools.", 0.89),
            ("Workshop opens on Sunday for beginners.", 0.7),
        ]
        .into_iter()
        .enumerate()
        {
            let mut trace = template.clone();
            trace.id = format!("fixture-{index}");
            trace.gist = text.into();
            trace.core = text.into();
            ranked.push(ScoredTrace {
                trace_id: trace.id.clone(),
                score,
                status: trace.status,
                base_score: score,
                anchor: 1.0,
            });
            store.traces.insert(trace.id.clone(), trace);
        }
        let allowed = ranked.iter().map(|c| c.trace_id.clone()).collect();
        (store, ranked, allowed)
    }

    #[test]
    fn repeated_content_yields_to_complementary_evidence_without_deletion() {
        let (store, ranked, allowed) = fixture();
        assert_eq!(
            select_diverse(&ranked, &allowed, &store, 2),
            vec!["fixture-0", "fixture-2"]
        );
        assert_eq!(
            select_diverse(&ranked, &allowed, &store, 3),
            vec!["fixture-0", "fixture-2", "fixture-1"]
        );
        assert!(select_diverse(&ranked, &allowed, &store, 0).is_empty());
    }

    #[test]
    fn independent_sources_are_not_treated_as_one_repetition() {
        let (mut store, ranked, allowed) = fixture();
        store.traces.get_mut("fixture-1").unwrap().source = "another witness".into();
        assert_eq!(
            select_diverse(&ranked, &allowed, &store, 2),
            vec!["fixture-0", "fixture-1"]
        );
    }

    #[test]
    fn ablation_filters_cannot_be_undone_by_diversity() {
        let (store, ranked, allowed) = fixture();
        let allowed = apply_bias(
            &allowed,
            &store,
            RecallBias::DropMarked,
            &["fixture-2".into()],
            3,
        );
        assert_eq!(
            select_diverse(&ranked, &allowed, &store, 3),
            vec!["fixture-0", "fixture-1"]
        );
    }
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
        RecallBias::Observed | RecallBias::Transfer => eligible.iter().take(cap).cloned().collect(),
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
    associates: &[AssociativeCue],
    association_accepted: &mut bool,
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
        if associates.is_empty() {
            narrator.reconstruct(trace, mood, query)
        } else {
            let proposed = narrator.reconstruct_associated(trace, mood, query, associates);
            let judgment = crate::recall::judge::judge_against_core(&proposed, &trace.core, scorer);
            let licensed = !judgment.kind.is_miss()
                && judgment.kind != crate::recall::judge::DetachKind::Unjudged
                && (judgment.kind != crate::recall::judge::DetachKind::Hold
                    || judgment.overlap >= profile.ground_min_overlap);
            *association_accepted = licensed;
            if licensed {
                proposed
            } else {
                trace.gist.clone()
            }
        }
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
            if !store.axiom_available(a) {
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
    let (hits, total) = topic_overlap(query, core);
    if total == 0 {
        0.0
    } else {
        hits as f32 / total as f32
    }
}

fn topic_overlap(query: &str, core: &str) -> (usize, usize) {
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
                        | "people"
                        | "benefit"
                        // Temporal/connective and instruction words do not
                        // establish a relation between observed episodes.
                        | "before" | "after" | "either" | "both" | "another"
                        | "own" | "next" | "first" | "last" | "than"
                        | "willing" | "give" | "choice" | "minutes"
                ) && subject
                    .as_ref()
                    .map_or(true, |s| !s.split_whitespace().any(|p| p == w))
            })
            .collect::<Vec<_>>()
    };
    let q = tokens(query);
    let c = tokens(core);
    if q.is_empty() || c.is_empty() {
        return (0, 0);
    }
    let hits = q.iter().filter(|t| c.contains(t)).count();
    (hits, q.len().min(c.len()))
}
