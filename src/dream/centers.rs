//! Schema centers: one prototype per schema. Periphery falls toward it.

use crate::core::model::{
    now_secs, Attribution, Channel, DriftEvent, DriftKind, SchemaCenter, TraceStatus,
};
use crate::core::store::MemoryStore;
use crate::encode::scoring::lexical_similarity;

/// Rebuild centers from living hours and axioms. One node per schema.
pub fn refresh(store: &mut MemoryStore) {
    let mut keys: Vec<String> = store
        .traces
        .values()
        .filter_map(|t| t.schema.clone())
        .collect();
    for a in store.living_axioms() {
        if let Some(s) = &a.schema {
            keys.push(s.clone());
        }
    }
    keys.sort();
    keys.dedup();

    let mut next = std::collections::HashMap::new();
    for schema in keys {
        if let Some(c) = build(store, &schema) {
            next.insert(schema, c);
        }
    }
    store.centers = next;
}

fn build(store: &MemoryStore, schema: &str) -> Option<SchemaCenter> {
    let mut hours: Vec<&crate::core::model::MemoryTrace> = store
        .traces
        .values()
        .filter(|t| t.schema.as_deref() == Some(schema))
        .filter(|t| t.channel != Channel::Log && t.personal.role == crate::SelfRole::Episode)
        .filter(|t| !t.suppressed && t.status != TraceStatus::Latent)
        .collect();
    hours.sort_by(|a, b| hub_key(b).cmp(&hub_key(a)).then_with(|| a.id.cmp(&b.id)));

    let axiom = store
        .living_axioms()
        .into_iter()
        .find(|a| a.schema.as_deref() == Some(schema));

    if hours.len() < 2 && axiom.is_none() {
        return None;
    }

    let hub = hours.first().copied();
    let core = if let Some(h) = hub {
        if !h.core.is_empty() {
            h.core.clone()
        } else {
            h.gist.clone()
        }
    } else {
        axiom.map(|a| a.statement.clone()).unwrap_or_default()
    };
    if core.trim().is_empty() {
        return None;
    }
    let valence = if let Some(a) = axiom {
        a.valence
    } else {
        hours.iter().map(|t| t.valence).sum::<f32>() / hours.len().max(1) as f32
    };
    let weight = hours.len() as f32 + axiom.map(|a| a.strength).unwrap_or(0.0);
    Some(SchemaCenter {
        schema: schema.to_string(),
        core,
        valence,
        weight,
        hub_id: hub.map(|t| t.id.clone()),
        axiom_id: axiom.map(|a| a.id.clone()),
    })
}

fn hub_key(t: &crate::core::model::MemoryTrace) -> (i32, i32, i32) {
    (
        (t.anchor * 1000.0) as i32,
        (t.permanence * 1000.0) as i32,
        (t.self_relevance * 1000.0) as i32,
    )
}

/// Pull faded Internal periphery onto the schema prototype. Core stays.
pub fn gravitate(store: &mut MemoryStore) -> u32 {
    let mut schemas: Vec<String> = store.centers.keys().cloned().collect();
    schemas.sort();
    let mut n = 0u32;
    for schema in schemas {
        let (hub, proto, valence) = {
            let Some(c) = store.visible_center(&schema) else {
                continue;
            };
            (c.hub_id.clone(), c.core.clone(), c.valence)
        };
        if proto.is_empty() {
            continue;
        }
        let ids: Vec<String> = store
            .traces
            .values()
            .filter(|t| t.schema.as_deref() == Some(schema.as_str()))
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            if hub.as_deref() == Some(id.as_str()) {
                continue;
            }
            let Some(t) = store.traces.get(&id) else {
                continue;
            };
            if t.suppressed
                || t.personal.role != crate::SelfRole::Episode
                || t.channel.verbatim()
                || t.status == TraceStatus::Latent
            {
                continue;
            }
            if t.attribution != Attribution::Internal {
                continue;
            }
            if t.fidelity >= 0.62 {
                continue;
            }
            if lexical_similarity(&t.gist, &proto) >= 0.50 {
                continue;
            }
            if let Some(t) = store.traces.get_mut(&id) {
                t.gist = proto.chars().take(280).collect();
                t.drifts.push(DriftEvent {
                    kind: DriftKind::Rewrite,
                    at: now_secs(),
                    note: "schema-center".into(),
                    fidelity_delta: -0.01,
                    valence_delta: 0.0,
                    disgust_delta: 0.0,
                });
                t.fidelity = (t.fidelity - 0.01).max(0.15);
                // Weak valence tug toward the prototype, not a overwrite.
                t.valence = 0.85 * t.valence + 0.15 * valence;
                t.clamp();
                n += 1;
            }
        }
    }
    n
}
