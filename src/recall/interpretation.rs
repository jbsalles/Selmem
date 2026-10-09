//! Bounded, auditable analogies from reported experience. These are tentative
//! interpretations, not new facts, self memories, or singleton beliefs.
use crate::core::model::{Attribution, EvidenceOrigin, MemoryOperation, MemoryTrace, TraceStatus};
use crate::core::store::MemoryStore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Participation,
    SocialHarm,
}

#[derive(Clone, Debug)]
pub struct ObservedInterpretation {
    pub subject: String,
    pub situation: Situation,
    pub evidence: String,
    pub confidence: f32,
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}
fn has(words: &[String], choices: &[&str]) -> bool {
    words.iter().any(|w| choices.contains(&w.as_str()))
}

/// Require a social relation AND a reported consequence. A mood sign, a schema
/// label, or a word such as "people" alone never supplies an interpretation.
fn infer(trace: &MemoryTrace) -> Option<ObservedInterpretation> {
    if trace.attribution != Attribution::External
        || trace.channel.verbatim()
        || trace.suppressed
        || matches!(trace.status, TraceStatus::Latent | TraceStatus::Myth)
    {
        return None;
    }
    let (subject, body) = crate::encode::core::reported_speech(&trace.core)?;
    // Resolve nothing across a negated/hypothetical sentence. Other affirmative
    // sentences can still supply evidence (e.g. "not-so-great ... . A group ...").
    let w: Vec<_> = body
        .split_inclusive(['.', '!', '?', ';', '\n'])
        .filter_map(|sentence| {
            let w = words(sentence);
            (!has(
                &w,
                &["not", "no", "never", "without", "if", "would", "wish"],
            ) && !sentence.contains("n't")
                && !sentence.contains("n’t"))
            .then_some(w)
        })
        .flatten()
        .collect();
    let social = has(
        &w,
        &["group", "people", "friends", "friend", "team", "community"],
    );
    let supportive = has(
        &w,
        &[
            "support",
            "supported",
            "supportive",
            "accept",
            "accepted",
            "acceptance",
        ],
    );
    let effect = has(
        &w,
        &[
            "helpful",
            "helped",
            "courage",
            "courageous",
            "confidence",
            "confident",
            "belong",
            "belonging",
            "proud",
            "hope",
            "ok",
        ],
    );
    let harm = has(
        &w,
        &[
            "hurt",
            "hurtful",
            "upset",
            "humiliated",
            "insulted",
            "rejected",
            "ridiculed",
        ],
    );
    let interaction = has(
        &w,
        &[
            "said",
            "told",
            "remarks",
            "comment",
            "comments",
            "rejected",
            "ridiculed",
        ],
    );
    let situation = if social && supportive && effect {
        Situation::Participation
    } else if social && harm && interaction {
        Situation::SocialHarm
    } else {
        return None;
    };
    Some(ObservedInterpretation {
        subject: subject.into(),
        situation,
        evidence: trace.core.clone(),
        confidence: 0.55_f32.min(trace.semantic.confidence),
    })
}

impl ObservedInterpretation {
    pub fn scope(&self) -> &'static str {
        match self.situation {
            Situation::Participation => "participation in a social group",
            Situation::SocialHarm => "harmful social interaction",
        }
    }
    pub fn render(&self, trace: &MemoryTrace) -> String {
        let (scope, hypothesis) = match self.situation {
            Situation::Participation => ("participation in a social group",
                "Acceptance and support were associated with a helpful outcome in this episode; this may be relevant when considering participation with unfamiliar people."),
            Situation::SocialHarm => ("harmful social interaction",
                "Hurtful interaction was associated with distress in this episode; this may be relevant when considering exposure to dismissive or unfamiliar people."),
        };
        format!("Tentative observed interpretation; subject={}; scope={scope}; confidence={:.2}; observation={}; evidence={:?}; {hypothesis} This is an analogy, not evidence that the new situation will have the same outcome, and not Claire's own experience.",
            self.subject, self.confidence, trace.observation_id.as_deref().unwrap_or(&trace.id), self.evidence)
    }

    fn relevant(&self, query: &str) -> bool {
        let w = words(query);
        let social = has(
            &w,
            &[
                "strangers",
                "people",
                "participants",
                "participant",
                "team",
                "group",
                "cooperative",
                "volunteer",
            ],
        );
        let participation = has(
            &w,
            &[
                "invited",
                "launch",
                "initiative",
                "activity",
                "attended",
                "attendance",
                "cooperative",
                "join",
                "joining",
                "participate",
            ],
        );
        let friction = has(
            &w,
            &[
                "dismisses",
                "dismissive",
                "sharply",
                "insult",
                "insults",
                "hostile",
                "ridicule",
                "ridicules",
                "hurtful",
            ],
        );
        let unfamiliar =
            has(&w, &["strangers", "unfamiliar"]) || query.to_lowercase().contains("never met");
        match self.situation {
            Situation::Participation => social && (participation || friction),
            Situation::SocialHarm => social && (friction || unfamiliar),
        }
    }
}

