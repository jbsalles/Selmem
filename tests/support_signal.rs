//! P0 regressions: observation-support and schema-center cuts are independent.
use selmem::{
    Attribution, EncodeInput, EntityProfile, OrganCut, PropositionLabel, PropositionScorer,
    RecallBias, RecallWrite, SchemaCenter, SelectiveMemory,
};

#[test]
fn recall_signal_and_schema_prototype_are_independent_ablation_axes() {
    let mut profile = EntityProfile::new("test");
    profile.encode_threshold = 0.05;
    let mut organ = SelectiveMemory::new(profile);
    organ.set_observation_evidence(true);
    let claim = "Mira supplied six batteries to the workshop.";
    let mut event = EncodeInput::new(claim);
    event.source = "sensor";
    event.observation_id = Some("first");
    event.permanence = 0.6; // Avoid the high-permanence explicit-cue score floor.
    event.self_relevance = 0.9;
    event.arousal = 0.8;
    event.attribution = Attribution::Internal;
    event.schema = Some("workshop".into());
    let id = organ.live_with(event).trace_id.expect("carrier trace");
    let mut repeated = EncodeInput::new(claim);
    repeated.source = "sensor";
    repeated.observation_id = Some("second");
    assert!(organ.live_with(repeated).kept);
    assert!(
        organ
            .store
            .observation_evidence
            .support_weight(&organ.store, &id)
            > 0.0
    );
    organ.store.centers.insert(
        "workshop".into(),
        SchemaCenter {
            schema: "workshop".into(),
            core: "A DISTINCT WORKSHOP SCHEMA PROTOTYPE".into(),
            valence: 0.0,
            weight: 1.0,
            hub_id: Some(id.clone()),
            axiom_id: None,
        },
    );
    let gist = organ.store.traces[&id].gist.clone();
    let core = organ.store.traces[&id].core.clone();

    let mut recall = |cut: OrganCut| {
        organ.cut = cut;
        let (memories, dump) = organ.remember_with(
            "Mira supplied six batteries to the workshop",
            RecallWrite::ReadOnly,
            RecallBias::ForceMarked,
            &[id.clone()],
        );
        let narrative = memories
            .iter()
            .find(|m| m.trace_id == id)
            .expect("selected carrier")
            .narrative
            .clone();
        let score = dump
            .candidates
            .iter()
            .find(|c| c.trace_id == id)
            .expect("ranked carrier")
            .score;
        (narrative, score)
    };

    let (full, full_score) = recall(OrganCut::full());
    let (without_support, no_support_score) = recall(OrganCut::no_support_signal());
    let (without_prototype, no_prototype_score) = recall(OrganCut::no_schema_prototype());
    let (without_both, _) = recall(OrganCut {
        support_signal: false,
        schema_prototype: false,
        ..OrganCut::full()
    });
    assert!(full.contains("Support strength"));
    assert!(full.contains("A DISTINCT WORKSHOP SCHEMA PROTOTYPE"));
    assert!(!without_support.contains("Support strength"));
    assert!(without_support.contains("2 distinct observation IDs"));
    assert!(without_support.contains("A DISTINCT WORKSHOP SCHEMA PROTOTYPE"));
    assert!(without_prototype.contains("Support strength"));
    assert!(!without_prototype.contains("A DISTINCT WORKSHOP SCHEMA PROTOTYPE"));
    assert!(!without_both.contains("Support strength"));
    assert!(!without_both.contains("A DISTINCT WORKSHOP SCHEMA PROTOTYPE"));
    assert!(
        full_score > no_support_score,
        "support should modestly boost recall ranking"
    );
    assert!(
        (full_score - no_prototype_score).abs() < 1e-5,
        "schema cues must not change recall scores"
    );
    assert_eq!(organ.store.traces[&id].gist, gist);
    assert_eq!(organ.store.traces[&id].core, core);
}

struct Contradiction;
impl PropositionScorer for Contradiction {
    fn score(&self, original: &str, next: &str) -> PropositionLabel {
        if original.contains("six batteries") && next.contains("seven batteries") {
            PropositionLabel::Contradict
        } else {
            PropositionLabel::Unknown
        }
    }
    fn name(&self) -> &str {
        "controlled-contradiction"
    }
}

#[test]
fn counter_observations_do_not_create_positive_reconstructive_weight() {
    let mut organ =
        SelectiveMemory::new(EntityProfile::new("test")).with_scorer(Box::new(Contradiction));
    organ.set_observation_evidence(true);
    let primary = "Mira supplied six batteries to the workshop.";
    let counter = "Mira supplied seven batteries to the workshop.";
    let id = add_evidence(&mut organ, "carrier", primary)
        .trace_id
        .expect("carrier");
    for name in ["counter-a", "counter-b", "counter-c"] {
        assert!(add_evidence(&mut organ, name, counter)
            .reason
            .contains("counter-observation"));
    }
    assert_eq!(
        organ
            .store
            .observation_evidence
            .support_weight(&organ.store, &id),
        0.0
    );
    assert_eq!(
        organ.store.observation_evidence.groups[&id].counts()[counter],
        3
    );
    assert!(add_evidence(&mut organ, "positive-b", primary).kept);
    let positive_weight = organ
        .store
        .observation_evidence
        .support_weight(&organ.store, &id);
    assert!(positive_weight > 0.0);
    assert!(add_evidence(&mut organ, "counter-d", counter).kept);
    assert_eq!(
        organ
            .store
            .observation_evidence
            .support_weight(&organ.store, &id),
        positive_weight
    );
    assert!(add_evidence(&mut organ, "positive-c", primary).kept);
    assert!(
        organ
            .store
            .observation_evidence
            .support_weight(&organ.store, &id)
            > positive_weight
    );
    assert_eq!(
        organ.store.observation_evidence.groups[&id].counts()[counter],
        4
    );
}

fn add_evidence<'a>(
    organ: &mut SelectiveMemory,
    observation_id: &'a str,
    claim: &'a str,
) -> selmem::EncodeDecision {
    let mut input = EncodeInput::new(claim);
    input.source = "sensor";
    input.observation_id = Some(observation_id);
    input.permanence = 0.9;
    organ.live_with(input)
}
