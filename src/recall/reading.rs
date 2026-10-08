//! Reading profile. A disposition derived from the book, never from the seal.
//! The mouth may speak from it. It is not an order and not a verb map.

use crate::core::model::{AxiomLayer, Mood, TraceStatus};
use crate::core::store::MemoryStore;

#[derive(Clone, Debug, Default)]
pub struct ReadingProfile {
    pub mood: String,
    pub valence_bias: f32,
    pub salient: Vec<SalientMark>,
    pub axioms: Vec<String>,
    pub stakes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SalientMark {
    pub id: String,
    pub fidelity: f32,
    pub access: f32,
    pub valence: f32,
}

impl ReadingProfile {
    pub fn from_book(store: &MemoryStore, mood: &Mood) -> Self {
        let salient = salient_marks(store);
        let bias = valence_bias(mood, &salient);
        let axioms = living_lines(store);
        let stakes = stakes_from(store, &axioms);
        Self {
            mood: mood_label(mood, bias),
            valence_bias: bias,
            salient,
            axioms,
            stakes,
        }
    }

    /// Personal disposition can transfer across topics. External observations
    /// contribute only when selected and relevant to the named query subject.
    pub fn for_query(store: &MemoryStore, mood: &Mood, query: &str, selected: &[String]) -> Self {
        let query_words = crate::encode::scoring::token_set(query);
        let mut scoped = MemoryStore::new();
        for (id, t) in &store.traces {
            if t.channel != crate::core::model::Channel::Selfhood {
                continue;
            }
            if t.suppressed || t.status == TraceStatus::Latent {
                continue;
            }
            if t.attribution == crate::core::model::Attribution::External {
                if !selected.contains(id) {
                    continue;
                }
                let Some((speaker, _)) = t.core.split_once(" said:") else {
                    continue;
                };
                let names = crate::encode::scoring::token_set(speaker);
                if names.is_empty() || !names.iter().all(|n| query_words.contains(n)) {
                    continue;
                }
                if crate::recall::retrieve::topic_relevance(query, &t.core) <= 0.0 {
                    continue;
                }
            }
            scoped.traces.insert(id.clone(), t.clone());
        }
        for (id, a) in &store.axioms {
            if !a.support_trace_ids.is_empty()
                && a.support_trace_ids
                    .iter()
                    .all(|id| scoped.traces.contains_key(id))
            {
                scoped.axioms.insert(id.clone(), a.clone());
            }
        }
        if scoped.traces.is_empty() {
            return Self::empty();
        }
        let observed_only = scoped
            .traces
            .values()
            .all(|t| t.attribution == crate::core::model::Attribution::External);
        let neutral = Mood::default();
        let scoped_mood = if observed_only { &neutral } else { mood };
        let mut reading = Self::from_book(&scoped, scoped_mood);
        let weight: f32 = reading.salient.iter().map(|m| m.fidelity * m.access).sum();
        if weight > 0.0 {
            reading.valence_bias = reading
                .salient
                .iter()
                .map(|m| m.valence * m.fidelity * m.access)
                .sum::<f32>()
                / weight;
            reading.mood = mood_label(scoped_mood, reading.valence_bias);
        }
        reading
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.mood.is_empty() && self.salient.is_empty() && self.axioms.is_empty()
    }

