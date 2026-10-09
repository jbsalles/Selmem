//! Fourth night pass: motif → belief → contextual trait. Evidence is provenance,
//! never the number of copies or associative neighbors of an observation.
use crate::core::model::{new_id, now_secs, AxiomLayer, IdentityAxiom, MemoryTrace, TraceStatus};
use crate::core::store::MemoryStore;
use crate::recall::narrator::Narrator;
use std::collections::{BTreeMap, BTreeSet, HashMap};

// Strings make semantic categories ordered without changing public enum APIs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Context {
    schema: String,
    stake_kind: String,
    bearer: String,
    loss_kind: String,
    attribution: String,
    subject: String,
}
type Family = (Context, String);
fn context(t: &MemoryTrace) -> Context {
    // The first annotated entity is the subject candidate. Object inventory
    // must not split otherwise equivalent observations of the same actor.
    let subject = t
        .semantic
        .entities
        .first()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    Context {
        schema: t.schema.clone().unwrap_or_else(|| "self".into()),
        stake_kind: t.stake_kind.token().into(),
        bearer: t.bearer.token().into(),
        loss_kind: t.loss_kind.token().into(),
        attribution: t.attribution.token().into(),
        subject,
    }
}
fn family(t: &MemoryTrace) -> Family {
    (context(t), t.stake_mark.clone())
}
fn observation(t: &MemoryTrace) -> &str {
    t.observation_id.as_deref().unwrap_or(&t.id)
}
fn independent(store: &MemoryStore, ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut sorted = ids.to_vec();
    sorted.sort_by(|a, b| {
        let ta = &store.traces[a];
        let tb = &store.traces[b];
        // Prefer a surviving original over a merged myth; duplicates cannot
        // acquire extra weight just by being rehearsed or copied.
        (ta.status == TraceStatus::Myth)
            .cmp(&(tb.status == TraceStatus::Myth))
            .then_with(|| hour_charge(tb).total_cmp(&hour_charge(ta)))
            .then(ta.core.cmp(&tb.core))
            .then(a.cmp(b))
    });
    sorted.retain(|id| seen.insert(observation(&store.traces[id]).to_string()));
    sorted.sort_by(|a, b| {
        store.traces[a]
            .core
            .cmp(&store.traces[b].core)
            .then(store.traces[a].created_at.cmp(&store.traces[b].created_at))
            .then(a.cmp(b))
    });
    sorted
}
fn compatible_axiom(store: &MemoryStore, ax: &IdentityAxiom, key: &Family) -> bool {
    ax.schema.as_deref() == Some(key.0.schema.as_str())
        && ax.stake_mark == key.1
        && ax.stake_kind.token() == key.0.stake_kind
        && ax.bearer.token() == key.0.bearer
        && ax.loss_kind.token() == key.0.loss_kind
        && ax
            .support_trace_ids
            .iter()
            .filter_map(|id| store.traces.get(id))
            .all(|t| family(t) == *key)
}

