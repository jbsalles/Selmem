//! Physiology. Narrative cases live in tests/cases/*.json (runner: tests/scenes.rs).

mod support;
#[path = "support/echo.rs"]
mod echo;

use selmem::{AxiomLayer, Channel, Embedder, EntityProfile, IdentityAxiom, SelectiveMemory, TraceStatus};
use selmem::EncodeInput;

#[test]
fn persist_roundtrip_keeps_lived_memory_and_archive() {
    let dir = std::env::temp_dir().join(format!("selmem-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("claire.selmem");

    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("Claire")).unwrap();
    let mut ev = EncodeInput::new("You stayed in the rain.");
    ev.valence = 0.7;
    ev.arousal = 0.5;
    ev.self_relevance = 0.9;
    ev.schema = Some("loyalty".into());
    let d = mem.live_with(ev);
    assert!(d.kept);
    mem.sleep();
    mem.save().unwrap();

    let mut loaded = SelectiveMemory::open(&path, EntityProfile::tender("autre")).unwrap();
    assert_eq!(loaded.profile.name, "Claire");
    assert_eq!(loaded.store.traces.len(), 1);
    let tid = d.trace_id.unwrap();
    assert!(loaded.store.traces[&tid].gist.contains("rain"));
    assert_eq!(
        loaded.audit(&tid).unwrap(),
        "You stayed in the rain."
    );
    let recalled = loaded.remember("rain");
    assert!(!recalled.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn http_api_live_remember_sleep() {
    // API /sleep jumps time; keep that jump out of parallel fixtures.
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire")).detach_clock();
    let jump_before = mem.clock.jump;
    let live = selmem::api::dispatch(
        &mut mem,
        "POST",
        "/live",
        "",
        r#"{"event":"You stayed in the rain.","valence":0.7,"arousal":0.5,"self_relevance":0.9,"schema":"loyalty"}"#,
    );
    assert_eq!(live.status, 200);
    assert!(live.body.contains("\"kept\":true"));

    let sleep = selmem::api::dispatch(&mut mem, "POST", "/sleep", "", "{}");
    assert_eq!(sleep.status, 200);
    assert_eq!(mem.clock.jump - jump_before, u64::from(mem.clock.scale) * 3600);

    let rec = selmem::api::dispatch(&mut mem, "POST", "/remember", "", r#"{"query":"the rain"}"#);
    assert_eq!(rec.status, 200);
    assert!(rec.body.contains("rain"));

    let who = selmem::api::dispatch(&mut mem, "GET", "/who", "", "");
    assert_eq!(who.status, 200);
    assert!(who.body.contains("Claire"));

    let turn = selmem::api::dispatch(
        &mut mem,
        "POST",
        "/turn",
        "",
        r#"{"text":"You stayed a little longer."}"#,
    );
    assert_eq!(turn.status, 200);
    assert!(turn.body.contains("\"reply\""));
}

#[test]
fn working_talk_holds_the_thread_and_stays_off_the_book() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new(
        "The project was cancelled for no reason; they stole the credit.",
    );
    ev.valence = -0.8;
    ev.arousal = 0.7;
    ev.disgust = 0.5;
    ev.self_relevance = 0.95;
    ev.schema = Some("injustice".into());
    assert!(mem.live_with(ev).kept);

    let first = mem.speak("We were talking about the cancelled project.");
    assert!(!first.trim().is_empty());
    assert!(
        mem.talk.topic.is_some(),
        "topic should be set after a content turn"
    );
    assert_eq!(mem.talk.turns.len(), 1);

    let before_traces = mem.store.traces.len();
    let _ = mem.speak("Et alors, tu en penses quoi ?");
    assert_eq!(mem.talk.turns.len(), 2);
    assert_eq!(
        mem.store.traces.len(),
        before_traces,
        "working talk must not mint traces"
    );
    let topic = mem.talk.topic.as_deref().unwrap_or("");
    assert!(
        topic.contains("projet")
            || topic.contains("annul")
            || topic.contains("injustice")
            || topic.contains("credit")
            || topic.contains("credit"),
        "follow-up must not wipe the topic, got {topic:?}"
    );

    let isolated = mem.speak_isolated("couleur favorite");
    assert!(!isolated.trim().is_empty());
    assert_eq!(mem.talk.turns.len(), 2, "isolated speak must not record");

    mem.clear_talk();
    assert!(mem.talk.is_empty());

    let listed = selmem::api::dispatch(&mut mem, "GET", "/talk", "", "");
    assert_eq!(listed.status, 200);
}

#[test]
fn sleep_merges_close_episodes_and_can_extinguish() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.12;
    profile.merge_similarity = 0.15;
    // Fusion is tested on fresh episodes, independently of other tests' clocks.
    let mut mem = SelectiveMemory::new(profile).detach_clock();
    for text in [
        "You stayed in the rain by the window.",
        "That rain again: you stayed by the window and did not leave.",
    ] {
        let mut ev = EncodeInput::new(text);
        ev.valence = 0.35;
        ev.arousal = 0.35;
        ev.self_relevance = 0.7;
        ev.schema = Some("loyalty".into());
        assert!(mem.live_with(ev).kept);
    }
    let mut bitter = EncodeInput::new("A gap that left a taste.");
    bitter.valence = -0.5;
    bitter.arousal = 0.4;
    bitter.disgust = 0.4;
    bitter.self_relevance = 0.7;
    bitter.permanence = 0.85;
    bitter.schema = Some("retrait".into());
    let id = mem.live_with(bitter).trace_id.unwrap();
    let before = mem.store.traces[&id].disgust;
    let report = mem.sleep();
    assert!(report.merged >= 1);
    let after = mem.store.traces[&id].disgust;
    assert!(after <= before);
}

#[test]
fn embeddings_rank_paraphrase_above_unrelated() {
    let e = selmem::HashEmbedder;
    let rain = e.embed("you stayed in the rain by the window");
    let para = e.embed("that rain again, you did not leave the window");
    let noise = e.embed("flight 442 leaves at eighteen forty");
    assert!(selmem::cosine(&rain, &para) > selmem::cosine(&rain, &noise));
}

#[test]
fn sqlite_roundtrip() {
    let dir = std::env::temp_dir().join(format!("selmem-sql-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("claire.db");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("Claire")).unwrap();
    let mut ev = EncodeInput::new("You stayed in the rain.");
    ev.valence = 0.7;
    ev.self_relevance = 0.9;
    ev.schema = Some("loyalty".into());
    assert!(mem.live_with(ev).kept);
    mem.store.traces.values_mut().next().unwrap().semantic.claim = "A later belief about loyalty".into();
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    assert_eq!(loaded.profile.name, "Claire");
    assert_eq!(loaded.store.traces.len(), 1);
    let t = loaded.store.traces.values().next().unwrap();
    assert!(!t.embedding.is_empty());
    assert_eq!(t.reality.claim, "You stayed in the rain.");
    assert_ne!(t.reality.claim, t.semantic.claim);
    assert!(t.observation_id.is_some());
    assert!(t.operations.iter().any(|op| op.kind == "encode"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sqlite_roundtrip_keeps_a_claim_that_diverged() {
    let dir = std::env::temp_dir().join(format!("selmem-sql-claim-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("claire.db");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("Claire")).unwrap();
    let mut ev = EncodeInput::new("You stayed in the rain.");
    ev.valence = 0.7;
    ev.self_relevance = 0.9;
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.semantic.claim = "I was left in the rain.".into();
        t.reality.claim = "You stayed in the rain.".into();
        t.valence = 1.7;
    }
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    let t = loaded.store.traces.values().next().unwrap();
    assert_eq!(t.reality.claim, "You stayed in the rain.");
    assert_eq!(t.semantic.claim, "I was left in the rain.");
    assert!(t.valence <= 1.0, "load clamps, got {}", t.valence);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn axiom_is_superseded_when_belief_changes() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_old".into(),
        statement: "I still doubted loyalty.".into(),
        support_trace_ids: vec![],
        valence: 0.0,
        strength: 0.3,
        created_at: 1,
        superseded_by: None,
        schema: Some("loyalty".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::Presence,
        bearer: selmem::core::model::Bearer::Other,
        loss_kind: selmem::core::model::LossKind::Time,
        stake_mark: String::new(),
    });
    for text in ["You stayed in the rain.", "You stayed near me."] {
        let mut ev = EncodeInput::new(text);
        ev.valence = 0.7;
        ev.arousal = 0.4;
        ev.self_relevance = 0.8;
        ev.permanence = 0.85;
        ev.schema = Some("loyalty".into());
        mem.live_with(ev);
    }
    for trace in mem.store.traces.values_mut() {
        trace.stake_mark.clear();
    }
    mem.sleep();
    let living = mem.who_am_i();
    assert!(living.iter().any(|a| a.id != "ax_old"));
    assert_eq!(
        mem.store.axioms["ax_old"].superseded_by.is_some(),
        true
    );
    assert!(mem.lineage("loyalty").len() >= 2);
}

#[test]
fn ebbinghaus_drops_detail_keeps_core() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("You stayed in the rain by the window, without a coat.");
    ev.valence = 0.2;
    ev.arousal = 0.2;
    ev.self_relevance = 0.5;
    ev.schema = Some("soir".into());
    let id = mem.live_with(ev).trace_id.unwrap();
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        // Weather a recent hour; old survivors have a separate hazard floor.
        t.created_at = t.created_at.saturating_sub(10 * 86_400);
        t.anchor = 0.0;
    }
    let core_before = mem.store.traces[&id].core.clone();
    let fid_before = mem.store.traces[&id].fidelity;
    mem.sleep();
    let t = &mem.store.traces[&id];
    assert_eq!(t.core, core_before);
    assert!(t.fidelity < fid_before);
}

#[test]
fn weak_keeps_cool_faster_than_charged_ones() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut dull = EncodeInput::new("The copier jammed again.");
    dull.self_relevance = 0.55;
    dull.arousal = 0.2;
    dull.permanence = 0.0;
    let weak_id = mem.live_with(dull).trace_id.expect("lower τ still keeps a weak hour");
    let mut hot = EncodeInput::new("You stayed in the rain. I will not forget you.");
    hot.valence = 0.7;
    hot.arousal = 0.6;
    hot.self_relevance = 0.95;
    hot.permanence = 0.4;
    let strong_id = mem.live_with(hot).trace_id.expect("charged hour kept");
    {
        let t = mem.store.traces.get_mut(&weak_id).unwrap();
        t.salience_at_encode = 0.22;
        t.anchor = 0.0;
        t.created_at = t.created_at.saturating_sub(20 * 86_400);
    }
    {
        let t = mem.store.traces.get_mut(&strong_id).unwrap();
        t.salience_at_encode = 0.82;
        t.created_at = t.created_at.saturating_sub(20 * 86_400);
    }
    mem.sleep();
    let weak = &mem.store.traces[&weak_id];
    let strong = &mem.store.traces[&strong_id];
    assert!(
        weak.access < strong.access,
        "low salience must cool faster: weak={} strong={}",
        weak.access,
        strong.access
    );
}

#[test]
fn trauma_is_anchored_and_resists_weather() {
    let mut mem = SelectiveMemory::new(EntityProfile::austere("Silas"));
    let mut ev = EncodeInput::new("You laughed at what I had told you in confidence.");
    ev.valence = -0.7;
    ev.arousal = 0.8;
    ev.disgust = 0.7;
    ev.self_relevance = 0.95;
    ev.schema = Some("humiliation".into());
    let id = mem.live_with(ev).trace_id.unwrap();
    assert!(mem.store.traces[&id].anchor >= 0.55);
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.created_at = t.created_at.saturating_sub(40 * 86_400);
    }
    let fid_before = mem.store.traces[&id].fidelity;
    mem.sleep();
    let t = &mem.store.traces[&id];
    assert!(t.fidelity + 0.02 >= fid_before * 0.55);
    assert!(!t.core.is_empty());
}

#[test]
fn identity_colors_a_related_event() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_bias".into(),
        statement: "I pull away from what looks like abandonment.".into(),
        support_trace_ids: vec![],
        valence: -0.7,
        strength: 0.8,
        created_at: 1,
        superseded_by: None,
        schema: Some("abandon".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    let mut ev = EncodeInput::new("You left without warning, once again.");
    ev.valence = -0.2;
    ev.arousal = 0.4;
    ev.self_relevance = 0.5;
    ev.schema = Some("abandon".into());
    let before = ev.valence;
    let d = mem.live_with(ev);
    assert!(d.kept);
    let t = mem.store.traces.values().next().unwrap();
    assert!(t.valence <= before);
}

#[test]
fn recall_can_reinterpret_meaning() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("You stayed in the rain.");
    ev.valence = 0.4;
    ev.arousal = 0.5;
    ev.self_relevance = 0.8;
    ev.schema = Some("loyalty".into());
    mem.live_with(ev);
    mem.mood.valence = -0.8;
    let _ = mem.remember("this rain");
    let t = mem.store.traces.values().next().unwrap();
    assert!(
        t.drifts.iter().any(|d| d.kind == selmem::DriftKind::Reinterpret) || t.valence < 0.4,
        "valence {} drifts {:?}",
        t.valence,
        t.drifts
    );
}

#[test]
fn detached_recall_corrects_after_several_misses() {
    let mut profile = EntityProfile::tender("Claire");
    profile.ground_strikes = 3;
    profile.narrator_firmness = 1.0;
    let mut mem = SelectiveMemory::new(profile).with_scorer(Box::new(
        support::FixedScorer(selmem::PropositionLabel::Contradict),
    ));
    let mut ev = EncodeInput::new("You stayed. Rain on the window.");
    ev.valence = 0.7;
    ev.arousal = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.schema = Some("loyalty".into());
    let id = mem.live_with(ev).trace_id.unwrap();
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "Flight 442 vanished in the fog without leaving an address.".into();
        t.cues.push("rain".into());
    }
    let first = mem.remember("the rain");
    assert!(!first.is_empty());
    assert!(
        first[0].narrative.contains("442") || first[0].narrative.contains("brouillard"),
        "first miss should still speak the drifted gist, got {}",
        first[0].narrative
    );
    assert!(!first[0].disclaimer.contains("journal"));
    assert_eq!(mem.store.traces[&id].detach_strikes, 1);
    let _ = mem.remember("the rain");
    assert_eq!(mem.store.traces[&id].detach_strikes, 2);
    let third = mem.remember("the rain");
    assert!(
        third[0].narrative.contains("rain")
            || third[0].narrative.contains("stayed")
            || third[0].narrative.contains("window"),
        "after threshold, gist must return toward core, got {}",
        third[0].narrative
    );
    assert!(!third[0].narrative.contains("442"));
    let t = &mem.store.traces[&id];
    assert!(t.drifts.iter().any(|d| d.kind == selmem::DriftKind::Ground));
    assert_eq!(t.detach_strikes, 0);
}

#[test]
fn fading_trace_may_distort_without_grounding() {
    let mut profile = EntityProfile::tender("Claire");
    profile.narrator_firmness = 1.0;
    profile.ground_strikes = 1;
    let mut mem = SelectiveMemory::new(profile);
    let mut ev = EncodeInput::new("Some ordinary rain.");
    ev.valence = 0.2;
    ev.self_relevance = 0.9;
    ev.permanence = 0.85;
    let id = mem.live_with(ev).trace_id.expect("kept so we can fade it after");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = selmem::TraceStatus::Cold;
        t.fidelity = 0.25;
        t.access = 0.1;
        t.permanence = 0.1;
        t.anchor = 0.05;
        t.gist = "Le vol 442 a disparu dans le brouillard.".into();
        t.cues.push("rain".into());
    }
    for _ in 0..5 {
        let _ = mem.remember("the rain");
    }
    let t = &mem.store.traces[&id];
    assert!(
        t.gist.contains("442"),
        "fading gist should keep warping, got {}",
        t.gist
    );
    assert!(!t.drifts.iter().any(|d| d.kind == selmem::DriftKind::Ground));
}

