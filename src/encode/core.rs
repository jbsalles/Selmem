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
    Some(p)
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
