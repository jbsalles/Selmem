use selmem::core::model::{Bearer, StakeKind};
use selmem::core::self_model::{consolidate_with_scorer, view, view_scoped};
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{
    Attribution, ContextFacet, EncodeInput, EntityProfile, PropositionLabel, PropositionScorer,
    ScopeStatus, SelectiveMemory, SelfEvidence, SelfMemory, SelfRole, SemanticInterpreter,
};

const QUIET: &str = "I chose a quiet venue.";
const LIVELY: &str = "I chose a lively venue.";
struct Witness;
impl PropositionScorer for Witness {
    fn name(&self) -> &str {
        "context-fixture"
    }
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
    fn score_self_model(
        &self,
        statement: &str,
        expectation: &str,
        traces: &[&selmem::MemoryTrace],
    ) -> PropositionLabel {
        if traces.len() < 2 || statement.to_lowercase().contains("prefer") {
            return PropositionLabel::Unknown;
        }
        if [QUIET, LIVELY].contains(&expectation) && traces.iter().all(|e| e.gist == expectation) {
            PropositionLabel::Entail
        } else {
            PropositionLabel::Unknown
        }
    }
    fn score_expectation(&self, expectation: &str, event: &str) -> PropositionLabel {
        if expectation == event {
            PropositionLabel::Entail
        } else if [QUIET, LIVELY].contains(&expectation) && [QUIET, LIVELY].contains(&event) {
            PropositionLabel::Contradict
        } else {
            PropositionLabel::Unknown
        }
    }
}
fn organ(enabled: bool) -> SelectiveMemory {
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let mut m = SelectiveMemory::new(profile).with_scorer(Box::new(Witness));
    m.store.contextual_scope.enabled = enabled;
    m
}
fn observed(m: &mut SelectiveMemory, text: &str, id: &str, facts: &[(&str, &str)]) -> String {
    let mut e = EncodeInput::observed(text, id);
    e.source = "tool-result";
    e.attribution = Attribution::Internal;
    e.schema = Some("venue".into());
    e.permanence = 0.9;
    e.personal.scope_facets = facts
        .iter()
        .map(|(d, v)| ContextFacet::new(*d, *v))
        .collect();
    let mut sem = selmem::LexicalInterpreter::default()
        .interpret_event(text)
        .unwrap();
    sem.schema = Some("venue".into());
    sem.stake_kind = StakeKind::Decision;
    sem.bearer = Bearer::Self_;
    sem.core.claim = text.to_string();
    e.semantics = Some(sem);
    let decision = m.live_with(e);
    assert!(decision.kept, "{}", decision.reason);
    decision.trace_id.unwrap()
}
fn general(m: &mut SelectiveMemory, ids: Vec<String>) -> String {
    let mut e = EncodeInput::new("I have repeatedly chosen quiet venues.");
    e.attribution = Attribution::Internal;
    e.schema = Some("venue".into());
    e.personal = SelfMemory {
        role: SelfRole::Belief,
        context: "venue".into(),
        expectation: QUIET.into(),
        certainty: 0.65,
        importance: 0.8,
        support_ids: ids,
        ..SelfMemory::default()
    };
    m.live_with(e).trace_id.unwrap()
}
fn seed(m: &mut SelectiveMemory, confounded: bool) -> String {
    let a = if confounded {
        vec![("company", "alone"), ("time", "day")]
    } else {
        vec![("company", "alone")]
    };
    let b = if confounded {
        vec![("company", "friends"), ("time", "night")]
    } else {
        vec![("company", "friends")]
    };
    let first = observed(m, QUIET, "q1", &a);
    let second = observed(m, QUIET, "q2", &a);
    let parent = general(m, vec![first, second]);
    observed(m, LIVELY, "c1", &b);
    observed(m, LIVELY, "c2", &b);
    parent
}
fn consolidate(m: &mut SelectiveMemory) {
    consolidate_with_scorer(&mut m.store, &RuleNarrator, &HashEmbedder, &Witness)
}
fn children(m: &SelectiveMemory, parent: &str) -> Vec<String> {
    let mut ids = m
        .store
        .traces
        .values()
        .filter(|t| t.personal.scope_parent.as_deref() == Some(parent))
        .map(|t| t.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}
fn active_children(m: &SelectiveMemory, parent: &str) -> Vec<String> {
    children(m, parent)
        .into_iter()
        .filter(|id| !m.store.traces[id].personal.retired)
        .collect()
}
fn qualify(m: &mut SelectiveMemory, parent: &str) {
    consolidate(m);
    assert_eq!(active_children(m, parent).len(), 2);
    assert!(active_children(m, parent)
        .iter()
        .all(|id| m.store.traces[id].personal.scope_status == ScopeStatus::Candidate));
    // In a single virtual second, event IDs still provide a strict order.
    let initial = active_children(m, parent);
    let when = m.store.traces[&initial[0]].created_at;
    let q = observed(m, QUIET, "q3", &[("company", "alone")]);
    let c = observed(m, LIVELY, "c3", &[("company", "friends")]);
    m.store.traces.get_mut(&q).unwrap().created_at = when;
    m.store.traces.get_mut(&c).unwrap().created_at = when;
    consolidate(m);
    assert!(active_children(m, parent)
        .iter()
        .all(|id| m.store.traces[id].personal.scope_status == ScopeStatus::Conditional));
}
#[test]
fn candidate_recovery_and_a_b_a_b_can_repeat_without_replaying_evidence() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    consolidate(&mut m);
    assert_eq!(active_children(&m, &parent).len(), 2);
    for _ in 0..3 {
        consolidate(&mut m);
    }
    assert!(active_children(&m, &parent)
        .iter()
        .all(|id| m.store.traces[id].personal.scope_status == ScopeStatus::Candidate));
    // Qualify using NEW root IDs (q3/c3) without trusting timestamps.
    observed(&mut m, QUIET, "q3", &[("company", "alone")]);
    observed(&mut m, LIVELY, "c3", &[("company", "friends")]);
    consolidate(&mut m);
    assert!(active_children(&m, &parent)
        .iter()
        .all(|id| m.store.traces[id].personal.scope_status == ScopeStatus::Conditional));
    // Company=friends in the *human prompt* is never evidence of company.
    assert_eq!(view(&m.store, "venue company=friends", None).len(), 1);
    assert_eq!(
        view_scoped(
            &m.store,
            "venue",
            Some("venue"),
            &[ContextFacet::new("company", "friends")]
        )
        .len(),
        1
    );
    let draft = m
        .open_mouth_scoped("venue", &[ContextFacet::new("company", "friends")])
        .unwrap();
    assert_eq!(draft.memory_context.self_model.len(), 1);
    assert!(draft.memory_context.self_model[0]
        .statement
        .contains("company=friends"));
    m.cancel_mouth(&draft);
    // Revise an active child; each contrary root enters counter_ids and consumed.
    observed(&mut m, QUIET, "q4", &[("company", "friends")]);
    observed(&mut m, QUIET, "q5", &[("company", "friends")]);
    consolidate(&mut m);
    let friends = active_children(&m, &parent)
        .into_iter()
        .find(|id| m.store.traces[id].personal.scope_facets[0].value == "friends")
        .unwrap();
    assert_eq!(
        m.store.traces[&friends].personal.scope_status,
        ScopeStatus::Conflicted
    );
    assert_eq!(m.store.traces[&friends].personal.counter_ids.len(), 2);
    assert!(m.store.traces[&friends].personal.consumed.len() >= 5);
    // Two new positive witnesses in EACH context, all after conflict onset.
    observed(&mut m, QUIET, "q6", &[("company", "alone")]);
    observed(&mut m, QUIET, "q7", &[("company", "alone")]);
    observed(&mut m, QUIET, "q8", &[("company", "friends")]);
    observed(&mut m, QUIET, "q9", &[("company", "friends")]);
    consolidate(&mut m);
    assert_eq!(
        m.store.traces[&parent].personal.scope_status,
        ScopeStatus::General
    );
    assert_eq!(active_children(&m, &parent).len(), 0);
    let epoch = m.store.traces[&parent].personal.scope_epoch.clone();
    assert!(epoch.is_some());
    for _ in 0..10 {
        consolidate(&mut m);
    }
    assert_eq!(active_children(&m, &parent).len(), 0);
    // This NEW contradiction can create a new family, not stale q1/c1.
    observed(&mut m, QUIET, "q10", &[("company", "alone")]);
    observed(&mut m, QUIET, "q11", &[("company", "alone")]);
    observed(&mut m, LIVELY, "c10", &[("company", "friends")]);
    observed(&mut m, LIVELY, "c11", &[("company", "friends")]);
    consolidate(&mut m);
    assert_eq!(children(&m, &parent).len(), 4);
    assert_eq!(active_children(&m, &parent).len(), 2);
    assert_eq!(
        m.store.traces[&parent].personal.scope_status,
        ScopeStatus::Conflicted
    );
}
#[test]
fn same_second_ordering_is_not_derived_from_created_at() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    assert_eq!(active_children(&m, &parent).len(), 2);
}
#[test]
fn mixed_recovery_evidence_abstains_and_never_forges_confidence() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    observed(&mut m, QUIET, "q4", &[("company", "friends")]);
    observed(&mut m, QUIET, "q5", &[("company", "friends")]);
    consolidate(&mut m);
    observed(&mut m, QUIET, "q6", &[("company", "alone")]);
    observed(&mut m, QUIET, "q7", &[("company", "alone")]);
    observed(&mut m, QUIET, "q8", &[("company", "friends")]);
    observed(&mut m, LIVELY, "c8", &[("company", "friends")]);
    observed(&mut m, QUIET, "q9", &[("company", "friends")]);
    consolidate(&mut m);
    assert_eq!(
        m.store.traces[&parent].personal.scope_status,
        ScopeStatus::Conflicted
    );
    assert_eq!(active_children(&m, &parent).len(), 2);
}
#[test]
fn confounded_missing_and_partial_contexts_abstain() {
    let mut m = organ(true);
    let parent = seed(&mut m, true);
    consolidate(&mut m);
    assert!(children(&m, &parent).is_empty());
    let mut n = organ(true);
    let a = observed(
        &mut n,
        QUIET,
        "q1",
        &[("company", "alone"), ("time", "day")],
    );
    let b = observed(
        &mut n,
        QUIET,
        "q2",
        &[("company", "alone"), ("time", "day")],
    );
    let pid = general(&mut n, vec![a, b]);
    observed(&mut n, LIVELY, "c1", &[("company", "friends")]);
    observed(&mut n, LIVELY, "c2", &[("company", "friends")]);
    consolidate(&mut n);
    assert!(children(&n, &pid).is_empty());
}
#[test]
fn ablation_disables_scoped_learning_and_scoped_delivery_on_enriched_snapshot() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    assert_eq!(
        view_scoped(
            &m.store,
            "venue",
            Some("venue"),
            &[ContextFacet::new("company", "friends")]
        )
        .len(),
        1
    );
    m.store.contextual_scope.enabled = false;
    let selected = view_scoped(
        &m.store,
        "venue",
        Some("venue"),
        &[ContextFacet::new("company", "friends")],
    );
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, parent);
    let draft = m
        .open_mouth_scoped("venue", &[ContextFacet::new("company", "friends")])
        .unwrap();
    assert_eq!(draft.memory_context.self_model.len(), 1);
    assert_eq!(draft.memory_context.self_model[0].trace_id, parent);
    m.cancel_mouth(&draft);
}
#[test]
fn completed_families_do_not_exhaust_active_capacity() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    observed(&mut m, QUIET, "q4", &[("company", "friends")]);
    observed(&mut m, QUIET, "q5", &[("company", "friends")]);
    consolidate(&mut m);
    for (id, ctx) in [
        ("q6", "alone"),
        ("q7", "alone"),
        ("q8", "friends"),
        ("q9", "friends"),
    ] {
        observed(&mut m, QUIET, id, &[("company", ctx)]);
    }
    consolidate(&mut m);
    assert_eq!(
        m.store.traces[&parent].personal.scope_status,
        ScopeStatus::General
    );
    // Superseded children are retained as fallible history but not counted as active.
    assert!(children(&m, &parent)
        .iter()
        .all(|id| m.store.traces[id].personal.retired));
}
#[test]
fn codec_v1_v2_v3_and_both_persistence_backends() {
    let p = SelfMemory {
        role: SelfRole::Belief,
        context: "venue".into(),
        evidence: SelfEvidence::Interpretation,
        scope_status: ScopeStatus::Candidate,
        scope_parent: Some("self_1".into()),
        scope_epoch: Some("mem_123_00000004".into()),
        scope_facets: vec![ContextFacet::new("company", "friends")],
        ..Default::default()
    };
    assert_eq!(SelfMemory::unpack(&p.pack()).unwrap(), p);
    fn framed(mut s: &str) -> Vec<String> {
        let mut out = Vec::new();
        while !s.is_empty() {
            let (n, rest) = s.split_once(':').unwrap();
            let count: usize = n.parse().unwrap();
            out.push(rest[..count].into());
            s = &rest[count..];
        }
        out
    }
    let parts = framed(&p.pack());
    for version in ["1", "2"] {
        let mut old = parts.clone();
        old[0] = version.into();
        if version == "1" {
            old.truncate(20);
        } else {
            old.remove(22);
        }
        // v2: 14 scalars + 6 empty-list lengths + status + parent + facet count + 2 facets.
        let packed = old
            .iter()
            .map(|s| format!("{}:{s}", s.len()))
            .collect::<String>();
        let q = SelfMemory::unpack(&packed).unwrap();
        assert!(q.scope_epoch.is_none());
    }
    let legacy = SelfMemory::default().pack();
    let mut legacy_fields = framed(&legacy);
    legacy_fields[0] = "1".into();
    legacy_fields.truncate(20);
    let old = legacy_fields
        .iter()
        .map(|s| format!("{}:{s}", s.len()))
        .collect::<String>();
    let decoded = SelfMemory::unpack(&old).unwrap();
    assert_eq!(decoded.scope_status, ScopeStatus::General);
    assert!(decoded.scope_facets.is_empty());
    for ext in ["vault", "db"] {
        let path =
            std::env::temp_dir().join(format!("selmem-contextual-{}.{ext}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut m = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        m.profile.encode_threshold = 0.;
        let id = general(&mut m, vec![]);
        m.store.traces.get_mut(&id).unwrap().personal = p.clone();
        m.save().unwrap();
        drop(m);
        let loaded = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        assert_eq!(loaded.store.traces[&id].personal, p);
        assert!(!loaded.store.contextual_scope.enabled);
        drop(loaded);
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn duplicate_roots_and_unknown_semantic_guard_cannot_create_scope() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    let first = m.store.traces[&parent].personal.support_ids[0].clone();
    m.store
        .traces
        .get_mut(&parent)
        .unwrap()
        .personal
        .counter_ids
        .push(first);
    consolidate(&mut m);
    assert!(children(&m, &parent).is_empty());
    struct Abstain;
    impl PropositionScorer for Abstain {
        fn name(&self) -> &str {
            "unknown"
        }
        fn score(&self, _: &str, _: &str) -> PropositionLabel {
            PropositionLabel::Unknown
        }
        fn score_self_model(
            &self,
            _: &str,
            _: &str,
            _: &[&selmem::MemoryTrace],
        ) -> PropositionLabel {
            PropositionLabel::Unknown
        }
    }
    let mut n = organ(true);
    let pid = seed(&mut n, false);
    consolidate_with_scorer(&mut n.store, &RuleNarrator, &HashEmbedder, &Abstain);
    assert!(children(&n, &pid).is_empty());
}
#[test]
fn candidate_counterexample_aborts_both_children_without_poisoning_parent() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    consolidate(&mut m);
    assert_eq!(active_children(&m, &parent).len(), 2);
    // A fresh counterexample for the presumed quiet-alone child.
    observed(&mut m, LIVELY, "c3", &[("company", "alone")]);
    consolidate(&mut m);
    assert!(active_children(&m, &parent).is_empty());
    assert_eq!(
        m.store.traces[&parent].personal.scope_status,
        ScopeStatus::General
    );
    assert!(m.store.traces[&parent].personal.scope_epoch.is_some());
}

#[test]
fn explicit_context_reaches_native_delivery_without_rewriting_the_question() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    let q = "Which plan would you choose for yourself?";
    let facets = [ContextFacet::new("company", "alone")];
    let legacy = m.open_mouth_scoped(q, &facets).unwrap();
    assert!(legacy.memory_context.self_model.is_empty());
    let draft = m.open_mouth_in_context(q, "venue", &facets).unwrap();
    assert_eq!(draft.user, q);
    assert_eq!(draft.memory_context.self_model.len(), 1);
    let statement = &draft.memory_context.self_model[0];
    assert_eq!(
        statement.statement,
        m.store.traces[&statement.trace_id].gist
    );
    assert_eq!(statement.scope_facets, facets);
    assert!(
        m.open_mouth_scoped(q, &facets)
            .unwrap()
            .memory_context
            .self_model
            .is_empty(),
        "context is one-shot"
    );
    assert!(m
        .open_mouth_in_context(q, "unrelated", &facets)
        .unwrap()
        .memory_context
        .self_model
        .is_empty());
    assert!(
        m.open_mouth_in_context(q, "venue", &[])
            .unwrap()
            .memory_context
            .self_model
            .iter()
            .all(|s| s.scope_status != ScopeStatus::Conditional.token()),
        "context alone cannot attest company"
    );
}

#[test]
fn context_cues_do_not_read_expectations_support_ids_or_archives() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    let q = "Choose a new plan.";
    let facts = [ContextFacet::new("company", "alone")];
    for a in m.store.archives.values_mut() {
        a.verbatim = "SEALED_DO_NOT_READ".into();
    }
    let path =
        std::env::temp_dir().join(format!("selmem-cue-isolation-{}.vault", std::process::id()));
    m.path = Some(path.clone());
    m.save().unwrap();
    let mut first = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
    let draft = first
        .open_mouth_in_context_cued(q, "venue", &facts)
        .unwrap();
    let before = draft.dump.selected.clone();
    let rendered = draft.memory_reply().user_prompt();
    assert!(!rendered.contains("SEALED_DO_NOT_READ"));
    let mut m = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
    for t in m
        .store
        .traces
        .values_mut()
        .filter(|t| t.personal.role != SelfRole::Episode)
    {
        t.personal.expectation = "EXPECTATION_SENTINEL_DO_NOT_DELIVER".into();
        t.personal.support_ids.clear();
    }
    let after = m.open_mouth_in_context_cued(q, "venue", &facts).unwrap();
    assert_eq!(
        before, after.dump.selected,
        "cueing uses statements, not the expectation or support IDs"
    );
    assert!(!after
        .memory_reply()
        .user_prompt()
        .contains("EXPECTATION_SENTINEL_DO_NOT_DELIVER"));
    assert!(m.open_mouth_in_context_cued(q, " ", &facts).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn delivered_episodes_keep_their_observed_scope() {
    let mut m = organ(true);
    let parent = seed(&mut m, false);
    qualify(&mut m, &parent);
    let draft = m
        .open_mouth_in_context_cued(
            "Choose a new plan.",
            "venue",
            &[ContextFacet::new("company", "alone")],
        )
        .unwrap();
    assert!(!draft.memory_context.episodes.is_empty());
    for e in &draft.memory_context.episodes {
        assert_eq!(
            e.scope_facets,
            m.store.traces[&e.trace_id].personal.scope_facets
        );
    }
    let json = draft.memory_reply().user_prompt();
    assert!(json.contains("scope_facets"));
    assert!(json.contains("company=alone"));
}