fn plant_important_drift(mem: &mut SelectiveMemory) -> String {
    let mut ev = EncodeInput::new("You stayed. Rain on the window.");
    ev.valence = 0.7;
    ev.arousal = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.schema = Some("loyalty".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "Flight 442 vanished in the fog without leaving an address.".into();
        t.cues.push("rain".into());
    }
    id
}

#[test]
fn zero_firmness_never_grounds_an_important_trace() {
    let mut loose = EntityProfile::tender("Claire");
    loose.narrator_firmness = 0.0;
    loose.ground_strikes = 1;
    let mut mem = SelectiveMemory::new(loose);
    let id = plant_important_drift(&mut mem);
    for _ in 0..8 {
        let _ = mem.remember("the rain");
    }
    let t = &mem.store.traces[&id];
    assert!(
        t.gist.contains("442"),
        "firmness 0 must leave the warped gist, got {}",
        t.gist
    );
    assert!(!t.drifts.iter().any(|d| d.kind == selmem::DriftKind::Ground));
}

#[test]
fn stronger_memory_grip_grounds_sooner_than_a_weak_one() {
    let mut hard_p = EntityProfile::austere("Silas");
    hard_p.narrator_firmness = 1.0;
    hard_p.ground_strikes = 2;
    let mut soft_p = EntityProfile::tender("Claire");
    soft_p.narrator_firmness = 0.25;
    soft_p.ground_strikes = 2;

    let mut hard = SelectiveMemory::new(hard_p).with_scorer(Box::new(
        support::FixedScorer(selmem::PropositionLabel::Contradict),
    ));
    let mut soft = SelectiveMemory::new(soft_p).with_scorer(Box::new(
        support::FixedScorer(selmem::PropositionLabel::Contradict),
    ));
    let hid = plant_important_drift(&mut hard);
    let sid = plant_important_drift(&mut soft);
    // The grounding policy uses memory grip, independently of narrator firmness.
    soft.store.traces.get_mut(&sid).unwrap().anchor = 0.20;
    soft.store.traces.get_mut(&sid).unwrap().fidelity = 0.40;

    for _ in 0..2 {
        let _ = hard.remember("the rain");
        let _ = soft.remember("the rain");
    }
    let hg = hard.store.traces[&hid]
        .drifts
        .iter()
        .any(|d| d.kind == selmem::DriftKind::Ground);
    let sg = soft.store.traces[&sid]
        .drifts
        .iter()
        .any(|d| d.kind == selmem::DriftKind::Ground);
    assert!(hg, "firm narrator should have pulled toward the core");
    assert!(!sg, "softer narrator should still be allowed the warped gist");
    assert!(
        soft.store.traces[&sid].gist.contains("442"),
        "soft side must still hold the warped gist, not an empty book"
    );
}

