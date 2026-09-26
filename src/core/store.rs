use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::model::{ArchiveRecord, Channel, IdentityAxiom, MemoryTrace};

#[derive(Default)]
pub struct MemoryStore {
    pub traces: HashMap<String, MemoryTrace>,
    pub archives: HashMap<String, ArchiveRecord>,
    pub axioms: HashMap<String, IdentityAxiom>,
    pub edges: HashMap<String, HashSet<String>>,
    /// Last night that ran merge + ladder. Shallow nights do not stamp this.
    pub last_deep_at: Option<u64>,
    /// Runtime-only. Pairs the merge pass refused under `merge_support_veto`.
    pub merges_refused: u32,
    /// Selfhood hours not yet through a deep night. Survives same-second benches.
    pub pending_night: Vec<String>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_archive(&mut self, record: ArchiveRecord) -> String {
        let id = record.id.clone();
        self.archives.insert(id.clone(), record);
        id
    }

    pub fn add_trace(&mut self, trace: MemoryTrace) -> String {
        let id = trace.id.clone();
        if trace.channel == Channel::Selfhood {
            self.pending_night.push(id.clone());
        }
        self.traces.insert(id.clone(), trace);
        id
    }

    pub fn add_axiom(&mut self, axiom: IdentityAxiom) -> String {
        let id = axiom.id.clone();
        self.axioms.insert(id.clone(), axiom);
        id
    }

    pub fn link(&mut self, a: &str, b: &str) {
        if a == b {
            return;
        }
        self.edges.entry(a.to_string()).or_default().insert(b.to_string());
        self.edges.entry(b.to_string()).or_default().insert(a.to_string());
    }

    pub fn active_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.traces.values().map(|t| t.id.clone()).collect();
        ids.sort_by(|a, b| {
            let ta = &self.traces[a];
            let tb = &self.traces[b];
            let ca = if ta.core.is_empty() { &ta.gist } else { &ta.core };
            let cb = if tb.core.is_empty() { &tb.gist } else { &tb.core };
            ca.cmp(cb).then(a.cmp(b))
        });
        ids
    }

    /// New Selfhood hours since the last deep night (all of them if none yet).
    pub fn hours_since_deep(&self) -> Vec<&MemoryTrace> {
        if !self.pending_night.is_empty() {
            let mut hours: Vec<&MemoryTrace> = self
                .pending_night
                .iter()
                .filter_map(|id| self.traces.get(id))
                .filter(|t| t.channel == Channel::Selfhood)
                .collect();
            hours.sort_by(|a, b| a.id.cmp(&b.id));
            return hours;
        }
        let since = self.last_deep_at.unwrap_or(0);
        let mut hours: Vec<&MemoryTrace> = self
            .traces
            .values()
            .filter(|t| t.channel == Channel::Selfhood && t.created_at > since)
            .collect();
        hours.sort_by(|a, b| a.id.cmp(&b.id));
        hours
    }

    pub fn night_charge(&self) -> f32 {
        self.hours_since_deep()
            .iter()
            .map(|t| t.arousal + t.disgust)
            .sum()
    }

    /// Marked ids, same-schema siblings, merge edges, axiom supports that touch the set.
    pub fn lineage(&self, marked: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut q: VecDeque<String> = VecDeque::new();
        for id in marked {
            if seen.insert(id.clone()) {
                q.push_back(id.clone());
                out.push(id.clone());
            }
        }
        let mut schemas: Vec<String> = Vec::new();
        for id in marked {
            if let Some(s) = self.traces.get(id).and_then(|t| t.schema.clone()) {
                if !s.is_empty() && !schemas.iter().any(|x| x == &s) {
                    schemas.push(s);
                }
            }
        }
        for t in self.traces.values() {
            let Some(s) = t.schema.as_ref() else { continue };
            if schemas.iter().any(|x| x == s) && seen.insert(t.id.clone()) {
                q.push_back(t.id.clone());
                out.push(t.id.clone());
            }
        }
        while let Some(id) = q.pop_front() {
            if let Some(neigh) = self.edges.get(&id) {
                for n in neigh {
                    if seen.insert(n.clone()) {
                        q.push_back(n.clone());
                        out.push(n.clone());
                    }
                }
            }
            for a in self.axioms.values() {
                if a.support_trace_ids.iter().any(|s| s == &id) {
                    for s in &a.support_trace_ids {
                        if seen.insert(s.clone()) {
                            q.push_back(s.clone());
                            out.push(s.clone());
                        }
                    }
                }
            }
        }
        out
    }

    pub fn living_axioms(&self) -> Vec<&IdentityAxiom> {
        self.axioms
            .values()
            .filter(|a| a.superseded_by.is_none())
            .collect()
    }

    /// Living axiom ids that list this hour as support.
    pub fn living_axiom_ids_for(&self, trace_id: &str) -> Vec<String> {
        let mut ids: Vec<String> = self
            .living_axioms()
            .into_iter()
            .filter(|a| a.support_trace_ids.iter().any(|s| s == trace_id))
            .map(|a| a.id.clone())
            .collect();
        ids.sort();
        ids
    }

    pub fn max_axiom_strength(&self) -> f32 {
        self.living_axioms()
            .into_iter()
            .map(|a| a.strength)
            .fold(0.0_f32, f32::max)
    }

    /// Drop an hour from the book. Archive goes if nothing else points at it.
    pub fn release_trace(&mut self, id: &str) -> Option<MemoryTrace> {
        let trace = self.traces.remove(id)?;
        self.edges.remove(id);
        for neigh in self.edges.values_mut() {
            neigh.remove(id);
        }
        if let Some(aid) = trace.archive_id.as_deref() {
            let used = self
                .traces
                .values()
                .any(|t| t.archive_id.as_deref() == Some(aid));
            if !used {
                self.archives.remove(aid);
            }
        }
        Some(trace)
    }
}
