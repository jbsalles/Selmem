//! Fact-core after the gate. Words must already be in the hour.

use crate::core::store::MemoryStore;
use crate::encode::EncodeDecision;
use crate::recall::narrator::Narrator;

fn content_tokens(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.chars().count() > 2)
        .collect()
}

/// The core may drop words. It may not introduce any.
/// That blocks pretrained "past" the hour never said.
pub fn accept_core(proposed: &str, event: &str) -> Option<String> {
    let p = proposed.trim();
    if p.is_empty() || p.chars().count() < 8 {
        return None;
    }
    if p.chars().count() > 1000 { return None; }
    let p = p.to_string();
    if let Some((speaker, _)) = reported_speech(event) {
        if reported_speech(&p).map(|(s, _)| s) != Some(speaker) { return None; }
    }
    let ev = content_tokens(event);
    let pr = content_tokens(&p);
    if pr.is_empty() {
        return None;
    }
    if pr.iter().any(|w| !ev.iter().any(|e| e == w)) {
        return None;
    }
    // Preserve source order, including short names and pronouns.
    let words = |s: &str| -> Vec<String> {
        s.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| !w.is_empty()).collect()
    };
    let source = words(event);
    let mut remaining = source.iter();
    if words(&p).iter().any(|w| !remaining.any(|e| e == w)) { return None; }
    if polarity_flipped(event, &p) {
        return None;
    }
    Some(p)
}

/// Global negation, plus a scoped one: "not WORD" in the hour must not come back as bare WORD.
fn polarity_flipped(event: &str, proposed: &str) -> bool {
    if has_neg(event) != has_neg(proposed) {
        return true;
    }
    let ev = event.to_lowercase();
    let pr = proposed.to_lowercase();
    for w in content_tokens(&ev) {
        let marked = format!("not {w}");
        let bare = pr.split(|c: char| c == ',' || c == ';').any(|clause| {
            clause.contains(&w) && !clause.contains("not ") && !clause.contains("n't")
        });
        if ev.contains(&marked) && bare {
            return true;
        }
    }
    false
}

fn has_neg(s: &str) -> bool {
    let low = format!(" {} ", s.to_lowercase());
    const NEG: &[&str] = &[
        " not ",
        " never ",
        " no ",
        " didn't ",
        " didnt ",
        " dont ",
        " don't ",
        " cannot ",
        " can't ",
        " cant ",
        " won't ",
        " wont ",
        " without ",
    ];
    NEG.iter().any(|n| low.contains(n)) || low.contains("n't")
}

/// After a keep: narrator may propose a core. Multi-part pastes and talk hours skip.
pub fn maybe_set_core(
    store: &mut MemoryStore,
    narrator: &dyn Narrator,
    decision: &EncodeDecision,
    source: &str,
    event: &str,
) {
    if !decision.kept || decision.parts > 1 || source == "talk" {
        return;
    }
    let Some(tid) = decision.trace_id.as_deref() else {
        return;
    };
    let Some(raw) = narrator.extract_core(event) else {
        return;
    };
    let Some(ok) = accept_core(&raw, event) else {
        return;
    };
    if let Some(t) = store.traces.get_mut(tid) {
        t.core = ok;
        if !t.channel.verbatim() && reported_speech(event).is_some() {
            t.gist = t.core.clone();
            t.interpretation.statement = t.core.clone();
        }
        t.semantic.claim = t.core.clone();
        t.reality.claim = t.core.clone();
    }
}

/// Bounded extractive baseline. Select a complete informative sentence instead
/// of spending the core budget on a greeting. Learned/annotated claims take
/// precedence at the gate; this fallback does not infer a psychology.
fn unframed_core(event: &str) -> String {
    let candidates: Vec<&str> = event
        .split_inclusive(['.', '!', '?', ';', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let best = candidates
        .iter()
        .enumerate()
        .filter(|(_, s)| s.split_whitespace().count() <= 32)
        .filter(|(_, s)| !polarity_flipped(event, s))
        .max_by_key(|(i, s)| (content_tokens(s).len(), std::cmp::Reverse(*i)))
        .map(|(_, s)| *s);
    if let Some(sentence) = best {
        return sentence.to_string();
    }
    // Long indivisible sentences remain lossy. Do not claim that this baseline
    // performs learned semantic compression or restores forgotten details.
    event
        .split_whitespace()
        .take(32)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Explicit quoted observations retain their speaker independently of first person speech.
pub fn reported_speech(event: &str) -> Option<(&str, &str)> {
    let (speaker, body) = event.split_once(" said:")?;
    let speaker = speaker.trim();
    if speaker.is_empty() || speaker.split_whitespace().count() > 6
        || speaker.chars().any(|c| !(c.is_alphabetic() || c == ' ' || c == '-' || c == '\''))
        || matches!(speaker.to_lowercase().as_str(), "i" | "we" | "you") {
        return None;
    }
    Some((speaker, body.trim()))
}

/// Reported observations keep complete sentences in source order within 64 words.
/// Unframed experiences retain the existing 32-word selective baseline.
pub fn extractive_core(event: &str) -> String {
    let Some((speaker, body)) = reported_speech(event) else { return unframed_core(event); };
    let greeting = |s: &str| {
        let clean = s.trim_matches(|c: char| !c.is_alphanumeric());
        let low = clean.to_lowercase();
        if matches!(low.as_str(), "how are you" | "how are you doing" | "how're ya doin" | "how are ya") { return true; }
        let mut words = clean.split_whitespace();
        let first = words.next().unwrap_or("").to_lowercase();
        if !matches!(first.as_str(), "hello" | "hi" | "hey") { return false; }
        match (words.next(), words.next()) {
            (None, None) => true,
            (Some(name), None) => name == "there" || name.chars().next().is_some_and(char::is_uppercase),
            _ => false,
        }
    };
    let mut out = format!("{speaker} said: ");
    let mut started = false;
    for sentence in body.split_inclusive(['.', '!', '?', ';', '\n']).map(str::trim).filter(|s| !s.is_empty()) {
        if !started && greeting(sentence) { continue; }
        started = true;
        let available = 64usize.saturating_sub(out.split_whitespace().count());
        if sentence.split_whitespace().count() > available {
            if out.ends_with(": ") {
                out.push_str(&sentence.split_whitespace().take(available).collect::<Vec<_>>().join(" "));
            }
            break;
        }
        if !out.ends_with(' ') { out.push(' '); }
        out.push_str(sentence);
    }
    if !started { return event.split_whitespace().take(64).collect::<Vec<_>>().join(" "); }
    out
}
