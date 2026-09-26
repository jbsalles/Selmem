//! Fourth night pass: motif → belief → trait. Latent traces do not mint.

use std::collections::HashMap;

use crate::core::model::{now_secs, new_id, AxiomLayer, IdentityAxiom, TraceStatus};
use crate::core::store::MemoryStore;
use crate::recall::narrator::Narrator;

pub fn run(store: &mut MemoryStore, narrator: &dyn Narrator) -> Vec<IdentityAxiom> {
    let prior: Vec<String> = store
        .living_axioms()
        .into_iter()
        .map(|a| a.id.clone())
        .collect();
    let mut axioms = extract_axioms(store, narrator);
    axioms.extend(promote_traits(store, narrator));
    rust_unused(store, &prior);
    axioms
}

/// Unused living axioms ease toward their floor. Spoken support blocks the rust.
fn rust_unused(store: &mut MemoryStore, prior: &[String]) {
    let spoken: HashMap<String, bool> = store
        .traces
        .iter()
        .map(|(id, t)| (id.clone(), t.rehearsals > 0))
        .collect();
    for id in prior {
        let Some(ax) = store.axioms.get(id) else { continue };
        let used = ax
            .support_trace_ids
            .iter()
            .any(|tid| spoken.get(tid).copied().unwrap_or(false));
        if used {
            continue;
        }
        let floor = ax.layer.strength_floor();
        if let Some(ax) = store.axioms.get_mut(id) {
            ax.strength = (ax.strength - 0.05).max(floor);
        }
    }
}

fn extract_axioms(store: &mut MemoryStore, narrator: &dyn Narrator) -> Vec<IdentityAxiom> {
    let mut evidence: HashMap<String, Vec<String>> = HashMap::new();
    let mut living: HashMap<String, Vec<String>> = HashMap::new();
    for t in store.traces.values() {
        if t.channel.verbatim() {
            continue;
        }
        let s = t
            .schema
            .clone()
            .unwrap_or_else(|| "self".into());
        match t.status {
            // Merged siblings stay Myth; they still count as episodes.
            TraceStatus::Active | TraceStatus::Cold => {
                evidence.entry(s.clone()).or_default().push(t.id.clone());
                living.entry(s.clone()).or_default().push(t.id.clone());
            }
            TraceStatus::Myth => {
                evidence.entry(s.clone()).or_default().push(t.id.clone());
            }
            // Latent must not mint a belief.
            _ => {}
        }
    }
    let existing: Vec<String> = store
        .living_axioms()
        .into_iter()
        .map(|a| a.statement.clone())
        .collect();

    let mut created = Vec::new();
    for (schema, ids) in evidence {
        if ids.len() < 2 {
            continue;
        }
        let Some(live_ids) = living.get(&schema) else {
            continue;
        };
        let mut support = ids.clone();
        extend_support(store, &mut support);
        if support.is_empty() {
            continue;
        }
        if let Some(prev_id) = store
            .living_axioms()
            .into_iter()
            .find(|a| a.schema.as_deref() == Some(schema.as_str()))
            .map(|a| a.id.clone())
        {
            keep_schema_axiom(store, &prev_id, &support, live_ids.len());
            continue;
        }
        let traces: Vec<&crate::core::model::MemoryTrace> = support
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect();
        let statement = match narrator.distill_axiom(&traces) {
            Some(s) if !s.trim().is_empty() => s,
            _ => match crate::recall::narrator::RuleNarrator.distill_axiom(&traces) {
                Some(s) => s,
                None => continue,
            },
        };
        if existing.iter().any(|s| s == &statement) {
            continue;
        }
        let n = live_ids.len().max(ids.len().min(2)) as f32;
        let mean_v = traces.iter().map(|t| t.valence).sum::<f32>() / n.max(1.0);
        let layer = if live_ids.len() >= 3 {
            AxiomLayer::Belief
        } else {
            AxiomLayer::Motif
        };
        let strength = match layer {
            AxiomLayer::Belief => (0.40 + 0.04 * (n - 3.0)).clamp(
                AxiomLayer::Belief.strength_floor(),
                AxiomLayer::Belief.strength_cap(),
            ),
            AxiomLayer::Motif => (0.28 + 0.04 * (n - 2.0)).clamp(
                AxiomLayer::Motif.strength_floor(),
                AxiomLayer::Motif.strength_cap(),
            ),
            AxiomLayer::Trait => 0.55,
        };
        let axiom = IdentityAxiom {
            id: new_id("ax"),
            statement,
            support_trace_ids: support,
            valence: mean_v,
            strength,
            created_at: now_secs(),
            superseded_by: None,
            schema: Some(schema),
            layer,
        };
        store.add_axiom(axiom.clone());
        created.push(axiom);
    }
    created
}