#[test]
fn grounding_never_exposes_the_archive() {
    let mut profile = EntityProfile::austere("Silas");
    profile.narrator_firmness = 1.0;
    profile.ground_strikes = 1;
    let mut mem = SelectiveMemory::new(profile).with_scorer(Box::new(
        support::FixedScorer(selmem::PropositionLabel::Contradict),
    ));
    let mut ev = EncodeInput::new("You stayed. Rain on the window.");
    ev.valence = 0.7;
    ev.arousal = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.schema = Some("loyalty".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    let secret = "VERBATIM-SEALED-991";
    {
        let aid = mem.store.traces[&id].archive_id.clone().unwrap();
        mem.store.archives.get_mut(&aid).unwrap().verbatim =
            format!("You stayed. The rain. {secret}");
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "Flight 442 vanished in the fog without leaving an address.".into();
        t.cues.push("rain".into());
    }
    assert!(
        mem.audit(&id).unwrap().contains(secret),
        "the seal must hold the planted journal before we ask whether recall leaks it"
    );
    let mut saw_ground = false;
    let mut saw_recall = false;
    for _ in 0..6 {
        let rec = mem.remember("the rain");
        assert!(
            !rec.is_empty(),
            "recall must return the living trace — an empty list would make the leak check vacuous"
        );
        saw_recall = true;
        for r in &rec {
            assert!(
                !r.narrative.contains(secret),
                "narrative leaked the journal: {}",
                r.narrative
            );
            assert!(!r.disclaimer.contains(secret));
        }
        if mem.store.traces[&id]
            .drifts
            .iter()
            .any(|d| d.kind == selmem::DriftKind::Ground)
        {
            saw_ground = true;
            break;
        }
    }
    assert!(saw_recall);
    assert!(saw_ground, "firm living trace should ground");
    let t = &mem.store.traces[&id];
    assert!(!t.gist.contains(secret), "gist must not become the journal");
    assert!(
        mem.audit(&id).unwrap().contains(secret),
        "the journal must still be there after grounding — absence from speech is not absence from the store"
    );
}

#[test]
fn latent_forgets_the_scene_keeps_the_reaction() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("You laughed at what I had told you in the rain.");
    ev.valence = -0.7;
    ev.arousal = 0.6;
    ev.disgust = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.schema = Some("humiliation".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = selmem::TraceStatus::Latent;
        t.fidelity = 0.2;
        t.access = 0.1;
    }
    assert!(
        mem.audit(&id)
            .unwrap()
            .contains("laughed")
            || mem.store.traces[&id].gist.contains("laughed"),
        "the scene must still exist on the trace/seal or the 'forgotten scene' check is empty"
    );
    let rec = mem.remember("cette humiliation in the rain");
    assert!(
        rec.is_empty() || rec.iter().all(|r| r.trace_id != id),
        "a latent hour has no scene to tell; remember must not return that trace"
    );
    for r in &rec {
        assert!(
            !r.narrative.contains("laughed") && !r.narrative.contains("told"),
            "latent recall must not replay the scene: {}",
            r.narrative
        );
    }
    let mut next = EncodeInput::new("Encore une humiliation in the rain.");
    next.valence = 0.0;
    next.disgust = 0.0;
    next.self_relevance = 0.55;
    next.permanence = 0.5;
    next.arousal = 0.35;
    let painted_id = mem
        .live_with(next)
        .trace_id
        .expect("follow-up must clear the gate — otherwise the paint check is skipped");
    let t = &mem.store.traces[&painted_id];
    assert!(
        t.valence < 0.0 || t.disgust > 0.05 || t.schema.as_deref() == Some("humiliation"),
        "latent charge should color the new event v={} d={} schema={:?}",
        t.valence,
        t.disgust,
        t.schema
    );
}

