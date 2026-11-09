//! Context qualification, contradiction and belief recovery.
use super::*;
fn propose_scope(
    store: &mut MemoryStore,
    episodes: &[MemoryTrace],
    embedder: &dyn Embedder,
    scorer: &dyn PropositionScorer,
) {
    let active = |t: &&MemoryTrace| {
        !t.personal.retired
            && t.personal.scope_parent.is_some()
            && matches!(
                t.personal.scope_status,
                ScopeStatus::Candidate | ScopeStatus::Conditional | ScopeStatus::Conflicted
            )
    };
    let mut capacity = 32usize.saturating_sub(store.traces.values().filter(active).count());
    if capacity < 2 {
        return;
    }
    let mut parents = store
        .traces
        .values()
        .filter(|t| {
            t.personal.role == SelfRole::Belief
                && t.personal.scope_status == ScopeStatus::General
                && !t.suppressed
                && !t.personal.retired
        })
        .collect::<Vec<_>>();
    parents.sort_by(|a, b| a.id.cmp(&b.id));
    let mut plans = Vec::new();
    for parent in parents {
        if capacity < 2 {
            break;
        }
        let p = &parent.personal;
        if p.scope_parent.is_some() || p.expectation.trim().is_empty()
            || p.expectation.to_lowercase().contains("prefer")
            || store.traces.values().any(|t| t.personal.scope_parent.as_deref() == Some(parent.id.as_str())
                && !t.personal.retired && t.personal.scope_status != ScopeStatus::Superseded)
            // Legacy v2 snapshots have no recovery checkpoint; old evidence cannot
            // be safely reused to recreate a completed family.
            || (p.scope_epoch.is_none() && store.traces.values().any(|t|
                t.personal.scope_parent.as_deref() == Some(parent.id.as_str()) && t.personal.retired))
        {
            continue;
        }
        // scope_epoch advances upon each recovered/aborted generation. No old
        // observations can bootstrap a later generation of the same family.
        let a = grounded(episodes, &p.support_ids, p.scope_epoch.as_deref());
        let b = grounded(episodes, &p.counter_ids, p.scope_epoch.as_deref());
        if a.len() < 2 || b.len() < 2 {
            continue;
        }
        let roots_a = a.iter().map(|e| root(e)).collect::<BTreeSet<_>>();
        if b.iter().any(|e| roots_a.contains(&root(e))) {
            continue;
        }
        let Some((dimension, left, right)) = discriminator(&a, &b) else {
            continue;
        };
        let claim = b[0].semantic.claim.trim();
        if claim.is_empty()
            || claim == p.expectation
            || ["prefer", "should", "must", "always", "because"]
                .iter()
                .any(|w| claim.to_lowercase().contains(w))
            || !b.iter().all(|e| e.semantic.claim.trim() == claim)
        {
            continue;
        }
        if !guarded_pattern(&a, &dimension, &left, &p.expectation, scorer)
            || !guarded_pattern(&b, &dimension, &right, claim, scorer)
        {
            continue;
        }
        plans.push((
            parent.id.clone(),
            dimension,
            left,
            right,
            a.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            b.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            claim.to_string(),
        ));
        capacity -= 2;
    }
    for (id, dimension, left, right, a, b, claim) in plans {
        let Some(parent) = store.traces.get(&id).cloned() else {
            continue;
        };
        if parent.personal.scope_status != ScopeStatus::General {
            continue;
        }
        let x = spawn_child(
            &parent,
            &dimension,
            &left,
            a,
            &parent.personal.expectation,
            episodes,
            embedder,
        );
        let y = spawn_child(&parent, &dimension, &right, b, &claim, episodes, embedder);
        if let Some(p) = store.traces.get_mut(&id) {
            p.personal.scope_status = ScopeStatus::Conflicted;
            p.personal.retired = false;
        }
        store.add_trace(x);
        store.add_trace(y);
    }
}
fn update_scope(store: &mut MemoryStore, episodes: &[MemoryTrace], scorer: &dyn PropositionScorer) {
    // Process candidate AND mature children so every later observation is
    // consumed exactly once, including contradictory and unknown labels.
    let mut ids = store
        .traces
        .values()
        .filter(|t| {
            !t.personal.retired
                && matches!(
                    t.personal.scope_status,
                    ScopeStatus::Candidate | ScopeStatus::Conditional | ScopeStatus::Conflicted
                )
                && t.personal.scope_parent.is_some()
        })
        .map(|t| t.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        let Some(t) = store.traces.get(&id) else {
            continue;
        };
        let Some(facet) = t.personal.scope_facets.first().cloned() else {
            continue;
        };
        let mut roots = t.personal.consumed.iter().cloned().collect::<BTreeSet<_>>();
        let mut updates = Vec::new();
        for e in episodes.iter().filter(|e| {
            observed_action(e) && after_trace(e, &id) && e.personal.scope_facets.contains(&facet)
        }) {
            let key = root(e);
            if !roots.insert(key.clone()) {
                continue;
            }
            updates.push((
                e.id.clone(),
                key,
                scorer.score_expectation(&t.personal.expectation, &e.gist),
            ));
        }
        if let Some(child) = store.traces.get_mut(&id) {
            for (eid, key, verdict) in updates {
                child.personal.consumed.push(key);
                match verdict {
                    PropositionLabel::Entail => child.personal.support_ids.push(eid),
                    PropositionLabel::Contradict => child.personal.counter_ids.push(eid),
                    PropositionLabel::Unknown => (),
                }
            }
        }
    }
    let mut families = BTreeMap::<String, Vec<String>>::new();
    for t in store.traces.values().filter(|t| {
        t.personal.scope_parent.is_some()
            && !t.personal.retired
            && matches!(
                t.personal.scope_status,
                ScopeStatus::Candidate | ScopeStatus::Conditional | ScopeStatus::Conflicted
            )
    }) {
        if let Some(id) = &t.personal.scope_parent {
            families.entry(id.clone()).or_default().push(t.id.clone());
        }
    }
    for (parent_id, ids) in families {
        if ids.len() != 2 {
            continue;
        }
        let candidates = ids.iter().all(|id| {
            store
                .traces
                .get(id)
                .is_some_and(|t| t.personal.scope_status == ScopeStatus::Candidate)
        });
        if candidates {
            let has_counter = ids.iter().any(|id| {
                store
                    .traces
                    .get(id)
                    .is_some_and(|t| !t.personal.counter_ids.is_empty())
            });
            if has_counter {
                let latest = ids
                    .iter()
                    .filter_map(|id| store.traces.get(id))
                    .flat_map(|t| t.personal.counter_ids.iter())
                    .filter_map(|id| trace_seq(id).map(|v| (v, id.clone())))
                    .max_by_key(|(seq, _)| *seq)
                    .map(|(_, id)| id);
                for id in &ids {
                    if let Some(t) = store.traces.get_mut(id) {
                        t.personal.scope_status = ScopeStatus::Superseded;
                        t.personal.retired = true;
                    }
                }
                if let Some(p) = store.traces.get_mut(&parent_id) {
                    p.personal.scope_status = ScopeStatus::General;
                    p.personal.scope_epoch = latest;
                }
                continue;
            }
            if ids.iter().all(|id| {
                store
                    .traces
                    .get(id)
                    .is_some_and(|t| t.personal.support_ids.len() >= 3)
            }) {
                for id in &ids {
                    if let Some(t) = store.traces.get_mut(id) {
                        t.personal.scope_status = ScopeStatus::Conditional;
                    }
                }
            }
        } else {
            // Contradictions revoke only the affected scope. The parent remains
            // available as a conflicted, explicitly uncertain fallback.
            for id in &ids {
                if let Some(t) = store.traces.get_mut(id) {
                    if t.personal.scope_status == ScopeStatus::Conditional
                        && t.personal.counter_ids.len() >= 2
                    {
                        t.personal.scope_epoch = t
                            .personal
                            .counter_ids
                            .iter()
                            .filter_map(|id| trace_seq(id).map(|seq| (seq, id.clone())))
                            .max_by_key(|(seq, _)| *seq)
                            .map(|(_, id)| id);
                        t.personal.scope_status = ScopeStatus::Conflicted;
                    }
                }
            }
        }
    }
}
fn recover_scope(
    store: &mut MemoryStore,
    episodes: &[MemoryTrace],
    scorer: &dyn PropositionScorer,
) {
    let mut families = BTreeMap::<String, Vec<String>>::new();
    for t in store.traces.values().filter(|t| {
        t.personal.scope_parent.is_some()
            && !t.personal.retired
            && matches!(
                t.personal.scope_status,
                ScopeStatus::Conditional | ScopeStatus::Conflicted
            )
    }) {
        if let Some(parent) = &t.personal.scope_parent {
            families
                .entry(parent.clone())
                .or_default()
                .push(t.id.clone());
        }
    }
    for (parent_id, ids) in families {
        if ids.len() != 2 {
            continue;
        }
        let Some(parent) = store.traces.get(&parent_id) else {
            continue;
        };
        if parent.personal.scope_status != ScopeStatus::Conflicted || parent.personal.retired {
            continue;
        }
        let children = ids
            .iter()
            .filter_map(|id| store.traces.get(id))
            .collect::<Vec<_>>();
        if children.len() != 2
            || !children
                .iter()
                .any(|t| t.personal.scope_status == ScopeStatus::Conflicted)
        {
            continue;
        }
        let Some(fa) = children[0].personal.scope_facets.first() else {
            continue;
        };
        let Some(fb) = children[1].personal.scope_facets.first() else {
            continue;
        };
        if fa.dimension != fb.dimension || fa.value == fb.value {
            continue;
        }
        // The conflict-onset marker is frozen on the first status transition.
        // Recovery evidence also contradicts the old child: using its *latest*
        // counterexample as cutoff would move the goalposts forever.
        let onset = children
            .iter()
            .filter(|t| t.personal.scope_status == ScopeStatus::Conflicted)
            .filter_map(|t| t.personal.scope_epoch.as_ref())
            .filter_map(|id| trace_seq(id).map(|n| (n, id.clone())))
            .max_by_key(|(n, _)| *n)
            .map(|(_, id)| id);
        let Some(cutoff) = onset else {
            continue;
        };
        let mut witnesses: Vec<&MemoryTrace> = Vec::new();
        let mut all_roots = BTreeSet::new();
        let mut qualified = true;
        for child in children {
            let Some(facet) = child.personal.scope_facets.first() else {
                qualified = false;
                break;
            };
            let mut local = Vec::new();
            for e in episodes.iter().filter(|e| {
                observed_action(e)
                    && after_trace(e, &cutoff)
                    && e.personal.scope_facets.contains(facet)
            }) {
                let key = root(e);
                if !all_roots.insert(key) {
                    continue;
                }
                match scorer.score_expectation(&parent.personal.expectation, &e.gist) {
                    PropositionLabel::Entail => local.push(e),
                    // Negative OR unscorable evidence prevents premature recovery.
                    PropositionLabel::Contradict | PropositionLabel::Unknown => {
                        qualified = false;
                    }
                }
            }
            if local.len() < 2 {
                qualified = false;
            }
            witnesses.extend(local);
        }
        if !qualified || witnesses.len() < 4 {
            continue;
        }
        if scorer.score_self_model(&parent.gist, &parent.personal.expectation, &witnesses)
            != PropositionLabel::Entail
        {
            continue;
        }
        let epoch = witnesses
            .iter()
            .filter_map(|e| trace_seq(&e.id).map(|n| (n, e.id.clone())))
            .max_by_key(|(n, _)| *n)
            .map(|(_, id)| id);
        if let Some(p) = store.traces.get_mut(&parent_id) {
            p.personal.scope_status = ScopeStatus::General;
            p.personal.scope_epoch = epoch;
            for e in &witnesses {
                if !p.personal.support_ids.contains(&e.id) {
                    p.personal.support_ids.push(e.id.clone());
                }
                let root_id = root(e);
                if !p.personal.consumed.contains(&root_id) {
                    p.personal.consumed.push(root_id);
                }
            }
        }
        for id in &ids {
            if let Some(t) = store.traces.get_mut(id) {
                t.personal.scope_status = ScopeStatus::Superseded;
                t.personal.retired = true;
            }
        }
    }
}
pub(super) fn differentiate(
    store: &mut MemoryStore,
    episodes: &[MemoryTrace],
    embedder: &dyn Embedder,
    scorer: &dyn PropositionScorer,
) {
    if !store.contextual_scope.enabled {
        return;
    }
    update_scope(store, episodes, scorer);
    recover_scope(store, episodes, scorer);
    propose_scope(store, episodes, embedder, scorer);
}
