//! Persist P0 / P1: C1 k=8 vs C3 vs C2 vs freeze vs one-cut ablations.
//! Stimulus: 12 dull days, five same-schema hours, 8 posts. `data/v01_persist.json`.
//!
//!   ./run.sh run --release --example persist -- --pairs 5 --seed 1 --last-k 8 --out selmem-persist-p1-grok-n5.json
//!   ./run.sh run --release --example persist -- --bias drop --pairs 1 --out selmem-persist-p1-drop.json
//!   ./run.sh run --release --example persist -- --p2 --pairs 1 --last-k 8 --out selmem-persist-p2.json
//!   ./run.sh run --release --example persist -- --verbose --pairs 1 --last-k 8
//!
//! Default arm is salient/neutral. Sparse probes: t0 + post+8 only. No creativity.
//! `--bias observed|force|drop|lineage`. Probes are read-only. JSON has book / rank / mouth.
//! `lineage` also drops same-schema siblings and axioms whose support traces descend from T0.

use selmem::{
    h2_holds, marker_holds, print_banner, print_pair_verbose, run_v01_opts, Arm, BenchOpts,
    Campaign, Condition, LlmSpec, PairReport, RecallBias,
};

fn main() {
    let pairs = arg_u32("--pairs").unwrap_or(1).max(1);
    let seed = arg_u32("--seed").unwrap_or(1);
    let last_k = arg_u32("--last-k").unwrap_or(8).max(1) as usize;
    let out = arg_str("--out").unwrap_or_else(|| "selmem-persist.json".into());
    let both_arms = flag("--both-arms");
    let ruminate = flag("--ruminate");
    let hearth = flag("--hearth");
    let axioms_only = flag("--axioms-only") || flag("--axioms");
    let verbose = flag("--verbose") || flag("-v") || std::env::var_os("SELMEM_VERBOSE").is_some();
    let grid_p2 = flag("--p2") || arg_str("--grid").as_deref() == Some("p2");
    let util = flag("--util-strength") || flag("--util");
    let veto = flag("--merge-veto") || flag("--veto");
    let bias = match arg_str("--bias").as_deref() {
        Some("force") | Some("ForceMarked") => RecallBias::ForceMarked,
        Some("drop") | Some("DropMarked") => RecallBias::DropMarked,
        Some("lineage") | Some("drop-lineage") | Some("DropLineage") | Some("family") => {
            RecallBias::DropLineage
        }
        _ => RecallBias::Observed,
    };

    let llm = LlmSpec::from_env();
    if verbose {
        let who = llm
            .as_ref()
            .map(|s| format!("{}  {}", s.url, s.model))
            .unwrap_or_else(|| "RuleNarrator".into());
        print_banner(
            "persist",
            &format!("{who}  pairs={pairs}  seed={seed}  last_k={last_k}"),
        );
    } else if let Some(s) = llm.as_ref() {
        println!(
            "LLM {} model={} pairs={} seed={} last_k={} persist",
            s.url, s.model, pairs, seed, last_k
        );
    } else {
        println!("RuleNarrator (no llm) pairs={pairs} last_k={last_k} persist");
    }

    let base = BenchOpts {
        last_k,
        persist_script: !ruminate && !hearth,
        ruminate_script: ruminate && !hearth,
        hearth_script: hearth,
        axioms_only,
        sparse_probes: true,
        recall_bias: bias,
        util_to_strength: util,
        merge_support_veto: veto,
        ..selmem::BenchOpts::default()
    };
    if ruminate {
        println!("script=ruminate (same meeting ×5, pinned)");
    }
    if hearth {
        println!("script=hearth (leaving ×2, sleep, return ×2)");
    }
    if axioms_only {
        println!("mouth=axioms-only");
    }
    println!(
        "bias={}",
        match bias {
            RecallBias::Observed => "observed",
            RecallBias::ForceMarked => "force",
            RecallBias::DropMarked => "drop",
            RecallBias::DropLineage => "lineage",
        }
    );
    let arms: &[Arm] = if both_arms {
        &[Arm::SalientNeutral, Arm::SalientSalient]
    } else {
        &[Arm::SalientNeutral]
    };
    let cells: Vec<(Condition, BenchOpts)> = if grid_p2 {
        println!("grid=p2 util/veto cells + ruminate+veto");
        p2_cells(last_k, bias, hearth, axioms_only)
    } else {
        Condition::p0_grid()
            .into_iter()
            .map(|c| (c, base))
            .collect()
    };

    let mut reports = Vec::new();
    for i in 1..=pairs {
        for (cond, opts) in &cells {
            for arm in arms {
                if !verbose {
                    println!(
                        "--- pair {i}/{pairs} {}{} {} ---",
                        cond.as_str(),
                        opts.cut_tag(),
                        arm.as_str()
                    );
                }
                let mut r = run_v01_opts(*cond, *arm, llm.as_ref(), *opts);
                r.pair_id = format!(
                    "{}{}_{}_{:03}",
                    cond.as_str(),
                    opts.cut_tag(),
                    arm.as_str(),
                    i
                );
                r.seed = seed;
                if verbose {
                    print_pair_verbose(&r, i, pairs);
                } else {
                    row(&r);
                    classify(&r);
                }
                reports.push(r);
                flush(&out, &reports);
            }
        }
    }
    println!("wrote {out} ({} rows)", reports.len());
}

