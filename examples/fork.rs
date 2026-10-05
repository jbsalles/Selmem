//! Stage 0 fork. Rule path, or five LLM seeds if a mouth is configured.
//! FailurePolicy::Error. A fallback is an invalid pair, not a pass.

use selmem::{run_fork, stage0_pass, ForkArm, ForkOrgan, ForkReport, LlmSpec};

fn main() {
    let llm = LlmSpec::from_env();
    let arms = [ForkArm::Fork, ForkArm::Fork3, ForkArm::Null, ForkArm::Leak];
    let organs = [
        ForkOrgan::Night,
        ForkOrgan::NoSleep,
        ForkOrgan::NoLadder,
        ForkOrgan::Last8,
        ForkOrgan::FullLog,
    ];
    let seeds: Vec<u32> = if llm.is_some() { vec![1, 2, 3, 4, 5] } else { vec![1] };
    if llm.is_none() {
        println!("no llm in .selmem / SELMEM_LLM — model pass blocked, rule seed 1 only");
    }
    let mut reports = Vec::new();
    for seed in seeds {
        for arm in arms {
            for organ in organs {
                let r = run_fork(arm, organ, llm.as_ref(), seed);
                print_cell(&r);
                reports.push(r);
            }
        }
    }
    let path = if llm.is_some() {
        "experiments/selmem-fork-llm.json"
    } else {
        "experiments/selmem-fork.json"
    };
    let narrator = reports.first().map(|r| r.narrator.as_str()).unwrap_or("rule");
    let body = reports.iter().map(json_cell).collect::<Vec<_>>().join(",\n");
    let _ = std::fs::create_dir_all("experiments");
    std::fs::write(path, format!("{{\n\"narrator\":\"{narrator}\",\n\"cells\":[\n{body}\n]\n}}\n")).expect("write");
    let seed1: Vec<_> = reports.iter().filter(|r| r.seed == 1).collect();
    let grab = |arm: &str, organ: &str| {
        seed1.iter().find(|r| r.arm == arm && r.organ == organ).copied()
    };
    let fork = grab("fork", "night");
    let null = grab("null", "night");
    let last8 = grab("fork", "last8");
    let full = grab("fork", "fulllog");
    let passed = match (fork, null, last8, full) {
        (Some(f), Some(n), Some(k), Some(g)) => {
            stage0_pass(f.d_beh, n.d_beh, k.d_beh, f.recited, f.recall, g.recall)
        }
        _ => false,
    };
    let fork3 = grab("fork3", "night");
    let last8_3 = grab("fork3", "last8");
    let full3 = grab("fork3", "fulllog");
    let passed3 = match (fork3, null, last8_3, full3) {
        (Some(f), Some(n), Some(k), Some(g)) => {
            stage0_pass(f.d_beh, n.d_beh, k.d_beh, f.recited, f.recall, g.recall)
        }
        _ => false,
    };
    println!(
        "stage0 seed 1 pass={passed} fork_beh={:.2} fork3_beh={:.2} fork3_pass={passed3} null_beh={:.2} last8_beh={:.2} wrote {path}",
        fork.map(|r| r.d_beh).unwrap_or(0.0),
        fork3.map(|r| r.d_beh).unwrap_or(0.0),
        null.map(|r| r.d_beh).unwrap_or(0.0),
        last8.map(|r| r.d_beh).unwrap_or(0.0),
    );
}

fn print_cell(r: &ForkReport) {
    println!(
        "=== {} {} seed={} ({}) valid={} split={} d_beh={:.2} recited={} recall={:.2} named={}/{} ===",
        r.arm, r.organ, r.seed, r.narrator, r.valid, r.book_split, r.d_beh, r.recited, r.recall, r.a_named, r.b_named
    );
    println!(
        "  book A {} {:+.2} axiom={:.2} {}",
        r.a_schema, r.a_valence, r.a_axiom_strength, r.a_axiom
    );
    println!(
        "  book B {} {:+.2} axiom={:.2} {}",
        r.b_schema, r.b_valence, r.b_axiom_strength, r.b_axiom
    );
    for (a, b) in &r.choice {
        println!("  A {} recited={} {}", a.stance, a.recited, a.reply);
        println!("  B {} recited={} {}", b.stance, b.recited, b.reply);
    }
}

fn json_cell(r: &ForkReport) -> String {
    format!(
        "{{\"arm\":\"{}\",\"organ\":\"{}\",\"seed\":{},\"valid\":{},\"book_split\":{},\"d_beh\":{:.3},\"recited\":{},\"recall\":{:.3},\"a_named\":{},\"b_named\":{},\"a_valence\":{:.3},\"b_valence\":{:.3},\"a_axiom\":\"{}\",\"b_axiom\":\"{}\"}}",
        r.arm,
        r.organ,
        r.seed,
        r.valid,
        r.book_split,
        r.d_beh,
        r.recited,
        r.recall,
        r.a_named,
        r.b_named,
        r.a_valence,
        r.b_valence,
        esc(&r.a_axiom),
        esc(&r.b_axiom)
    )
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