#[test]
fn world_fact_does_not_cool_or_mythologize() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("Le rendez-vous est mardi 10h, salle B.");
    ev.channel = Channel::World;
    ev.utility = 0.95;
    ev.permanence = 0.2;
    ev.self_relevance = 0.1;
    let id = mem.live_with(ev).trace_id.expect("world is kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.created_at = t.created_at.saturating_sub(86_400 * 400);
        t.last_recalled_at = None;
        t.access = 0.02;
        t.fidelity = 0.2;
    }
    for _ in 0..4 {
        mem.sleep();
    }
    let t = &mem.store.traces[&id];
    assert_eq!(t.status, TraceStatus::Active);
    assert!(t.access >= 0.85, "world access pinned, got {}", t.access);
    let rec = mem.remember("rendez-vous mardi");
    assert!(
        rec.iter().any(|r| r.narrative.contains("mardi") || r.narrative.contains("salle")),
        "world fact must remain recallable: {:?}",
        rec.iter().map(|r| &r.narrative).collect::<Vec<_>>()
    );
}

#[test]
fn merge_keeps_the_stronger_core() {
    let mut profile = EntityProfile::tender("Claire");
    profile.merge_similarity = 0.1;
    let mut mem = SelectiveMemory::new(profile);
    let mut weak = EncodeInput::new("Ordinary rain on the pane.");
    weak.schema = Some("loyalty".into());
    weak.valence = 0.2;
    weak.self_relevance = 0.4;
    weak.permanence = 0.1;
    let weak_id = mem.live_with(weak).trace_id.expect("kept");
    let mut strong = EncodeInput::new("You stayed in the rain. I will not forget you.");
    strong.schema = Some("loyalty".into());
    strong.valence = 0.8;
    strong.arousal = 0.7;
    strong.self_relevance = 0.95;
    strong.permanence = 0.9;
    let strong_id = mem.live_with(strong).trace_id.expect("kept");
    {
        let w = mem.store.traces.get_mut(&weak_id).unwrap();
        w.anchor = 0.05;
        w.gist = "Ordinary rain.".into();
        w.core = "ordinary rain".into();
    }
    {
        let s = mem.store.traces.get_mut(&strong_id).unwrap();
        s.anchor = 0.9;
        s.gist = "You stayed in the rain.".into();
        s.core = "stayed in the rain".into();
    }
    mem.sleep();
    let strong = &mem.store.traces[&strong_id];
    let weak = &mem.store.traces[&weak_id];
    assert!(
        strong.core.contains("stayed") || strong.core.contains("rain"),
        "anchored episode core must survive the night, got {}",
        strong.core
    );
    assert!(strong.anchor >= 0.9);
    assert_ne!(
        strong.status,
        TraceStatus::Myth,
        "weak anecdote must not absorb the anchored episode"
    );
    let _ = weak;
}

