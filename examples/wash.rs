//! Wash. Rule path, or three LLM seeds if SELMEM_LLM and SELMEM_API_KEY are set.
//! FailurePolicy::Error. A fallback is an invalid pair, not a wow.

use selmem::{run_wash_seed, LlmSpec, WashArm, WashReport};

fn main() {
    let llm = LlmSpec::from_env();
    let arms = [WashArm::Repeated, WashArm::Once, WashArm::Neutral, WashArm::NoSleep, WashArm::NoLadder];
    let seeds: Vec<u32> = if llm.is_some() { vec![1, 2, 3] } else { vec![1] };
    let mut reports = Vec::new();
    if llm.is_none() {
        println!("no SELMEM_LLM / SELMEM_API_KEY — model pass blocked, rule seed 1 only");
    }
    for seed in seeds {
        for arm in arms {
            let r = run_wash_seed(arm, llm.as_ref(), seed);
            print_arm(&r);
            reports.push(r);
        }
    }
    let path = if llm.is_some() { "experiments/selmem-wash-llm.json" } else { "experiments/selmem-wash.json" };
    let narrator = reports.first().map(|r| r.narrator.as_str()).unwrap_or("rule");
    let body = reports.iter().map(json_arm).collect::<Vec<_>>().join(",\n");
    let _ = std::fs::create_dir_all("experiments");
    std::fs::write(path, format!("{{\n\"narrator\":\"{narrator}\",\n\"arms\":[\n{body}\n]\n}}\n")).expect("write");
    let salient: Vec<_> = reports.iter().filter(|r| r.arm == "repeated").collect();
    let wow_n = salient.iter().filter(|r| r.wow).count();
    let valid_n = salient.iter().filter(|r| r.valid).count();
    println!("salient wow {wow_n}/{} valid {valid_n}/{} wrote {path}", salient.len(), salient.len());
}

fn print_arm(r: &WashReport) {
    println!("=== {} seed={} ({}) valid={} wow={} ===", r.arm, r.seed, r.narrator, r.valid, r.wow);
    println!("  A stance={} spoken_salient={} axiom={} {} {:.2}", r.a.stance, r.a.spoken_salient, r.a.axiom_schema, r.a.axiom_layer, r.a.axiom_strength);
    println!("    {}", r.a.reply);
    println!("  B stance={} spoken_salient={} axiom={} {:.2}", r.b.stance, r.b.spoken_salient, r.b.axiom_schema, r.b.axiom_strength);
    println!("    {}", r.b.reply);
}

fn json_arm(r: &WashReport) -> String {
    format!(
        "{{\"arm\":\"{}\",\"seed\":{},\"valid\":{},\"wow\":{},\"probe_hits_salient\":{},\"a\":{},\"b\":{}}}",
        r.arm, r.seed, r.valid, r.wow, r.probe_hits_salient, mouth(&r.a), mouth(&r.b)
    )
}

fn mouth(m: &selmem::experiment::WashMouth) -> String {
    format!(
        "{{\"stance\":\"{}\",\"spoken_salient\":{},\"axiom_schema\":\"{}\",\"axiom_layer\":\"{}\",\"axiom_strength\":{:.3},\"reply\":\"{}\"}}",
        m.stance, m.spoken_salient, esc(&m.axiom_schema), esc(&m.axiom_layer), m.axiom_strength, esc(&m.reply)
    )
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}
