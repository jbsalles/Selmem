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
    let p: String = p.chars().take(500).collect();
    let ev = content_tokens(event);
    let pr = content_tokens(&p);
    if pr.is_empty() {
        return None;
    }
    if pr.iter().any(|w| !ev.iter().any(|e| e == w)) {
        return None;
    }
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
        " not ", " never ", " no ", " didn't ", " didnt ", " dont ", " don't ",
        " cannot ", " can't ", " cant ", " won't ", " wont ", " without ",
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
        t.semantic.claim = t.core.clone();
    }
}