#[test]
fn latent_traces_do_not_mint_a_belief() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    for event in [
        "First humiliation in the rain.",
        "Second humiliation in the rain.",
        "Third humiliation in the rain.",
    ] {
        let mut ev = EncodeInput::new(event);
        ev.schema = Some("humiliation".into());
        ev.valence = -0.6;
        ev.self_relevance = 0.9;
        ev.permanence = 0.9;
        let id = mem.live_with(ev).trace_id.expect("kept");
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = TraceStatus::Latent;
        t.fidelity = 0.2;
        t.access = 0.1;
    }
    mem.sleep();
    assert_eq!(
        mem.store
            .traces
            .values()
            .filter(|t| t.schema.as_deref() == Some("humiliation"))
            .count(),
        3,
        "the three scenes must still be in the book — no-belief is meaningless on an empty store"
    );
    assert!(
        !mem.store
            .living_axioms()
            .iter()
            .any(|a| a.schema.as_deref() == Some("humiliation")),
        "forgotten scenes must not mint a motif or belief: {:?}",
        mem.store
            .living_axioms()
            .iter()
            .map(|a| (&a.layer, &a.schema, &a.statement))
            .collect::<Vec<_>>()
    );
}

#[test]
fn latent_can_return_as_a_cold_core_after_rehearsal() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("You laughed at the red umbrella I was holding in the rain.");
    ev.schema = Some("humiliation".into());
    ev.valence = -0.7;
    ev.self_relevance = 0.9;
    ev.permanence = 0.5;
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = TraceStatus::Latent;
        t.fidelity = 0.2;
        t.access = 0.1;
        t.rehearsals = 0;
        t.core = "humiliation in the rain".into();
        t.gist = "You laughed at the red umbrella.".into();
    }
    for _ in 0..2 {
        let mut next = EncodeInput::new("Encore une humiliation in the rain.");
        next.schema = Some("humiliation".into());
        next.valence = -0.2;
        next.self_relevance = 0.5;
        mem.live_with(next);
    }
    assert!(mem.store.traces[&id].rehearsals >= 2);
    mem.sleep();
    let t = &mem.store.traces[&id];
    assert_eq!(t.status, TraceStatus::Cold);
    assert!(
        mem.audit(&id).unwrap().contains("umbrella"),
        "the sealed scene must still name the umbrella"
    );
    assert!(
        !t.gist.contains("umbrella"),
        "original scene detail must not return in the gist: {}",
        t.gist
    );
    assert!(
        t.gist.contains("humiliation") || t.gist.contains("rain"),
        "revived blur should be the core, got {}",
        t.gist
    );
}

