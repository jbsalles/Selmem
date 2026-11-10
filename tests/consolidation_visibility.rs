use selmem::core::store::MemoryStore;
use selmem::dream::{centers, confab, ladder, merge, rewrite};
use selmem::{
    EncodeInput, EntityProfile, HashEmbedder, MemoryTrace, RuleNarrator, SelectiveMemory,
};

fn trace(id: &str, text: &str) -> MemoryTrace {
    let mut p = EntityProfile::new("test");
    p.encode_threshold = 0.;
    let mut m = SelectiveMemory::new(p);
    let mut e = EncodeInput::new(text);
    e.arousal = 0.8;
    e.self_relevance = 0.9;
    e.permanence = 0.5;
    let key = m.live_with(e).trace_id.unwrap();
    let mut t = m.store.traces.remove(&key).unwrap();
    t.id = id.into();
    t.observation_id = Some(id.into());
    t.archive_id = None;
    t.schema = Some("workshop".into());
    t.gist = text.into();
    t.core = text.into();
    t.anchor = 0.2;
    t.valence = 0.1;
    t.personal = Default::default();
    t
}

fn axiom(id: &str, strength: f32, supports: &[&str]) -> selmem::IdentityAxiom {
    selmem::persist::snapshot::assemble_axiom(
        id.into(),
        format!("{id} workshop expectation."),
        -0.6,
        strength,
        10,
        None,
        Some("workshop".into()),
        selmem::AxiomLayer::Belief,
        supports.iter().map(|s| (*s).into()).collect(),
    )
}

#[test]
fn suppression_prevents_merge_and_ladder_propagation() {
    let mut s = MemoryStore::new();
    let mut a = trace("visible", "Mira brought tools to the workshop.");
    a.anchor = 0.6;
    let mut b = trace("hidden", "Mira brought secret plans to the workshop.");
    b.suppressed = true;
    s.add_trace(a.clone());
    s.add_trace(b);
    let mut p = EntityProfile::new("test");
    p.merge_similarity = 0.;
    assert_eq!(merge::run(&mut s, &p, &HashEmbedder, true), 0);
    assert_eq!(s.traces["visible"].core, a.core);
    s.traces.get_mut("hidden").unwrap().suppressed = false;
    assert_eq!(merge::run(&mut s, &p, &HashEmbedder, true), 1);

    let mut s = MemoryStore::new();
    for id in ["a", "b"] {
        let mut t = trace(id, "Mira delivered the promised workshop tools.");
        t.valence = 0.8;
        t.suppressed = true;
        s.add_trace(t);
    }
    assert!(ladder::run(&mut s, &RuleNarrator).is_empty());
    for t in s.traces.values_mut() {
        t.suppressed = false;
    }
    assert!(!ladder::run(&mut s, &RuleNarrator).is_empty());
}

#[test]
fn derived_axioms_and_cached_centers_are_withheld_reversibly() {
    let mut s = MemoryStore::new();
    s.add_trace(trace("hidden", "Secret workshop plans disappeared."));
    s.add_axiom(axiom("belief", 0.8, &["hidden"]));
    centers::refresh(&mut s);
    s.traces.get_mut("hidden").unwrap().suppressed = true;
    assert!(s.living_axioms().is_empty());
    assert!(s.visible_center("workshop").is_none());
    assert!(rewrite::skip_rewrite(&s, "hidden"));
    let t = s.traces.get_mut("hidden").unwrap();
    t.fidelity = 0.3;
    assert!(!confab::is_hole(t));
    centers::refresh(&mut s);
    assert!(s.centers.is_empty());
    s.traces.get_mut("hidden").unwrap().suppressed = false;
    assert_eq!(s.living_axioms().len(), 1);
    centers::refresh(&mut s);
    assert!(s.visible_center("workshop").is_some());
}