/// Materialize only during the ladder pass. No sleep => no transfer hypothesis.
/// Store evidence in existing operation records: file and SQLite persistence
/// already retain these. No snapshot schema change or migration is needed.
pub fn consolidate(store: &mut MemoryStore) -> usize {
    let mut count = 0;
    for trace in store.traces.values_mut() {
        let Some(i) = infer(trace) else {
            continue;
        };
        let after = i.render(trace);
        if trace
            .operations
            .iter()
            .any(|op| op.kind == "situational-interpretation" && op.after == after)
        {
            continue;
        }
        trace.record_operation(MemoryOperation {
            kind: "situational-interpretation".into(),
            at: crate::core::model::now_secs(),
            source_trace_ids: vec![trace.id.clone()],
            source_axiom_ids: Vec::new(),
            source_center: None,
            before: trace.core.clone(),
            after,
            confidence: i.confidence,
            origin: EvidenceOrigin::Event,
        });
        count += 1;
    }
    count
}

pub fn for_query(trace: &MemoryTrace, query: &str) -> Option<ObservedInterpretation> {
    if factual_query(query) {
        return None;
    }
    let i = stored(trace)?;
    if !i.relevant(query) {
        return None;
    }
    Some(i)
}

pub fn stored(trace: &MemoryTrace) -> Option<ObservedInterpretation> {
    let i = infer(trace)?;
    let rendered = i.render(trace);
    // Revalidate against the frozen core on every read. Historical or forged
    // operation text cannot add new concepts to the interpreter.
    trace
        .operations
        .iter()
        .any(|op| {
            op.kind == "situational-interpretation"
                && op.after == rendered
                && op.source_trace_ids == [trace.id.clone()]
        })
        .then_some(i)
}

pub fn factual_query(query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.contains("according to")
        || q.contains("what happened")
        || q.contains("what effect")
        || q.contains("when did")
        || q.contains("where did")
        || q.contains("do you remember")
        || [
            "who ", "what ", "which ", "did ", "was ", "were ", "has ", "how did ",
        ]
        .iter()
        .any(|prefix| q.starts_with(prefix))
}

/// A support-group question requires that relation in one evidence span.
/// "A group insulted me ... people support me" is not a support group.
pub fn factual_relation_matches(query: &str, core: &str) -> bool {
    let q = words(query);
    let c = words(core);
    if q.windows(2).any(|p| p == ["support", "group"]) {
        return c.windows(2).any(|p| p == ["support", "group"]);
    }
    true
}

/// Explicitly named observed subjects exclude other subjects. An unnamed
/// design task may use an explicitly attributed analogy from either person.
pub fn subject_matches(store: &MemoryStore, trace: &MemoryTrace, query: &str) -> bool {
    let q = words(query);
    let contains = |subject: &str| {
        let name = words(subject);
        !name.is_empty() && q.windows(name.len()).any(|p| p == name)
    };
    let named = store
        .traces
        .values()
        .filter_map(|t| crate::encode::core::reported_speech(&t.core))
        .any(|(subject, _)| contains(subject));
    !named || crate::encode::core::reported_speech(&trace.core).is_some_and(|(s, _)| contains(s))
}