#[test]
fn reconsolidation_does_not_engrave_an_unrelated_sentence() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("You stayed in the rain.");
    ev.schema = Some("loyalty".into());
    ev.valence = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.8;
    let id = mem.live_with(ev).trace_id.unwrap();
    let before = mem.store.traces[&id].gist.clone();
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        selmem::dream::apply_reconsolidation(
            t,
            "Marc never resigned, it was an invented misunderstanding.",
            &mem.profile,
            -0.9,
        );
    }
    let t = &mem.store.traces[&id];
    assert_eq!(t.gist, before, "poisoned reconstruct must not become the book");
    assert!(!t.core.contains("Marc"));
    assert!(
        mem.audit(&id).unwrap().contains("rain"),
        "the seal must still hold the original hour"
    );
}

#[test]
fn merged_episodes_still_count_as_belief_evidence() {
    let mut profile = EntityProfile::tender("Claire");
    profile.merge_similarity = 0.05;
    let mut mem = SelectiveMemory::new(profile);
    for event in [
        "First humiliation in the cold rain.",
        "Second humiliation in the cold rain.",
        "Third humiliation in the cold rain.",
    ] {
        let mut ev = EncodeInput::new(event);
        ev.schema = Some("humiliation".into());
        ev.valence = -0.6;
        ev.arousal = 0.6;
        ev.self_relevance = 0.9;
        ev.permanence = 0.8;
        assert!(mem.live_with(ev).kept);
    }
    mem.sleep();
    let myths = mem
        .store
        .traces
        .values()
        .filter(|t| t.status == TraceStatus::Myth && t.schema.as_deref() == Some("humiliation"))
        .count();
    let living = mem
        .store
        .living_axioms()
        .into_iter()
        .filter(|a| a.schema.as_deref() == Some("humiliation"))
        .collect::<Vec<_>>();
    assert!(
        !living.is_empty(),
        "merge must not wipe the identity ladder, myths={myths}"
    );
    assert!(
        living
            .iter()
            .any(|a| a.layer == AxiomLayer::Belief || a.support_trace_ids.len() >= 2),
        "three aligned hours must support a motif/belief whether or not a myth was minted: {:?}",
        living
            .iter()
            .map(|a| (&a.layer, a.support_trace_ids.len()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn spent_latent_hour_can_leave_the_book() {
    let mut profile = EntityProfile::tender("Claire");
    profile.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(profile);
    let mut ev = EncodeInput::new("Une remarque banale entendue dans le couloir.");
    ev.valence = 0.04;
    ev.arousal = 0.12;
    ev.self_relevance = 0.20;
    ev.permanence = 0.08;
    ev.schema = Some("quotidien".into());
    let id = mem.live_with(ev).trace_id.expect("kept under a low gate");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = TraceStatus::Latent;
        t.fidelity = 0.12;
        t.access = 0.04;
        t.anchor = 0.0;
        t.permanence = 0.08;
        t.salience_at_encode = 0.20;
        t.rehearsals = 0;
        t.created_at = t.created_at.saturating_sub(2 * 86_400);
        t.last_recalled_at = None;
        t.channel = Channel::Selfhood;
    }
    // Test release eligibility without weather changing the seeded latent state.
    let latent = std::iter::once(id.clone()).collect();
    let released = selmem::dream::release::run(&mut mem.store, &latent);
    assert!(
        mem.store.traces.get(&id).is_none(),
        "a spent latent with no axiom must leave the book"
    );
    assert!(released >= 1);
}

#[test]
fn one_night_does_not_release_a_fresh_hour() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("Tu es resté sous la pluie près de la fenêtre.");
    ev.valence = 0.6;
    ev.arousal = 0.5;
    ev.self_relevance = 0.9;
    ev.permanence = 0.85;
    ev.schema = Some("fidélité".into());
    let id = mem.live_with(ev).trace_id.unwrap();
    let report = mem.sleep();
    assert_eq!(report.released, 0);
    assert!(mem.store.traces.contains_key(&id));
}

#[test]
fn pin_keeps_an_hour_that_would_weather() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("JB is the conceptor.");
    ev.valence = 0.1;
    ev.arousal = 0.3;
    ev.self_relevance = 0.4;
    ev.permanence = 0.1;
    let id = mem.live_with(ev).trace_id.expect("kept");
    assert!(mem.pin(&id));
    assert!(mem.store.traces[&id].permanence >= 0.92);
    assert!(mem.store.traces[&id].anchor >= 0.85);
}

#[test]
fn short_hour_stays_one_fact() {
    let text = "You stayed in the rain.\nThe window was open.\nNobody spoke.";
    assert_eq!(selmem::segment_facts(text).len(), 1);
}

#[test]
fn long_paste_is_sliced_then_each_slice_is_compressed() {
    let mut lines = Vec::new();
    for i in 1..=24 {
        lines.push(format!(
            "Fact {i}: the team closed ticket {i} after the review on floor two."
        ));
    }
    let blob = lines.join("\n");
    let parts = selmem::segment_facts(&blob);
    assert!(parts.len() >= 3, "got {} parts", parts.len());
    assert!(parts.iter().all(|p| p.lines().count() <= 10));
    assert!(parts.iter().all(|p| p.lines().count() >= 4 || parts.len() == 1));

    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new(&blob);
    ev.valence = 0.4;
    ev.arousal = 0.5;
    ev.self_relevance = 0.8;
    ev.permanence = 0.5;
    ev.schema = Some("travail".into());
    let d = mem.live_with(ev);
    assert!(d.kept);
    assert!(d.parts >= 3);
    assert_eq!(d.kept_n, d.parts);
    assert_eq!(mem.store.traces.len(), d.kept_n);

    let gists: Vec<String> = mem.store.traces.values().map(|t| t.gist.clone()).collect();
    assert!(
        gists.iter().any(|g| g.contains("ticket 1")),
        "first slice must keep its own head, not only the document head: {gists:?}"
    );
    assert!(
        gists.iter().any(|g| g.contains("ticket 15") || g.contains("Fact 15")),
        "a later slice must be compressed on its own lines: {gists:?}"
    );
}

#[test]
fn semantic_split_keeps_verbatim_excerpts() {
    let src = "The board cancelled the project on Monday.\nThe team kept the prototype in the drawer.\nNobody wrote the lesson down.";
    let proposed = vec![
        "The board cancelled the project on Monday.".into(),
        "The team kept the prototype in the drawer.".into(),
        "Nobody wrote the lesson down.".into(),
    ];
    let parts = selmem::lossless_parts(src, &proposed).expect("excerpts must pass");
    assert_eq!(parts.len(), 3);
    assert!(src.contains(&parts[0]));
    assert!(src.contains(&parts[1]));
}

#[test]
fn paraphrased_split_falls_back_to_word_pack() {
    let src = "The board cancelled the project on Monday.\nThe team kept the prototype in the drawer.\nNobody wrote the lesson down.\nA fourth line about the budget review.\nA fifth line about the corridor.\nA sixth line about the kettle.\nA seventh line about the stand-up.\nAn eighth line about the mail.\nA ninth line about the badge.\nA tenth line about the rain.\nAn eleventh line about the copier.";
    let proposed = vec![
        "Leadership axed the initiative.".into(),
        "Staff hid a mock-up.".into(),
    ];
    assert!(selmem::lossless_parts(src, &proposed).is_none());
    let parts = selmem::split_event(src, Some(&proposed));
    assert_eq!(parts, selmem::segment_facts(src));
}

#[test]
fn segment_reply_must_be_a_json_array() {
    let raw = "Here you go:\n[\"Alpha fact one.\", \"Beta fact two.\"]\n";
    let got = selmem::parse_segment_reply(raw).unwrap();
    assert_eq!(got, ["Alpha fact one.", "Beta fact two."]);
    assert!(selmem::parse_segment_reply("the project was cancelled").is_none());
}

#[test]
fn log_channel_survives_sleep_verbatim() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let line = "Step 18: SELECT film_id FROM PAGILA.FILM f JOIN FILM_CATEGORY fc";
    let id = mem.live_log(line).trace_id.expect("log is always kept");
    assert_eq!(mem.store.traces[&id].channel, Channel::Log);
    assert!(
        mem.store.traces[&id].gist.contains("film_id"),
        "log gist must keep the tool line, got {}",
        mem.store.traces[&id].gist
    );
    mem.sleep();
    let t = &mem.store.traces[&id];
    assert_eq!(t.status, TraceStatus::Active);
    assert!(t.gist.contains("film_id"));
    let rec = mem.remember("film_id step 18");
    assert!(
        rec.iter().any(|r| r.narrative.contains("film_id")),
        "log must retrieve the tool line: {:?}",
        rec.iter().map(|r| &r.narrative).collect::<Vec<_>>()
    );
}

