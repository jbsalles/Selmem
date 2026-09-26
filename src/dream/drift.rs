use crate::core::model::{now_secs, DriftEvent, DriftKind, MemoryTrace};
use crate::core::profile::EntityProfile;

pub fn apply_reconsolidation(
    trace: &mut MemoryTrace,
    narrative: &str,
    profile: &EntityProfile,
    mood_valence: f32,
) {
    if trace.channel.verbatim() {
        return;
    }
    let resist = 1.0 - 0.75 * trace.anchor;
    let eta = profile.reconsolidation_eta * (0.5 + 0.5 * trace.arousal) * resist;
    if !narrative.is_empty() && narrative != trace.gist && eta >= 0.15 {
        // Spoken recall may wander. The book only moves if the sentence
        // still talks about this hour. Core is never written here.
        if crate::encode::accept_core(narrative, &trace.core).is_some()
            || crate::encode::accept_core(narrative, &trace.gist).is_some()
        {
            trace.gist = blend_text(&trace.gist, narrative, eta);
        }
    }
    let before_v = trace.valence;
    let pull = eta * 0.32 * resist * (mood_valence - trace.valence);
    trace.valence = (trace.valence + pull).clamp(-1.0, 1.0);
    if mood_valence < -0.25 && trace.valence < 0.0 {
        trace.disgust = (trace.disgust + eta * 0.08 * resist).min(profile.disgust_cap);
    }
    if (trace.valence - before_v).abs() >= 0.06 {
        trace.drifts.push(DriftEvent {
            kind: DriftKind::Reinterpret,
            at: now_secs(),
            note: format!("sense shifted ({before_v:.2} → {:.2})", trace.valence),
            fidelity_delta: -0.02,
            valence_delta: trace.valence - before_v,
            disgust_delta: 0.0,
        });
    }
    let loss = profile.fidelity_loss_on_recall * resist;
    trace.fidelity = (trace.fidelity - loss).max(0.15);
    trace.clamp();
}

/// Stabilité S en jours. Plus S est grand, plus le détail tient.
pub fn stability_days(trace: &MemoryTrace, profile: &EntityProfile) -> f32 {
    let base = 1.0 / profile.decay_lambda.max(0.02);
    let hold = 0.28 + 0.72 * trace.salience_at_encode.clamp(0.0, 1.0);
    base * hold
        * (1.0 + 0.8 * trace.arousal)
        * (1.0 + 2.2 * trace.permanence)
        * (1.0 + 5.0 * trace.anchor)
        * (1.0 + 0.35 * (1.0 + trace.rehearsals as f32).ln())
}

/// Retention du détail (Ebbinghaus) : exp(-t / S).
/// La charge sémantique (`core`) n'entre pas dans cette courbe.
pub fn detail_retention(trace: &MemoryTrace, profile: &EntityProfile, now: u64) -> f32 {
    let t_days = (now.saturating_sub(trace.created_at) as f32) / 86_400.0;
    let s = stability_days(trace, profile).max(0.2);
    (-t_days / s).exp().clamp(0.0, 1.0)
}

/// Fait tomber le détail du gist vers le core. Ne touche pas valence/schema/core.
pub fn weather(trace: &mut MemoryTrace, profile: &EntityProfile) -> Option<DriftEvent> {
    if trace.channel.verbatim() {
        return None;
    }
    let r = detail_retention(trace, profile, now_secs());
    let floor = 0.15 + 0.85 * trace.anchor * 0.5;
    let target = (0.18 + 0.82 * r).max(floor);
    if target >= trace.fidelity - 0.008 {
        return None;
    }
    let old_f = trace.fidelity;
    trace.fidelity = target;
    // Charged Selfhood keeps its gist; fidelity may still fall.
    let charged = trace.self_relevance >= 0.80 && trace.valence.abs() >= 0.40;
    if !trace.core.is_empty() && !charged {
        trace.gist = fade_gist(&trace.gist, &trace.core, trace.fidelity);
    }
    let event = DriftEvent {
        kind: DriftKind::Weather,
        at: now_secs(),
        note: format!("ebbinghaus r={r:.2}"),
        fidelity_delta: trace.fidelity - old_f,
        valence_delta: 0.0,
        disgust_delta: 0.0,
    };
    trace.drifts.push(event.clone());
    trace.clamp();
    Some(event)
}

fn fade_gist(gist: &str, core: &str, fid: f32) -> String {
    if fid >= 0.62 {
        return gist.to_string();
    }
    if fid >= 0.38 {
        let first = gist
            .split([',', '—', '/'])
            .next()
            .unwrap_or(gist)
            .trim()
            .trim_end_matches('.');
        if first.chars().count() >= 8 {
            format!("{first}.")
        } else {
            core.to_string()
        }
    } else {
        core.to_string()
    }
}

