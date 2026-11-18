//! Supplement targeted recall with surviving, accessible lived traces.
use selmem::{Narrator, RuleNarrator, SelectiveMemory, TraceStatus};
use std::collections::BTreeSet;

pub fn retained(memory: &SelectiveMemory) -> Vec<String> {
    let mut traces: Vec<_> = memory
        .store
        .traces
        .values()
        .filter(|trace| {
            !trace.suppressed
                && !trace.channel.verbatim()
                && trace.status != TraceStatus::Latent
                && trace.access > 0.0
        })
        .collect();
    let importance = |trace: &selmem::MemoryTrace| trace.access * trace.salience_at_encode;
    traces.sort_by(|a, b| {
        importance(b)
            .total_cmp(&importance(a))
            .then(a.created_at.cmp(&b.created_at))
            .then(a.core.cmp(&b.core))
            .then(a.gist.cmp(&b.gist))
    });
    let mut seen = BTreeSet::new();
    traces
        .iter()
        .filter_map(|trace| {
            let text = RuleNarrator.reconstruct(trace, &Default::default(), "");
            let text = text.trim();
            (!text.is_empty() && seen.insert(text.to_owned())).then(|| text.to_owned())
        })
        .collect()
}

pub fn extend(mut selected: String, retained: &[String]) -> String {
    let mut seen: BTreeSet<String> = selected.split("\n\n").map(str::to_owned).collect();
    for text in retained {
        if seen.insert(text.clone()) {
            if !selected.is_empty() {
                selected.push_str("\n\n");
            }
            selected.push_str(text);
        }
    }
    selected
}
