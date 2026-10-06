//! P3: self_congruence is measured at paint and drives Internal conflict.

use selmem::{
    measure_congruence, Attribution, AxiomLayer, EncodeInput, EntityProfile, IdentityAxiom,
    SelectiveMemory,
};

fn charged_internal(text: &str) -> EncodeInput<'_> {
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

fn add_axiom(mem: &mut SelectiveMemory, id: &str, valence: f32) {
    mem.store.add_axiom(IdentityAxiom {
        id: id.into(),
        statement: "The file was taken from me.".into(),
        support_trace_ids: vec![],
        valence,
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
}

#[test]
fn no_axiom_charged_hour_is_incongruent() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(charged_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let t = &mem.store.traces[&id];
    assert!(t.self_congruence < 0.40, "got {}", t.self_congruence);
    assert!(selmem::dream::rewrite::is_conflict(&mem.store, t));
}

#[test]
fn dull_hour_without_axiom_stays_mid() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let mut ev = EncodeInput::new("Someone reprints the agenda with the same items.");
    ev.valence = 0.05;
    ev.schema = Some("office".into());
    ev.permanence = 0.85;
    let id = mem.live_with(ev).trace_id.expect("kept");
    let c = mem.store.traces[&id].self_congruence;
    assert!((c - 0.50).abs() < 0.02, "got {c}");
}

#[test]
fn same_sign_axiom_raises_congruence_and_skips_rewrite() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    add_axiom(&mut mem, "ax_neg", -0.60);
    let id = mem
        .live_with(charged_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let t = &mem.store.traces[&id];
    assert!(t.self_congruence >= 0.40, "got {}", t.self_congruence);
    assert!(!selmem::dream::rewrite::is_conflict(&mem.store, t));
    assert!(selmem::dream::rewrite::skip_rewrite(&mem.store, &id));
}

#[test]
fn opposite_axiom_is_conflict() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    add_axiom(&mut mem, "ax_pos", 0.70);
    let id = mem
        .live_with(charged_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let t = &mem.store.traces[&id];
    assert!(t.self_congruence < 0.40, "got {}", t.self_congruence);
    assert!(selmem::dream::rewrite::is_conflict(&mem.store, t));
}

#[test]
fn live_congruence_updates_when_an_axiom_arrives() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(charged_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    assert!(selmem::dream::rewrite::is_conflict(
        &mem.store,
        &mem.store.traces[&id]
    ));
    add_axiom(&mut mem, "ax_neg", -0.60);
    let live = measure_congruence(
        &mem.store,
        mem.store.traces[&id].schema.as_deref(),
        mem.store.traces[&id].valence,
    );
    assert!(live >= 0.40, "got {live}");
    assert!(!selmem::dream::rewrite::is_conflict(
        &mem.store,
        &mem.store.traces[&id]
    ));
}

#[test]
fn file_roundtrip_keeps_congruence() {
    let dir = std::env::temp_dir().join(format!("selmem-congr-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::open(&path, p.clone()).unwrap();
    let id = mem
        .live_with(charged_internal(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    let c0 = mem.store.traces[&id].self_congruence;
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, p).unwrap();
    let c1 = loaded.store.traces[&id].self_congruence;
    assert!((c0 - c1).abs() < 1e-4, "{c0} vs {c1}");
    let _ = std::fs::remove_dir_all(&dir);
}
