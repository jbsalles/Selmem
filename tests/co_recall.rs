//! Co-recall changes reconstruction inputs without changing retrieval. No paid LLM.
use selmem::core::association::{AssociationGraph, AssociativeCue};
use selmem::recall::{PropositionLabel, PropositionScorer};
use selmem::{
    Attribution, EncodeInput, EntityProfile, MemoryTrace, Mood, Narrator, RecallBias, RecallWrite,
    SelectiveMemory,
};

struct Witness;
impl PropositionScorer for Witness {
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        PropositionLabel::Entail
    }
    fn name(&self) -> &str {
        "explicit-test-witness"
    }
}
struct Framing {
    unsafe_claim: bool,
}
impl Narrator for Framing {
    fn supports_associations(&self) -> bool {
        true
    }
    fn reconstruct(&self, t: &MemoryTrace, _: &Mood, _: &str) -> String {
        t.core.clone()
    }
    fn reconstruct_associated(
        &self,
        t: &MemoryTrace,
        _: &Mood,
        _: &str,
        cues: &[AssociativeCue],
    ) -> String {
        assert!(!cues.is_empty());
        if self.unsafe_claim {
            format!("{} because Jordan deliberately sabotaged it.", t.core)
        } else {
            format!("Looking back, {}", t.core)
        }
    }
    fn distill_axiom(&self, _: &[&MemoryTrace]) -> Option<String> {
        None
    }
}
fn memory(attr: Attribution) -> SelectiveMemory {
    let mut mem = SelectiveMemory::new(EntityProfile::new("Claire"))
        .detach_clock()
        .with_narrator(Box::new(Framing {
            unsafe_claim: false,
        }))
        .with_scorer(Box::new(Witness));
    mem.clock.origin_real = 4_102_444_800;
    mem.clock.scale = 1;
    mem.cut.reconsolidate = false;
    mem.cut.ground = false;
    for (source, text) in [
        (
            "one",
            "During the team meeting my presentation was interrupted.",
        ),
        (
            "two",
            "During the team meeting my request for help received no answer.",
        ),
    ] {
        let mut e = EncodeInput::new(text);
        e.permanence = 1.0;
        e.self_relevance = 0.9;
        e.arousal = 0.7;
        e.observation_id = Some(source);
        e.source = source;
        e.attribution = attr;
        assert!(mem.live_with(e).kept);
    }
    // Frozen synthetic claims isolate this mechanism from compression variation.
    for t in mem.store.traces.values_mut() {
        t.core = t.gist.clone();
    }
    assert_eq!(mem.store.traces.len(), 2);
    mem
}
const QUERY: &str = "team meeting presentation request help";
fn train(mem: &mut SelectiveMemory) {
    mem.set_associations(true, false);
    for _ in 0..12 {
        mem.begin_association_episode();
        let (_, dump) = mem.remember_with(QUERY, RecallWrite::Live, RecallBias::Observed, &[]);
        assert_eq!(dump.selected.len(), 2);
        assert_eq!(dump.associations_reinforced.len(), 1);
    }
    assert_eq!(mem.store.associations.pairs.len(), 1);
}
#[test]
fn bounded_decay_and_episode_deduplication() {
    let mut g = AssociationGraph::default();
    g.learn = true;
    let active = vec![
        ("a".into(), "oa".into(), 1.0),
        ("b".into(), "ob".into(), 1.0),
    ];
    assert_eq!(g.reinforce(&active, 100).len(), 1);
    let first = g.weight("a", "b", 100);
    assert!(g.reinforce(&active, 110).is_empty());
    assert_eq!(g.weight("a", "b", 100), first);
    for _ in 0..100 {
        g.begin_episode(110);
        g.reinforce(&active, 110);
    }
    let weight = g.weight("a", "b", 110);
    assert!(weight > 0.9 && weight <= 1.0);
    assert!((g.weight("a", "b", 110 + 30 * 86400) - weight / 2.0).abs() < 0.00001);
    let before = g.clone();
    let _ = g.weight("a", "b", 110 + 60 * 86400);
    assert_eq!(g, before);
}
#[test]
fn one_observation_and_nonfinite_activation_do_not_create_links() {
    let mut g = AssociationGraph::default();
    g.learn = true;
    assert!(g
        .reinforce(
            &[
                ("a".into(), "same".into(), 1.0),
                ("b".into(), "same".into(), 1.0)
            ],
            100
        )
        .is_empty());
    assert!(g
        .reinforce(
            &[
                ("a".into(), "one".into(), f32::NAN),
                ("b".into(), "two".into(), 1.0)
            ],
            100
        )
        .is_empty());
    assert!(g.pairs.is_empty());
}
#[test]
fn same_selection_different_reconstruction_and_no_read_only_writes() {
    let mut mem = memory(Attribution::Internal);
    train(&mut mem);
    mem.set_associations(true, false);
    let (plain, dp) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    mem.set_associations(true, true);
    let before = mem.store.associations.encode();
    let frozen: Vec<_> = mem
        .store
        .active_ids()
        .iter()
        .map(|id| mem.store.traces[id].gist.clone())
        .collect();
    let (framed, df) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert_eq!(dp.selected, df.selected);
    assert_eq!(
        dp.candidates
            .iter()
            .map(|c| (&c.trace_id, c.score))
            .collect::<Vec<_>>(),
        df.candidates
            .iter()
            .map(|c| (&c.trace_id, c.score))
            .collect::<Vec<_>>()
    );
    assert!(plain
        .iter()
        .zip(&framed)
        .all(|(a, b)| a.narrative != b.narrative));
    assert_eq!(df.associations_used.len(), 2);
    assert!(df.associations_used.iter().all(|u| u.accepted));
    assert!(df.associations_reinforced.is_empty());
    assert_eq!(before, mem.store.associations.encode());
    assert_eq!(
        frozen,
        mem.store
            .active_ids()
            .iter()
            .map(|id| mem.store.traces[id].gist.clone())
            .collect::<Vec<_>>()
    );
}
#[test]
fn unrelated_or_forced_readout_cannot_train_associations() {
    let mut mem = memory(Attribution::Internal);
    mem.set_associations(true, true);
    let marked = mem.store.active_ids();
    let (_, d) = mem.remember_with(QUERY, RecallWrite::Live, RecallBias::ForceMarked, &marked);
    assert!(d.associations_reinforced.is_empty());
    assert!(mem.store.associations.pairs.is_empty());
    mem.remember("unrelated telescope asteroid orbit");
    assert!(mem.store.associations.pairs.is_empty());
}
#[test]
fn external_observations_are_not_reconstructed_as_own_associations() {
    let mut mem = memory(Attribution::External);
    train(&mut mem);
    mem.set_associations(false, true);
    let (_, d) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(d.associations_used.is_empty());
}
#[test]
fn added_causal_claim_is_rejected_even_with_associations_and_ground_cut() {
    let mut mem = memory(Attribution::Internal);
    train(&mut mem);
    mem.set_associations(false, true);
    let mut mem = mem.with_narrator(Box::new(Framing { unsafe_claim: true }));
    let (out, d) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(out.iter().all(|r| !r.narrative.contains("Jordan")));
    assert_eq!(d.associations_used.len(), 2);
    assert!(d.associations_used.iter().all(|u| !u.accepted));
}
#[test]
fn unknown_associated_proposition_is_not_accepted() {
    let mut mem = memory(Attribution::Internal);
    train(&mut mem);
    mem.set_associations(false, true);
    let mut mem = mem.with_scorer(Box::new(selmem::NullScorer));
    let (out, d) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(out.iter().all(|r| !r.narrative.starts_with("Looking back")));
    assert!(d.associations_used.iter().all(|u| !u.accepted));
}
#[test]
fn both_vaults_preserve_weights_flags_and_episode_deduplication() {
    for suffix in ["selmem", "db"] {
        let path =
            std::env::temp_dir().join(format!("selmem-co-recall-{}.{suffix}", std::process::id()));
        let mut mem = memory(Attribution::Internal);
        train(&mut mem);
        mem.set_associations(true, true);
        mem.path = Some(path.clone());
        let graph = mem.store.associations.encode();
        mem.save().unwrap();
        let mut loaded = SelectiveMemory::open(&path, EntityProfile::new("ignored"))
            .unwrap()
            .with_narrator(Box::new(Framing {
                unsafe_claim: false,
            }))
            .with_scorer(Box::new(Witness));
        assert_eq!(graph, loaded.store.associations.encode());
        let (_, d) = loaded.remember_with(QUERY, RecallWrite::Live, RecallBias::Observed, &[]);
        assert!(d.associations_reinforced.is_empty());
        let id = loaded.store.active_ids()[0].clone();
        loaded.store.release_trace(&id);
        assert!(loaded.store.associations.pairs.is_empty());
        std::fs::remove_file(&path).unwrap();
    }
}
#[test]
fn old_flat_snapshot_loads_with_associations_disabled() {
    let path = std::env::temp_dir().join(format!(
        "selmem-co-recall-old-{}.selmem",
        std::process::id()
    ));
    let mut mem = memory(Attribution::Internal);
    mem.path = Some(path.clone());
    mem.save().unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        raw.lines()
            .filter(|l| !l.starts_with("associations "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    let loaded = SelectiveMemory::open(&path, EntityProfile::new("ignored")).unwrap();
    assert_eq!(loaded.store.associations, AssociationGraph::default());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn codec_rejects_corruption_and_preserves_utf8_ids() {
    let mut g = AssociationGraph::default();
    g.learn = true;
    g.reinforce(
        &[
            ("a é space".into(), "oa".into(), 1.0),
            ("b 雨".into(), "ob".into(), 1.0),
        ],
        100,
    );
    assert_eq!(g, AssociationGraph::decode(&g.encode()).unwrap());
    assert!(AssociationGraph::decode("1 1 1 1 100 100 1 61 62 NaN 1 1 100").is_err());
    assert!(AssociationGraph::decode("1 1 1 1 100 100 1 61 62 0.5 1 1").is_err());
    assert!(AssociationGraph::decode(&(g.encode() + " extra")).is_err());
}
#[test]
fn separated_recalls_with_equal_exposure_do_not_associate() {
    let mut mem = memory(Attribution::Internal);
    mem.set_associations(true, false);
    mem.profile.max_recall = 1;
    let queries: Vec<_> = mem
        .store
        .active_ids()
        .iter()
        .map(|id| mem.store.traces[id].core.clone())
        .collect();
    for _ in 0..12 {
        mem.begin_association_episode();
        let mut seen = std::collections::BTreeSet::new();
        for q in &queries {
            let (_, d) = mem.remember_with(q, RecallWrite::Live, RecallBias::Observed, &[]);
            assert_eq!(d.selected.len(), 1);
            seen.insert(d.selected[0].clone());
        }
        assert_eq!(seen.len(), 2);
    }
    assert!(mem.store.associations.pairs.is_empty());
}
#[test]
fn a_rules_backend_does_not_claim_to_use_associative_context() {
    let mut mem = memory(Attribution::Internal);
    train(&mut mem);
    mem.set_associations(false, true);
    let mut mem = mem.with_narrator(Box::new(selmem::RuleNarrator));
    let (_, d) = mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(d.associations_used.is_empty());
}
#[test]
fn learning_does_not_feed_the_same_recall_and_live_reconstruction_is_audited() {
    let mut mem = memory(Attribution::Internal);
    mem.set_associations(true, true);
    let (_, first) = mem.remember_with(QUERY, RecallWrite::Live, RecallBias::Observed, &[]);
    assert!(first.associations_used.is_empty());
    assert_eq!(first.associations_reinforced.len(), 1);
    train(&mut mem);
    mem.set_associations(false, true);
    let before = mem
        .store
        .traces
        .values()
        .map(|t| t.operations.len())
        .sum::<usize>();
    mem.remember_with(QUERY, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert_eq!(
        before,
        mem.store
            .traces
            .values()
            .map(|t| t.operations.len())
            .sum::<usize>()
    );
    let (out, d) = mem.remember_with(QUERY, RecallWrite::Live, RecallBias::Observed, &[]);
    assert_eq!(d.associations_used.len(), 2);
    for r in out {
        let t = &mem.store.traces[&r.trace_id];
        let op = t
            .operations
            .iter()
            .find(|op| op.kind == "associative-reconstruction")
            .unwrap();
        assert_eq!(op.after, r.narrative);
        assert_eq!(op.before, t.gist);
        assert_eq!(op.source_trace_ids.len(), 1);
        assert!(op.source_axiom_ids.is_empty());
    }
}
#[test]
fn fusion_invalidates_associations_instead_of_inheriting_weights() {
    let mut mem = memory(Attribution::Internal);
    train(&mut mem);
    for t in mem.store.traces.values_mut() {
        t.channel = selmem::core::model::Channel::Selfhood;
        t.schema = Some("test-meeting".into());
        t.anchor = 0.1;
        t.stake_kind = selmem::core::model::StakeKind::None;
        t.bearer = selmem::core::model::Bearer::Self_;
        t.loss_kind = selmem::core::model::LossKind::None;
        t.valence = 0.0;
        t.disgust = 0.0;
        t.stake_mark.clear();
    }
    mem.profile.merge_similarity = -1.0;
    assert_eq!(
        selmem::dream::merge::run(&mut mem.store, &mem.profile, &selmem::HashEmbedder, false),
        1
    );
    assert!(mem.store.associations.pairs.is_empty());
}
#[test]
fn http_reconstruct_receives_separate_fallible_cues_and_target_core() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(e) => panic!("no local HTTP request: {e}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            data.extend_from_slice(&buf[..n]);
            if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&data[..pos]);
                let length: usize = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap();
                if data.len() >= pos + 4 + length {
                    break;
                }
            }
        }
        let body = r#"{"choices":[{"message":{"content":"The team meeting presentation was interrupted."}}]}"#;
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        String::from_utf8(data).unwrap()
    });
    let mem = memory(Attribution::Internal);
    let t = mem.store.traces.values().next().unwrap();
    let http = selmem::HttpNarrator::parse(
        &format!("http://{addr}/v1/chat/completions"),
        "local-test",
        None,
    )
    .unwrap();
    assert!(http.supports_associations());
    let answer = http.reconstruct_associated(
        t,
        &mem.mood,
        QUERY,
        &[AssociativeCue {
            trace_id: "associate".into(),
            gist: "A separately recalled request went unanswered.".into(),
            attribution: Attribution::External,
            weight: 0.7,
        }],
    );
    assert!(answer.contains("presentation"));
    let request = worker.join().unwrap();
    assert!(request.contains("Target core"));
    assert!(request.contains("not your biography"));
    assert!(request.contains("A separately recalled request went unanswered"));
    assert!(request.contains("External"));
    assert!(!request.contains("archive_id"));
}
