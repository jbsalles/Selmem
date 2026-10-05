//! Behavioral audit of the integrity corrections.
//! Does not read ORGAN-INTEGRITY.md. A pass here is a runtime fact.

use selmem::{
    Attribution, Channel, EncodeInput, EntityProfile, SelectiveMemory, TraceStatus,
};

fn keep(mem: &mut SelectiveMemory, event: &str) -> String {
    let mut ev = EncodeInput::new(event);
    ev.valence = 0.4;
    ev.arousal = 0.4;
    ev.self_relevance = 0.7;
    ev.permanence = 0.5;
    ev.schema = Some("audit".into());
    mem.live_with(ev).trace_id.expect("kept")
}

#[test]
fn sqlite_keeps_diverged_claim_ops_and_observation() {
    let dir = std::env::temp_dir().join(format!("selmem-audit-sql-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("entity.db");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    let id = keep(&mut mem, "You stayed in the rain by the window.");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.semantic.claim = "I was left.".into();
        t.reality.claim = "You stayed in the rain by the window.".into();
        t.valence = 1.7;
    }
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    let t = loaded.store.traces.values().next().unwrap();
    assert_eq!(t.reality.claim, "You stayed in the rain by the window.");
    assert_eq!(t.semantic.claim, "I was left.");
    assert_ne!(t.reality.claim, t.semantic.claim);
    assert!(t.observation_id.is_some());
    assert!(t.operations.iter().any(|op| op.kind == "encode" && op.origin == selmem::EvidenceOrigin::Event));
    assert!(t.valence <= 1.0, "out of range valence survived sqlite: {}", t.valence);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn post_p1_file_without_attr_does_not_load() {
    let dir = std::env::temp_dir().join(format!("selmem-audit-attr-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    let _ = keep(&mut mem, "The Lyon file is cancelled in front of the team.");
    mem.save().unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(raw.contains("attr "), "writer must emit attr, not rely on a default");
    let stripped = raw.replace("attr none\n", "").replace("attr external\n", "").replace("attr internal\n", "");
    assert!(!stripped.contains("attr "), "strip failed");
    let bad = dir.join("no-attr.selmem");
    std::fs::write(&bad, stripped).unwrap();
    let loaded = SelectiveMemory::open(bad.to_str().unwrap(), EntityProfile::tender("x"));
    assert!(loaded.is_err(), "missing attr on a current file must not become None");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn merge_does_not_import_a_word_absent_from_the_hour() {
    let mut profile = EntityProfile::tender("A");
    profile.merge_similarity = 0.1;
    profile.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(profile);
    let mut a = EncodeInput::new("You stayed in the rain by the window.");
    a.schema = Some("loyalty".into());
    a.valence = 0.6;
    a.self_relevance = 0.8;
    a.permanence = 0.9;
    let keep = mem.live_with(a).trace_id.expect("keep");
    let mut b = EncodeInput::new("You stayed in the rain near the door.");
    b.schema = Some("loyalty".into());
    b.valence = 0.5;
    b.self_relevance = 0.6;
    b.permanence = 0.4;
    let other = mem.live_with(b).trace_id.expect("other");
    {
        let t = mem.store.traces.get_mut(&other).unwrap();
        t.core = format!("{} unicornword", t.core);
        t.anchor = 0.1;
    }
    {
        let t = mem.store.traces.get_mut(&keep).unwrap();
        t.anchor = 0.7;
    }
    selmem::dream::merge::run(&mut mem.store, &mem.profile, &selmem::HashEmbedder, false);
    let core = mem.store.traces[&keep].core.clone();
    assert!(!core.to_lowercase().contains("unicornword"), "foreign core word entered keeper: {core}");
}

#[test]
fn long_word_overlap_does_not_rehearse_or_revive() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    let mut ev = EncodeInput::new("The committee postponed the hearing.");
    ev.schema = Some("hearing".into());
    ev.valence = -0.4;
    ev.self_relevance = 0.8;
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.status = TraceStatus::Latent;
        t.rehearsals = 1;
        t.core = "committee postponed hearing".into();
        t.gist = "committee postponed hearing".into();
    }
    let mut next = EncodeInput::new("A different committee met about the budget.");
    next.valence = 0.1;
    next.self_relevance = 0.4;
    mem.live_with(next);
    let t = &mem.store.traces[&id];
    assert_eq!(t.rehearsals, 1, "long word must not rehearse");
    assert_eq!(t.status, TraceStatus::Latent, "long word must not revive");
}

#[test]
fn rewrite_records_an_operation() {
    let mut profile = EntityProfile::tender("A");
    profile.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(profile);
    let mut ev = EncodeInput::new("The project was cancelled in front of the team.");
    ev.valence = -0.7;
    ev.arousal = 0.8;
    ev.self_relevance = 0.9;
    ev.attribution = Attribution::Internal;
    ev.schema = Some("lyon-file".into());
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = "short".into();
        t.detach_strikes = 24;
        t.anchor = 0.2;
    }
    let n = selmem::dream::rewrite::run(
        &mut mem.store,
        &mem.profile,
        &selmem::RuleNarrator,
        &selmem::HashEmbedder,
        true,
        &selmem::NullScorer,
    );
    assert!(n >= 1, "rewrite did not run");
    assert!(
        mem.store.traces[&id]
            .operations
            .iter()
            .any(|op| op.kind == "rewrite"),
        "free rewrite left no operation"
    );
}

#[test]
fn merges_refused_survives_sqlite_reload() {
    let dir = std::env::temp_dir().join(format!("selmem-audit-refused-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("entity.db");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    mem.store.merges_refused = 4;
    let _ = keep(&mut mem, "You stayed in the rain.");
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    assert_eq!(loaded.store.merges_refused, 4);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn outside_score_stores_the_label_and_does_not_write() {
    struct Fixed;
    impl selmem::PropositionScorer for Fixed {
        fn score(&self, _claim: &str, _sentence: &str) -> selmem::PropositionLabel {
            selmem::PropositionLabel::Contradict
        }
        fn name(&self) -> &str {
            "fixed"
        }
    }
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A")).with_scorer(Box::new(Fixed));
    let id = keep(&mut mem, "Paul refused the offer.");
    let claim = mem.store.traces[&id].reality.claim.clone();
    let core = mem.store.traces[&id].core.clone();
    let gist = mem.store.traces[&id].gist.clone();
    let ops = mem.store.traces[&id].operations.len();
    let label = mem.score_against_claim(&id, "Paul accepted the offer.");
    assert_eq!(label, selmem::PropositionLabel::Contradict);
    let t = &mem.store.traces[&id];
    assert_eq!(t.reality.claim, claim);
    assert_eq!(t.core, core);
    assert_eq!(t.gist, gist);
    assert_eq!(t.operations.len(), ops);
    assert!(mem.store.measures.iter().any(|(tid, lab)| tid == &id && lab == "contradict"));
    assert_eq!(
        selmem::accept_core("Paul betray Jean.", "Paul did not betray Jean, but Jean betrayed Paul."),
        None
    );
    assert_eq!(
        selmem::PropositionLabel::parse("Paul refused the offer"),
        selmem::PropositionLabel::Unknown
    );
}

#[test]
fn sealed_token_loads_active_and_leaves_a_drift() {
    let dir = std::env::temp_dir().join(format!("selmem-audit-sealed-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    let _ = keep(&mut mem, "You stayed in the rain.");
    mem.save().unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    let patched = raw.replacen(" active ", " sealed ", 1);
    assert_ne!(raw, patched);
    let bad = dir.join("sealed.selmem");
    std::fs::write(&bad, patched).unwrap();
    let loaded = SelectiveMemory::open(bad.to_str().unwrap(), EntityProfile::tender("x")).unwrap();
    let t = loaded.store.traces.values().next().unwrap();
    assert_ne!(t.status, TraceStatus::Myth);
    assert!(t.channel == Channel::Selfhood || t.channel == Channel::World);
    assert!(
        t.drifts.iter().any(|d| d.note.contains("sealed token loaded as active")),
        "sealed load left no trace: {:?}",
        t.drifts.iter().map(|d| &d.note).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