fn flush(path: &str, reports: &[PairReport]) {
    let json = Campaign {
        reports: reports.to_vec(),
    }
    .to_json();
    if let Err(e) = std::fs::write(path, json) {
        eprintln!("write {path}: {e}");
    }
}

fn row(r: &PairReport) {
    let last = r.post.last().unwrap_or(&r.t0);
    println!(
        "{} valid={} persist={} marker={}  pre_fp={:.3} t0_fp={:.3} last_fp={:.3} Δfp={:+.3}  traces {}/{}→{}/{} axioms {}/{}  book {}/{} rank {:?}/{:?} sel {}/{}",
        r.pair_id,
        r.valid,
        h2_holds(r),
        marker_holds(r),
        r.pre.fingerprint_distance,
        r.t0.fingerprint_distance,
        last.fingerprint_distance,
        r.delta_fingerprint,
        r.pre.a.traces,
        r.pre.b.traces,
        last.a.traces,
        last.b.traces,
        last.a.axioms,
        last.b.axioms,
        last.retrieve_a.t0_in_book,
        last.retrieve_b.t0_in_book,
        last.retrieve_a.t0_rank,
        last.retrieve_b.t0_rank,
        last.retrieve_a.t0_selected,
        last.retrieve_b.t0_selected
    );
    println!(
        "  strength {:.2}/{:.2} refused {}/{} t0_reh {}/{}",
        last.a.axiom_strength_max,
        last.b.axiom_strength_max,
        last.a.merges_refused,
        last.b.merges_refused,
        last.a.t0_rehearsals,
        last.b.t0_rehearsals
    );
    if let Some(why) = &r.invalid_reason {
        println!("  invalid: {why}");
    }
}

fn classify(r: &PairReport) {
    let last = match r.post.last() {
        Some(p) => p,
        None => return,
    };
    for (q, a, b) in &last.replies {
        let short = if q.len() > 48 { &q[..48] } else { q };
        println!(
            "  [{}] A={}  B={}",
            short,
            bucket(a),
            bucket(b)
        );
    }
}

fn p2_cells(
    last_k: usize,
    bias: RecallBias,
    hearth: bool,
    axioms_only: bool,
) -> Vec<(Condition, BenchOpts)> {
    let persist = |util, veto| BenchOpts {
        last_k,
        persist_script: !hearth,
        ruminate_script: false,
        hearth_script: hearth,
        axioms_only,
        sparse_probes: true,
        recall_bias: bias,
        util_to_strength: util,
        merge_support_veto: veto,
        ..BenchOpts::default()
    };
    let mut cells = vec![
        (Condition::C2, persist(false, false)),
        (Condition::C2, persist(true, false)),
        (Condition::C2, persist(false, true)),
        (Condition::C2, persist(true, true)),
        (Condition::C2NoLadder, persist(true, false)),
    ];
    if !hearth {
        cells.push((
            Condition::C2,
            BenchOpts {
                last_k,
                persist_script: false,
                ruminate_script: true,
                sparse_probes: true,
                recall_bias: bias,
                merge_support_veto: true,
                ..BenchOpts::default()
            },
        ));
    }
    cells
}

fn bucket(s: &str) -> &'static str {
    let low = s.to_lowercase();
    let recall = [
        "injust",
        "unjust",
        "annul",
        "cancel",
        "effort",
        "projet",
        "project you spent",
        "administrative notice",
        "avis admin",
    ];
    if recall.iter().any(|k| low.contains(k)) {
        return "recall";
    }
    let bias = [
        "mefi",
        "méfi",
        "distrust",
        "suspicion",
        "suspicious",
        "test",
        "récidiv",
        "recidiv",
        "again",
        "pattern",
        "motif",
        "late once",
        "en retard",
        "discard",
        "already gave",
        "sour",
        "wary",
        "guarded",
        "accountability",
    ];
    if bias.iter().any(|k| low.contains(k)) {
        return "bias";
    }
    "neutral"
}

fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

fn arg_str(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
        if let Some(v) = a.strip_prefix(&format!("{flag}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn arg_u32(flag: &str) -> Option<u32> {
    arg_str(flag)?.parse().ok()
}
