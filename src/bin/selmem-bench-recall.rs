//! Deterministic, read-only ranking benchmark with independent axiom support.
//! This exact file also builds against the pre-refactor baseline.
use selmem::core::model::{Bearer, IdentityAxiom, LossKind, StakeKind};
use selmem::{
    Attribution, AxiomLayer, EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory,
};
use std::time::Instant;

fn fingerprint(memory: &SelectiveMemory, ids: &[String]) -> u64 {
    let mut digest = 0xcbf29ce484222325_u64;
    for id in ids {
        let core = &memory.store.traces[id].core;
        for byte in core.bytes().chain(std::iter::once(0)) {
            digest = (digest ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    digest
}

fn scores_fingerprint(memory: &SelectiveMemory, dump: &selmem::RetrievalDump) -> u64 {
    let mut digest = 0xcbf29ce484222325_u64;
    for scored in &dump.candidates {
        for byte in memory.store.traces[&scored.trace_id]
            .core
            .bytes()
            .chain(scored.score.to_bits().to_le_bytes())
            .chain(scored.base_score.to_bits().to_le_bytes())
            .chain(scored.anchor.to_bits().to_le_bytes())
        {
            digest = (digest ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    digest
}

fn measure(size: usize, repeats: usize) {
    let mut profile = EntityProfile::new("benchmark");
    profile.encode_threshold = 0.0;
    let mut memory = SelectiveMemory::new(profile).detach_clock();
    memory.clock.origin_real = 4_102_444_800;
    memory.clock.scale = 1;
    let mut ids = Vec::with_capacity(size);
    for i in 0..size {
        let sentence = format!(
            "During the team meeting I completed project architecture review milestone {i} for group {}.",
            i % 17
        );
        let observation_id = format!("benchmark-{i}");
        let mut input = EncodeInput::observed(&sentence, &observation_id);
        input.utility = 0.9;
        input.permanence = 0.9;
        input.self_relevance = 0.9;
        input.arousal = 0.7;
        input.attribution = Attribution::Internal;
        let result = memory.live_with(input);
        assert!(result.kept, "event {i} was not admitted");
        ids.push(result.trace_id.expect("kept event has a trace ID"));
    }
    // Exercise the expensive axiom-support path: every axiom must have two
    // independent observations. A large number of axioms makes the old nested
    // scan expensive; both versions must still produce identical answers.
    for (i, pair) in ids.chunks_exact(2).enumerate() {
        memory.store.axioms.insert(
            format!("bench-axiom-{i}"),
            IdentityAxiom {
                id: format!("bench-axiom-{i}"),
                statement: "Project reviews are dependable".into(),
                support_trace_ids: pair.to_vec(),
                valence: 0.5,
                strength: 0.8,
                created_at: 4_102_444_800,
                superseded_by: None,
                schema: Some("project".into()),
                layer: AxiomLayer::Trait,
                stake_kind: StakeKind::Decision,
                bearer: Bearer::Self_,
                loss_kind: LossKind::None,
                stake_mark: "project".into(),
            },
        );
    }
    let query = "team meeting project architecture review";
    let (warm_memories, warmup) =
        memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    assert!(
        !warm_memories.is_empty() && !warmup.selected.is_empty(),
        "benchmark recall returned nothing"
    );
    let expected = warmup.selected.clone();
    let digest = fingerprint(&memory, &expected);
    let score_digest = scores_fingerprint(&memory, &warmup);
    let mut latencies = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let start = Instant::now();
        let (_, dump) =
            memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(dump.selected, expected, "selected trace IDs changed");
        assert_eq!(
            fingerprint(&memory, &dump.selected),
            digest,
            "selected cores changed"
        );
        assert_eq!(
            scores_fingerprint(&memory, &dump),
            score_digest,
            "ranked scores changed"
        );
        latencies.push(elapsed);
    }
    latencies.sort_by(f64::total_cmp);
    let p50 = latencies[repeats / 2];
    let p95 = latencies[((repeats * 95) / 100).min(repeats - 1)];
    println!(
        "{size},{},{},{repeats},{digest:016x},{score_digest:016x},{p50:.3},{p95:.3}",
        memory.store.traces.len(),
        memory.store.axioms.len()
    );
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sizes: Vec<usize> = if args.is_empty() {
        vec![100, 500, 1_000]
    } else {
        args.iter()
            .map(|arg| {
                let n = arg
                    .parse::<usize>()
                    .expect("memory size must be an integer");
                assert!(
                    (2..=10_000).contains(&n) && n % 2 == 0,
                    "size must be even and 2..=10000"
                );
                n
            })
            .collect()
    };
    println!(
        "requested_events,stored_traces,axioms,repeats,selection_digest,score_digest,p50_ms,p95_ms"
    );
    for size in sizes {
        measure(size, 11);
    }
}