fn keep_schema_axiom(
    store: &mut MemoryStore,
    prev_id: &str,
    support: &[String],
    live_n: usize,
) {
    let Some(ax) = store.axioms.get_mut(prev_id) else {
        return;
    };
    for id in support {
        if !ax.support_trace_ids.iter().any(|x| x == id) {
            ax.support_trace_ids.push(id.clone());
        }
    }
    if live_n >= 3 && ax.layer == AxiomLayer::Motif {
        ax.layer = AxiomLayer::Belief;
        ax.strength = ax.strength.max(AxiomLayer::Belief.strength_floor());
    }
    let cap = ax.layer.strength_cap();
    if ax.strength > cap {
        ax.strength = cap;
    }
}

fn promote_traits(store: &mut MemoryStore, narrator: &dyn Narrator) -> Vec<IdentityAxiom> {
    let beliefs: Vec<IdentityAxiom> = store
        .living_axioms()
        .into_iter()
        .filter(|a| a.layer == AxiomLayer::Belief && a.strength >= 0.45)
        .cloned()
        .collect();
    if beliefs.len() < 2 {
        return Vec::new();
    }
    let pos: Vec<_> = beliefs.iter().filter(|a| a.valence > 0.2).collect();
    let neg: Vec<_> = beliefs.iter().filter(|a| a.valence < -0.2).collect();
    let mut out = Vec::new();
    for (bucket, label) in [(pos, "trust"), (neg, "withdrawal")] {
        if bucket.len() < 2 {
            continue;
        }
        if store
            .living_axioms()
            .iter()
            .any(|a| a.layer == AxiomLayer::Trait && a.schema.as_deref() == Some(label))
        {
            continue;
        }
        let mut support: Vec<String> = bucket
            .iter()
            .flat_map(|a| a.support_trace_ids.clone())
            .collect();
        extend_support(store, &mut support);
        if support.is_empty() {
            continue;
        }
        let traces: Vec<&crate::core::model::MemoryTrace> = support
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect();
        let statement = narrator.distill_axiom(&traces).unwrap_or_else(|| {
            if label == "trust" {
                "I attach slowly, but I stay.".into()
            } else {
                "I pull away when someone vanishes without warning.".into()
            }
        });
        let mean_v = bucket.iter().map(|a| a.valence).sum::<f32>() / bucket.len() as f32;
        let axiom = IdentityAxiom {
            id: new_id("ax"),
            statement,
            support_trace_ids: support,
            valence: mean_v,
            strength: AxiomLayer::Trait.strength_cap() * 0.72,
            created_at: now_secs(),
            superseded_by: None,
            schema: Some(label.into()),
            layer: AxiomLayer::Trait,
        };
        store.add_axiom(axiom.clone());
        out.push(axiom);
    }
    out
}

fn extend_support(store: &MemoryStore, ids: &mut Vec<String>) {
    let mut extra = Vec::new();
    for id in ids.iter() {
        let Some(neigh) = store.edges.get(id) else { continue };
        for n in neigh {
            if !ids.iter().any(|x| x == n) && !extra.iter().any(|x| x == n) {
                extra.push(n.clone());
            }
        }
    }
    ids.extend(extra);
}
