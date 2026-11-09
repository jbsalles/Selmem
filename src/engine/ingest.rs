//! Encoding admission pipeline.
use super::*;
impl SelectiveMemory {
    /// Same gate as `live_with`. `hold` writes the live thread; sleep commit does not.
    ///
    /// Order is the organ: interpret → paint → maybe hear → split → gate →
    /// maybe accept_core → blend mood. `commit_talk` calls this with `hold = false`.
    pub(super) fn ingest(&mut self, mut input: EncodeInput<'_>, hold: bool) -> EncodeDecision {
        if let Err(reason) = input.personal.validate() {
            return EncodeDecision {
                kept: false,
                score: 0.0,
                reason,
                trace_id: None,
                archive_id: String::new(),
                parts: 0,
                kept_n: 0,
            };
        }
        if input.personal.role != crate::SelfRole::Episode
            && (input.attribution != crate::Attribution::Internal
                || input.channel != crate::Channel::Selfhood)
        {
            return EncodeDecision {
                kept: false,
                score: 0.0,
                reason: "self propositions require explicit internal attribution and Selfhood"
                    .into(),
                trace_id: None,
                archive_id: String::new(),
                parts: 0,
                kept_n: 0,
            };
        }
        let axioms: Vec<String> = self
            .store
            .living_axioms()
            .into_iter()
            .map(|ax| ax.statement.clone())
            .collect();
        if input.semantics.is_none() {
            if let Some(interpreter) = &self.semantic_interpreter {
                match interpreter.interpret_event(input.event) {
                    Ok(semantics) => input.semantics = Some(semantics),
                    Err(reason) => {
                        return EncodeDecision {
                            kept: false,
                            score: 0.0,
                            reason: format!("semantic interpretation failed: {reason}"),
                            trace_id: None,
                            archive_id: String::new(),
                            parts: 0,
                            kept_n: 0,
                        }
                    }
                }
            }
        }
        if let Some(semantics) = &input.semantics {
            if let Err(reason) = semantics.validate() {
                return EncodeDecision {
                    kept: false,
                    score: 0.0,
                    reason,
                    trace_id: None,
                    archive_id: String::new(),
                    parts: 0,
                    kept_n: 0,
                };
            }
            input.valence = semantics.valence;
            input.arousal = semantics.arousal;
            input.disgust = semantics.disgust;
            input.self_relevance = semantics.self_relevance;
            input.goal_align = semantics.goal_relevance;
            if input.attribution == crate::core::model::Attribution::None {
                input.attribution = semantics.attribution;
            }
            input.schema = semantics.schema.clone();
        } else {
            // Compatibility path for historical protocols / old narrator implementations.
            encode::interpret(&mut input, &self.mood, &axioms, self.narrator.as_ref());
        }
        let event_owned = input.event.to_string();
        let proposed = encode::propose_split(&event_owned, self.narrator.as_ref());
        let parts = encode::split_event(&event_owned, proposed.as_deref());
        if parts.len() > 1 && input.semantics.is_some() && input.part_semantics.is_none() {
            if let Some(interpreter) = &self.semantic_interpreter {
                let annotations: Result<Vec<_>, String> = parts
                    .iter()
                    .map(|part| {
                        let s = interpreter.interpret_event(part)?;
                        s.validate()?;
                        Ok(s)
                    })
                    .collect();
                match annotations {
                    Ok(s) => input.part_semantics = Some(s),
                    Err(reason) => {
                        return EncodeDecision {
                            kept: false,
                            score: 0.0,
                            reason: format!("semantic interpretation failed: {reason}"),
                            trace_id: None,
                            archive_id: String::new(),
                            parts: parts.len(),
                            kept_n: 0,
                        }
                    }
                }
            } else {
                return EncodeDecision {
                    kept: false,
                    score: 0.0,
                    reason: "multipart annotations require per-slice semantics".into(),
                    trace_id: None,
                    archive_id: String::new(),
                    parts: parts.len(),
                    kept_n: 0,
                };
            }
        }
        if let Some(annotations) = &input.part_semantics {
            if annotations.len() != parts.len() || annotations.iter().any(|s| s.validate().is_err())
            {
                return EncodeDecision {
                    kept: false,
                    score: 0.0,
                    reason: "invalid per-slice annotations".into(),
                    trace_id: None,
                    archive_id: String::new(),
                    parts: parts.len(),
                    kept_n: 0,
                };
            }
        }
        let prior_self =
            crate::core::self_model::view(&self.store, input.event, input.schema.as_deref());
        if !prior_self.is_empty() && input.personal.role == crate::SelfRole::Episode {
            if let Some(observation) = input.observation_id {
                if self.store.traces.values().any(|t| {
                    t.source == input.source && t.observation_id.as_deref() == Some(observation)
                }) {
                    return EncodeDecision {
                        kept: false,
                        score: 0.0,
                        reason: "observation identity already retained".into(),
                        trace_id: None,
                        archive_id: String::new(),
                        parts: parts.len(),
                        kept_n: 0,
                    };
                }
            }
        }
        if parts.len() == 1
            && input.paint_from.is_none()
            && input.personal.role == crate::SelfRole::Episode
            && prior_self.is_empty()
        {
            use crate::core::evidence::ObservationUpdate;
            let update = crate::core::evidence::update(
                &mut self.store,
                input.source,
                input.channel,
                input.observation_id,
                input.event,
                self.scorer.as_deref(),
            );
            match update {
                ObservationUpdate::Updated(id, counter) => {
                    self.mood.blend(
                        &Mood {
                            valence: input.valence,
                            arousal: input.arousal,
                            disgust: input.disgust,
                        },
                        self.profile.mood_blend,
                    );
                    return EncodeDecision {
                        kept: true,
                        score: 0.0,
                        reason: if counter {
                            "counter-observation retained; salience not evaluated"
                        } else {
                            "observation support updated; salience not evaluated"
                        }
                        .into(),
                        trace_id: Some(id),
                        archive_id: String::new(),
                        parts: 1,
                        kept_n: 0,
                    };
                }
                ObservationUpdate::Duplicate
                | ObservationUpdate::Conflict
                | ObservationUpdate::Capacity => {
                    return EncodeDecision {
                        kept: false,
                        score: 0.0,
                        reason: format!("observation evidence {update:?}; salience not evaluated"),
                        trace_id: None,
                        archive_id: String::new(),
                        parts: 1,
                        kept_n: 0,
                    };
                }
                ObservationUpdate::None => (),
            }
        }
        if input.channel == crate::Channel::Selfhood
            && input.personal.role == crate::SelfRole::Episode
        {
            input.personal.appraisal = crate::core::self_model::appraise_scoped(
                &self.store,
                input.event,
                input.schema.as_deref(),
                &input.personal.scope_facets,
                self.scorer.as_deref().unwrap_or(&crate::recall::NullScorer),
            );
        }
        // The active lived self supersedes legacy axiom coloration.
        if !self.store.self_policy.selection || input.personal.appraisal.prior_ids.is_empty() {
            encode::paint(&mut self.store, &self.mood, &mut input);
        }
        // How long the hour lasts follows how hard the text hit, unless the
        // caller already pinned permanence (shared protocol days).
        if input.permanence <= 0.0 {
            let shock = (input.valence.abs() * 0.55
                + input.disgust * 0.70
                + (input.arousal - 0.18).max(0.0) * 0.25)
                .clamp(0.0, 1.0);
            input.permanence = (0.16 + 0.80 * shock).clamp(0.08, 0.97);
            input.self_relevance = input.self_relevance.max(0.32 + 0.65 * shock);
        }
        if hold {
            self.talk.hear(input.event, input.schema.as_deref());
        }
        let valence = input.valence;
        let arousal = input.arousal;
        let disgust = input.disgust;
        let input_source = input.source;
        let observation_id = input.observation_id.map(str::to_owned);
        let observation_channel = input.channel;
        let observation_is_external = input.paint_from.is_none() && input_source != "talk";
        let annotated_core = input.semantics.is_some();
        let decision = encode::gate::encode_with_interpreter(
            &mut self.store,
            &self.profile,
            input,
            self.embedder.as_ref(),
            proposed.as_deref(),
            self.semantic_interpreter.as_deref(),
        );
        if !annotated_core {
            encode::maybe_set_core(
                &mut self.store,
                self.narrator.as_ref(),
                &decision,
                input_source,
                &event_owned,
            );
        }
        if decision.kept {
            if observation_is_external && decision.parts == 1 {
                if let (Some(id), Some(observation)) =
                    (decision.trace_id.as_deref(), observation_id.as_deref())
                {
                    let traces = &self.store.traces;
                    self.store
                        .observation_evidence
                        .groups
                        .retain(|key, _| traces.contains_key(key));
                    if traces.contains_key(id) {
                        self.store.observation_evidence.seed(
                            id,
                            input_source,
                            observation_channel,
                            observation,
                            &event_owned,
                        );
                    }
                }
            }
            self.mood.blend(
                &Mood {
                    valence,
                    arousal,
                    disgust,
                },
                self.profile.mood_blend,
            );
        }
        decision
    }
}
