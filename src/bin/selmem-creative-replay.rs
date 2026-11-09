//! Controlled, fixed-selection replay of a completed GENESIS preview.
//! This binary adds an experimental guard; it does not modify the organ.
use selmem::util::json;
use selmem::util::sha256;
#[path = "selmem-creative-replay/collect.rs"]
mod collect;
#[path = "selmem-creative-replay/rate.rs"]
mod rate;
use json::{num, obj, text, Json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};
include!("shared/creative_replay_common.rs");

fn main() {
    if let Err(e) = cli(std::env::args().skip(1).collect()) {
        eprintln!("CREATIVE REPLAY: {e}");
        std::process::exit(1);
    }
}
fn cli(mut args: Vec<String>) -> Result<()> {
    if args.is_empty() || args.iter().any(|a| a == "--help") {
        println!("selmem-creative-replay collect --source COMPLETED_PREVIEW --out FRESH_DIR --config .selmem --llm grok --model grok-4.3 [--max-http-calls 250] [--resume] [--prepare-only] [--mock]\nselmem-creative-replay template RUN FRESH_RATINGS.json\nselmem-creative-replay combine FRESH_COMBINED.json JUDGE_A.json JUDGE_B.json\nselmem-creative-replay analyze RUN COMBINED_RATINGS.json");
        return Ok(());
    }
    let command = args.remove(0);
    match command.as_str() {
        "collect" => collect::cli(args),
        "template" => {
            require(args.len() == 2, "template RUN FRESH_OUTPUT")?;
            rate::template(Path::new(&args[0]), Path::new(&args[1]))
        }
        "combine" => {
            require(
                args.len() >= 3,
                "combine FRESH_OUTPUT INPUT_A INPUT_B [INPUT...]",
            )?;
            let mut all = vec![];
            for p in &args[1..] {
                all.extend(load(Path::new(p))?.array()?.to_vec());
            }
            fresh(Path::new(&args[0]), &arr(all))
        }
        "analyze" => {
            require(args.len() == 2, "analyze RUN RATINGS")?;
            let dir = Path::new(&args[0]);
            let report = rate::analyze(dir, &load(Path::new(&args[1]))?)?;
            let p = dir.join("replay-ratings.json");
            fresh(&p, &report)?;
            println!("{}", p.display());
            Ok(())
        }
        _ => Err(format!("unknown command: {command}")),
    }
}
