use selmem::core::self_model::{appraise, consolidate, view};
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{
    Attribution, EncodeInput, EntityProfile, PropositionLabel, PropositionScorer, SelectiveMemory,
    SelfEvidence, SelfMemory, SelfRole,
};
struct Contrary;
impl PropositionScorer for Contrary {
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Contradict
    }
    fn name(&self) -> &str {
        "fixture"
    }
}
fn organ() -> SelectiveMemory {
    let mut p = EntityProfile::new("Claire");
    p.encode_threshold = 0.;
    SelectiveMemory::new(p)
}
fn seed(m: &mut SelectiveMemory, role: SelfRole) -> String {
    let mut e = EncodeInput::new("I expect Alice to keep promises.");
    e.attribution = Attribution::Internal;
    e.schema = Some("trust Alice".into());
    e.personal = SelfMemory {
        role,
        context: "trust Alice".into(),
        expectation: "Alice keeps promises.".into(),
        importance: 0.9,
        certainty: 0.7,
        ..SelfMemory::default()
    };
    m.live_with(e).trace_id.unwrap()
}
fn counter(m: &mut SelectiveMemory, id: &str) -> selmem::EncodeDecision {
    let mut e = EncodeInput::observed("Alice broke the promise.", id);
    e.schema = Some("trust Alice".into());
    m.live_with(e)
}
#[test]
fn prior_self_changes_admission_and_cut_removes_it() {
    let mut m = organ();
    let id = seed(&mut m, SelfRole::Belief);
    m.profile.encode_threshold = 100.;
    let mut e = EncodeInput::new("Alice arrived with the document.");
    e.schema = Some("trust Alice".into());
    let d = m.live_with(e);
    assert!(d.kept);
    assert!(m.store.traces[d.trace_id.as_ref().unwrap()]
        .personal
        .appraisal
        .prior_ids
        .contains(&id));
    assert_eq!(m.store.traces[&id].personal.certainty, 0.7);
    m.store.self_policy.selection = false;
    let mut e = EncodeInput::new("Alice arrived with another document.");
    e.schema = Some("trust Alice".into());
    assert!(!m.live_with(e).kept);
}
#[test]
fn retries_and_repeated_nights_do_not_multiply_evidence() {
    let mut m = organ().with_scorer(Box::new(Contrary));
    let id = seed(&mut m, SelfRole::Belief);
    assert!(counter(&mut m, "same").kept);
    assert!(!counter(&mut m, "same").kept);
    consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    let before = m.store.traces[&id].personal.clone();
    assert!(before.certainty < 0.7);
    assert_eq!(before.counter_ids.len(), 1);
    for _ in 0..20 {
        consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    }
    assert_eq!(m.store.traces[&id].personal, before);
}
#[test]
fn uncertain_importance_and_independent_revision_route() {
    let mut m = organ();
    let id = seed(&mut m, SelfRole::Belief);
    m.store.traces.get_mut(&id).unwrap().personal.certainty = 0.01;
    let a = appraise(
        &m.store,
        "Alice broke the promise.",
        Some("trust Alice"),
        &Contrary,
    );
    assert_eq!(a.continuity, 0.9);
    m.store.traces.get_mut(&id).unwrap().personal.importance = 0.01;
    let a = appraise(
        &m.store,
        "Alice broke the promise.",
        Some("trust Alice"),
        &Contrary,
    );
    assert_eq!(a.revision, 0.8);
}
#[test]
fn external_assertion_cannot_become_self() {
    let mut m = organ();
    let mut e = EncodeInput::new("You are a coward.");
    e.attribution = Attribution::External;
    e.personal.role = SelfRole::Belief;
    e.personal.context = "conflict".into();
    assert!(!m.live_with(e).kept);
    assert!(m.store.traces.is_empty());
}
#[test]
fn aspirations_are_not_achieved_traits() {
    let mut m = organ().with_scorer(Box::new(Contrary));
    let id = seed(&mut m, SelfRole::Aspiration);
    counter(&mut m, "one");
    consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    let p = &m.store.traces[&id].personal;
    assert_eq!(p.role, SelfRole::Aspiration);
    assert_eq!(p.certainty, 0.7);
    assert_eq!(p.counter_ids.len(), 1);
}
#[test]
fn suppressed_view_has_no_archive_fallback() {
    let mut m = organ();
    let id = seed(&mut m, SelfRole::Belief);
    assert_eq!(view(&m.store, "Alice", None).len(), 1);
    m.store.traces.get_mut(&id).unwrap().suppressed = true;
    assert!(view(&m.store, "Alice", None).is_empty());
    assert!(!m.store.archives.is_empty());
}
#[test]
fn opening_is_bounded_and_deterministic() {
    let mut m = organ();
    m.profile.encode_threshold = 100.;
    let mut admitted = 0;
    for i in 0..2000 {
        let text = format!("An ordinary object number {i}.");
        let a = appraise(&m.store, &text, None, &selmem::recall::NullScorer);
        let b = appraise(&m.store, &text, None, &selmem::recall::NullScorer);
        assert_eq!(a.opening, b.opening);
        if m.live_with(EncodeInput::observed(&text, &text)).kept {
            admitted += 1;
        }
    }
    assert_eq!(admitted, 16);
}
#[test]
fn reports_do_not_receive_opening_override() {
    let mut m = organ();
    m.profile.encode_threshold = 100.;
    for i in 0..100 {
        let text = format!("An unverified report number {i}.");
        assert!(!m.live_with(EncodeInput::new(&text)).kept);
    }
}
#[test]
fn file_and_sqlite_roundtrip_all_personal_state() {
    for ext in ["vault", "db"] {
        let path = std::env::temp_dir().join(format!("self-model-{}.{ext}", std::process::id()));
        let mut m = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        m.profile.encode_threshold = 0.;
        let id = seed(&mut m, SelfRole::Commitment);
        let p = &mut m.store.traces.get_mut(&id).unwrap().personal;
        p.support_ids = vec!["support-é".into()];
        p.counter_ids = vec!["counter".into()];
        p.consumed = vec!["source:observation".into()];
        p.foundations_missing = true;
        p.protected_until = 12345;
        p.appraisal.prior_ids = vec!["prior".into()];
        p.appraisal.contradicts = vec!["prior".into()];
        p.appraisal.revision = 0.8;
        let expected = p.clone();
        m.save().unwrap();
        drop(m);
        let m = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        assert_eq!(m.store.traces[&id].personal, expected);
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn codec_preserves_unicode_and_rejects_corruption() {
    let p = SelfMemory {
        role: SelfRole::Belief,
        context: "éthique\n: Alice".into(),
        expectation: "Une promesse\nest tenue.".into(),
        support_ids: vec!["a\tb:c".into()],
        ..SelfMemory::default()
    };
    assert_eq!(SelfMemory::unpack(&p.pack()).unwrap(), p);
    assert!(SelfMemory::unpack("999:x").is_err());
}
#[test]
fn forgotten_foundation_reduces_certainty_once() {
    let mut m = organ();
    let id = seed(&mut m, SelfRole::Belief);
    m.store
        .traces
        .get_mut(&id)
        .unwrap()
        .personal
        .support_ids
        .push("lost".into());
    consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    let p = m.store.traces[&id].personal.clone();
    assert!(p.foundations_missing && p.certainty < 0.7);
    consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    assert_eq!(m.store.traces[&id].personal, p);
    assert!(!view(&m.store, "Alice", None).is_empty());
}
#[test]
fn sleep_bootstraps_from_distinct_internal_observations() {
    let mut m = organ();
    for i in 0..2 {
        let root = format!("action-{i}");
        let mut e = EncodeInput::observed("I refused to hide the error from Alice.", &root);
        e.attribution = Attribution::Internal;
        e.schema = Some("honesty Alice".into());
        let mut semantics = selmem::encode::semantic::SemanticInterpreter::interpret_event(
            &selmem::encode::semantic::LexicalInterpreter,
            e.event,
        )
        .unwrap();
        semantics.schema = e.schema.clone();
        semantics.stake_kind = selmem::core::model::StakeKind::Decision;
        semantics.bearer = selmem::core::model::Bearer::Self_;
        e.semantics = Some(semantics);
        e.permanence = 0.9;
        assert!(m.live_with(e).kept);
    }
    m.sleep_deep();
    let bs: Vec<_> = m
        .store
        .traces
        .values()
        .filter(|t| t.personal.role == SelfRole::Belief)
        .collect();
    assert_eq!(bs.len(), 1);
    assert_eq!(bs[0].personal.support_ids.len(), 2);
    assert!(!bs[0].personal.expectation.is_empty());
    assert!(!bs[0].reality.verifiable);
    assert!(bs[0].archive_id.is_none());
}
#[test]
fn self_delivery_is_labeled_and_respects_component_cut() {
    let mut m = organ();
    seed(&mut m, SelfRole::Aspiration);
    let d = m.open_mouth("What matters about Alice and promises?");
    let r = d.memory_reply();
    let c = r.filtered_context();
    assert_eq!(c.self_model.len(), 1);
    assert_eq!(c.self_model[0].role, "aspiration");
    assert!(c.episodes.is_empty());
    let mut o = r.options.clone();
    o.dispositions = false;
    assert!(c.filtered(&o).self_model.is_empty());
    m.cancel_mouth(&d);
}
#[test]
fn contradiction_protection_is_temporary() {
    let mut m = organ().with_scorer(Box::new(Contrary));
    seed(&mut m, SelfRole::Belief);
    let id = counter(&mut m, "event").trace_id.unwrap();
    let until = m.store.traces[&id].personal.protected_until;
    assert!(until > m.clock.now());
    m.sleep_deep();
    assert!(m.store.traces.contains_key(&id));
    m.advance_hours(25.);
    assert!(until < m.clock.now());
    assert!(m.store.traces[&id].permanence < 0.8);
}
#[test]
fn reports_and_interpretations_are_not_counterevidence() {
    for evidence in [SelfEvidence::Report, SelfEvidence::Interpretation] {
        let mut m = organ().with_scorer(Box::new(Contrary));
        let id = seed(&mut m, SelfRole::Belief);
        let mut e = EncodeInput::new("Alice says I never keep promises.");
        e.personal.evidence = evidence;
        e.schema = Some("trust Alice".into());
        m.live_with(e);
        consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
        assert_eq!(m.store.traces[&id].personal.certainty, 0.7);
        assert!(m.store.traces[&id].personal.counter_ids.is_empty());
    }
}
#[test]
fn selection_cut_still_allows_learning_from_normally_admitted_observations() {
    let mut m = organ().with_scorer(Box::new(Contrary));
    let id = seed(&mut m, SelfRole::Belief);
    m.store.self_policy.selection = false;
    assert!(counter(&mut m, "observed").kept);
    consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    assert!(m.store.traces[&id].personal.certainty < 0.7);
}
#[test]
fn retired_belief_cannot_bootstrap_again_from_consumed_observations() {
    let mut m = organ().with_scorer(Box::new(Contrary));
    let id = seed(&mut m, SelfRole::Belief);
    m.store.traces.get_mut(&id).unwrap().personal.certainty = 0.2;
    for i in 0..2 {
        let root = format!("counter-{i}");
        decision(
            &mut m,
            "I failed to verify the incident records.",
            &root,
            "trust Alice",
            selmem::core::model::StakeKind::Decision,
        );
        consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    }
    assert!(m.store.traces[&id].personal.retired);
    for _ in 0..20 {
        consolidate(&mut m.store, &RuleNarrator, &HashEmbedder);
    }
    assert_eq!(
        m.store
            .traces
            .values()
            .filter(|t| t.personal.role == SelfRole::Belief && !t.personal.retired)
            .count(),
        0
    );
}
fn decision(
    m: &mut SelectiveMemory,
    event: &str,
    id: &str,
    context: &str,
    kind: selmem::core::model::StakeKind,
) -> String {
    let mut e = EncodeInput::observed(event, id);
    e.source = "world";
    e.attribution = Attribution::Internal;
    e.permanence = 1.;
    let mut sem = selmem::encode::semantic::SemanticInterpreter::interpret_event(
        &selmem::encode::semantic::LexicalInterpreter,
        event,
    )
    .unwrap();
    sem.schema = Some(context.into());
    sem.stake_kind = kind;
    sem.bearer = selmem::core::model::Bearer::Self_;
    e.semantics = Some(sem);
    m.live_with(e).trace_id.unwrap()
}
#[test]
fn explicit_context_mismatch_is_not_overridden_by_shared_gist_words() {
    let mut m = organ();
    let id = seed(&mut m, SelfRole::Belief);
    let t = m.store.traces.get_mut(&id).unwrap();
    t.personal.context = "verification".into();
    t.gist = "I noticed a routine note during the handoff.".into();
    assert!(view(
        &m.store,
        "I noticed a routine note about coordination during the handoff.",
        Some("coordination")
    )
    .is_empty());
    assert!(view(&m.store, "routine note during handoff", None).is_empty());
    assert_eq!(view(&m.store, "verification", None).len(), 1);
}
struct UncheckedProposal;
impl selmem::Narrator for UncheckedProposal {
    fn reconstruct(&self, t: &selmem::MemoryTrace, _: &selmem::Mood, _: &str) -> String {
        t.gist.clone()
    }
    fn distill_axiom(&self, _: &[&selmem::MemoryTrace]) -> Option<String> {
        None
    }
    fn distill_self(&self, _: &[&selmem::MemoryTrace], _: &str) -> Option<(String, String)> {
        Some((
            "I closed the incident despite seeing a note later.".into(),
            "I will ignore notes unless they are enforced.".into(),
        ))
    }
}
#[test]
fn one_action_and_one_topic_reminder_do_not_bootstrap_a_behavior() {
    let mut m = organ();
    decision(
        &mut m,
        "I closed the incident without checking.",
        "incident-A",
        "verification",
        selmem::core::model::StakeKind::Decision,
    );
    decision(
        &mut m,
        "I noticed a verification note.",
        "note-B",
        "verification",
        selmem::core::model::StakeKind::Presence,
    );
    consolidate(&mut m.store, &UncheckedProposal, &HashEmbedder);
    assert!(!m
        .store
        .traces
        .values()
        .any(|t| t.personal.role == SelfRole::Belief));
}
#[test]
fn unchecked_cross_episode_causal_proposal_is_rejected_when_no_guard_is_available() {
    let mut m = organ();
    decision(
        &mut m,
        "I closed the first incident without checking.",
        "incident-A",
        "verification",
        selmem::core::model::StakeKind::Decision,
    );
    decision(
        &mut m,
        "I checked records in another incident.",
        "incident-B",
        "verification",
        selmem::core::model::StakeKind::Decision,
    );
    consolidate(&mut m.store, &UncheckedProposal, &HashEmbedder);
    assert!(!m
        .store
        .traces
        .values()
        .any(|t| t.personal.role == SelfRole::Belief));
    assert_eq!(m.store.self_consolidation_audit.len(), 1);
    assert_eq!(
        m.store.self_consolidation_audit[0].verdict,
        PropositionLabel::Unknown
    );
    let traces = m.store.traces.values().collect::<Vec<_>>();
    let payload = selmem::core::self_model::guard_input("statement", "expectation", &traces);
    assert!(
        payload.contains("incident-A")
            && payload.contains("incident-B")
            && payload.contains("encoded_at")
    );
}

struct PatternProposal;
impl selmem::Narrator for PatternProposal {
    fn reconstruct(&self, t: &selmem::MemoryTrace, _: &selmem::Mood, _: &str) -> String {
        t.gist.clone()
    }
    fn distill_axiom(&self, _: &[&selmem::MemoryTrace]) -> Option<String> {
        None
    }
    fn distill_self(&self, _: &[&selmem::MemoryTrace], _: &str) -> Option<(String, String)> {
        Some((
            "I have skipped verification on separate occasions.".into(),
            "I may skip verification again.".into(),
        ))
    }
}
struct PatternWitness;
impl PropositionScorer for PatternWitness {
    fn name(&self) -> &str {
        "finite-provenance-test"
    }
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
    fn score_self_model(&self, _: &str, _: &str, _: &[&selmem::MemoryTrace]) -> PropositionLabel {
        PropositionLabel::Entail
    }
    fn score_expectation(&self, _: &str, event: &str) -> PropositionLabel {
        match event {
            "I skipped verification." => PropositionLabel::Entail,
            "I verified the records." => PropositionLabel::Contradict,
            _ => PropositionLabel::Unknown,
        }
    }
}
#[test]
fn mixed_group_does_not_turn_counterexamples_or_unknowns_into_support() {
    use selmem::core::{model::StakeKind, self_model::consolidate_with_scorer};
    let mut m = organ();
    let a = decision(
        &mut m,
        "I skipped verification.",
        "skip-A",
        "verification",
        StakeKind::Decision,
    );
    let b = decision(
        &mut m,
        "I skipped verification.",
        "skip-B",
        "verification",
        StakeKind::Decision,
    );
    let c = decision(
        &mut m,
        "I verified the records.",
        "check-C",
        "verification",
        StakeKind::Decision,
    );
    let u = decision(
        &mut m,
        "I closed the case.",
        "unknown-D",
        "verification",
        StakeKind::Decision,
    );
    consolidate_with_scorer(
        &mut m.store,
        &PatternProposal,
        &HashEmbedder,
        &PatternWitness,
    );
    let belief = m
        .store
        .traces
        .values()
        .find(|t| t.personal.role == SelfRole::Belief)
        .unwrap();
    assert_eq!(belief.personal.support_ids.len(), 2);
    assert!(belief.personal.support_ids.contains(&a) && belief.personal.support_ids.contains(&b));
    assert_eq!(belief.personal.counter_ids, vec![c.clone()]);
    assert!(!belief.personal.support_ids.contains(&u));
    assert_eq!(belief.personal.consumed.len(), 3);
    let audit = &m.store.self_consolidation_audit[0];
    assert_eq!(audit.reason, "accepted");
    assert_eq!(audit.source_trace_ids.len(), 4);
    assert_eq!(audit.counter_trace_ids, vec![c]);
    assert_eq!(audit.unknown_trace_ids, vec![u]);
}
#[test]
fn group_guard_alone_cannot_replace_two_individually_supported_actions() {
    use selmem::core::{model::StakeKind, self_model::consolidate_with_scorer};
    let mut m = organ();
    decision(
        &mut m,
        "I skipped verification.",
        "skip-A",
        "verification",
        StakeKind::Decision,
    );
    decision(
        &mut m,
        "I verified the records.",
        "check-B",
        "verification",
        StakeKind::Decision,
    );
    decision(
        &mut m,
        "I closed the case.",
        "unknown-C",
        "verification",
        StakeKind::Decision,
    );
    consolidate_with_scorer(
        &mut m.store,
        &PatternProposal,
        &HashEmbedder,
        &PatternWitness,
    );
    assert!(!m
        .store
        .traces
        .values()
        .any(|t| t.personal.role == SelfRole::Belief));
    let a = &m.store.self_consolidation_audit[0];
    assert_eq!(a.reason, "insufficient_individual_support");
    assert_eq!(a.verdict, PropositionLabel::Unknown);
}
struct AbstainingProposal;
impl selmem::Narrator for AbstainingProposal {
    fn reconstruct(&self, t: &selmem::MemoryTrace, _: &selmem::Mood, _: &str) -> String {
        t.gist.clone()
    }
    fn distill_axiom(&self, _: &[&selmem::MemoryTrace]) -> Option<String> {
        None
    }
    fn distill_self(&self, _: &[&selmem::MemoryTrace], _: &str) -> Option<(String, String)> {
        Some((String::new(), String::new()))
    }
}
#[test]
fn narrator_abstention_is_audited_without_injecting_a_replacement_belief() {
    use selmem::core::model::StakeKind;
    let mut m = organ();
    for id in ["action-A", "action-B"] {
        decision(
            &mut m,
            "I skipped verification.",
            id,
            "verification",
            StakeKind::Decision,
        );
    }
    consolidate(&mut m.store, &AbstainingProposal, &HashEmbedder);
    assert!(!m
        .store
        .traces
        .values()
        .any(|t| t.personal.role == SelfRole::Belief));
    assert_eq!(
        m.store.self_consolidation_audit[0].reason,
        "narrator_abstained"
    );
    assert_eq!(
        m.store.self_consolidation_audit[0].source_trace_ids.len(),
        2
    );
}