/// Profile-directed retelling of the *stored* gist. Not a wrapper.
/// Tender (embellish > disgust): gild, keep presence.
/// Austere (disgust > embellish): harden, name the failure.
pub fn retell(gist: &str, profile: &EntityProfile, valence: f32, disgust: f32) -> String {
    let v = &profile.voice;
    if v.is_empty() {
        return gist.to_string();
    }
    if !v.marker.is_empty() && gist.contains(&v.marker) {
        return gist.to_string();
    }
    let mut s = gist.trim().trim_end_matches('.').to_string();
    for (from, to) in &v.replacements {
        if !from.is_empty() {
            s = s.replace(from, to);
        }
    }
    let tail = if valence >= 0.1 && disgust < 0.25 {
        v.suffix_warm.as_str()
    } else {
        v.suffix_cold.as_str()
    };
    if !tail.is_empty() {
        s.push_str(". ");
        s.push_str(tail.trim_start_matches(". "));
    }
    if !s.ends_with('.') {
        s.push('.');
    }
    clip(&s, 240)
}

pub fn sculpt(trace: &mut MemoryTrace, profile: &EntityProfile) -> Option<DriftEvent> {
    if trace.channel.verbatim() {
        return None;
    }
    let resist = 1.0 - 0.7 * trace.anchor;
    let charged = trace.self_relevance >= 0.80 && trace.valence.abs() >= 0.40;
    let told = retell(&trace.gist, profile, trace.valence, trace.disgust);
    let text_changed = told != trace.gist && !charged;
    if text_changed {
        trace.gist = told;
        trace.fidelity = (trace.fidelity - 0.03 * resist).max(0.15);
    }

    if trace.disgust >= 0.35 || trace.valence <= -0.45 {
        let delta = profile.disgust_gain * (0.5 + 0.5 * trace.arousal) * resist;
        if delta < 0.004 {
            if text_changed {
                return text_drift(trace);
            }
            return None;
        }
        let old = trace.disgust;
        trace.disgust = (trace.disgust + delta).min(profile.disgust_cap);
        trace.valence = (trace.valence - 0.35 * delta).max(-1.0);
        trace.fidelity = (trace.fidelity - 0.03 * resist).max(0.15);
        let event = DriftEvent {
            kind: DriftKind::AmplifyDisgust,
            at: now_secs(),
            note: "withdrawal amplified".into(),
            fidelity_delta: -0.03 * resist,
            valence_delta: -0.35 * delta,
            disgust_delta: trace.disgust - old,
        };
        trace.drifts.push(event.clone());
        trace.clamp();
        return Some(event);
    }

    if trace.valence >= 0.25 && trace.disgust < 0.25 {
        let gain = profile.embellish_gain * (0.4 + 0.6 * trace.self_relevance) * resist;
        if gain < 0.004 {
            if text_changed {
                return text_drift(trace);
            }
            return None;
        }
        trace.valence = (trace.valence + gain).min(1.0);
        trace.fidelity = (trace.fidelity - 0.04 * resist).max(0.15);
        let event = DriftEvent {
            kind: DriftKind::Embellish,
            at: now_secs(),
            note: "embellissement identitaire".into(),
            fidelity_delta: -0.04 * resist,
            valence_delta: gain,
            disgust_delta: 0.0,
        };
        trace.drifts.push(event.clone());
        trace.clamp();
        return Some(event);
    }

    if text_changed {
        return text_drift(trace);
    }
    None
}

fn text_drift(trace: &mut MemoryTrace) -> Option<DriftEvent> {
    let event = DriftEvent {
        kind: DriftKind::Rewrite,
        at: now_secs(),
        note: "retell".into(),
        fidelity_delta: -0.03,
        valence_delta: 0.0,
        disgust_delta: 0.0,
    };
    trace.drifts.push(event.clone());
    trace.clamp();
    Some(event)
}

fn blend_text(old: &str, new: &str, eta: f32) -> String {
    let eta = eta.clamp(0.0, 1.0);
    if eta < 0.12 {
        return old.to_string();
    }
    if eta > 0.82 {
        return clip(new, 220);
    }
    let old_bits = split_bits(old);
    let new_bits = split_bits(new);
    if old_bits.is_empty() {
        return clip(new, 220);
    }
    if new_bits.is_empty() {
        return old.to_string();
    }
    let keep_old = ((1.0 - eta) * old_bits.len() as f32).round() as usize;
    let take_new = (eta * new_bits.len() as f32).round().max(1.0) as usize;
    let mut out = Vec::new();
    out.extend(old_bits.into_iter().take(keep_old.max(1)));
    for s in new_bits.into_iter().take(take_new) {
        if !out.iter().any(|o| o == &s) {
            out.push(s);
        }
    }
    clip(&out.join(". "), 220)
}

fn split_bits(s: &str) -> Vec<String> {
    s.split(|c| c == '.' || c == '—' || c == ';' || c == '/')
        .map(|p| p.trim().trim_end_matches('.').to_string())
        .filter(|p| p.chars().count() >= 4)
        .collect()
}

fn clip(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= n {
        t.to_string()
    } else {
        t.chars().take(n).collect()
    }
}
