pub mod file;
pub mod snapshot;
pub mod sqlite;

pub use file::{load, save};
pub use snapshot::Snapshot;

use crate::core::model;
use crate::core::store::MemoryStore;

pub fn bump_id_counter(store: &MemoryStore) {
    let mut max = 0u64;
    for id in store
        .traces
        .keys()
        .chain(store.archives.keys())
        .chain(store.axioms.keys())
    {
        if let Some(hex) = id.rsplit('_').next() {
            if let Ok(n) = u64::from_str_radix(hex, 16) {
                max = max.max(n);
            }
        }
    }
    model::set_next_id(max + 1);
}

pub fn prune_orphaned_archives(store: &mut MemoryStore) {
    let used: std::collections::HashSet<&str> = store
        .traces
        .values()
        .filter_map(|t| t.archive_id.as_deref())
        .collect();
    store
        .archives
        .retain(|id, a| used.contains(id.as_str()) || a.released_from.is_some());
}