    /// Disposition block. Ids and numbers, never an hour and never the seal.
    pub fn render(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut out = format!(
            "[mood: {}] [valence_bias: {:.2}]",
            self.mood, self.valence_bias
        );
        if !self.salient.is_empty() {
            out.push_str(" [salient:");
            for m in &self.salient {
                out.push_str(&format!(
                    " {{id:{}, fidelity:{:.2}, access:{:.2}, valence:{:.2}}}",
                    m.id, m.fidelity, m.access, m.valence
                ));
            }
            out.push(']');
        }
        if !self.axioms.is_empty() {
            out.push_str(" [axioms:");
            for a in &self.axioms {
                out.push(' ');
                out.push_str(a);
            }
            out.push(']');
        }
        if !self.stakes.is_empty() {
            out.push_str(" [recent_stakes:");
            for s in &self.stakes {
                out.push(' ');
                out.push_str(s);
            }
            out.push(']');
        }
        out
    }
}

fn mood_label(mood: &Mood, bias: f32) -> String {
    if mood.disgust >= 0.45 || bias <= -0.15 {
        "guarded".into()
    } else if bias >= 0.15 {
        "open".into()
    } else {
        "level".into()
    }
}

fn valence_bias(mood: &Mood, salient: &[SalientMark]) -> f32 {
    if salient.is_empty() {
        return mood.valence;
    }
    let acc: f32 = salient.iter().map(|m| m.valence).sum();
    acc / salient.len() as f32
}

fn salient_marks(store: &MemoryStore) -> Vec<SalientMark> {
    let mut marks: Vec<SalientMark> = store
        .traces
        .values()
        .filter(|t| t.status != TraceStatus::Latent)
        .filter(|t| t.valence.abs() >= 0.40 || t.self_relevance >= 0.80)
        .map(|t| SalientMark {
            id: t.id.clone(),
            fidelity: t.fidelity,
            access: t.access,
            valence: t.valence,
        })
        .collect();
    marks.sort_by(|a, b| {
        b.valence
            .abs()
            .total_cmp(&a.valence.abs())
            .then_with(|| store.traces[&a.id].core.cmp(&store.traces[&b.id].core))
            .then_with(|| b.valence.total_cmp(&a.valence))
            .then_with(|| b.fidelity.total_cmp(&a.fidelity))
            .then_with(|| b.access.total_cmp(&a.access))
            .then(a.id.cmp(&b.id))
    });
    marks.truncate(4);
    marks
}

fn living_lines(store: &MemoryStore) -> Vec<String> {
    let mut axioms: Vec<_> = store
        .axioms
        .values()
        .filter(|a| a.superseded_by.is_none())
        .collect();
    axioms.sort_by(|a, b| {
        let rank = |l| match l {
            AxiomLayer::Trait => 2,
            AxiomLayer::Belief => 1,
            AxiomLayer::Motif => 0,
        };
        rank(b.layer)
            .cmp(&rank(a.layer))
            .then_with(|| b.strength.total_cmp(&a.strength))
            .then(a.statement.cmp(&b.statement))
            .then(a.id.cmp(&b.id))
    });
    axioms
        .into_iter()
        .take(4)
        .map(|a| a.statement.clone())
        .collect()
}

/// Schema and sense only. The hour text is not a stake name.
fn stakes_from(store: &MemoryStore, axioms: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut ordered: Vec<_> = store.axioms.values().collect();
    ordered.sort_by(|a, b| {
        a.statement
            .cmp(&b.statement)
            .then_with(|| b.valence.total_cmp(&a.valence))
            .then(a.id.cmp(&b.id))
    });
    for a in ordered {
        if a.superseded_by.is_some() {
            continue;
        }
        if a.layer != AxiomLayer::Belief && a.strength < 0.40 {
            continue;
        }
        let schema = a.schema.clone().unwrap_or_else(|| "unnamed".into());
        let sense = if a.valence <= -0.15 {
            "against"
        } else if a.valence >= 0.15 {
            "for"
        } else {
            "under"
        };
        let line = format!("{schema} {sense}");
        if !out.contains(&line) {
            out.push(line);
        }
        if out.len() == 3 {
            break;
        }
    }
    let _ = axioms;
    out
}

/// Direction of a reply, judged on register words only. Not an act map.
pub fn direction_score(reply: &str) -> f32 {
    let b = reply.to_lowercase();
    let mut score = 0.0;
    for w in ["room", "welcome", "ease", "open"] {
        if b.contains(w) {
            score += 1.0;
        }
    }
    for w in ["measure", "distance", "guard", "tight"] {
        if b.contains(w) {
            score -= 1.0;
        }
    }
    score
}

pub fn register_marks(reply: &str) -> (usize, f32) {
    let words = reply.split_whitespace().count();
    let hedges = ["before", "still", "measure", "yet"]
        .iter()
        .filter(|w| reply.to_lowercase().contains(*w))
        .count() as f32;
    (words, hedges)
}
