//! Qualified belief consolidation; kept separate from scope recognition.
use super::*;
pub fn consolidate(store: &mut MemoryStore, narrator: &dyn Narrator, embedder: &dyn Embedder) {
    consolidate_with_scorer(store, narrator, embedder, &crate::recall::NullScorer)
}
pub fn consolidate_with_scorer(
    store: &mut MemoryStore,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    scorer: &dyn PropositionScorer,
) {
    store.self_consolidation_audit.clear();
    if !store.self_policy.consolidation {
        return;
    }
    let episodes: Vec<_> = store
        .active_ids()
        .into_iter()
        .filter_map(|id| store.traces.get(&id))
        .filter(|t| {
            available(t)
                && t.personal.role == SelfRole::Episode
                && t.personal.evidence == SelfEvidence::Observation
                && t.reality.verifiable
                && t.source != "talk"
                && t.source != "self-consolidation"
                && !t
                    .drifts
                    .iter()
                    .any(|d| matches!(d.kind, DriftKind::Merge | DriftKind::Confabulate))
        })
        .cloned()
        .collect();
    let live: BTreeSet<_> = store
        .traces
        .values()
        .filter(|t| available(t))
        .map(|t| t.id.clone())
        .collect();
    let scope_enabled = store.contextual_scope.enabled;
    for t in store.traces.values_mut().filter(|t| {
        t.personal.role != SelfRole::Episode && (scope_enabled || t.personal.scope_parent.is_none())
    }) {
        let p = &mut t.personal;
        let missing = p.support_ids.iter().any(|id| !live.contains(id));
        if missing && !p.foundations_missing {
            p.certainty = (p.certainty - 0.1).max(0.0);
        }
        p.foundations_missing = missing;
        if t.suppressed || p.retired {
            continue;
        }
        let mut delta = 0.0f32;
        for e in &episodes {
            let a = &e.personal.appraisal;
            let support = a.supports.contains(&t.id);
            let counter = a.contradicts.contains(&t.id);
            if !support && !counter {
                continue;
            }
            let key = root(e);
            if p.consumed.contains(&key) {
                continue;
            }
            p.consumed.push(key);
            let weight = 0.15 * e.semantic.confidence.clamp(0.0, 1.0);
            if counter {
                p.counter_ids.push(e.id.clone());
                delta -= weight;
            } else {
                p.support_ids.push(e.id.clone());
                delta += weight;
            }
        }
        if p.role == SelfRole::Belief {
            p.certainty = (p.certainty + delta.clamp(-0.15, 0.15)).clamp(0.0, 0.95);
            // While the experimental contextual loop is operating, preserve
            // unresolved self-beliefs so recovery can actually happen.
            let unresolved = scope_enabled
                && (p.scope_parent.is_some()
                    || (p.support_ids.len() >= 2 && p.counter_ids.len() >= 2));
            if p.certainty <= 0.05 && !p.counter_ids.is_empty() && !unresolved {
                p.retired = true;
            }
        }
        t.confidence = p.certainty;
    }
    differentiate(store, &episodes, embedder, scorer);
    let mut groups: BTreeMap<String, Vec<&MemoryTrace>> = BTreeMap::new();
    for e in &episodes {
        if e.attribution != Attribution::Internal
            || e.stake_kind != StakeKind::Decision
            || e.bearer != Bearer::Self_
        {
            continue;
        }
        if let Some(s) = e.schema.as_ref().filter(|s| !s.is_empty()) {
            groups.entry(s.clone()).or_default().push(e);
        }
    }
    for (context, mut group) in groups.into_iter().take(128) {
        // Retiring a belief must not bootstrap it again from the same evidence.
        let used: BTreeSet<_> = store
            .traces
            .values()
            .filter(|t| t.personal.retired && t.personal.context == context)
            .flat_map(|t| t.personal.consumed.iter().cloned())
            .collect();
        let mut roots = BTreeSet::new();
        group.retain(|t| {
            let r = root(t);
            !used.contains(&r) && roots.insert(r)
        });
        if group.len() < 2
            || store.traces.values().any(|t| {
                !t.personal.retired
                    && t.personal.role != SelfRole::Episode
                    && t.personal.context == context
            })
        {
            continue;
        }
        group.truncate(8);
        let proposal = narrator.distill_self(&group, &context);
        let (statement, expectation) = proposal.clone().unwrap_or_default();
        let mut check = SelfConsolidationAudit {
            context: context.clone(),
            statement: statement.clone(),
            expectation: expectation.clone(),
            source_trace_ids: group.iter().map(|t| t.id.clone()).collect(),
            support_trace_ids: vec![],
            counter_trace_ids: vec![],
            unknown_trace_ids: vec![],
            verdict: PropositionLabel::Unknown,
            reason: "narrator_abstained".into(),
        };
        if proposal.is_none() || statement.trim().is_empty() || expectation.trim().is_empty() {
            store.self_consolidation_audit.push(check);
            continue;
        }
        if statement.chars().count() > 800 || expectation.chars().count() > 800 {
            check.reason = "invalid_proposal".into();
            store.self_consolidation_audit.push(check);
            continue;
        }
        let baseline = repeated_action_summary(&group, &context);
        let exact = baseline
            .as_ref()
            .is_some_and(|p| p.0 == statement && p.1 == expectation);
        check.verdict = if exact {
            PropositionLabel::Entail
        } else {
            scorer.score_self_model(&statement, &expectation, &group)
        };
        if check.verdict != PropositionLabel::Entail {
            check.reason = "proposal_rejected".into();
            store.self_consolidation_audit.push(check);
            continue;
        }
        // A group passing the summary guard does not make every member support.
        // Label each action against the proposed expectation before attribution.
        for e in &group {
            match if exact {
                PropositionLabel::Entail
            } else {
                scorer.score_expectation(&expectation, &e.gist)
            } {
                PropositionLabel::Entail => check.support_trace_ids.push(e.id.clone()),
                PropositionLabel::Contradict => check.counter_trace_ids.push(e.id.clone()),
                PropositionLabel::Unknown => check.unknown_trace_ids.push(e.id.clone()),
            }
        }
        if check.support_trace_ids.len() < 2 {
            check.verdict = PropositionLabel::Unknown;
            check.reason = "insufficient_individual_support".into();
            store.self_consolidation_audit.push(check);
            continue;
        }
        check.reason = "accepted".into();
        let support = group
            .iter()
            .filter(|t| check.support_trace_ids.contains(&t.id))
            .copied()
            .collect::<Vec<_>>();
        let consumed = group
            .iter()
            .filter(|t| {
                check.support_trace_ids.contains(&t.id) || check.counter_trace_ids.contains(&t.id)
            })
            .map(|t| root(t))
            .collect();
        let mut t = support[0].clone();
        t.id = new_id("self");
        t.gist = statement;
        t.core = t.gist.clone();
        t.personal = SelfMemory {
            role: SelfRole::Belief,
            evidence: SelfEvidence::Interpretation,
            context,
            expectation,
            importance: support.iter().map(|t| t.self_relevance).fold(0.0, f32::max),
            certainty: 0.35,
            support_ids: check.support_trace_ids.clone(),
            counter_ids: check.counter_trace_ids.clone(),
            consumed,
            ..SelfMemory::default()
        };
        store.self_consolidation_audit.push(check);
        t.archive_id = None;
        t.observation_id = None;
        t.source = "self-consolidation".into();
        t.reality = RealityAnchor::default();
        t.operations.clear();
        t.drifts.clear();
        t.rehearsals = 0;
        t.created_at = now_secs();
        t.last_recalled_at = None;
        t.last_consolidated_at = Some(now_secs());
        t.embedding = embedder.embed(&t.gist);
        t.semantic.claim = t.gist.clone();
        t.semantic.confidence = 0.35;
        t.interpretation.statement = t.gist.clone();
        t.interpretation.confidence = 0.35;
        t.confidence = 0.35;
        t.permanence = 0.4;
        t.anchor = 0.;
        store.add_trace(t);
    }
}
