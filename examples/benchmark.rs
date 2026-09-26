//! Benchmark v0.1. Writes JSON after every pair so a crash does not wipe the run.
//!
//!   ./run.sh run --release --example benchmark -- --pairs 10 --out selmem-v01-n10.json
//!
//! LLM: llm= in `.selmem` or SELMEM_LLM. Without it this is RuleNarrator and
//! ten pairs are identical.

use selmem::{
    h2_holds, print_banner, print_pair_verbose, run_v01_k, Arm, Campaign, Condition, LlmSpec,
    PairReport,
};

fn main() {
    let pairs = arg_u32("--pairs").unwrap_or(1).max(1);
    let seed = arg_u32("--seed").unwrap_or(1);
    let last_k = arg_u32("--last-k").unwrap_or(24).max(1) as usize;
    let out = arg_str("--out").unwrap_or_else(|| "selmem-v01.json".into());
    let p0 = flag("--p0");
    let verbose = flag("--verbose") || flag("-v") || std::env::var_os("SELMEM_VERBOSE").is_some();

    let llm = LlmSpec::from_env();
    if verbose {
        let who = llm
            .as_ref()
            .map(|s| format!("{}  {}", s.url, s.model))
            .unwrap_or_else(|| "RuleNarrator".into());
        print_banner(
            "v0.1",
            &format!("{who}  pairs={pairs}  seed={seed}  last_k={last_k}"),
        );
    } else if let Some(s) = llm.as_ref() {
        println!("LLM {} model={} pairs={} seed={} last_k={}", s.url, s.model, pairs, seed, last_k);
    } else {
        println!("RuleNarrator (no llm) pairs={} — C2 fingerprints will repeat", pairs);
    }

    let mut reports = Vec::new();
    for i in 1..=pairs {
        let p0_conds = Condition::p0_grid();
        let v01_conds = Condition::v01_grid();
        let conds: &[Condition] = if p0 { &p0_conds } else { &v01_conds };
        for cond in conds.iter().copied() {
            for arm in [Arm::SalientNeutral, Arm::SalientSalient] {
                if !verbose {
                    println!("--- pair {i}/{pairs} {} {} ---", cond.as_str(), arm.as_str());
                }
                let mut r = run_v01_k(cond, arm, llm.as_ref(), last_k);
                r.pair_id = format!("{}_{}_{:03}", cond.as_str(), arm.as_str(), i);
                r.seed = seed;
                if verbose {
                    print_pair_verbose(&r, i, pairs);
                } else {
                    row(&r);
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
        "{} valid={} persist={}  pre_fp={:.3} t0_fp={:.3} last_fp={:.3} Δfp={:+.3}  traces {}/{}→{}/{}",
        r.pair_id,
        r.valid,
        h2_holds(r),
        r.pre.fingerprint_distance,
        r.t0.fingerprint_distance,
        last.fingerprint_distance,
        r.delta_fingerprint,
        r.pre.a.traces,
        r.pre.b.traces,
        last.a.traces,
        last.b.traces
    );
    if let Some(why) = &r.invalid_reason {
        println!("  invalid: {why}");
    }
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

fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}
