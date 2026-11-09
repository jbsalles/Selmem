//! Offline regressions for independent time, persistence, revision and vector spaces.
use selmem::core::model::{now_secs, ClockGuard, MemoryClock};
use selmem::encode::embed::EmbeddingResult;
use selmem::{Embedder, EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
fn path(ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "organ-fix-{}-{}.{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed),
        ext
    ))
}
fn add(m: &mut SelectiveMemory) -> String {
    let mut e = EncodeInput::new("Mira delivered the promised batteries to the workshop.");
    e.valence = 0.6;
    e.arousal = 0.8;
    e.self_relevance = 0.9;
    e.permanence = 1.;
    e.schema = Some("workshop cooperation".into());
    m.live_with(e).trace_id.unwrap()
}
#[test]
fn default_organs_are_independent_and_detach_preserves_their_timeline() {
    let mut a = SelectiveMemory::new(EntityProfile::new("a"));
    let b = SelectiveMemory::new(EntityProfile::new("b"));
    let b0 = b.clock.now();
    a.advance_hours(100.);
    assert!(a.clock.now() >= b0 + 360000);
    assert!(b.clock.now() < b0 + 100);
    let before = a.clock.clone();
    a = a.detach_clock();
    assert_eq!(a.clock.jump, before.jump);
    assert_eq!(a.clock.origin_real, before.origin_real);
    a.clock.set_scale(200);
    let t = a.clock.now();
    a.clock.set_scale(1);
    assert!(a.clock.now().abs_diff(t) <= 200);
    let id = add(&mut a);
    assert!(a.suppress(&id));
    assert!(a.reinterpret(&id, "I can cooperate cautiously."));
    let trace = &a.store.traces[&id];
    assert!(trace.operations.last().unwrap().at >= trace.created_at);
    assert!(trace.drifts.last().unwrap().at >= trace.created_at);
}
#[test]
fn nested_clock_guards_restore_the_outer_clock() {
    let mut a = MemoryClock::default();
    a.advance_hours(100.);
    let b = MemoryClock::default();
    let outer = ClockGuard::push(a.clone());
    {
        let _inner = ClockGuard::push(b.clone());
        assert!(now_secs() < a.now() - 350000);
    }
    assert!(now_secs().abs_diff(a.now()) <= 24);
    drop(outer);
}
#[test]
fn revision_and_complete_trace_state_survive_both_formats_without_changing_facts() {
    for ext in ["vault", "db"] {
        let p = path(ext);
        let mut m = SelectiveMemory::open(&p, EntityProfile::new("Name with spaces")).unwrap();
        let id = add(&mut m);
        let original = m.store.traces[&id].clone();
        m.advance_hours(72.);
        m.reinterpret(&id, "I can cooperate cautiously.");
        let t = m.store.traces.get_mut(&id).unwrap();
        t.interpretation.valence = -0.3;
        t.stake_mark = "shared resources".into();
        let aid = t.archive_id.clone().unwrap();
        m.store.archives.get_mut(&aid).unwrap().core = "explicit archive core".into();
        let clock = m.clock.clone();
        m.save().unwrap();
        let bytes = std::fs::read(&p).unwrap();
        let mut loaded = SelectiveMemory::open(&p, EntityProfile::new("fallback")).unwrap();
        assert_eq!(loaded.profile.name, "Name with spaces");
        assert_eq!(loaded.clock.jump, clock.jump);
        assert_eq!(loaded.clock.origin_real, clock.origin_real);
        let t = &loaded.store.traces[&id];
        assert_eq!(t.interpretation.statement, "I can cooperate cautiously.");
        assert_eq!(t.interpretation.valence, -0.3);
        assert_eq!(t.stake_mark, "shared resources");
        assert_eq!(t.core, original.core);
        assert_eq!(t.semantic.claim, original.semantic.claim);
        assert_eq!(t.reality.claim, original.reality.claim);
        assert_eq!(
            t.operations.last().unwrap().source_center.as_deref(),
            Some("workshop cooperation")
        );
        assert_eq!(loaded.store.archives[&aid].core, "explicit archive core");
        let state = format!("{:?}", loaded.store.traces[&id]);
        let (hits, _) = loaded.remember_with(
            "Mira batteries",
            RecallWrite::ReadOnly,
            RecallBias::ForceMarked,
            &[id.clone()],
        );
        assert!(hits[0].narrative.contains(
            "[Tentative interpretation; not an observed fact] I can cooperate cautiously."
        ));
        assert_eq!(format!("{:?}", loaded.store.traces[&id]), state);
        assert!(
            std::fs::read(&p).unwrap() == bytes,
            "snapshot bytes changed"
        );
        std::fs::remove_file(p).unwrap();
    }
}
struct Vector(&'static str, Vec<f32>);
impl Embedder for Vector {
    fn space_id(&self) -> String {
        self.0.into()
    }
    fn embed_result(&self, _: &str) -> EmbeddingResult {
        EmbeddingResult::External(self.1.clone())
    }
}
#[test]
fn equal_dimensions_do_not_imply_compatible_spaces_and_open_reindexes_safely() {
    let p = path("vault");
    let mut m = SelectiveMemory::open(&p, EntityProfile::new("a"))
        .unwrap()
        .with_embedder(Box::new(Vector("a", vec![1., 0.])));
    let id = add(&mut m);
    assert_eq!(m.store.traces[&id].embedding, vec![1., 0.]);
    m = m.with_embedder(Box::new(Vector("b", vec![0., 1.])));
    assert_eq!(m.store.traces[&id].embedding, vec![0., 1.]);
    m.save().unwrap();
    let loaded = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
    assert_eq!(loaded.store.embedding_space.as_deref(), Some("hash96-v1"));
    assert_eq!(loaded.store.traces[&id].embedding.len(), 96);
    std::fs::remove_file(p).unwrap();
}
#[test]
fn api_reports_save_errors_and_keeps_last_committed_file() {
    let p = path("vault");
    let mut m = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
    add(&mut m);
    m.save().unwrap();
    let bytes = std::fs::read(&p).unwrap();
    // Parent is a regular file, so neither backend can create a child snapshot.
    m.path = Some(p.join("child.vault"));
    let response = selmem::api::dispatch(&mut m, "POST", "/sleep", "", "");
    assert_eq!(response.status, 500);
    assert!(response.body.contains("persistence failed"));
    assert_eq!(std::fs::read(&p).unwrap(), bytes);
    std::fs::remove_file(p).unwrap();
}
#[test]
fn ambiguous_scorer_answers_abstain() {
    use selmem::PropositionLabel as L;
    assert_eq!(L::parse("entail but actually contradict"), L::Unknown);
    assert_eq!(L::parse("entail\ncontradict"), L::Unknown);
    assert_eq!(L::parse(" Entail \n"), L::Entail);
}
#[test]
fn concurrent_flat_saves_are_atomic_and_leave_no_temporary_files() {
    let dir = path("directory");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("shared.vault");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let p = p.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                let mut m =
                    SelectiveMemory::open(&p, EntityProfile::new(format!("writer {i}"))).unwrap();
                add(&mut m);
                b.wait();
                m.save().unwrap();
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let m = SelectiveMemory::open(&p, EntityProfile::new("fallback")).unwrap();
    assert_eq!(m.store.traces.len(), 1);
    assert!(m.profile.name.starts_with("writer "));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn shallow_and_repeated_deep_nights_preserve_revision_facts_and_support_integrity() {
    for ext in ["vault", "db"] {
        let p = path(ext);
        let mut m = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
        let id = add(&mut m);
        m.reinterpret(&id, "I can cooperate cautiously.");
        let original = m.store.traces[&id].clone();
        let archive = m.audit(&id).unwrap().to_string();
        // Force the shallow maintenance path, independently of the adaptive budget.
        {
            let _clock = ClockGuard::push(m.clock.clone());
            selmem::dream::dream_kind(
                &mut m.store,
                &m.profile,
                &selmem::RuleNarrator,
                &selmem::HashEmbedder,
                m.cut,
                selmem::dream::NightKind::Shallow,
                &selmem::NullScorer,
            );
        }
        assert!(m.store.axioms.is_empty());
        m.save().unwrap();
        for _ in 0..20 {
            m = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
            m.advance_hours(24.);
            m.sleep_deep();
            let t = &m.store.traces[&id];
            assert_eq!(t.core, original.core);
            assert_eq!(t.reality.claim, original.reality.claim);
            assert_eq!(t.interpretation.statement, "I can cooperate cautiously.");
            assert_eq!(m.audit(&id), Some(archive.as_str()));
            for a in m.store.axioms.values() {
                let unique: std::collections::HashSet<_> = a.support_trace_ids.iter().collect();
                assert_eq!(unique.len(), a.support_trace_ids.len());
                assert!(a
                    .support_trace_ids
                    .iter()
                    .all(|id| m.store.traces.contains_key(id)));
            }
            m.save().unwrap();
        }
        std::fs::remove_file(p).unwrap();
    }
}
#[test]
fn merge_preserves_incompatible_stakes_and_the_thirty_day_boundary() {
    let mut m = SelectiveMemory::new(EntityProfile::new("a"));
    m.profile.merge_similarity = 0.1;
    let a = add(&mut m);
    let b = add(&mut m);
    for id in [&a, &b] {
        let t = m.store.traces.get_mut(id).unwrap();
        t.anchor = 0.1;
        t.valence = 0.;
    }
    m.store.traces.get_mut(&a).unwrap().stake_mark = "time".into();
    m.store.traces.get_mut(&b).unwrap().stake_mark = "status".into();
    assert_eq!(
        selmem::dream::merge::run(&mut m.store, &m.profile, &selmem::HashEmbedder, false),
        0
    );
    let t = m.store.traces.get_mut(&a).unwrap();
    t.stake_mark = "same".into();
    t.created_at = 4_102_444_800;
    let t = m.store.traces.get_mut(&b).unwrap();
    t.stake_mark = "same".into();
    t.created_at = 4_102_444_800 + 30 * 24 * 3600 + 1;
    t.valence = 0.35;
    assert_eq!(
        selmem::dream::merge::run(&mut m.store, &m.profile, &selmem::HashEmbedder, false),
        0
    );
    m.store.traces.get_mut(&b).unwrap().created_at = 4_102_444_800 + 30 * 24 * 3600;
    assert_eq!(
        selmem::dream::merge::run(&mut m.store, &m.profile, &selmem::HashEmbedder, false),
        1
    );
}
#[test]
fn http_scorer_accepts_explicit_labels_and_abstains_on_error_or_ambiguous_answers() {
    use selmem::{HttpScorer, PropositionLabel as L, PropositionScorer};
    use std::io::{Read, Write};
    for (status, response, expected) in [
        (
            200,
            r#"{"choices":[{"message":{"content":"entail"}}]}"#,
            L::Entail,
        ),
        (
            200,
            r#"{"choices":[{"message":{"content":"contradict"}}]}"#,
            L::Contradict,
        ),
        (
            200,
            r#"{"choices":[{"message":{"content":"entail but contradict"}}]}"#,
            L::Unknown,
        ),
        (
            200,
            r#"{"choices":[{"message":{"content":null,"reasoning_content":"entail"}}]}"#,
            L::Unknown,
        ),
        (200, r#"{"content":"entail"}"#, L::Unknown),
        (429, r#"{"error":"capacity"}"#, L::Unknown),
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/chat/completions", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = socket.read(&mut buffer).unwrap();
                assert!(n > 0);
                data.extend_from_slice(&buffer[..n]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&data[..end]);
                    let len = header
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if data.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            write!(socket,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
        });
        let scorer = HttpScorer::parse(&url, "fixture", None).unwrap();
        assert_eq!(
            scorer.score("Mira delivered batteries.", "Mira supplied batteries."),
            expected
        );
        server.join().unwrap();
    }
}
#[test]
fn explicitly_empty_semantic_fields_are_not_reinferred_on_load() {
    for ext in ["vault", "db"] {
        let p = path(ext);
        let mut m = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
        let id = add(&mut m);
        let t = m.store.traces.get_mut(&id).unwrap();
        t.semantic.claim.clear();
        t.semantic.entities.clear();
        t.semantic.actions.clear();
        t.reality.claim.clear();
        t.interpretation.statement.clear();
        m.save().unwrap();
        let loaded = SelectiveMemory::open(&p, EntityProfile::new("a")).unwrap();
        let t = &loaded.store.traces[&id];
        assert!(t.semantic.claim.is_empty());
        assert!(t.semantic.entities.is_empty());
        assert!(t.semantic.actions.is_empty());
        assert!(t.reality.claim.is_empty());
        assert!(t.interpretation.statement.is_empty());
        std::fs::remove_file(p).unwrap();
    }
}
