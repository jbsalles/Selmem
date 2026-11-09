//! Query preparation and readout construction.
use super::*;
impl SelectiveMemory {
    pub(super) fn open_mouth_inner(
        &mut self,
        user: &str,
        hold: bool,
        bias: RecallBias,
        marked: &[String],
        axioms_only: bool,
    ) -> MouthDraft {
        let query_scope = std::mem::take(&mut self.query_scope);
        let query_context = self.query_context.take();
        let query_self_cues = std::mem::take(&mut self.query_self_cues);
        let _clock = self.enter_clock();
        self.mouth_held = true;
        if hold {
            self.talk.hear(user, None);
        }
        let mut query = if hold {
            self.talk.recall_query(user)
        } else {
            user.to_string()
        };
        // Explicit context activates native top-down recall, not forced trace IDs.
        // self_cues lesion keeps the episodic store but withholds belief gists
        // from the retrieval query (pre-retrieval belief ablation).
        if query_self_cues && self.cut.self_cues {
            for t in crate::core::self_model::view_scoped(
                &self.store,
                user,
                query_context.as_deref(),
                &query_scope,
            )
            .into_iter()
            .take(4)
            {
                query.push('\n');
                query.push_str(&t.gist);
            }
        }
        let write = if hold {
            RecallWrite::Live
        } else {
            RecallWrite::ReadOnly
        };
        let previous_cap = self.profile.max_recall;
        if self.memory_context_options.format == crate::MemoryContextFormat::Structured
            && self.memory_context_options.max_episodes != 4
        {
            self.profile.max_recall = self.memory_context_options.max_episodes;
        }
        let (recalled, dump) = self.remember_with(&query, write, bias, marked);
        self.profile.max_recall = previous_cap;
        // Structured delivery already enforces its episode and word budgets.
        // Do not discard selected evidence at the legacy two-scene boundary.
        let delivery_cap =
            if hold && self.memory_context_options.format == crate::MemoryContextFormat::Legacy {
                2
            } else {
                self.memory_context_options.max_episodes
            };
        let delivered_ids: Vec<String> = if axioms_only {
            Vec::new()
        } else {
            recalled
                .iter()
                .take(if hold { delivery_cap } else { usize::MAX })
                .map(|r| r.trace_id.clone())
                .collect()
        };
        if hold {
            // Spoken utility: the hour that actually entered the mouth, not every neighbor.
            if let Some(id) = dump.selected.first() {
                self.note_spoken(id);
            }
        }
        let empty = WorkingTalk::default();
        let talk = if hold { &self.talk } else { &empty };
        let scoped_reading = crate::recall::reading::ReadingProfile::for_query(
            &self.store,
            &self.mood,
            user,
            &dump.selected,
        );
        let external_axiom_allowed = |a: &crate::core::model::IdentityAxiom| {
            let external = a
                .support_trace_ids
                .iter()
                .filter_map(|id| self.store.traces.get(id))
                .any(|t| t.attribution == crate::core::model::Attribution::External);
            !external || scoped_reading.axioms.contains(&a.statement)
        };
        // Isolated probes: retrieved scenes + living axioms.
        // Stance-without-scene lives in recall/stance.rs (ablation only).
        let (memories, axioms) = if hold {
            let memories: Vec<String> = recalled
                .into_iter()
                .take(delivery_cap)
                .map(|r| {
                    let core = self
                        .store
                        .traces
                        .get(&r.trace_id)
                        .map(|t| t.core.as_str())
                        .unwrap_or("");
                    pin_happened(&r.narrative, core)
                })
                .collect();
            let mut axioms = vec![format!("I am {}.", self.profile.name)];
            axioms.extend(
                self.who_am_i()
                    .into_iter()
                    .filter(|a| external_axiom_allowed(a))
                    .take(4)
                    .map(|a| a.statement.clone()),
            );
            (memories, axioms)
        } else {
            let lineage = crate::recall::retrieve::lineage_of(&self.store, marked);
            let lineage_schemas: Vec<String> = lineage
                .iter()
                .filter_map(|id| self.store.traces.get(id).and_then(|t| t.schema.clone()))
                .collect();
            let drop_ax = matches!(bias, RecallBias::DropLineage);
            let axioms: Vec<String> = self
                .who_am_i()
                .into_iter()
                .filter(|a| {
                    if !external_axiom_allowed(a) {
                        return false;
                    }
                    if drop_ax
                        && crate::recall::retrieve::axiom_supported_by_lineage(
                            &a.support_trace_ids,
                            a.schema.as_deref(),
                            &lineage,
                            &lineage_schemas,
                        )
                    {
                        return false;
                    }
                    // Only the strongest belief may color a probe that does not name the hour.
                    // Every other belief stays behind the probe's own tokens, or an unrelated stake colors the offer.
                    let strongest_belief = self
                        .who_am_i()
                        .into_iter()
                        .find(|a| a.layer == crate::core::model::AxiomLayer::Belief)
                        .map(|a| a.id.clone());
                    if a.layer == crate::core::model::AxiomLayer::Belief
                        && strongest_belief.as_deref() == Some(a.id.as_str())
                    {
                        return true;
                    }
                    let mut probe_cloud = std::collections::HashMap::new();
                    for token in crate::encode::scoring::token_set(user) {
                        probe_cloud.insert(token, 1.0);
                    }
                    crate::recall::retrieve::statement_anchored(&probe_cloud, &a.statement)
                })
                .map(|a| a.statement.clone())
                .collect();
            let memories: Vec<String> = if axioms_only {
                Vec::new()
            } else {
                recalled.into_iter().map(|r| r.narrative).collect()
            };
            (memories, axioms)
        };
        let mut axioms = axioms;
        if self.memory_context_options.format == crate::MemoryContextFormat::Legacy {
            for t in crate::core::self_model::view_scoped(
                &self.store,
                user,
                query_context.as_deref(),
                &query_scope,
            )
            .into_iter()
            .take(4)
            .rev()
            {
                axioms.insert(0,format!("Lived {} in {} (certainty {:.2}; foundations missing {}; counterexamples {}): {}",t.personal.role.token(),t.personal.context,t.personal.certainty,t.personal.foundations_missing,t.personal.counter_ids.len(),t.gist));
            }
        }
        for interpretation in scoped_reading
            .axioms
            .iter()
            .filter(|s| s.starts_with("Tentative observed interpretation;"))
        {
            if !axioms.contains(interpretation) {
                axioms.push(interpretation.clone());
            }
        }
        let mut memory_context =
            if self.memory_context_options.format == crate::MemoryContextFormat::Structured {
                crate::MemoryContext::from_selected(
                    &self.store,
                    &delivered_ids,
                    &memories,
                    &axioms,
                    &self.mood,
                    &self.memory_context_options,
                    self.clock.now(),
                )
            } else {
                crate::MemoryContext::default()
            };
        // The agent's direct identity is constitutive; no recall required.
        memory_context.agent_name = Some(self.profile.name.clone());
        memory_context.attach_self_view_in_context(
            &self.store,
            user,
            query_context.as_deref(),
            &self.memory_context_options,
            &query_scope,
        );
        if hold && asks_about_human_name(user) && self.memory_context_options.dispositions {
            if let Some(t) = self
                .store
                .traces
                .values()
                .filter(|t| {
                    t.schema.as_deref() == Some("interlocutor-name")
                        && t.attribution == crate::core::model::Attribution::External
                        && !t.suppressed
                        && t.status == crate::core::model::TraceStatus::Active
                        && t.access >= 0.15
                })
                .max_by(|a, b| {
                    a.created_at
                        .cmp(&b.created_at)
                        .then_with(|| a.id.cmp(&b.id))
                })
            {
                memory_context.dispositions.truncate(3);
                memory_context.contextual_statements.insert(
                    0,
                    crate::recall::memory_context::ContextStatement {
                        statement: format!(
                            "Fallible recollection of what the human told me: {}",
                            t.gist
                        ),
                        truncated: false,
                    },
                );
            }
        }
        if hold && asks_about_nights(user) && self.memory_context_options.dispositions {
            let count = self.remembered_nights();
            if count > 0 {
                memory_context.dispositions.truncate(3);
                memory_context.contextual_statements.insert(0,crate::recall::memory_context::ContextStatement {
                    statement: format!("I can currently recall at least {count} nights of sleep. This is not necessarily the total: earlier nights may have been forgotten."),
                    truncated: false,
                });
            }
        }
        let reading = if self.drop_stake {
            String::new()
        } else {
            scoped_reading.render()
        };
        MouthDraft {
            user: user.to_string(),
            memories,
            axioms,
            mood: self.mood.clone(),
            talk: talk.clone(),
            hold,
            dump,
            memory_context,
            memory_context_options: self.memory_context_options.clone(),
            reading,
            epoch: {
                self.mouth_epoch = self.mouth_epoch.saturating_add(1);
                self.mouth_epoch
            },
        }
    }
}
