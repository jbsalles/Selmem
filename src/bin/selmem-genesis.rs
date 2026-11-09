//! Curated GENESIS qualification and cumulative-learning experiments.
#[path = "selmem-genesis/http.rs"]
mod http;
use selmem::util::json;
use selmem::util::sha256;
#[path = "selmem-genesis/creative_life.rs"]
mod creative_life;
#[path = "selmem-genesis/creative_preview.rs"]
mod creative_preview;
#[path = "selmem-genesis/creativity.rs"]
mod creativity;
#[path = "selmem-genesis/learning.rs"]
mod learning;
#[cfg(test)]
#[path = "selmem-genesis/lifecycle_tests.rs"]
mod lifecycle_tests;
#[path = "selmem-genesis/native.rs"]
mod native;
#[path = "selmem-genesis/positive_control.rs"]
mod positive_control;
#[path = "selmem-genesis/preview_guard.rs"]
mod preview_guard;
#[path = "selmem-genesis/progress.rs"]
mod progress;
#[path = "selmem-genesis/runner.rs"]
mod runner;
#[path = "selmem-genesis/self_bridge.rs"]
mod self_bridge;
#[path = "selmem-genesis/self_emergence.rs"]
mod self_emergence;
#[path = "selmem-genesis/self_pilot.rs"]
mod self_pilot;
#[path = "selmem-genesis/self_preference.rs"]
mod self_preference;
#[path = "selmem-genesis/sleep_quality.rs"]
mod sleep_quality;
#[path = "selmem-genesis/sleep_resume.rs"]
mod sleep_resume;
#[cfg(test)]
#[path = "selmem-genesis/tests.rs"]
mod tests;
#[path = "selmem-genesis/tooling/mod.rs"]
mod tooling;
#[path = "selmem-genesis/transport.rs"]
mod transport;
fn cli(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args.iter().any(|a| a == "--help") {
        println!("GENESIS: self-emergence | self-preference | self-bridge | self-pilot-v3 | self-pilot-v2 | self-pilot | positive | learning | sleep | creative | creative-causal | creative-life | creative-life-causal | creative-preview\n./run.sh run --release --bin selmem-genesis -- positive --config .selmem --llm grok --model grok-4.3 --out DIR --max-http-calls 60\n./run.sh run --release --bin selmem-genesis -- learning --config .selmem --llm grok --model grok-4.3 --source QUALIFIED_DIR --out DIR --observation-evidence on|off --max-http-calls 800\nsleep --source EVIDENCE_ON_DIR --locomo FIXTURE.json --out DIR --config .selmem --llm grok --max-http-calls 3000 [--resume-from DIR] [--prepare-only]\ncreative --locomo FIXTURE.json --out DIR [--mock] [--prepare-only] [--repeats 3] [--context-words 600] [--max-http-calls 3000]\ncreative-causal --source CREATIVE_DIR --plan PLAN.json --out DIR [--mock]\ncreative-preview --fixture FIXTURE.json --out DIR [--guarded] [--resume] [--prepare-only] [--mock]\nself-pilot-v3 --out DIR [--mock|--prepare-only|--bootstrap-only] [--llm PROVIDER --model MODEL --max-http-calls 256]\nself-pilot-v2 --out DIR [--mock|--prepare-only] [--llm PROVIDER --model MODEL --max-http-calls 256]\nself-pilot --out DIR [--mock|--prepare-only] [--llm PROVIDER --model MODEL --max-http-calls 256]\nself-preference --out DIR [--mock|--prepare-only] [--llm PROVIDER --model MODEL --max-http-calls 36]\nOffline tools: tools prepare-creative|prepare-preview|prepare-life|prepare-sleep|rate-creative|rate-preview|rate-life|code-mechanisms|combine-ratings|life-profile\nAll collection commands support --mock. Mock runs are technical rehearsals. Outputs must be fresh.");
        return Ok(());
    }
    let command = args.remove(0);
    if command == "tools" {
        return tooling::cli(args);
    }
    if ![
        "self-emergence",
        "self-preference",
        "self-bridge",
        "self-pilot-v3",
        "self-pilot-v2",
        "self-pilot",
        "positive",
        "learning",
        "sleep",
        "creative",
        "creative-causal",
        "creative-life",
        "creative-life-causal",
        "creative-preview",
    ]
    .contains(&command.as_str())
    {
        return Err(format!("retired or unknown command {command}; use --help"));
    }
    if let Some(i) = args.iter().position(|a| a == "--config") {
        let path = args.get(i + 1).ok_or("--config requires a path")?;
        selmem::Config::load_path(std::path::Path::new(path))
            .map_err(|e| format!("runtime config: {e}"))?;
    }
    match command.as_str() {
        "self-emergence" => self_emergence::cli(args),
        "self-preference" => self_preference::cli(args),
        "self-bridge" => self_bridge::cli(args),
        "self-pilot-v3" => self_pilot::cli_v3(args),
        "self-pilot-v2" => self_pilot::cli_v2(args),
        "self-pilot" => self_pilot::cli(args),
        "creative-preview" => creative_preview::cli(args),
        "creative-life" => creative_life::cli(args),
        "creative-life-causal" => creative_life::causal_cli(args),
        "positive" => positive_control::cli(args),
        "learning" => learning::cli(args),
        "sleep" => sleep_quality::cli(args),
        "creative" => creativity::cli(args, false),
        "creative-causal" => creativity::cli(args, true),
        _ => unreachable!(),
    }
}
fn main() {
    if let Err(e) = cli(std::env::args().skip(1).collect()) {
        progress::line(format!("ERROR {e}"));
        eprintln!("GENESIS: {e}");
        std::process::exit(1);
    }
}
