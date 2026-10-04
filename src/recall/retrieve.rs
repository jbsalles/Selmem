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
}

#[derive(Clone, Debug, Default)]
pub struct RetrievalDump {
    pub candidates: Vec<ScoredTrace>,
    pub selected: Vec<String>,
    pub pulled: u32,
    pub reconsolidated: u32,
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
) -> RecallOutcome {
    let query_embedding = embedder.embed(query);
    let live = write == RecallWrite::Live;
    let cloud = context_cloud(store, query);
    let ids: Vec<String> = store.active_ids();
    let mut ranked: Vec<ScoredTrace> = ids
        .into_iter()
        .filter_map(|trace_id| {
            if live {
                let trace = store.traces.get_mut(&trace_id)?;
                refresh_access(trace, profile);
            }
            let trace = store.traces.get(&trace_id)?;
            if trace.suppressed && !matches!(bias, RecallBias::ForceMarked) {
                return None;
            }
            let talk = trace
                .archive_id
                .as_ref()
                .and_then(|id| store.archives.get(id))
                .map(|a| a.source == "talk")
                .unwrap_or(false);
            if talk && trace.access < 0.10 {
                return None;
            }
            let mut score = recall_score_emb(trace, query, Some(&query_embedding), mood, profile);
            let anchor = cloud_anchor(&cloud, trace);
            if anchor <= 0.0 {
                score = 0.0;
            } else {
                score *= anchor;
            }
            // Frozen δ. If DropLineage stops flattening Grok D, this is too large.
            if !store.living_axiom_ids_for(&trace_id).is_empty() {
                score += 0.12;
            }
            // Episode probes ("what happened that day") must not lose to a
            // sharp World calendar line. Lived Selfhood keeps the floor.
            if episode_ask(query) {
                if trace.channel.verbatim() {
                    score *= 0.32;
                } else {
                    score += 0.20;
                }
            }
            if let Some(schema) = trace.schema.as_deref() {
                let q = query.to_ascii_lowercase();
                for part in schema.split('-') {
                    if part.len() > 3 && q.contains(part) {
                        score += 0.18;
                        break;
                    }
                }
            }
            Some(ScoredTrace {
                trace_id,
                score,
                status: trace.status,
            })
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
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

        let (narrative, disclaimer, fidelity, pulled, reconsolidated) = if channel.verbatim()
        {
            (
                gist,
                "verbatim record, not distorted".to_string(),
                store.traces[trace_id].fidelity,
                false,
                false,
            )
        } else {
            speak_self(
                store, profile, narrator, query, mood, cut, write, trace_id,
            )
        };

        if live {
            if let Some(trace) = store.traces.get_mut(trace_id) {
                // Access clock only. Rehearsal is spoken utility, stamped in speak.
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
        if crate::recall::ground::should_force_core_rewrite(trace, profile, &generated, &core) {
            Some(narrator.recontextualize(trace, &core, profile))
        } else {
            None
        }
    };
    let outcome = {
        let trace = store.traces.get_mut(trace_id).unwrap();
        crate::recall::ground::apply_grounding(trace, profile, &generated, &core, narrator_rewrite)
    };
    let reconsolidated = if !outcome.pulled_toward_core && cut.reconsolidate {
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
        || q.contains("that day")
        || q.contains("this day")
        || q.contains("how did you feel")
        || q.contains("how sure")
        || q.contains("do you remember")
        || q.contains("raconte")
        || q.contains("ce jour")
}

/// Tokens of the hours the present already touches, weighted by age and relevance.
/// A same-schema neighbor of a touched hour joins at half, still weighted by its own age.
pub fn context_cloud_pub(store: &crate::core::store::MemoryStore, query: &str) -> std::collections::HashMap<String, f32> {
    context_cloud(store, query)
}

fn context_cloud(store: &crate::core::store::MemoryStore, query: &str) -> std::collections::HashMap<String, f32> {
    use crate::encode::scoring::{behavior_weight, lexical_similarity, token_set};
    let mut weights: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    let ids = store.active_ids();
    let mut seeds: Vec<(String, f32)> = Vec::new();
    for id in &ids {
        let Some(t) = store.traces.get(id) else { continue };
        let mut sim = lexical_similarity(query, &t.gist);
        sim = sim.max(lexical_similarity(query, &t.core));
        for cue in &t.cues {
            sim = sim.max(lexical_similarity(query, cue));
        }
        if sim > 0.0 {
            seeds.push((id.clone(), sim));
        }
    }
    let add = |weights: &mut std::collections::HashMap<String, f32>, text: &str, w: f32| {
        if w <= 0.0 { return; }
        for tok in token_set(text) {
            *weights.entry(tok).or_insert(0.0) += w;
        }
    };
    for (id, sim) in &seeds {
        let Some(t) = store.traces.get(id) else { continue };
        let w = sim * behavior_weight(t) * t.self_relevance.max(0.05) * t.access.max(0.05);
        add(&mut weights, &t.gist, w);
        add(&mut weights, &t.core, w * 0.5);
        let schema = t.schema.clone();
        let Some(schema) = schema else { continue };
        for other_id in &ids {
            if other_id == id { continue; }
            let Some(o) = store.traces.get(other_id) else { continue };
            if o.schema.as_deref() != Some(schema.as_str()) { continue; }
            let ow = 0.5 * behavior_weight(o) * o.self_relevance.max(0.05) * o.access.max(0.05);
            add(&mut weights, &o.gist, ow);
            add(&mut weights, &o.core, ow * 0.5);
        }
    }
    weights
}

pub fn statement_anchored(cloud: &std::collections::HashMap<String, f32>, statement: &str) -> bool {
    if cloud.is_empty() { return false; }
    crate::encode::scoring::token_set(statement).iter().any(|t| cloud.contains_key(t))
}

fn cloud_anchor(cloud: &std::collections::HashMap<String, f32>, trace: &crate::core::model::MemoryTrace) -> f32 {
    if cloud.is_empty() {
        return 0.0;
    }
    let mass: f32 = cloud.values().sum();
    if mass <= 0.0 {
        return 0.0;
    }
    let mut hit = 0.0;
    for tok in crate::encode::scoring::token_set(&trace.gist) {
        hit += cloud.get(&tok).copied().unwrap_or(0.0);
    }
    for tok in crate::encode::scoring::token_set(&trace.core) {
        hit += 0.5 * cloud.get(&tok).copied().unwrap_or(0.0);
    }
    (hit / mass).clamp(0.0, 1.0)
}
