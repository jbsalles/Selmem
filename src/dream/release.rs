//! Last night pass: a spent latent hour may leave.
//! Only traces that were already latent *before* this night.

use crate::core::model::TraceStatus;
use crate::core::store::MemoryStore;

/// Scene already gone, charge unused, no living axiom leans on it → leave the book.
/// Not a cap. Benches never reach this state.
pub fn run(
    store: &mut MemoryStore,
    previously_latent: &std::collections::HashSet<String>,
) -> u32 {
    let supported: std::collections::HashSet<String> = store
        .living_axioms()
        .into_iter()
        .flat_map(|a| a.support_trace_ids.iter().cloned())
        .collect();
    let drop: Vec<String> = store
        .traces
        .values()
        .filter(|t| {
            previously_latent.contains(&t.id)
                && !t.channel.verbatim()
                && t.status == TraceStatus::Latent
                && t.access < 0.10
                && t.anchor < 0.50
                && t.permanence < 0.80
                && !supported.contains(&t.id)
        })
        .map(|t| t.id.clone())
        .collect();
    let n = drop.len() as u32;
    for id in drop {
        store.release_trace(&id);
    }
    n
}
