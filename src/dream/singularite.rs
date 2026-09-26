use crate::engine::SelectiveMemory;
use crate::core::model::{IdentityAxiom, MemoryTrace};
use crate::encode::scoring::token_set;
use crate::core::store::MemoryStore;

#[derive(Clone, Debug)]
pub struct Fingerprint {
    pub name: String,
    pub n_traces: usize,
    pub n_myths: usize,
    pub n_axioms: usize,
    pub mean_valence: f32,
    pub mean_disgust: f32,
    pub mean_fidelity: f32,
    pub mean_anchor: f32,
    pub mood_valence: f32,
    pub tokens: Vec<String>,
    pub founders: Vec<String>,
    pub n_traits: usize,
    pub n_contradictions: usize,
}

pub fn fingerprint(mem: &SelectiveMemory) -> Fingerprint {
    from_store(&mem.profile.name, &mem.store, mem.mood.valence)
}

pub fn from_store(name: &str, store: &MemoryStore, mood_valence: f32) -> Fingerprint {
    let traces: Vec<_> = store.traces.values().collect();
    let n = traces.len().max(1) as f32;
    let n_myths = traces
        .iter()
        .filter(|t| t.status == crate::core::model::TraceStatus::Myth)
        .count();
    let mean_valence = traces.iter().map(|t| t.valence).sum::<f32>() / n;
    let mean_disgust = traces.iter().map(|t| t.disgust).sum::<f32>() / n;
    let mean_fidelity = traces.iter().map(|t| t.fidelity).sum::<f32>() / n;
    let mean_anchor = traces.iter().map(|t| t.anchor).sum::<f32>() / n;

    let mut blob = String::new();
    for t in &traces {
        blob.push_str(if t.core.is_empty() { &t.gist } else { &t.core });
        blob.push(' ');
        if let Some(s) = &t.schema {
            blob.push_str(s);
            blob.push(' ');
        }
    }
    for a in store.living_axioms() {
        blob.push_str(&a.statement);
        blob.push(' ');
    }
    let mut ranked: Vec<_> = traces.iter().copied().collect();
    ranked.sort_by(|a, b| {
        b.anchor
            .partial_cmp(&a.anchor)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ca = if a.core.is_empty() { &a.gist } else { &a.core };
                let cb = if b.core.is_empty() { &b.gist } else { &b.core };
                ca.cmp(cb)
            })
    });
    let founders: Vec<String> = ranked
        .iter()
        .take(3)
        .map(|t| {
            if t.core.is_empty() {
                t.gist.clone()
            } else {
                t.core.clone()
            }
        })
        .collect();
    let n_traits = store
        .living_axioms()
        .into_iter()
        .filter(|a| a.layer == crate::core::model::AxiomLayer::Trait)
        .count();
    let mut schema_sign: std::collections::HashMap<&str, (i32, i32)> = std::collections::HashMap::new();
    for t in &traces {
        if let Some(s) = t.schema.as_deref() {
            let e = schema_sign.entry(s).or_insert((0, 0));
            if t.valence > 0.2 {
                e.0 += 1;
            } else if t.valence < -0.2 {
                e.1 += 1;
            }
        }
    }
    let n_contradictions = schema_sign.values().filter(|(p, n)| *p > 0 && *n > 0).count();
    Fingerprint {
        name: name.to_string(),
        n_traces: traces.len(),
        n_myths,
        n_axioms: store.living_axioms().len(),
        mean_valence,
        mean_disgust,
        mean_fidelity,
        mean_anchor,
        mood_valence,
        tokens: token_set(&blob),
        founders,
        n_traits,
        n_contradictions,
    }
}

/// 0 = clones indistinguables, 1 = vies disjointes.
pub fn distance(a: &Fingerprint, b: &Fingerprint) -> f32 {
    let jac = jaccard(&a.tokens, &b.tokens);
    let dv = (a.mean_valence - b.mean_valence).abs() / 2.0;
    let dd = (a.mean_disgust - b.mean_disgust).abs();
    let df = (a.mean_fidelity - b.mean_fidelity).abs();
    let da = (a.mean_anchor - b.mean_anchor).abs();
    let dm = (a.mood_valence - b.mood_valence).abs() / 2.0;
    let dn = {
        let maxn = a.n_traces.max(b.n_traces).max(1) as f32;
        (a.n_traces as f32 - b.n_traces as f32).abs() / maxn
    };
    let fj = jaccard(&token_set(&a.founders.join(" ")), &token_set(&b.founders.join(" ")));
    let dt = {
        let m = a.n_traits.max(b.n_traits).max(1) as f32;
        (a.n_traits as f32 - b.n_traits as f32).abs() / m
    };
    let dc = {
        let m = a.n_contradictions.max(b.n_contradictions).max(1) as f32;
        (a.n_contradictions as f32 - b.n_contradictions as f32).abs() / m
    };
    (0.24 * (1.0 - jac)
        + 0.12 * (1.0 - fj)
        + 0.14 * dv
        + 0.14 * dd
        + 0.08 * df
        + 0.10 * da
        + 0.06 * dm
        + 0.04 * dn
        + 0.05 * dt
        + 0.03 * dc)
        .clamp(0.0, 1.0)
}

/// Ancrage à l'encodage : trauma, réussite vive, serment.
pub fn seed_anchor(valence: f32, arousal: f32, disgust: f32, permanence: f32, self_rel: f32) -> f32 {
    let trauma = if disgust >= 0.40 && arousal >= 0.40 {
        0.55 + 0.25 * disgust
    } else {
        0.0
    };
    let triumph = if valence >= 0.62 && arousal >= 0.45 {
        0.48 + 0.2 * valence
    } else {
        0.0
    };
    let vow = if permanence >= 0.75 { permanence } else { 0.0 };
    let selfhood = if self_rel >= 0.85 && valence.abs() >= 0.5 {
        0.35
    } else {
        0.0
    };
    trauma.max(triumph).max(vow).max(selfhood).min(1.0)
}

pub fn anchor_score(trace: &MemoryTrace, axioms: &[&IdentityAxiom]) -> f32 {
    if trace.channel.verbatim() {
        return 0.0;
    }
    let seed = seed_anchor(
        trace.valence,
        trace.arousal,
        trace.disgust,
        trace.permanence,
        trace.self_relevance,
    );
    let axiomatic = axioms.iter().any(|a| {
        a.superseded_by.is_none() && a.support_trace_ids.iter().any(|id| id == &trace.id)
    });
    let ax = if axiomatic { 0.72 } else { 0.0 };
    seed.max(ax).max(trace.anchor)
}

pub fn apply_anchors(store: &mut MemoryStore) {
    let axioms: Vec<IdentityAxiom> = store.living_axioms().into_iter().cloned().collect();
    let refs: Vec<&IdentityAxiom> = axioms.iter().collect();
    for t in store.traces.values_mut() {
        let a = anchor_score(t, &refs);
        if a > t.anchor {
            t.anchor = a;
        }
        t.clamp();
    }
}

fn jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.iter().filter(|t| b.binary_search(t).is_ok()).count();
    let union = a.len() + b.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}
