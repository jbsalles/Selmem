//! Selection policy; supports transfer, marked traces, and diversity.
use super::*;

pub(super) fn choose_trace_ids(
    ranked: &[ScoredTrace],
    store: &MemoryStore,
    profile: &EntityProfile,
    query: &str,
    bias: RecallBias,
    marked: &[String],
) -> Vec<String> {
    let eligible: Vec<String> = ranked
        .iter()
        .filter(|c| eligible(c))
        .map(|c| c.trace_id.clone())
        .collect();

    let chosen_ids = if bias == RecallBias::Transfer {
        select_transfer(ranked, store, profile.max_recall)
    } else if bias != RecallBias::ForceMarked {
        let allowed = apply_bias(&eligible, store, bias, marked, eligible.len());
        select_diverse_for_query(ranked, &allowed, store, profile.max_recall, query)
    } else {
        apply_bias(&eligible, store, bias, marked, profile.max_recall)
    };

    chosen_ids
}
