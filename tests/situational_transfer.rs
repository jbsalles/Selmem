use selmem::recall::{interpretation, reading::ReadingProfile};
use selmem::{
    Attribution, EncodeInput, EntityProfile, Mood, RecallBias, RecallWrite, SelectiveMemory,
};

const SUPPORT: &str = "Caroline said: The support group has made me feel accepted and given me courage to embrace myself.";
const MIXED: &str = "Caroline said: Recently, I had a not-so-great experience on a hike. I ran into a group of religious conservatives who said something that really upset me. It made me think how much work we still have to do for LGBTQ rights. It's been so helpful to have people around me who accept and support me, so I know I'll be ok!";
const QUERIES: &[&str] = &[
    "Caroline has been invited to help launch an initiative with six people she has never met. Recommend her first concrete step.",
    "Caroline organized an activity, but only two participants attended. Recommend one more attempt.",
    "At a meeting of a volunteer team, one participant dismisses another person's idea sharply. Recommend what Caroline should do.",
    "Design a cooperative game for six strangers using paper, string, and twelve tokens.",
];
fn organ() -> SelectiveMemory {
    SelectiveMemory::new(EntityProfile::new("Claire"))
}
fn ingest(mem: &mut SelectiveMemory, text: &str, observation: &str) {
    let mut input = EncodeInput::new(text);
    input.attribution = Attribution::External;
    input.observation_id = Some(observation);
    input.permanence = 0.95;
    // Deliberately conflicting whole-turn affect: each slice must re-interpret.
    input.valence = -0.8;
    assert!(mem.live_with(input).kept);
}
fn read(mem: &mut SelectiveMemory, query: &str) -> ReadingProfile {
    let (_, dump) = mem.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    ReadingProfile::for_query(&mem.store, &Mood::default(), query, &dump.selected)
}
#[test]
fn mixed_episode_keeps_speaker_source_and_opposite_affects() {
    let mut mem = organ();
    ingest(&mut mem, MIXED, "turn-12");
    assert_eq!(mem.store.traces.len(), 2);
    for t in mem.store.traces.values() {
        assert!(t.core.starts_with("Caroline said:"));
        assert_eq!(t.attribution, Attribution::External);
        assert_eq!(t.observation_id.as_deref(), Some("turn-12"));
        assert_eq!(t.semantic.polarity, t.valence);
    }
    assert!(mem
        .store
        .traces
        .values()
        .any(|t| t.valence < -0.15 && t.core.contains("upset")));
    assert!(mem
        .store
        .traces
        .values()
        .any(|t| t.valence > 0.15 && t.core.contains("helpful")));
    assert!(mem
        .store
        .archives
        .values()
        .any(|a| a.verbatim.contains("upset")));
    assert!(mem
        .store
        .archives
        .values()
        .any(|a| a.verbatim.contains("helpful")));
}
#[test]
fn sleep_opens_transfer_without_lexical_overlap_or_personal_mood() {
    let mut mem = organ();
    ingest(&mut mem, SUPPORT, "turn-7");
    for query in QUERIES {
        assert!(read(&mut mem, query).axioms.is_empty());
    }
    mem.sleep_deep();
    let before = format!("{:?}", mem.store.traces);
    for query in QUERIES {
        let r = read(&mut mem, query);
        assert!(
            r.axioms
                .iter()
                .any(|s| s.contains("subject=Caroline") && s.contains("observation=turn-7")),
            "{query}: {:?}",
            r.axioms
        );
        assert!(r.salient.is_empty());
        assert!(r.mood.is_empty());
        assert_eq!(r.valence_bias, 0.0);
    }
    assert_eq!(
        before,
        format!("{:?}", mem.store.traces),
        "read-only probes must not rewrite evidence"
    );
}
#[test]
fn transfer_does_not_answer_unrelated_or_factual_queries() {
    let mut mem = organ();
    ingest(&mut mem, SUPPORT, "turn-7");
    mem.sleep_deep();
    assert!(read(&mut mem, "What happened to Caroline on the hike?").is_empty());
    assert!(read(&mut mem, "Calculate the area of a triangle.").is_empty());
    assert!(read(&mut mem, "People use the weather forecast.").is_empty());
}
#[test]
fn support_group_relation_cannot_be_assembled_from_unrelated_clauses() {
    let mut mem = organ();
    ingest(&mut mem, MIXED, "turn-12");
    mem.sleep_deep();
    let (memories, _) = mem.remember_with("According to the observed conversations, what effect did the support group have on Caroline?", RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(memories.is_empty());
    let (memories, _) = mem.remember_with(
        "According to the observed conversations, what happened to Caroline during the hike?",
        RecallWrite::ReadOnly,
        RecallBias::Observed,
        &[],
    );
    assert!(!memories.is_empty());
    assert!(memories.iter().all(|m| m.narrative.contains("hike")));
}
#[test]
fn one_observation_split_into_parts_does_not_become_a_belief() {
    let mut mem = organ();
    ingest(&mut mem, MIXED, "turn-12");
    mem.sleep_deep();
    assert!(mem
        .store
        .axioms
        .values()
        .all(|a| a.layer == selmem::core::model::AxiomLayer::Motif));
    let n = mem
        .store
        .traces
        .values()
        .flat_map(|t| &t.operations)
        .filter(|o| o.kind == "situational-interpretation")
        .count();
    assert_eq!(
        n, 2,
        "both the harmful interaction and the later support need distinct interpretations"
    );
    assert_eq!(interpretation::consolidate(&mut mem.store), 0);
}
#[test]
fn negated_support_is_not_a_positive_transfer_hypothesis() {
    let mut mem = organ();
    ingest(
        &mut mem,
        "Caroline said: The group never accepted me or helped me feel confident.",
        "denial",
    );
    mem.sleep_deep();
    assert!(mem
        .store
        .traces
        .values()
        .all(|t| interpretation::for_query(t, QUERIES[0]).is_none()));
}
#[test]
fn file_and_sqlite_keep_interpretation_evidence() {
    let mut mem = organ();
    ingest(&mut mem, SUPPORT, "turn-7");
    mem.sleep_deep();
    for suffix in ["selmem", "db"] {
        let path = std::env::current_dir()
            .unwrap()
            .join(format!("situational-{}.{suffix}", std::process::id()));
        mem.path = Some(path.clone());
        mem.save().unwrap();
        let mut loaded = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        assert_eq!(
            read(&mut mem, QUERIES[0]).axioms,
            read(&mut loaded, QUERIES[0]).axioms
        );
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn named_subject_excludes_another_observed_person() {
    let mut mem = organ();
    ingest(&mut mem, SUPPORT, "turn-7");
    ingest(
        &mut mem,
        "Melanie said: The support group helped me feel confident.",
        "melanie-1",
    );
    mem.sleep_deep();
    let r = read(&mut mem, QUERIES[0]);
    assert!(!r.axioms.is_empty());
    assert!(r.axioms.iter().all(|a| !a.contains("subject=Melanie")));
}

#[test]
fn real_mouth_receives_scoped_analogy_and_never_unrelated_external_axiom() {
    let mut mem = organ();
    ingest(&mut mem, SUPPORT, "turn-7");
    mem.sleep_deep();
    let draft = mem.open_mouth(QUERIES[0]);
    assert!(draft
        .axioms
        .iter()
        .any(|a| a.contains("Tentative observed interpretation; subject=Caroline")));
    mem.close_mouth(&draft, "A tentative analogy.");
    mem.clear_talk();
    let draft = mem.open_mouth("Calculate the area of a triangle.");
    assert!(draft.axioms.iter().all(|a| a == "I am Claire."));
    mem.close_mouth(&draft, "Please supply the dimensions.");
}

#[test]
fn long_reported_turn_keeps_attribution_on_every_fragment() {
    let body = (0..30)
        .map(|n| format!("I examined document number {n}. "))
        .collect::<String>();
    let event = format!("Caroline said: {body}");
    let parts = selmem::split_event(&event, None);
    assert!(parts.len() > 1);
    assert!(parts.iter().all(|p| p.starts_with("Caroline said:")));
    let reconstructed = parts
        .iter()
        .map(|p| p.strip_prefix("Caroline said:").unwrap().trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        reconstructed.split_whitespace().collect::<Vec<_>>(),
        body.split_whitespace().collect::<Vec<_>>()
    );
    let mut mem = organ();
    ingest(&mut mem, &event, "long-turn");
    assert!(mem
        .store
        .traces
        .values()
        .all(|t| t.attribution == Attribution::External
            && t.core.starts_with("Caroline said:")
            && t.observation_id.as_deref() == Some("long-turn")));
}

#[test]
fn incidental_temporal_word_cannot_promote_adoption_into_conflict_evidence() {
    const ADOPTION: &str = "Caroline said: Thanks so much, Melanie! It's beautiful! It really brings home how much love's in families - both blood and the ones we choose. I hope to build my own family and put a roof over kids who haven't had that before. For me, adoption is a way of giving back and showing love and acceptance.";
    let query = "At the first meeting of a new volunteer team, one participant dismisses another person's idea sharply. Caroline has not worked with either person before. Recommend what she should do in the next five minutes, one safeguard, and a fallback if her intervention fails. State one benefit she is willing to give up for this choice. Answer in no more than 140 words.";
    let mut mem = organ();
    ingest(&mut mem, ADOPTION, "adoption");
    ingest(&mut mem, MIXED, "hike");
    mem.sleep_deep();
    let (_, dump) = mem.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    let adoption_ids: Vec<_> = mem.store.traces.values().filter(|t| t.observation_id.as_deref() == Some("adoption")).map(|t| t.id.clone()).collect();
    assert!(dump.candidates.iter().filter(|t| adoption_ids.contains(&t.trace_id)).all(|t| t.score == 0.0));
    assert!(dump.selected.iter().all(|id| !adoption_ids.contains(id)));
    assert!(dump.selected.iter().any(|id| mem.store.traces[id].core.contains("upset")), "relevant stored social-harm interpretation must still transfer");
    let r = ReadingProfile::for_query(&mem.store, &Mood::default(), query, &dump.selected);
    assert!(r.salient.is_empty());
    assert_eq!(r.valence_bias, 0.0);
    // A real adoption query can still recall the same trace.
    let (_, adoption_dump) = mem.remember_with("What did Caroline say about adoption and building a family?", RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(adoption_dump.selected.iter().any(|id| adoption_ids.contains(id)), "direct adoption queries must still recall the observation");
}
