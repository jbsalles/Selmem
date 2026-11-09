//! Regression: reopening between nights must preserve the axiom family key.
use selmem::core::model::{Bearer, LossKind, StakeKind};
use selmem::{Attribution, EncodeInput, EntityProfile, SelectiveMemory};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
fn path(ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "selmem-axiom-context-{}-{}.{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed),
        ext
    ))
}
fn seeded(p: &std::path::Path) -> SelectiveMemory {
    let mut m = SelectiveMemory::open(p, EntityProfile::new("observer")).unwrap();
    for i in 0..3 {
        let id = format!("episode-{i}");
        let mut e = EncodeInput::new("Mira delivered the promised batteries to the workshop.");
        e.observation_id = Some(&id);
        e.valence = 0.8;
        e.arousal = 0.8;
        e.self_relevance = 0.9;
        e.utility = 0.9;
        e.permanence = 1.;
        e.schema = Some("workshop cooperation".into());
        e.attribution = Attribution::External;
        assert!(m.live_with(e).kept);
    }
    for t in m.store.traces.values_mut() {
        t.stake_kind = StakeKind::Decision;
        t.bearer = Bearer::Other;
        t.loss_kind = LossKind::Time;
        t.stake_mark = "cooperation".into();
    }
    m.sleep_deep();
    assert!(!m.store.axioms.is_empty());
    m
}
#[test]
fn file_and_sqlite_preserve_context_and_do_not_multiply_axioms() {
    for ext in ["vault", "db"] {
        let p = path(ext);
        let m = seeded(&p);
        let initial = m.store.axioms.len();
        let expected = m
            .store
            .axioms
            .values()
            .map(|a| {
                (
                    a.id.clone(),
                    a.stake_kind,
                    a.bearer,
                    a.loss_kind,
                    a.stake_mark.clone(),
                )
            })
            .collect::<Vec<_>>();
        m.save().unwrap();
        drop(m);
        for _ in 0..20 {
            let mut m = SelectiveMemory::open(&p, EntityProfile::new("observer")).unwrap();
            for (id, k, b, l, mark) in &expected {
                let a = &m.store.axioms[id];
                assert_eq!(
                    (a.stake_kind, a.bearer, a.loss_kind, &a.stake_mark),
                    (*k, *b, *l, mark)
                );
            }
            m.advance_hours(24.);
            m.sleep_deep();
            assert_eq!(
                m.store.axioms.len(),
                initial,
                "{ext}: duplicate consolidation after reopen"
            );
            m.save().unwrap();
        }
        std::fs::remove_file(p).unwrap();
    }
}
#[test]
fn legacy_file_infers_context_from_lived_support_without_archive() {
    let p = path("vault");
    let m = seeded(&p);
    let original = m.store.axioms.values().next().unwrap().clone();
    m.save().unwrap();
    drop(m);
    let text = std::fs::read_to_string(&p).unwrap();
    let mut it = text.split_inclusive('\n');
    let mut old = String::new();
    while let Some(line) = it.next() {
        if line.starts_with("axiom_context ") {
            let len = it.next().unwrap().trim().parse::<usize>().unwrap();
            let mark = it.next().unwrap();
            assert_eq!(mark.trim_end_matches('\n').len(), len);
            continue;
        }
        old.push_str(line);
    }
    std::fs::write(&p, old).unwrap();
    let mut m = SelectiveMemory::open(&p, EntityProfile::new("observer")).unwrap();
    let a = &m.store.axioms[&original.id];
    assert_eq!(
        (a.stake_kind, a.bearer, a.loss_kind, &a.stake_mark),
        (
            original.stake_kind,
            original.bearer,
            original.loss_kind,
            &original.stake_mark
        )
    );
    let n = m.store.axioms.len();
    m.sleep_deep();
    assert_eq!(m.store.axioms.len(), n);
    std::fs::remove_file(p).unwrap();
}
#[test]
fn explicit_empty_context_is_preserved_not_inferred() {
    for ext in ["vault", "db"] {
        let p = path(ext);
        let mut m = seeded(&p);
        let id = m.store.axioms.keys().next().unwrap().clone();
        let a = m.store.axioms.get_mut(&id).unwrap();
        a.stake_kind = StakeKind::None;
        a.bearer = Bearer::World;
        a.loss_kind = LossKind::None;
        a.stake_mark.clear();
        m.save().unwrap();
        drop(m);
        let m = SelectiveMemory::open(&p, EntityProfile::new("observer")).unwrap();
        let a = &m.store.axioms[&id];
        assert_eq!(a.stake_kind, StakeKind::None);
        assert_eq!(a.bearer, Bearer::World);
        assert!(a.stake_mark.is_empty());
        std::fs::remove_file(p).unwrap();
    }
}
