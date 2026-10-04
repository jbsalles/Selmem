//! Fill a hole. Distinct from gild: gild polishes a detail that is still there.

use crate::core::model::{
    now_secs, Attribution, Channel, DriftEvent, DriftKind, MemoryTrace, TraceStatus,
};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::encode::scoring::lexical_similarity;

const HOLE_FIDELITY: f32 = 0.42;

pub fn is_hole(trace: &MemoryTrace) -> bool {
    if trace.channel.verbatim() || trace.channel == Channel::Log {
        return false;
    }
    if trace.status == TraceStatus::Latent {
        return false;
    }
    if trace.attribution == Attribution::External {
        return false;
    }
    // Internal distorts by denial retell, not by filling a hole with a neighbor.
    if trace.attribution == Attribution::Internal {
        return false;
    }
    if trace.permanence >= 0.80 || trace.anchor >= 0.85 {
        return false;
    }
    if trace.fidelity >= HOLE_FIDELITY {
        return false;
    }
    if trace.core.trim().is_empty() {
        return false;
    }
    let close = lexical_similarity(&trace.gist, &trace.core) >= 0.65;
    let short = trace.gist.chars().count() <= trace.core.chars().count() + 12;
    close || short
}

/// Night hook. One fill per hour, only when the gist has collapsed onto the core.
pub fn run(store: &mut MemoryStore, profile: &EntityProfile) -> u32 {
    let ids = store.active_ids();
    let mut n = 0u32;
    for id in ids {
        if fill_if_hole(store, &id, profile) {
            n += 1;
        }
    }
    n
}

pub fn fill_if_hole(store: &mut MemoryStore, id: &str, profile: &EntityProfile) -> bool {
    let Some(t) = store.traces.get(id) else {
        return false;
    };
    if !is_hole(t) {
        return false;
    }
    if t.drifts.iter().any(|d| d.kind == DriftKind::Confabulate) {
        return false;
    }
    let Some((fill, source_ids, axiom_ids)) = filler(store, t, profile) else {
        return false;
    };
    if fill.trim().is_empty() || fill == t.gist {
        return false;
    }
    if lexical_similarity(&fill, &t.core) >= 0.92 {
        return false;
    }
    let core = t.core.clone();
    if let Some(t) = store.traces.get_mut(id) {
        let before = t.gist.clone();
        t.gist = fill.chars().take(280).collect();
        t.drifts.push(DriftEvent {
            kind: DriftKind::Confabulate,
            at: now_secs(),
            note: "hole filled".into(),
            fidelity_delta: -0.02,
            valence_delta: 0.0,
            disgust_delta: 0.0,
        });
        t.record_operation(crate::core::model::MemoryOperation {
            kind: "confab".into(),
            at: now_secs(),
            source_trace_ids: source_ids,
            source_axiom_ids: axiom_ids,
            source_center: t.schema.clone(),
            before,
            after: t.gist.clone(),
            confidence: t.confidence,
            origin: crate::core::model::EvidenceOrigin::Confabulation,
        });
        t.fidelity = (t.fidelity - 0.02).max(0.15);
        t.recompute_confidence();
        // A confabulated clause is not event evidence. Keep it findable, not sure.
        t.confidence = (t.confidence * 0.62).max(0.12);
        t.clamp();
        debug_assert_eq!(t.core, core);
        return true;
    }
    false
}

fn filler(store: &MemoryStore, t: &MemoryTrace, profile: &EntityProfile) -> Option<(String, Vec<String>, Vec<String>)> {
    let schema = t.schema.as_deref();
    let mut extra = String::new();
    let mut source_ids = Vec::new();
    let mut axiom_ids = Vec::new();
    if let Some(s) = schema {
        if let Some(c) = store.centers.get(s) {
            extra = first_new_clause(&c.core, &t.core);
            if !extra.is_empty() {
                if let Some(hub) = &c.hub_id {
                    source_ids.push(hub.clone());
                }
            }
        }
        if extra.is_empty() {
            if let Some(a) = store
                .living_axioms()
                .into_iter()
                .find(|a| a.schema.as_deref() == Some(s))
            {
                extra = first_new_clause(&a.statement, &t.core);
                if !extra.is_empty() {
                    axiom_ids.push(a.id.clone());
                    source_ids.extend(a.support_trace_ids.iter().cloned());
                }
            }
        }
    }
    let _ = profile;
    if extra.is_empty() {
        return None;
    }
    let base = t.core.trim().trim_end_matches('.');
    Some((format!("{base}. {extra}"), source_ids, axiom_ids))
}

fn first_new_clause(src: &str, core: &str) -> String {
    let clause = src
        .split(['.', ';', '—'])
        .map(|s| s.trim())
        .find(|s| s.chars().count() >= 8 && !clause_licensed(s, core))
        .unwrap_or("")
        .to_string();
    if clause.is_empty() || clause_licensed(&clause, core) {
        String::new()
    } else {
        let mut c = clause;
        if !c.ends_with('.') {
            c.push('.');
        }
        c
    }
}

fn clause_licensed(clause: &str, core: &str) -> bool {
    lexical_similarity(clause, core) >= 0.72
}
