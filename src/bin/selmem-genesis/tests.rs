use super::{
    json::{obj, text, Json},
    runner::*,
    transport,
};
#[test]
fn retired_commands_fail_before_any_http() {
    let out = super::lifecycle_tests::root();
    let _session = transport::begin(&out, 10);
    assert!(super::cli(vec![
        "contrast".into(),
        "--out".into(),
        out.to_string_lossy().into()
    ])
    .unwrap_err()
    .contains("retired"));
    assert_eq!(transport::used(), 0);
    assert!(!out.exists());
}
#[test]
fn explicit_runtime_config_is_loaded_before_mock_execution() {
    let out = super::lifecycle_tests::root();
    assert!(super::cli(vec![
        "positive".into(),
        "--mock".into(),
        "--config".into(),
        "/nonexistent-genesis-test.selmem".into(),
        "--out".into(),
        out.to_string_lossy().into()
    ])
    .unwrap_err()
    .contains("runtime config"));
    assert!(!out.exists());
}
#[test]
fn support_lesion_keeps_trace_but_removes_evidence_on_a_clone() {
    let root = super::lifecycle_tests::root();
    std::fs::create_dir_all(&root).unwrap();
    let cfg = obj([
        ("memory_backend", text("rules")),
        ("associations_learn", Json::Bool(true)),
        ("associations_reconstruct", Json::Bool(true)),
        ("observation_evidence", Json::Bool(true)),
    ]);
    let m = Memory::new("selmem_full", root.join("original.vault"), &cfg);
    m.init().unwrap();
    for i in 0..2 {
        m.call(
            "live",
            obj([(
                "event",
                obj([
                    ("id", text(format!("{i}"))),
                    ("text", text("Mira supplied six batteries to the workshop.")),
                    ("critical", Json::Bool(true)),
                ]),
            )]),
        )
        .unwrap();
    }
    let before = file_hash(&m.path).unwrap();
    let clone = m.copy(root.join("lesion.vault")).unwrap();
    clone.lesion("observation_evidence", &[]).unwrap();
    let state = clone.call("inspect", obj([])).unwrap();
    assert_eq!(array(&state, "traces").unwrap().len(), 1);
    assert!(array(&state, "observation_evidence").unwrap().is_empty());
    assert_eq!(file_hash(&m.path).unwrap(), before);
    assert_eq!(
        n(
            &array(&m.call("inspect", obj([])).unwrap(), "observation_evidence").unwrap()[0],
            "observations"
        )
        .unwrap(),
        2.
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn probe_never_mutates_the_snapshot() {
    let root = super::lifecycle_tests::root();
    std::fs::create_dir_all(&root).unwrap();
    let cfg = obj([
        ("memory_backend", text("rules")),
        ("associations_learn", Json::Bool(true)),
        ("associations_reconstruct", Json::Bool(true)),
    ]);
    let m = Memory::new("selmem_full", root.join("memory.vault"), &cfg);
    m.init().unwrap();
    m.call(
        "live",
        obj([(
            "event",
            obj([
                ("id", text("event")),
                ("text", text("The appointment is Tuesday in room B.")),
                ("critical", Json::Bool(true)),
            ]),
        )]),
    )
    .unwrap();
    let before = file_hash(&m.path).unwrap();
    m.context("appointment room").unwrap();
    assert_eq!(file_hash(&m.path).unwrap(), before);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_native_snapshot_is_not_replaced_by_an_empty_one() {
    let root = super::lifecycle_tests::root();
    let m = Memory::new("selmem_full", root.join("missing.vault"), &obj([]));
    assert!(m
        .call("inspect", obj([]))
        .unwrap_err()
        .contains("snapshot missing"));
    assert!(!m.path.exists());
}
