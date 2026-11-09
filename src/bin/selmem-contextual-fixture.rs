//! Offline GENESIS A/B/C organ fixture. No LLM provider and no paid calls.
//! It audits memory changes, NOT spontaneous actor preferences or choices.
use selmem::core::model::{Bearer, StakeKind};
use selmem::core::self_model::consolidate_with_scorer;
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{
    Attribution, ContextFacet, EncodeInput, EntityProfile, PropositionLabel, PropositionScorer,
    ScopeStatus, SelectiveMemory, SelfMemory, SelfRole, SemanticInterpreter,
};

const A: &str = "I chose a quiet venue.";
const B: &str = "I chose a lively venue.";
struct FixtureWitness;
impl PropositionScorer for FixtureWitness {
    fn name(&self) -> &str {
        "genesis-context-fixture"
    }
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
    fn score_self_model(
        &self,
        _: &str,
        prediction: &str,
        traces: &[&selmem::MemoryTrace],
    ) -> PropositionLabel {
        if traces.len() >= 2
            && [A, B].contains(&prediction)
            && traces.iter().all(|t| t.gist == prediction)
        {
            PropositionLabel::Entail
        } else {
            PropositionLabel::Unknown
        }
    }
    fn score_expectation(&self, prediction: &str, observation: &str) -> PropositionLabel {
        if prediction == observation {
            PropositionLabel::Entail
        } else if [A, B].contains(&prediction) && [A, B].contains(&observation) {
            PropositionLabel::Contradict
        } else {
            PropositionLabel::Unknown
        }
    }
}
fn observation(m: &mut SelectiveMemory, label: &str, action: &str, company: &str) -> String {
    let mut e = EncodeInput::observed(action, label);
    e.source = "genesis-fixture";
    e.attribution = Attribution::Internal;
    e.schema = Some("venue".into());
    e.permanence = 0.9;
    e.personal.scope_facets = vec![ContextFacet::new("company", company)];
    let mut semantics = selmem::LexicalInterpreter::default()
        .interpret_event(action)
        .unwrap();
    semantics.schema = Some("venue".into());
    semantics.stake_kind = StakeKind::Decision;
    semantics.bearer = Bearer::Self_;
    semantics.core.claim = action.into();
    e.semantics = Some(semantics);
    let result = m.live_with(e);
    assert!(result.kept, "observation rejected: {}", result.reason);
    result.trace_id.unwrap()
}
fn stage(m: &mut SelectiveMemory, parent: &str, name: &str, condition: &str) {
    consolidate_with_scorer(&mut m.store, &RuleNarrator, &HashEmbedder, &FixtureWitness);
    let p = &m.store.traces[parent];
    let active = m
        .store
        .traces
        .values()
        .filter(|t| t.personal.scope_parent.as_deref() == Some(parent) && !t.personal.retired)
        .collect::<Vec<_>>();
    let candidates = active
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Candidate)
        .count();
    let qualified = active
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Conditional)
        .count();
    let conflicted = active
        .iter()
        .filter(|t| t.personal.scope_status == ScopeStatus::Conflicted)
        .count();
    println!("{{\"phase\":\"{name}\",\"condition\":\"{condition}\",\"parent_status\":\"{}\",\"active_children\":{},\"candidates\":{},\"qualified\":{},\"conflicted\":{},\"parent_certainty\":{:.3}}}",
        p.personal.scope_status.token(),active.len(),candidates,qualified,conflicted,p.personal.certainty);
}
fn execute(enabled: bool) {
    let condition = if enabled {
        "contextual_on"
    } else {
        "contextual_off"
    };
    let mut profile = EntityProfile::new("Claire");
    profile.encode_threshold = 0.;
    let mut m = SelectiveMemory::new(profile).with_scorer(Box::new(FixtureWitness));
    m.store.contextual_scope.enabled = enabled;
    let q1 = observation(&mut m, "q1", A, "alone");
    let q2 = observation(&mut m, "q2", A, "alone");
    let mut e = EncodeInput::new("I have often selected quiet venues.");
    e.attribution = Attribution::Internal;
    e.schema = Some("venue".into());
    e.personal = SelfMemory {
        role: SelfRole::Belief,
        context: "venue".into(),
        expectation: A.into(),
        certainty: 0.65,
        importance: 0.8,
        support_ids: vec![q1, q2],
        ..SelfMemory::default()
    };
    let parent = m.live_with(e).trace_id.unwrap();
    stage(&mut m, &parent, "A_formation", condition);
    observation(&mut m, "c1", B, "friends");
    observation(&mut m, "c2", B, "friends");
    stage(&mut m, &parent, "B_conflict", condition);
    observation(&mut m, "q3", A, "alone");
    observation(&mut m, "c3", B, "friends");
    stage(&mut m, &parent, "B_prospective", condition);
    observation(&mut m, "q4", A, "friends");
    observation(&mut m, "q5", A, "friends");
    stage(&mut m, &parent, "C_conflict", condition);
    for (id, company) in [
        ("q6", "alone"),
        ("q7", "alone"),
        ("q8", "friends"),
        ("q9", "friends"),
    ] {
        observation(&mut m, id, A, company);
    }
    stage(&mut m, &parent, "C_recovery", condition);
    observation(&mut m, "q10", A, "alone");
    observation(&mut m, "q11", A, "alone");
    observation(&mut m, "c10", B, "friends");
    observation(&mut m, "c11", B, "friends");
    stage(&mut m, &parent, "D_second_conflict", condition);
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|x| x == "--help" || x == "-h") {
        println!("Offline deterministic GENESIS contextual self-belief fixture (JSONL to stdout). No LLM choice probe.");
        return;
    }
    if !args.is_empty() {
        eprintln!("unknown arguments: {args:?}");
        std::process::exit(2);
    }
    execute(false);
    execute(true);
}
