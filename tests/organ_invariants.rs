use selmem::core::model::{Bearer, TraceStatus};
use selmem::{Attribution, AxiomLayer, EncodeInput, EntityProfile, SelectiveMemory};

fn event(mem: &mut SelectiveMemory, text: &str, origin: &str, mark: &str, v: f32) -> String {
    let mut input = EncodeInput::new(text);
    input.observation_id = Some(origin);
    input.permanence = 1.0;
    input.valence = v;
    input.arousal = 0.9;
    input.self_relevance = 0.9;
    input.schema = Some("collaboration".into());
    let id = mem.live_with(input).trace_id.expect("kept");
    let t = mem.store.traces.get_mut(&id).unwrap();
    t.stake_mark = mark.into();
    t.bearer = Bearer::Other;
    t.attribution = Attribution::External;
    id
}
fn ladder(mem: &mut SelectiveMemory) {
    selmem::dream::ladder::run(&mut mem.store, &selmem::RuleNarrator);
}
#[test]
fn annotation_core_is_retained_and_anchor_is_compact() {
    let text="Alice walked through a long corridor in the office before she said the project was cancelled.";
    let claim = "Alice said the project was cancelled.";
    let mut s =
        selmem::SemanticInterpreter::interpret_event(&selmem::LexicalInterpreter, text).unwrap();
    s.core.claim = claim.into();
    let mut input = EncodeInput::new(text);
    input.permanence = 1.0;
    input.semantics = Some(s);
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let id = mem.live_with(input).trace_id.unwrap();
    let t = &mem.store.traces[&id];
    assert_eq!(t.core, claim);
    assert_eq!(t.semantic.claim, claim);
    assert_eq!(t.reality.claim, claim);
    assert_eq!(mem.audit(&id), Some(text));
}
#[test]
fn greetings_do_not_consume_the_extractive_core() {
    let text = "Hello! How are you? A group said something that upset Caroline during the hike.";
    assert_eq!(
        selmem::encode::core::extractive_core(text),
        "A group said something that upset Caroline during the hike."
    );
}
#[test]
fn copies_of_one_observation_cannot_mint_a_belief() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    for _ in 0..4 {
        event(
            &mut mem,
            "Alice kept the project open.",
            "same-observation",
            "project",
            0.8,
        );
    }
    ladder(&mut mem);
    assert!(mem.who_am_i().iter().all(|a| a.layer == AxiomLayer::Motif));
    assert!(mem
        .who_am_i()
        .iter()
        .all(|a| a.support_trace_ids.len() == 1));
}
#[test]
fn association_is_not_independent_evidence() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let a = event(&mut mem, "Alice opened the project.", "a", "project", 0.8);
    let b = event(&mut mem, "Bob closed the garden.", "b", "garden", -0.8);
    mem.store.link(&a, &b);
    ladder(&mut mem);
    for ax in mem.who_am_i() {
        assert!(ax
            .support_trace_ids
            .iter()
            .all(|id| mem.store.traces[id].stake_mark == ax.stake_mark));
    }
}
#[test]
fn actors_and_attribution_do_not_share_a_belief() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    for i in 0..3 {
        let id = event(
            &mut mem,
            "Alice opened the project.",
            &format!("event-{i}"),
            "project",
            0.8,
        );
        let t = mem.store.traces.get_mut(&id).unwrap();
        match i {
            0 => {}
            1 => t.bearer = Bearer::Self_,
            _ => t.attribution = Attribution::Internal,
        }
    }
    ladder(&mut mem);
    assert_eq!(mem.who_am_i().len(), 3);
    assert!(mem.who_am_i().iter().all(|a| a.layer == AxiomLayer::Motif));
}
#[test]
fn independent_contrary_events_can_revise_a_strong_belief() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    for i in 0..3 {
        event(
            &mut mem,
            "Alice opened the project.",
            &format!("past-{i}"),
            "project",
            0.8,
        );
    }
    ladder(&mut mem);
    let old = mem.who_am_i()[0].id.clone();
    event(
        &mut mem,
        "Alice closed the project.",
        "new-0",
        "project",
        -0.8,
    );
    ladder(&mut mem);
    assert!(mem.store.axioms[&old].superseded_by.is_none());
    for i in 1..3 {
        event(
            &mut mem,
            "Alice closed the project.",
            &format!("new-{i}"),
            "project",
            -0.8,
        );
    }
    ladder(&mut mem);
    assert!(mem.store.axioms[&old].superseded_by.is_some());
    assert!(mem.who_am_i().iter().any(|a| a.valence < -0.2));
    let revised = mem.who_am_i()[0].id.clone();
    ladder(&mut mem);
    assert!(
        mem.store.axioms[&revised].superseded_by.is_none(),
        "old evidence must not become new contrary evidence"
    );
}
#[test]
fn contextual_traits_preserve_the_observer_and_do_not_assign_personality() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    for mark in ["project", "garden"] {
        for i in 0..3 {
            event(
                &mut mem,
                "Alice opened the project.",
                &format!("{mark}-{i}"),
                mark,
                0.8,
            );
        }
    }
    ladder(&mut mem);
    let traits: Vec<_> = mem
        .who_am_i()
        .into_iter()
        .filter(|a| a.layer == AxiomLayer::Trait)
        .collect();
    assert_eq!(traits.len(), 1);
    assert_eq!(traits[0].bearer, Bearer::Other);
    assert_eq!(
        traits[0].schema.as_deref(),
        Some("contextual:collaboration")
    );
    assert!(!traits[0].statement.contains("trust"));
    assert!(!traits[0].statement.contains("withdrawal"));
}
#[test]
fn merge_and_recall_do_not_depend_on_archives() {
    let mut a = SelectiveMemory::new(EntityProfile::new("test"));
    a.profile.merge_similarity = 0.0;
    for (i, text) in [
        "Alice opened the project window.",
        "Alice opened the project window slowly.",
    ]
    .iter()
    .enumerate()
    {
        let id = event(&mut a, text, &format!("e{i}"), "project", 0.2);
        let t = a.store.traces.get_mut(&id).unwrap();
        t.anchor = 0.1;
        t.permanence = 0.5;
        t.status = TraceStatus::Active;
    }
    let mut b = SelectiveMemory::new(a.profile.clone());
    b.store.traces = a.store.traces.clone();
    b.store.edges = a.store.edges.clone();
    for archive in a.store.archives.values_mut() {
        archive.verbatim = "ABSOLUTELY DIFFERENT ARCHIVE".into();
        archive.source = "talk".into();
    }
    let ma = selmem::dream::merge::run(&mut a.store, &a.profile, &selmem::HashEmbedder, false);
    let mb = selmem::dream::merge::run(&mut b.store, &b.profile, &selmem::HashEmbedder, false);
    assert!(ma > 0);
    assert_eq!(ma, mb);
    for (id, t) in &a.store.traces {
        assert_eq!(t.core, b.store.traces[id].core);
        assert_eq!(t.gist, b.store.traces[id].gist);
    }
    let ra = a
        .remember_with(
            "Alice project",
            selmem::RecallWrite::ReadOnly,
            selmem::RecallBias::Observed,
            &[],
        )
        .0;
    let rb = b
        .remember_with(
            "Alice project",
            selmem::RecallWrite::ReadOnly,
            selmem::RecallBias::Observed,
            &[],
        )
        .0;
    assert_eq!(
        ra.iter().map(|r| &r.narrative).collect::<Vec<_>>(),
        rb.iter().map(|r| &r.narrative).collect::<Vec<_>>()
    );
}
#[test]
fn reading_selection_is_stable_under_insertion_order() {
    let mut a = SelectiveMemory::new(EntityProfile::new("test"));
    for i in 0..8 {
        event(
            &mut a,
            &format!("Alice opened project {i}."),
            &format!("e{i}"),
            "project",
            0.8,
        );
    }
    ladder(&mut a);
    let expected = selmem::recall::reading::ReadingProfile::from_book(&a.store, &a.mood).render();
    let mut pairs: Vec<_> = a.store.traces.drain().collect();
    pairs.sort_by(|a, b| b.0.cmp(&a.0));
    a.store.traces.extend(pairs);
    assert_eq!(
        expected,
        selmem::recall::reading::ReadingProfile::from_book(&a.store, &a.mood).render()
    );
}

