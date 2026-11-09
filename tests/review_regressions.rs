use selmem::core::evidence::{ObservationEvidence, ObservationGroup};
use selmem::*;
use std::collections::BTreeMap;
fn group(source: &str, n: usize, offset: usize) -> ObservationGroup {
    let observations = (offset..offset + n)
        .map(|i| (format!("obs-{i}"), format!("claim-{i}")))
        .collect::<BTreeMap<_, _>>();
    ObservationGroup {
        source: source.into(),
        channel: "Selfhood".into(),
        primary_claim: format!("claim-{offset}"),
        observations,
        updated_at: 0,
    }
}
#[test]
fn evidence_transfer_rejects_provenance_conflicts_and_unserializable_unions() {
    for (a, b) in [
        (group("A", 1, 0), group("B", 1, 0)),
        (group("A", 8, 0), group("A", 1, 8)),
    ] {
        let mut e = ObservationEvidence {
            enabled: true,
            groups: BTreeMap::from([("keep".into(), a), ("other".into(), b)]),
        };
        let before = e.clone();
        assert!(!e.can_transfer_on_merge("keep", "other"));
        e.transfer_on_merge("keep", "other");
        assert_eq!(e, before);
        assert_eq!(ObservationEvidence::decode(&e.encode()).unwrap(), e);
    }
    let mut e = ObservationEvidence {
        enabled: true,
        groups: BTreeMap::from([
            ("keep".into(), group("A", 1, 0)),
            ("other".into(), group("A", 1, 0)),
        ]),
    };
    e.groups
        .get_mut("other")
        .unwrap()
        .observations
        .insert("obs-0".into(), "conflicting claim".into());
    assert!(!e.can_transfer_on_merge("keep", "other"));
}
#[test]
fn disabled_signal_does_not_drop_merge_metadata() {
    let mut e = ObservationEvidence {
        enabled: false,
        groups: BTreeMap::from([
            ("keep".into(), group("A", 1, 0)),
            ("other".into(), group("A", 1, 1)),
        ]),
    };
    e.transfer_on_merge("keep", "other");
    assert_eq!(e.groups["keep"].observations.len(), 2);
    assert!(!e.groups.contains_key("other"));
    assert_eq!(ObservationEvidence::decode(&e.encode()).unwrap(), e);
}
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "selmem-review-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn all_cut_flags_survive_file_and_sqlite_roundtrip() {
    for ext in ["vault", "sqlite"] {
        let path = temp(ext).with_extension(ext);
        let mut m = SelectiveMemory::new(EntityProfile::new("Claire"));
        m.cut = OrganCut::no_self_cues();
        m.cut.support_signal = false;
        m.cut.schema_prototype = false;
        m.path = Some(path.clone());
        m.save().unwrap();
        let loaded = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        assert!(
            !loaded.cut.self_cues && !loaded.cut.support_signal && !loaded.cut.schema_prototype
        );
        if ext == "vault" {
            let s = std::fs::read_to_string(&path).unwrap();
            let old = s
                .lines()
                .map(|l| {
                    if l.starts_with("cut ") {
                        l.split_whitespace().take(7).collect::<Vec<_>>().join(" ")
                    } else {
                        l.into()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            std::fs::write(&path, old).unwrap();
            let loaded = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
            assert!(
                loaded.cut.self_cues && loaded.cut.support_signal && loaded.cut.schema_prototype
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn risk_cli_builds_risk_histories_and_records_axis() {
    let out = temp("risk");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_selmem-contextual-longlife"))
        .args([
            "--out",
            out.to_str().unwrap(),
            "--axis",
            "risk",
            "--pairs",
            "1",
            "--distractors",
            "80",
        ])
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(out.join("snapshots/invest-normal-on.vault").is_file());
    assert!(!out.join("snapshots/workspace-normal-on.vault").exists());
    let manifest = std::fs::read_to_string(out.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"axis\":\"risk\""));
    std::fs::remove_dir_all(out).unwrap();
}
#[test]
fn merge_rejects_cross_source_evidence_before_mutating_carriers() {
    let mut p = EntityProfile::new("Claire");
    p.encode_threshold = 0.;
    p.merge_similarity = 0.;
    let mut m = SelectiveMemory::new(p);
    let mut ids = Vec::new();
    for (source, event) in [
        ("A", "Mira supplied batteries today."),
        ("B", "Mira supplied batteries yesterday."),
    ] {
        let mut i = EncodeInput::new(event);
        i.source = source;
        i.attribution = Attribution::Internal;
        i.schema = Some("workshop".into());
        let id = m.live_with(i).trace_id.unwrap();
        let t = m.store.traces.get_mut(&id).unwrap();
        t.embedding.clear();
        t.anchor = 0.3;
        t.valence = 0.1;
        t.disgust = 0.;
        t.stake_mark.clear();
        t.personal.appraisal.prior_ids.clear();
        t.personal.protected_until = 0;
        m.store
            .observation_evidence
            .groups
            .insert(id.clone(), group(source, 1, 0));
        ids.push(id);
    }
    assert_eq!(
        selmem::dream::merge::run(&mut m.store, &m.profile, &HashEmbedder, false),
        0
    );
    assert!(ids
        .iter()
        .all(|id| m.store.traces[id].status != TraceStatus::Myth));
    assert_eq!(m.store.observation_evidence.groups.len(), 2);
}
