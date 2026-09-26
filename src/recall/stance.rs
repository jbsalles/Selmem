//! Ablation helpers: abstract charge without the episode gist.
//!
//! Default `speak_isolated` does not use these. Persist showed that withholding
//! the scene makes C2 A ≈ C2 B; the live path keeps retrieved narratives.

use crate::core::model::{MemoryTrace, Mood, TraceStatus};
use crate::core::store::MemoryStore;

pub fn is_charged(trace: &MemoryTrace) -> bool {
    if trace.channel.verbatim() {
        return false;
    }
    if matches!(trace.status, TraceStatus::Myth) {
        return false;
    }
    if trace.schema.as_deref() == Some("daily") {
        return false;
    }
    trace.disgust >= 0.25
        || trace.valence <= -0.40
        || trace.valence >= 0.55
        || trace.anchor >= 0.70
}

pub fn charged_mood(store: &MemoryStore) -> Mood {
    let mut v = 0.0;
    let mut a = 0.0;
    let mut d = 0.0;
    let mut n = 0.0;
    for t in store.traces.values() {
        if !is_charged(t) {
            continue;
        }
        n += 1.0;
        v += t.valence;
        a += t.arousal;
        d += t.disgust;
    }
    if n < 1.0 {
        return Mood::default();
    }
    Mood {
        valence: v / n,
        arousal: a / n,
        disgust: d / n,
    }
}

/// One line per charged schema. No episode wording.
pub fn isolated_stance(store: &MemoryStore) -> Vec<String> {
    let mut best: Vec<(String, f32, f32, f32)> = Vec::new();
    for t in store.traces.values() {
        if !is_charged(t) {
            continue;
        }
        let schema = t
            .schema
            .clone()
            .unwrap_or_else(|| "self".to_string());
        let weight = t.disgust + t.valence.abs() + t.anchor;
        if let Some(row) = best.iter_mut().find(|r| r.0 == schema) {
            if weight > row.1 {
                *row = (schema, weight, t.valence, t.disgust);
            }
        } else {
            best.push((schema, weight, t.valence, t.disgust));
        }
    }
    best.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    best.into_iter()
        .take(3)
        .filter_map(|(schema, _, valence, disgust)| phrase(&schema, valence, disgust))
        .collect()
}

fn phrase(schema: &str, valence: f32, disgust: f32) -> Option<String> {
    let s = schema.to_lowercase();
    if s == "daily" {
        return None;
    }
    let line = if s.contains("injust") || s.contains("unfair") || s.contains("betray") {
        if valence < 0.0 || disgust > 0.2 {
            "Others can discard what you already gave."
        } else {
            "How other people treat what you gave still matters."
        }
    } else if s.contains("reconnaissance")
        || s.contains("recogn")
        || s.contains("praise")
    {
        "What you already gave has been kept in view."
    } else if valence <= -0.4 || disgust >= 0.25 {
        "A sour self-relevant charge is still on."
    } else if valence >= 0.55 {
        "A warm self-relevant charge is still on."
    } else {
        return None;
    };
    Some(line.to_string())
}

pub fn query_hits_episode(trace: &MemoryTrace, query: &str) -> bool {
    let q = tokens(query);
    if q.is_empty() {
        return false;
    }
    let blob = format!("{} {}", trace.core, trace.gist);
    let t = tokens(&blob);
    let hits = q.iter().filter(|w| t.iter().any(|x| x == *w)).count();
    hits >= 2
}

fn tokens(s: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "a", "an", "the", "to", "of", "and", "or", "in", "on", "for", "you", "your",
        "that", "this", "with", "without", "what", "how", "do", "does", "make",
        "makes", "is", "are", "be", "it", "i", "i'd", "i’m", "we", "they",
    ];
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP.contains(w))
        .map(|w| w.to_string())
        .collect()
}

pub fn stance_is_abstract(line: &str) -> bool {
    let low = line.to_lowercase();
    !low.contains("cancel")
        && !low.contains("annul")
        && !low.contains("unjust")
        && !low.contains("injust")
        && !low.contains("project")
        && !low.contains("projet")
        && !low.contains("colleague")
        && !low.contains("notice")
}