pub fn run(store: &mut MemoryStore, _narrator: &dyn Narrator) -> Vec<IdentityAxiom> {
    crate::recall::interpretation::consolidate(store);
    let prior: Vec<_> = store.living_axioms().iter().map(|a| a.id.clone()).collect();
    let mut axioms = extract_axioms(store);
    axioms.extend(promote_traits(store));
    rust_unused(store, &prior);
    axioms
}
fn rust_unused(store: &mut MemoryStore, prior: &[String]) {
    let spoken: HashMap<_, _> = store
        .traces
        .iter()
        .map(|(id, t)| (id.clone(), t.rehearsals > 0))
        .collect();
    for id in prior {
        if let Some(ax) = store.axioms.get_mut(id) {
            if !ax
                .support_trace_ids
                .iter()
                .any(|id| spoken.get(id).copied().unwrap_or(false))
            {
                ax.strength = (ax.strength - 0.05).max(ax.layer.strength_floor());
            }
        }
    }
}
fn extract_axioms(store: &mut MemoryStore) -> Vec<IdentityAxiom> {
    let mut groups: BTreeMap<Family, Vec<String>> = BTreeMap::new();
    for t in store.traces.values() {
        if !t.channel.verbatim()
            && matches!(
                t.status,
                TraceStatus::Active | TraceStatus::Cold | TraceStatus::Myth
            )
        {
            groups.entry(family(t)).or_default().push(t.id.clone());
        }
    }
    let mut created = Vec::new();
    for (key, ids) in groups {
        let mut support = independent(store, &ids);
        let mut charge = schema_charge(store, &support);
        if support.len() < 2 && charge < 0.28 {
            continue;
        }
        let live_n = support
            .iter()
            .filter(|id| store.traces[*id].status != TraceStatus::Myth)
            .count();
        if live_n == 0 {
            continue;
        }
        let prev = store
            .living_axioms()
            .into_iter()
            .filter(|a| a.layer != AxiomLayer::Trait && compatible_axiom(store, a, &key))
            .min_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)))
            .cloned();
        let mut mean_v = weighted_valence(store, &support);
        let mut revision = false;
        if let Some(old) = &prev {
            let mut known = BTreeSet::new();
            let mut generation = vec![old.id.clone()];
            let mut ancestors = BTreeSet::new();
            while let Some(id) = generation.pop() {
                if !ancestors.insert(id.clone()) {
                    continue;
                }
                if let Some(ax) = store.axioms.get(&id) {
                    known.extend(
                        ax.support_trace_ids
                            .iter()
                            .filter_map(|id| store.traces.get(id))
                            .map(|t| observation(t).to_string()),
                    );
                }
                generation.extend(
                    store
                        .axioms
                        .values()
                        .filter(|a| a.superseded_by.as_deref() == Some(&id))
                        .map(|a| a.id.clone()),
                );
            }
            let contrary: Vec<_> = support
                .iter()
                .filter(|id| {
                    let t = &store.traces[*id];
                    !known.contains(observation(t))
                        && old.valence * t.valence < 0.0
                        && t.valence.abs() >= 0.2
                })
                .cloned()
                .collect();
            let flipped =
                old.valence * mean_v < 0.0 || (old.valence.abs() < 0.15 && mean_v.abs() >= 0.30);
            // Strong beliefs resist one contrary episode, but three independent
            // charged observations can revise them. Replaying the night cannot.
            revision = (old.strength < 0.36 && flipped)
                || (contrary.len() >= 3
                    && schema_charge(store, &contrary) >= old.strength.max(0.45));
            if revision && !contrary.is_empty() {
                support = contrary;
                charge = schema_charge(store, &support);
                mean_v = weighted_valence(store, &support);
            }
            if !revision {
                let agreeing: Vec<_> = support
                    .iter()
                    .filter(|id| old.valence * store.traces[*id].valence >= 0.0)
                    .cloned()
                    .collect();
                let agreeing_charge = schema_charge(store, &agreeing);
                if let Some(ax) = store.axioms.get_mut(&old.id) {
                    for id in &agreeing {
                        if !ax.support_trace_ids.contains(id) {
                            ax.support_trace_ids.push(id.clone());
                        }
                    }
                    ax.support_trace_ids.sort();
                    if agreeing.len() >= 3
                        && agreeing_charge >= 0.45
                        && ax.layer == AxiomLayer::Motif
                    {
                        ax.layer = AxiomLayer::Belief;
                        ax.strength = ax.strength.max(AxiomLayer::Belief.strength_floor());
                    }
                }
                continue;
            }
        }
        let layer = if support.len() >= 3 && charge >= 0.45 {
            AxiomLayer::Belief
        } else {
            AxiomLayer::Motif
        };
        let first = &store.traces[&support[0]];
        let ax = IdentityAxiom {
            id: new_id("ax"),
            statement: stake_axiom(store, &key.0.schema, &key.1, &support, mean_v),
            support_trace_ids: support,
            valence: mean_v,
            strength: (0.22 + charge).clamp(layer.strength_floor(), layer.strength_cap()),
            created_at: now_secs(),
            superseded_by: None,
            schema: Some(key.0.schema.clone()),
            layer,
            stake_kind: first.stake_kind,
            bearer: first.bearer,
            loss_kind: first.loss_kind,
            stake_mark: key.1,
        };
        if revision {
            if let Some(old) = prev.and_then(|p| store.axioms.get_mut(&p.id)) {
                old.superseded_by = Some(ax.id.clone());
            }
        }
        store.add_axiom(ax.clone());
        created.push(ax);
    }
    created
}
fn promote_traits(store: &mut MemoryStore) -> Vec<IdentityAxiom> {
    let mut groups: BTreeMap<(Context, i8), Vec<IdentityAxiom>> = BTreeMap::new();
    for ax in store.living_axioms() {
        if ax.layer != AxiomLayer::Belief || ax.strength < 0.45 {
            continue;
        }
        let traces: Vec<_> = ax
            .support_trace_ids
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect();
        let Some(t) = traces.first() else { continue };
        let ctx = context(t);
        if traces.iter().any(|t| context(t) != ctx) {
            continue;
        }
        let sign = if ax.valence > 0.2 {
            1
        } else if ax.valence < -0.2 {
            -1
        } else {
            0
        };
        groups.entry((ctx, sign)).or_default().push(ax.clone());
    }
    let mut out = Vec::new();
    for ((ctx, _), mut beliefs) in groups {
        beliefs.sort_by(|a, b| a.stake_mark.cmp(&b.stake_mark).then(a.id.cmp(&b.id)));
        let mut seen = BTreeSet::new();
        let mut distinct = Vec::new();
        let mut support = Vec::new();
        for ax in beliefs {
            let ids = independent(store, &ax.support_trace_ids);
            let origins: Vec<_> = ids
                .iter()
                .map(|id| observation(&store.traces[id]).to_string())
                .collect();
            if origins.is_empty() || origins.iter().any(|id| seen.contains(id)) {
                continue;
            }
            seen.extend(origins);
            support.extend(ids);
            distinct.push(ax);
        }
        if distinct.len() < 2 {
            continue;
        }
        // Contextual regularity, not positive→trust or negative→withdrawal.
        let schema = format!("contextual:{}", ctx.schema);
        let first = &store.traces[&support[0]];
        if store.living_axioms().iter().any(|a| {
            a.layer == AxiomLayer::Trait
                && a.schema.as_deref() == Some(&schema)
                && a.stake_kind == first.stake_kind
                && a.bearer == first.bearer
                && a.loss_kind == first.loss_kind
                && a.support_trace_ids
                    .iter()
                    .filter_map(|id| store.traces.get(id))
                    .all(|t| context(t) == ctx)
        }) {
            continue;
        }
        support.sort();
        let v = weighted_valence(store, &support);
        let ax = IdentityAxiom {
            id: new_id("ax"),
            statement: stake_axiom(store, &schema, "", &support, v),
            support_trace_ids: support,
            valence: v,
            strength: AxiomLayer::Trait.strength_cap() * 0.72,
            created_at: now_secs(),
            superseded_by: None,
            schema: Some(schema),
            layer: AxiomLayer::Trait,
            stake_kind: first.stake_kind,
            bearer: first.bearer,
            loss_kind: first.loss_kind,
            stake_mark: String::new(),
        };
        store.add_axiom(ax.clone());
        out.push(ax);
    }
    out
}
fn hour_charge(t: &MemoryTrace) -> f32 {
    let raw = t.valence.abs() * t.arousal.max(0.05) * t.self_relevance.max(0.05);
    if t.permanence < 0.40 {
        raw * 0.25
    } else {
        raw
    }
}
fn schema_charge(store: &MemoryStore, ids: &[String]) -> f32 {
    ids.iter().map(|id| hour_charge(&store.traces[id])).sum()
}
fn weighted_valence(store: &MemoryStore, ids: &[String]) -> f32 {
    let mut w = 0.0;
    let mut acc = 0.0;
    for id in ids {
        let t = &store.traces[id];
        let c = hour_charge(t).max(0.02);
        w += c;
        acc += c * t.valence;
    }
    if w <= 0.0 {
        0.0
    } else {
        acc / w
    }
}
fn stake_axiom(
    store: &MemoryStore,
    schema: &str,
    mark: &str,
    ids: &[String],
    valence: f32,
) -> String {
    let first = &store.traces[&ids[0]];
    let sign = if valence <= -0.2 {
        "against"
    } else if valence >= 0.2 {
        "for"
    } else {
        "under"
    };
    format!(
        "stake={} bearer={} loss={} agency={} mark={mark} n={} sign={sign} schema={schema}",
        first.stake_kind.token(),
        first.bearer.token(),
        first.loss_kind.token(),
        first.attribution.token(),
        ids.len()
    )
}