#[test]
fn live_speak_pins_the_core_when_gist_drifted() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire")).with_narrator(Box::new(echo::EchoNarrator));
    let mut ev = EncodeInput::new("The project was cancelled in front of the team.");
    ev.self_relevance = 0.9;
    ev.permanence = 0.9;
    ev.arousal = 0.6;
    ev.valence = -0.5;
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "A quiet wound. Credit gone. Distance.".into();
        t.core = "project cancelled in front of the team".into();
        t.access = 1.0;
        t.cues.push("work".into());
        t.cues.push("project".into());
        t.cues.push("cancelled".into());
    }
    let reply = mem.speak("What happened at work?");
    assert!(
        reply.contains("cancelled") || reply.contains("project") || reply.contains("what happened"),
        "live speak must still carry the fact, got {reply}"
    );
}

#[test]
fn isolated_probe_puts_scene_before_axiom() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire")).with_narrator(Box::new(echo::EchoNarrator));
    let mut ev = EncodeInput::new(
        "You walk into a meeting and learn the project was killed and given to someone else.",
    );
    ev.valence = -0.7;
    ev.arousal = 0.8;
    ev.self_relevance = 0.9;
    ev.permanence = 0.8;
    ev.schema = Some("wound".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_wound".into(),
        statement: "I pull away from what looks like: wound.".into(),
        support_trace_ids: vec![id],
        valence: -0.7,
        strength: 0.44,
        created_at: 1,
        superseded_by: None,
        schema: Some("wound".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    let reply = mem.speak_isolated(
        "A colleague goes quiet after a meeting that concerned your work. What stays with you?",
    );
    let hour = reply.find("killed").or_else(|| reply.find("project"));
    let axiom = reply.find("I pull away from what looks like: wound");
    assert!(hour.is_some(), "scene must be in the mouth, got {reply}");
    if let (Some(h), Some(a)) = (hour, axiom) {
        assert!(h < a, "scene before axiom, got {reply}");
    }
}

#[test]
fn release_keeps_sealed_archive_for_audit() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new("On 19 January lunch was billed to cost centre 4412.");
    ev.valence = 0.05;
    ev.arousal = 0.10;
    ev.self_relevance = 0.20;
    ev.permanence = 0.10;
    ev.schema = Some("admin".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    assert_eq!(
        mem.audit(&id).unwrap(),
        "On 19 January lunch was billed to cost centre 4412."
    );
    let aid = mem.store.traces[&id].archive_id.clone().expect("archive");
    assert!(mem.store.release_trace(&id).is_some());
    assert!(!mem.store.traces.contains_key(&id));
    assert!(mem.store.archives.contains_key(&aid));
    let tomb = &mem.store.archives[&aid];
    assert_eq!(tomb.released_from.as_deref(), Some(id.as_str()));
    assert!(tomb.released_at.is_some());
    assert_eq!(
        mem.audit(&id).unwrap(),
        "On 19 January lunch was billed to cost centre 4412."
    );
    selmem::persist::prune_orphaned_archives(&mut mem.store);
    assert!(mem.store.archives.contains_key(&aid), "tomb must survive prune");
    assert!(mem.remember("cost centre 4412").is_empty());
}

