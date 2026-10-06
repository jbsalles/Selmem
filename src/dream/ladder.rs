//! Fourth night pass: motif → belief → trait. Latent traces do not mint.

use std::collections::HashMap;

use crate::core::model::{now_secs, new_id, AxiomLayer, IdentityAxiom, TraceStatus};
use crate::core::store::MemoryStore;
use crate::recall::narrator::Narrator;

pub fn run(store: &mut MemoryStore, _narrator: &dyn Narrator) -> Vec<IdentityAxiom> {
    let prior: Vec<String> = store
        .living_axioms()
        .into_iter()
        .map(|a| a.id.clone())
        .collect();
    let mut axioms = extract_axioms(store);
    axioms.extend(promote_traits(store));
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

fn extract_axioms(store: &mut MemoryStore) -> Vec<IdentityAxiom> {
    let mut evidence: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut living: HashMap<(String, String), Vec<String>> = HashMap::new();
    for t in store.traces.values() {
        if t.channel.verbatim() {
            continue;
        }
        let s = (
            t.schema.clone().unwrap_or_else(|| "self".into()),
            t.stake_mark.clone(),
        );
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
    for ((schema, mark), ids) in evidence {
        let charge = schema_charge(store, &ids);
        // Count still mints a dull motif. Charge lets one wound mint without a second copy.
        if ids.len() < 2 && charge < 0.28 {
            continue;
        }
        let Some(live_ids) = living.get(&(schema.clone(), mark.clone())) else {
            continue;
        };
        let mut support = ids.clone();
        extend_support(store, &mut support);
        if support.is_empty() {
            continue;
        }
        let prev = store
            .living_axioms()
            .into_iter()
            .find(|a| a.schema.as_deref() == Some(schema.as_str()) && a.stake_mark == mark)
            .map(|a| (a.id.clone(), a.strength, a.valence));
        let traces_for_mean: Vec<&crate::core::model::MemoryTrace> = support
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect();
        let preview_v = if traces_for_mean.is_empty() {
            0.0
        } else {
            traces_for_mean.iter().map(|t| t.valence).sum::<f32>()
                / traces_for_mean.len() as f32
        };
        if let Some((ref prev_id, prev_strength, prev_v)) = prev {
            let flipped = prev_v * preview_v < 0.0
                || (prev_v.abs() < 0.15 && preview_v.abs() >= 0.30);
            if prev_strength >= 0.36 || !flipped {
                keep_schema_axiom(store, prev_id, &support, live_ids.len(), charge);
                continue;
            }
            // Weak living axiom, opposite sense: mint a replacement below.
        }
        let traces: Vec<&crate::core::model::MemoryTrace> = support
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect();
        let preview_v = weighted_valence(store, &support);
        let statement = stake_axiom(&schema, &mark, &traces, preview_v);
        if existing.iter().any(|s| s == &statement) {
            continue;
        }
        let mean_v = weighted_valence(store, &support);
        // One wound is a motif. A belief needs the same schema several times.
        let layer = if live_ids.len() >= 3 && charge >= 0.45 {
            AxiomLayer::Belief
        } else {
            AxiomLayer::Motif
        };
        let strength = (0.22 + charge).clamp(layer.strength_floor(), layer.strength_cap());
        let axiom = IdentityAxiom {
            id: new_id("ax"),
            statement,
            support_trace_ids: support,
            valence: mean_v,
            strength,
            created_at: now_secs(),
            superseded_by: None,
            schema: Some(schema.clone()),
            layer,
            stake_kind: traces.first().map(|t| t.stake_kind).unwrap_or_default(),
            bearer: traces.first().map(|t| t.bearer).unwrap_or_default(),
            loss_kind: traces.first().map(|t| t.loss_kind).unwrap_or_default(),
            stake_mark: mark.clone(),
        };
        if let Some((ref prev_id, prev_strength, prev_v)) = prev {
            let flipped = prev_v * preview_v < 0.0
                || (prev_v.abs() < 0.15 && preview_v.abs() >= 0.30);
            if prev_strength < 0.36 && flipped {
                if let Some(old) = store.axioms.get_mut(prev_id) {
                    old.superseded_by = Some(axiom.id.clone());
                }
            }
        }
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
    charge: f32,
) {
    let Some(ax) = store.axioms.get_mut(prev_id) else {
        return;
    };
    for id in support {
        if !ax.support_trace_ids.iter().any(|x| x == id) {
            ax.support_trace_ids.push(id.clone());
        }
    }
    // Three dull hours are not a belief. The mint path already requires charge.
    if live_n >= 3 && charge >= 0.45 && ax.layer == AxiomLayer::Motif {
        ax.layer = AxiomLayer::Belief;
        ax.strength = ax.strength.max(AxiomLayer::Belief.strength_floor());
    }
    let cap = ax.layer.strength_cap();
    if ax.strength > cap {
        ax.strength = cap;
    }
}

fn promote_traits(store: &mut MemoryStore) -> Vec<IdentityAxiom> {
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
        let statement = stake_axiom(label, "", &traces, 0.0);
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
            stake_kind: crate::core::model::StakeKind::None,
            bearer: crate::core::model::Bearer::Self_,
            loss_kind: crate::core::model::LossKind::None,
            stake_mark: String::new(),
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

fn hour_charge(t: &crate::core::model::MemoryTrace) -> f32 {
    let raw = t.valence.abs() * t.arousal.max(0.05) * t.self_relevance.max(0.05);
    if t.permanence < 0.40 {
        raw * 0.25
    } else {
        raw
    }
}

fn schema_charge(store: &MemoryStore, ids: &[String]) -> f32 {
    ids.iter()
        .filter_map(|id| store.traces.get(id))
        .map(hour_charge)
        .sum()
}

fn weighted_valence(store: &MemoryStore, ids: &[String]) -> f32 {
    let mut w = 0.0;
    let mut acc = 0.0;
    for id in ids {
        let Some(t) = store.traces.get(id) else { continue };
        let c = hour_charge(t).max(0.02);
        w += c;
        acc += c * t.valence;
    }
    if w <= 0.0 { 0.0 } else { acc / w }
}

fn stake_axiom(schema: &str, mark: &str, traces: &[&crate::core::model::MemoryTrace], valence: f32) -> String {
    let kind = traces.first().map(|t| t.stake_kind.token()).unwrap_or("none");
    let bearer = traces.first().map(|t| t.bearer.token()).unwrap_or("world");
    let loss = traces.first().map(|t| t.loss_kind.token()).unwrap_or("none");
    let sign = if valence <= -0.2 { "against" } else if valence >= 0.2 { "for" } else { "under" };
    format!(
        "stake={kind} bearer={bearer} loss={loss} mark={mark} n={} sign={sign} schema={schema}",
        traces.len()
    )
}
