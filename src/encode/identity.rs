use crate::core::model::{AxiomLayer, Mood};
use crate::core::store::MemoryStore;
use crate::encode::EncodeInput;

/// Living identity colors a new event before the salience gate.
pub fn paint(store: &mut MemoryStore, mood: &Mood, input: &mut EncodeInput<'_>) {
    let axioms = store.living_axioms();
    if axioms.is_empty() {
        input.valence = (input.valence + 0.15 * mood.valence).clamp(-1.0, 1.0);
        input.arousal = (input.arousal + 0.08 * mood.arousal).clamp(0.0, 1.0);
        paint_latent(store, input);
        input.self_congruence = measure_congruence(store, input.schema.as_deref(), input.valence);
        return;
    }

    let event = input.event.to_lowercase();
    let mut pull_v = 0.0;
    let mut pull_d = 0.0;
    let mut pull_s = 0.0;
    let mut schema_hit: Option<String> = None;
    let mut hits = 0.0;

    for a in axioms {
        let w = match a.layer {
            AxiomLayer::Trait => 1.0,
            AxiomLayer::Belief => 0.7,
            AxiomLayer::Motif => 0.35,
        } * a.strength.max(0.15);
        let stmt = a.statement.to_lowercase();
        let schema = a.schema.as_deref().unwrap_or("");
        let related = event_related(&event, schema, &stmt);
        if !related {
            continue;
        }
        hits += w;
        pull_v += w * a.valence;
        if a.valence < -0.2 {
            pull_d += w * 0.35;
        }
        pull_s += w;
        if schema_hit.is_none() && !schema.is_empty() {
            schema_hit = Some(schema.to_string());
        }
    }

    if hits > 0.0 {
        let v = pull_v / hits;
        input.valence = (0.65 * input.valence + 0.35 * v).clamp(-1.0, 1.0);
        input.disgust = (input.disgust + pull_d / hits.max(1.0)).clamp(0.0, 1.0);
        input.self_relevance = (input.self_relevance + 0.15 * pull_s.min(1.0)).clamp(0.0, 1.0);
        input.goal_align = (input.goal_align + 0.2 * pull_s.min(1.0)).clamp(0.0, 1.0);
        if input.schema.is_none() {
            input.schema = schema_hit;
        }
    }

    input.valence = (input.valence + 0.12 * mood.valence).clamp(-1.0, 1.0);
    input.arousal = (input.arousal + 0.08 * mood.arousal).clamp(0.0, 1.0);

    paint_latent(store, input);
    input.self_congruence = measure_congruence(store, input.schema.as_deref(), input.valence);
}

/// Does this hour describe the living self? Schema axioms only. No axiom +
/// a charged valence cannot sit anywhere → low congruence.
pub fn measure_congruence(store: &MemoryStore, schema: Option<&str>, valence: f32) -> f32 {
    let mut n = 0u32;
    let mut acc = 0.0f32;
    if let Some(s) = schema {
        for a in store.living_axioms() {
            if a.schema.as_deref() == Some(s) {
                n += 1;
                acc += a.valence;
            }
        }
    }
    if n == 0 {
        return if valence.abs() >= 0.40 { 0.22 } else { 0.50 };
    }
    let identity_v = acc / n as f32;
    let align = (identity_v * valence).clamp(-1.0, 1.0);
    (0.50 + 0.50 * align).clamp(0.0, 1.0)
}

/// Scene forgotten, charge still pulls the next event.
fn paint_latent(store: &mut MemoryStore, input: &mut EncodeInput<'_>) {
    use crate::core::model::TraceStatus;
    let event = input.event.to_lowercase();
    let mut pull_v = 0.0;
    let mut pull_d = 0.0;
    let mut n = 0.0;
    let mut touched: Vec<String> = Vec::new();
    for t in store.traces.values() {
        if t.status != TraceStatus::Latent {
            continue;
        }
        let schema = t.schema.as_deref().unwrap_or("");
        if !event_related(&event, schema, &t.core) && !event_related(&event, schema, &t.gist) {
            continue;
        }
        let strong = schema_or_key(&event, schema, &t.core) || schema_or_key(&event, schema, &t.gist);
        n += 1.0;
        pull_v += t.valence;
        pull_d += t.disgust;
        if strong {
            touched.push(t.id.clone());
            if input.schema.is_none() && !schema.is_empty() {
                input.schema = Some(schema.to_string());
            }
        }
    }
    for id in touched {
        if let Some(t) = store.traces.get_mut(&id) {
            t.rehearsals = t.rehearsals.saturating_add(1);
            t.access = (t.access + 0.08).clamp(0.0, 1.0);
            t.fidelity = (t.fidelity + 0.04).clamp(0.0, 1.0);
        }
    }
    if n > 0.0 {
        input.valence = (0.7 * input.valence + 0.3 * (pull_v / n)).clamp(-1.0, 1.0);
        input.disgust = (input.disgust + 0.25 * (pull_d / n)).clamp(0.0, 1.0);
    }
}

fn schema_or_key(event: &str, schema: &str, statement: &str) -> bool {
    if !schema.is_empty() && event.contains(&schema.to_lowercase()) {
        return true;
    }
    let keys = [
        "fidél", "loyal", "stayed", "abandon", "parti", "left", "trahi", "betray",
        "humili", "confian", "trust", "pluie", "rain", "aimer", "love", "peur", "fear",
        "honte", "shame",
    ];
    keys.iter().any(|k| event.contains(k) && (statement.contains(k) || schema.contains(k)))
}

fn event_related(event: &str, schema: &str, statement: &str) -> bool {
    if schema_or_key(event, schema, statement) {
        return true;
    }
    // Long-word overlap tints the next input. It does not rehearse, so it cannot revive.
    statement.split_whitespace().any(|w| w.len() > 5 && event.contains(w))
}
