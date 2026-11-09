//! Reconstruct, ground, and write back one recall transaction.
use super::*;
pub fn recall_with(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    narrator: &dyn Narrator,
    embedder: &dyn Embedder,
    query: &str,
    mood: &Mood,
    cut: OrganCut,
    write: RecallWrite,
    bias: RecallBias,
    marked: &[String],
    scorer: &dyn crate::recall::PropositionScorer,
) -> RecallOutcome {
    let live = write == RecallWrite::Live;
    let ranked = rank_candidates(store, profile, embedder, query, mood, cut, bias);
    let chosen_ids = choose_trace_ids(&ranked, store, profile, query, bias, marked);

    // Freeze all cue content and weights before the first trace can write back.
    let before: Vec<_> = chosen_ids
        .iter()
        .map(|id| store.traces[id].clone())
        .collect();
    let now = now_secs();
    let mut cues = std::collections::BTreeMap::new();
    if store.associations.reconstruct && cut.reconstruct && narrator.supports_associations() {
        for target in &before {
            if target.attribution == crate::core::model::Attribution::External
                || target.channel.verbatim()
                || target.core.is_empty()
            {
                continue;
            }
            let mut selected: Vec<_> = before
                .iter()
                .filter(|other| {
                    other.id != target.id
                        && !other.suppressed
                        && other.status != TraceStatus::Latent
                        && other.observation_id.as_deref().unwrap_or(&other.id)
                            != target.observation_id.as_deref().unwrap_or(&target.id)
                })
                .filter_map(|other| {
                    let weight = store.associations.weight(&target.id, &other.id, now);
                    if weight < 0.05 {
                        return None;
                    }
                    Some(AssociativeCue {
                        trace_id: other.id.clone(),
                        gist: other.gist.clone(),
                        attribution: other.attribution,
                        weight,
                    })
                })
                .collect();
            selected.sort_by(|a, b| {
                b.weight
                    .total_cmp(&a.weight)
                    .then(a.trace_id.cmp(&b.trace_id))
            });
            selected.truncate(2);
            cues.insert(target.id.clone(), selected);
        }
    }
    let failures_before = narrator.failure_log().failures;
    let mut associations_used = Vec::new();
    let mut recalled = Vec::new();
    let mut pulled_n = 0u32;
    let mut recon_n = 0u32;
    for trace_id in &chosen_ids {
        let (channel, gist, schema) = {
            let trace = store.traces.get(trace_id).unwrap();
            (trace.channel, trace.gist.clone(), trace.schema.clone())
        };

        let associates = cues.get(trace_id).map(Vec::as_slice).unwrap_or(&[]);
        let mut association_accepted = false;
        let (narrative, disclaimer, fidelity, pulled, reconsolidated) = if channel.verbatim() {
            (
                gist,
                "verbatim record, not distorted".to_string(),
                store.traces[trace_id].fidelity,
                false,
                false,
            )
        } else {
            speak_self(
                store,
                profile,
                narrator,
                query,
                mood,
                cut,
                write,
                trace_id,
                scorer,
                associates,
                &mut association_accepted,
            )
        };

        if live && association_accepted && !pulled {
            let original = before.iter().find(|t| t.id == *trace_id).unwrap();
            if original.gist != narrative {
                if let Some(trace) = store.traces.get_mut(trace_id) {
                    trace.record_operation(crate::core::model::MemoryOperation {
                        kind: "associative-reconstruction".into(),
                        at: now,
                        source_trace_ids: associates.iter().map(|c| c.trace_id.clone()).collect(),
                        source_axiom_ids: Vec::new(),
                        source_center: None,
                        before: original.gist.clone(),
                        after: narrative.clone(),
                        confidence: original.confidence,
                        origin: crate::core::model::EvidenceOrigin::Reconstruction,
                    });
                }
            }
        }
        if !associates.is_empty() {
            associations_used.push(AssociationUse {
                target: trace_id.clone(),
                cues: associates.to_vec(),
                accepted: association_accepted,
            });
        }
        if live {
            let operational = store
                .traces
                .get(trace_id)
                .map(|t| t.channel.verbatim() && t.source != "talk")
                .unwrap_or(false);
            if let Some(trace) = store.traces.get_mut(trace_id) {
                if operational {
                    trace.access = 1.0;
                } else {
                    refresh_access(trace, profile);
                }
                trace.last_recalled_at = Some(now_secs());
            }
        }
        if pulled {
            pulled_n += 1;
        }
        if reconsolidated {
            recon_n += 1;
        }
        recalled.push(RecalledMemory {
            trace_id: trace_id.clone(),
            narrative,
            fidelity,
            schema,
            channel,
            disclaimer,
            pulled_toward_core: pulled,
            reconsolidated,
        });
    }
    let associations_reinforced = if live
        && cut.reconstruct
        && matches!(bias, RecallBias::Observed | RecallBias::Transfer)
        && narrator.failure_log().failures == failures_before
    {
        let active: Vec<_> = before
            .iter()
            .filter_map(|trace| {
                if trace.suppressed
                    || trace.status == TraceStatus::Latent
                    || trace.channel.verbatim()
                {
                    return None;
                }
                let recalled = recalled.iter().find(|r| r.trace_id == trace.id)?;
                if recalled.pulled_toward_core || recalled.narrative.trim().is_empty() {
                    return None;
                }
                let judgment = crate::recall::judge::judge_against_core(
                    &recalled.narrative,
                    &trace.core,
                    scorer,
                );
                // An unchanged gist also counts as an observed activation, without
                // interpreting an unknown proposition as corroborated evidence.
                if recalled.narrative != trace.gist
                    && (judgment.kind.is_miss()
                        || judgment.kind == crate::recall::judge::DetachKind::Unjudged)
                {
                    return None;
                }
                let relevance = topic_relevance(query, &trace.core);
                if relevance <= 0.0 {
                    return None;
                }
                let activation = relevance.clamp(0.0, 1.0)
                    * crate::encode::scoring::access_value(trace, profile).clamp(0.0, 1.0);
                Some((
                    trace.id.clone(),
                    trace
                        .observation_id
                        .clone()
                        .unwrap_or_else(|| trace.id.clone()),
                    activation,
                ))
            })
            .collect();
        store.associations.reinforce(&active, now)
    } else {
        Vec::new()
    };
    // Attach provenance-derived counts only after reconstruction/grounding and
    // co-recall learning. They never enter gist write-back or the frozen core.
    for memory in &mut recalled {
        if let Some(trace) = store.traces.get_mut(&memory.trace_id) {
            if write == RecallWrite::Live
                && before
                    .iter()
                    .any(|old| old.id == trace.id && old.gist != trace.gist)
            {
                trace.embedding = embedder.embed(&trace.gist);
            }
            // Subjective revision is available to the policy, never written into factual gist.
            if trace.operations.iter().any(|op| op.kind == "reinterpret") {
                memory
                    .narrative
                    .push_str("\n[Tentative interpretation; not an observed fact] ");
                memory.narrative.push_str(&trace.interpretation.statement);
            }
        }
        if let Some(metadata) = store.observation_evidence.render(store, &memory.trace_id) {
            memory.narrative.push_str("\n");
            memory.narrative.push_str(&metadata);
            if cut.support_signal {
                let w = store
                    .observation_evidence
                    .support_weight(store, &memory.trace_id);
                if w > 0.0 {
                    memory.narrative.push_str(&format!(
                        "\nSupport strength (reconstructive signal only, not additional episodes): {:.2}",
                        w
                    ));
                }
            }
        }
        // Optional schema cue is independent of observation support.
        // Does not alter selection; only colors the narrative for the mouth.
        if cut.schema_prototype {
            if let Some(trace) = store.traces.get(&memory.trace_id) {
                if trace.attribution == crate::core::model::Attribution::Internal {
                    if let Some(schema) = &trace.schema {
                        if let Some(center) = store.centers.get(schema) {
                            let proto = center.core.chars().take(120).collect::<String>();
                            if !proto.is_empty() {
                                memory.narrative.push_str(&format!(
                                    "\nSchema prototype (context only): {proto}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    RecallOutcome {
        memories: recalled,
        dump: RetrievalDump {
            associations_used,
            associations_reinforced,
            candidates: ranked,
            selected: chosen_ids,
            pulled: pulled_n,
            reconsolidated: recon_n,
            scorer: scorer.name().to_string(),
        },
    }
}
