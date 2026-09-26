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
    let ids: Vec<String> = store.active_ids();
    let mut ranked: Vec<ScoredTrace> = ids
        .into_iter()
        .filter_map(|trace_id| {
            if live {
                let trace = store.traces.get_mut(&trace_id)?;
                refresh_access(trace, profile);
            }
            let trace = store.traces.get(&trace_id)?;
            let score = recall_score_emb(trace, query, Some(&query_embedding), mood, profile);
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
