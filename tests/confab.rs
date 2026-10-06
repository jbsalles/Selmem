//! P5: filling a hole is not gilding a detail.

use selmem::{
    Attribution, Channel, DriftKind, EncodeInput, EntityProfile, IdentityAxiom, AxiomLayer,
    SelectiveMemory,
};

fn dull_pos<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.55;
    ev.arousal = 0.40;
    ev.disgust = 0.0;
    ev.self_relevance = 0.60;
    ev.permanence = 0.35;
    ev.schema = Some("hearth".into());
    ev.channel = Channel::Selfhood;
    ev
}

fn hole_hour<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.20;
    ev.arousal = 0.30;
    ev.self_relevance = 0.55;
    ev.permanence = 0.35;
    ev.schema = Some("office".into());
    ev.channel = Channel::Selfhood;
    ev.attribution = Attribution::None;
    ev
}

#[test]
fn sharp_positive_hour_gilds_and_does_not_confabulate() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(dull_pos(
            "They laughed at the table and stayed until the lamps went out.",
        ))
        .trace_id
        .expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.fidelity = 0.95;
        t.valence = 0.55;
        t.disgust = 0.05;
    }
    let _ = selmem::dream::drift::sculpt(
        mem.store.traces.get_mut(&id).unwrap(),
        &mem.profile,
        false,
    );
    let t = &mem.store.traces[&id];
    assert!(
        !t.drifts.iter().any(|d| d.kind == DriftKind::Confabulate),
        "a sharp gist is not a hole"
    );
    assert!(
        t.drifts
            .iter()
            .any(|d| d.kind == DriftKind::Embellish || d.kind == DriftKind::Rewrite),
        "gild or retell may run; kinds={:?}",
        t.drifts.iter().map(|d| format!("{:?}", d.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn collapsed_gist_is_filled_from_the_axiom_not_from_gild() {
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let id = mem
        .live_with(hole_hour(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_office".into(),
        statement: "Credit gone in public.".into(),
        support_trace_ids: vec![id.clone()],
        valence: -0.6,
        strength: 0.4,
        created_at: 1,
        superseded_by: None,
        schema: Some("office".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = t.core.clone();
        t.fidelity = 0.28;
        t.anchor = 0.20;
        t.permanence = 0.30;
    }
    let core0 = mem.store.traces[&id].core.clone();
    assert!(selmem::dream::confab::is_hole(&mem.store.traces[&id]));
    assert!(selmem::dream::confab::fill_if_hole(
        &mut mem.store,
        &id,
        &mem.profile
    ));
    let t = &mem.store.traces[&id];
    assert_eq!(t.core, core0, "core does not receive the fill");
    assert!(
        t.drifts.iter().any(|d| d.kind == DriftKind::Confabulate),
        "fill must be tagged Confabulate"
    );
    assert!(
        !t.drifts.iter().any(|d| d.kind == DriftKind::Embellish),
        "a hole is not an embellishment"
    );
    let g = t.gist.to_lowercase();
    assert!(g.contains(&core0.to_lowercase()) || selmem::encode::scoring::lexical_similarity(&t.gist, &core0) >= 0.30);
    assert!(
        g.contains("credit") || g.contains("public") || g.contains("softness") || g.contains("time"),
        "fill must add material absent from the core: gist={} core={}",
        t.gist,
        core0
    );
}

#[test]
fn external_hour_is_not_confabulated() {
    let mut p = EntityProfile::tender("A");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::new(p);
    let mut ev = hole_hour("The copier jammed in room B.");
    ev.attribution = Attribution::External;
    let id = mem.live_with(ev).trace_id.expect("kept");
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = t.core.clone();
        t.fidelity = 0.25;
        t.anchor = 0.10;
    }
    assert!(!selmem::dream::confab::is_hole(&mem.store.traces[&id]));
    assert!(!selmem::dream::confab::fill_if_hole(
        &mut mem.store,
        &id,
        &mem.profile
    ));
}

#[test]
fn file_roundtrip_keeps_confab_token() {
    let dir = std::env::temp_dir().join(format!("selmem-confab-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.selmem");
    let mut p = EntityProfile::tender("B");
    p.encode_threshold = 0.05;
    let mut mem = SelectiveMemory::open(&path, p.clone()).unwrap();
    let id = mem
        .live_with(hole_hour(
            "The project was cancelled in front of the team.",
        ))
        .trace_id
        .expect("kept");
    mem.store.add_axiom(IdentityAxiom {
        id: "ax_office".into(),
        statement: "Credit gone in public.".into(),
        support_trace_ids: vec![id.clone()],
        valence: -0.6,
        strength: 0.4,
        created_at: 1,
        superseded_by: None,
        schema: Some("office".into()),
        layer: AxiomLayer::Belief,
        stake_kind: selmem::core::model::StakeKind::None,
        bearer: selmem::core::model::Bearer::World,
        loss_kind: selmem::core::model::LossKind::None,
        stake_mark: String::new(),
    });
    {
        let t = mem.store.traces.get_mut(&id).unwrap();
        t.gist = t.core.clone();
        t.fidelity = 0.28;
        t.anchor = 0.20;
        t.permanence = 0.30;
    }
    assert!(selmem::dream::confab::fill_if_hole(
        &mut mem.store,
        &id,
        &mem.profile
    ));
    mem.save().unwrap();
    let loaded = SelectiveMemory::open(&path, p).unwrap();
    let t = &loaded.store.traces[&id];
    assert!(t.drifts.iter().any(|d| d.kind == DriftKind::Confabulate));
    let _ = std::fs::remove_dir_all(&dir);
}