#[test]
fn center_and_confabulation_selection_do_not_depend_on_map_order() {
    for reverse in [false, true] {
        for _ in 0..16 {
            let mut s = MemoryStore::new();
            let mut candidates = vec![
                axiom("weak", 0.2, &[]),
                axiom("winner", 0.8, &[]),
                axiom("zebra", 0.8, &[]),
            ];
            if reverse {
                candidates.reverse();
            }
            for a in candidates {
                s.add_axiom(a);
            }
            centers::refresh(&mut s);
            assert_eq!(s.centers["workshop"].axiom_id.as_deref(), Some("winner"));
            s.centers.clear();
            let mut t = trace("target", "The toolbox stood beside the door.");
            t.fidelity = 0.3;
            s.add_trace(t);
            assert!(confab::fill_if_hole(
                &mut s,
                "target",
                &EntityProfile::new("test")
            ));
            assert_eq!(
                s.traces["target"]
                    .operations
                    .last()
                    .unwrap()
                    .source_axiom_ids,
                vec!["winner"]
            );
        }
    }
}

#[test]
fn suppression_and_selection_survive_both_snapshot_formats() {
    for ext in ["selmem", "db"] {
        let path = std::env::temp_dir().join(format!("visibility-{}.{}", std::process::id(), ext));
        let _ = std::fs::remove_file(&path);
        let mut m = SelectiveMemory::open(&path, EntityProfile::new("test")).unwrap();
        let mut t = trace("hidden", "Secret workshop plans disappeared.");
        t.suppressed = true;
        m.store.add_trace(t);
        m.store.add_axiom(axiom("hidden-belief", 0.9, &["hidden"]));
        m.store.add_axiom(axiom("winner", 0.8, &[]));
        m.save().unwrap();
        let mut loaded = SelectiveMemory::open(&path, EntityProfile::new("test")).unwrap();
        centers::refresh(&mut loaded.store);
        assert_eq!(
            loaded.store.centers["workshop"].axiom_id.as_deref(),
            Some("winner")
        );
        loaded.unsuppress("hidden");
        centers::refresh(&mut loaded.store);
        assert_eq!(
            loaded.store.centers["workshop"].axiom_id.as_deref(),
            Some("hidden-belief")
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn release_preserves_support_for_a_withheld_axiom() {
    let mut s = MemoryStore::new();
    let mut t = trace("hidden", "Secret workshop plans disappeared.");
    t.suppressed = true;
    t.status = selmem::TraceStatus::Latent;
    t.access = 0.;
    t.anchor = 0.;
    t.permanence = 0.;
    s.add_trace(t);
    s.add_axiom(axiom("belief", 0.8, &["hidden"]));
    let latent = std::collections::HashSet::from(["hidden".into()]);
    assert_eq!(selmem::dream::release::run(&mut s, &latent), 0);
    assert!(s.living_axioms().is_empty());
    assert_eq!(s.axioms["belief"].support_trace_ids, vec!["hidden"]);
    s.axioms.clear();
    assert_eq!(selmem::dream::release::run(&mut s, &latent), 1);
}

#[derive(Default)]
struct Neighbors(std::sync::Mutex<Vec<(String, Vec<String>)>>);
impl selmem::Narrator for Neighbors {
    fn reconstruct(&self, t: &MemoryTrace, _: &selmem::Mood, _: &str) -> String {
        t.gist.clone()
    }
    fn distill_axiom(&self, _: &[&MemoryTrace]) -> Option<String> {
        None
    }
    fn rewrite(
        &self,
        t: &MemoryTrace,
        others: &[&MemoryTrace],
        _: &EntityProfile,
    ) -> Option<String> {
        self.0
            .lock()
            .unwrap()
            .push((t.id.clone(), others.iter().map(|t| t.id.clone()).collect()));
        None
    }
}

#[test]
fn rewrite_never_receives_suppressed_neighbors_or_cached_hubs() {
    let mut s = MemoryStore::new();
    s.add_trace(trace("visible", "The toolbox stood beside the door."));
    let mut t = trace("hidden", "Secret workshop plans disappeared.");
    t.suppressed = true;
    s.add_trace(t);
    s.centers.insert(
        "workshop".into(),
        selmem::core::model::SchemaCenter {
            schema: "workshop".into(),
            core: "Secret workshop plans disappeared.".into(),
            weight: 2.,
            valence: -0.6,
            hub_id: Some("hidden".into()),
            axiom_id: None,
        },
    );
    let narrator = Neighbors::default();
    rewrite::run(
        &mut s,
        &EntityProfile::new("test"),
        &narrator,
        &HashEmbedder,
        false,
        &selmem::NullScorer,
    );
    let calls = narrator.0.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "visible");
    assert!(calls[0].1.is_empty());
}