#[test]
fn named_subjects_do_not_share_a_belief() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    for (i, name) in ["Alice", "Bob", "Charlie"].iter().enumerate() {
        event(
            &mut mem,
            &format!("{name} opened the project."),
            &format!("e{i}"),
            "project",
            0.8,
        );
    }
    ladder(&mut mem);
    assert_eq!(mem.who_am_i().len(), 3);
    assert!(mem.who_am_i().iter().all(|a| a.layer == AxiomLayer::Motif));
}
struct UnjudgedRewrite;
impl selmem::Narrator for UnjudgedRewrite {
    fn reconstruct(&self, t: &selmem::MemoryTrace, _: &selmem::Mood, _: &str) -> String {
        t.gist.clone()
    }
    fn distill_axiom(&self, _: &[&selmem::MemoryTrace]) -> Option<String> {
        None
    }
    fn rewrite(
        &self,
        t: &selmem::MemoryTrace,
        _: &[&selmem::MemoryTrace],
        _: &EntityProfile,
    ) -> Option<String> {
        Some(format!("I recall {}", t.gist))
    }
}
#[test]
fn null_scorer_reports_abstentions_without_writing() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let id = event(&mut mem, "Alice opened the project.", "e", "project", 0.2);
    let t = mem.store.traces.get_mut(&id).unwrap();
    t.attribution = Attribution::None;
    t.permanence = 0.5;
    t.anchor = 0.1;
    t.self_relevance = 0.4;
    let before = t.gist.clone();
    let report = selmem::dream::rewrite::run_report(
        &mut mem.store,
        &mem.profile,
        &UnjudgedRewrite,
        &selmem::HashEmbedder,
        true,
        &selmem::NullScorer,
    );
    assert_eq!(report.proposed, 1);
    assert_eq!(report.unjudged, 1);
    assert_eq!(report.rewritten, 0);
    assert_eq!(mem.store.traces[&id].gist, before);
}
#[test]
fn trace_provenance_survives_file_and_sqlite_roundtrip() {
    for extension in ["selmem", "db"] {
        let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
        let id = event(
            &mut mem,
            "Alice opened the project.",
            "observation-42",
            "project",
            0.8,
        );
        mem.store.traces.get_mut(&id).unwrap().source = "talk".into();
        mem.store.archives.clear();
        let path = std::env::temp_dir().join(format!(
            "selmem-organ-{}-{id}.{extension}",
            std::process::id()
        ));
        mem.path = Some(path.clone());
        mem.save().unwrap();
        let loaded = SelectiveMemory::open(&path, EntityProfile::new("test")).unwrap();
        assert_eq!(loaded.store.traces[&id].source, "talk");
        assert_eq!(
            loaded.store.traces[&id].observation_id.as_deref(),
            Some("observation-42")
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn reported_event_retains_actor_and_incident_before_reassurance() {
    let text = "Caroline said: Hey Mel! How're ya doin'? Recently, I had a not-so-great experience on a hike. I ran into a group of religious conservatives who said something that really upset me. It made me think how much work we still have to do for LGBTQ rights. It's been so helpful to have people around me who accept and support me, so I know I'll be ok!";
    let mut mem = SelectiveMemory::new(EntityProfile::new("Claire"));
    let mut input = EncodeInput::new(text);
    input.permanence = 1.0;
    input.attribution = Attribution::External;
    let id = mem.live_with(input).trace_id.unwrap();
    let t = &mem.store.traces[&id];
    assert_eq!(t.bearer, Bearer::Other);
    assert_eq!(t.semantic.entities[0], "Caroline");
    assert_eq!(t.core, t.gist);
    assert!(t.core.starts_with("Caroline said: Recently"));
    assert!(t.core.contains("really upset me."));
    assert!(t.core.split_whitespace().count() <= 64);
    assert!(!t.core.contains("Hey Mel"));
}
#[test]
fn core_cannot_reverse_actors_or_invent_short_words() {
    assert!(selmem::accept_core("Bob betrayed Alice", "Alice betrayed Bob").is_none());
    assert!(selmem::accept_core("Jo betrayed Bob", "Alice betrayed Bob").is_none());
    assert!(selmem::accept_core("Alice betrayed Bob", "Yesterday Alice betrayed Bob").is_some());
}
#[test]
fn first_person_entity_is_not_lost_because_it_is_one_letter() {
    use selmem::SemanticInterpreter;
    let s = selmem::LexicalInterpreter.interpret_event("I kept my promise.").unwrap();
    assert_eq!(s.core.entities[0], "I");
    assert_eq!(s.bearer, Bearer::Self_);
}
#[test]
fn suppressed_traces_cannot_seed_context() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let id = event(&mut mem, "Alice opened the project.", "a", "project", 0.8);
    mem.store.traces.get_mut(&id).unwrap().suppressed = true;
    assert!(selmem::recall::retrieve::context_cloud_pub(&mem.store, "Alice project").is_empty());
}
#[test]
fn repeated_graph_links_cannot_amplify_context() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let a = event(&mut mem, "Alice opened the project.", "a", "project", 0.8);
    let c = event(&mut mem, "Alice closed project.", "c", "project", 0.8);
    let b = event(&mut mem, "Zebra crossed desert.", "b", "desert", -0.8);
    mem.store.edges.clear();
    mem.store.traces.get_mut(&b).unwrap().schema = Some("journey".into());
    mem.store.edges.insert(a, [b.clone()].into_iter().collect());
    let first = selmem::recall::retrieve::context_cloud_pub(&mem.store, "Alice project");
    mem.store.edges.insert(c, [b].into_iter().collect());
    assert_eq!(first, selmem::recall::retrieve::context_cloud_pub(&mem.store, "Alice project"));
}
#[test]
fn greetings_with_facts_are_not_discarded() {
    let text = "Alice said: Hey I lost my job! My friends helped me.";
    assert_eq!(selmem::encode::core::extractive_core(text), text);
}
#[test]
fn episode_and_schema_bonus_cannot_revive_unanchored_memory() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let id = event(&mut mem, "Zebra crossed desert.", "b", "desert", -0.8);
    let t = mem.store.traces.get_mut(&id).unwrap();
    t.embedding.clear();
    t.cues.clear();
    t.schema = Some("calendar".into());
    let (recalled, dump) = mem.remember_with("What happened that day calendar?", selmem::RecallWrite::ReadOnly, selmem::RecallBias::Observed, &[]);
    assert!(recalled.is_empty());
    let c = dump.candidates.iter().find(|c| c.trace_id == id).unwrap();
    assert_eq!(c.anchor, 0.0);
    assert_eq!(c.score, 0.0);
}
#[test]
fn proposed_core_cannot_drop_reported_speech_attribution() {
    let text = "Alice said: I kept my promise.";
    assert!(selmem::accept_core("I kept my promise.", text).is_none());
    assert_eq!(selmem::accept_core(text, text).as_deref(), Some(text));
}
#[test]
fn latent_scene_cannot_seed_lexical_context() {
    let mut mem = SelectiveMemory::new(EntityProfile::new("test"));
    let id = event(&mut mem, "Alice opened the project.", "a", "project", 0.8);
    mem.store.traces.get_mut(&id).unwrap().status = TraceStatus::Latent;
    assert!(selmem::recall::retrieve::context_cloud_pub(&mem.store, "Alice project").is_empty());
}
