//! P1 storage + P2 skip_rewrite policy.

mod support;
#[path = "support/rewrite.rs"]
mod rewrite;

use selmem::{
    Attribution, AxiomLayer, EncodeInput, EntityProfile, IdentityAxiom, SelectiveMemory,
};

fn pin(event: &str, attr: Attribution) -> EncodeInput<'_> {
    let mut ev = EncodeInput::new(event);
    ev.valence = 0.2;
    ev.arousal = 0.4;
    ev.self_relevance = 0.7;
    ev.permanence = 0.85;
    ev.schema = Some("lyon-file".into());
    ev.attribution = attr;
    ev
}

#[test]
fn encode_defaults_to_none() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    let ev = EncodeInput::new("Someone reprints the agenda with the same items.");
    assert_eq!(ev.attribution, Attribution::None);
    assert!(mem.live_with(ev).kept);
    let t = mem.store.traces.values().next().unwrap();
    assert_eq!(t.attribution, Attribution::None);
}

#[test]
fn file_roundtrip_keeps_internal() {
    let dir = std::env::temp_dir().join(format!("selmem-attr-file-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    assert!(mem.live_with(pin("The Lyon file is cancelled in front of the team.", Attribution::Internal)).kept);
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    let t = loaded.store.traces.values().next().unwrap();
    assert_eq!(t.attribution, Attribution::Internal);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sqlite_roundtrip_keeps_external() {
    let dir = std::env::temp_dir().join(format!("selmem-attr-sql-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.db");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("A")).unwrap();
    assert!(mem.live_with(pin("They cancelled the Lyon file.", Attribution::External)).kept);
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    let t = loaded.store.traces.values().next().unwrap();
    assert_eq!(t.attribution, Attribution::External);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_witness_vault_loads() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("experiments/witness/witness-a.selmem");
    if !path.is_file() {
        return;
    }
    let loaded = SelectiveMemory::open(path.to_str().unwrap(), EntityProfile::tender("A"))
        .expect("existing witness vault must still load");
    assert!(!loaded.store.traces.is_empty());
    let external = loaded
        .store
        .traces
        .values()
        .filter(|t| t.attribution == Attribution::External)
        .count();
    assert_eq!(external, 1, "seed A pins one External hour");
    assert!(loaded
        .store
        .traces
        .values()
        .all(|t| t.attribution == Attribution::None || t.attribution == Attribution::External));
}

#[test]
fn live_json_can_pin_without_changing_default() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    let pinned = selmem::api::dispatch(
        &mut mem,
        "POST",
        "/live",
        "",
        r#"{"event":"The Lyon file is cancelled.","attribution":"internal","permanence":0.9}"#,
    );
    assert_eq!(pinned.status, 200);
    let t = mem.store.traces.values().next().unwrap();
    assert_eq!(t.attribution, Attribution::Internal);

    let mut other = SelectiveMemory::new(EntityProfile::tender("B"));
    let plain = selmem::api::dispatch(
        &mut other,
        "POST",
        "/live",
        "",
        r#"{"event":"The copier jammed again.","permanence":0.9}"#,
    );
    assert_eq!(plain.status, 200);
    let t = other.store.traces.values().next().unwrap();
    assert_eq!(t.attribution, Attribution::None);
}

fn t0_internal(text: &str) -> EncodeInput<'_> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.70;
    ev.arousal = 0.80;
    ev.disgust = 0.55;
    ev.self_relevance = 0.90;
    ev.permanence = 0.85;
    ev.schema = Some("lyon-file".into());
    ev.attribution = Attribution::Internal;
    ev
}

#[test]
fn internal_conflict_without_axiom_does_not_skip_rewrite() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(t0_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    assert!(
        !selmem::dream::rewrite::skip_rewrite(&mem.store, &id),
        "Internal + |valence| and no axiom is conflict"
    );
}

#[test]
fn external_skips_rewrite() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let mut ev = t0_internal("The project was cancelled in front of the team.");
    ev.attribution = Attribution::External;
    let id = mem.live_with(ev).trace_id.expect("kept");
    assert!(selmem::dream::rewrite::skip_rewrite(&mem.store, &id));
}

#[test]
fn internal_same_sign_axiom_skips_rewrite() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(t0_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_lyon".into(),
        statement: "The file was taken from me.".into(),
        support_trace_ids: vec![id.clone()],
        valence: -0.6,
        strength: 0.4,
        created_at: 1,
        superseded_by: None,
        schema: Some("lyon-file".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    assert!(selmem::dream::rewrite::skip_rewrite(&mem.store, &id));
}

#[test]
fn internal_conflict_night_moves_gist_keeps_core() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p)
        .with_narrator(Box::new(rewrite::RewriteNarrator))
        .with_scorer(Box::new(support::FixedScorer(selmem::PropositionLabel::Entail)));
    // Isolate the rewrite pass: a newly minted matching axiom can resolve conflict.
    mem.cut.ladder = false;
    let id = mem
        .live_with(t0_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let gist0 = mem.store.traces[&id].gist.clone();
    let core0 = mem.store.traces[&id].core.clone();
    let _ = mem.sleep_deep();
    let t = &mem.store.traces[&id];
    let moved = t.gist != gist0
        || t.drifts.iter().any(|d| {
            matches!(
                d.kind,
                selmem::DriftKind::Rewrite | selmem::DriftKind::Embellish | selmem::DriftKind::AmplifyDisgust
            )
        });
    assert!(moved, "conflict Internal must move; gist={}", t.gist);
    assert_eq!(t.core, core0, "core field stays frozen");
}
