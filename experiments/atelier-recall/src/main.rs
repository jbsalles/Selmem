mod actor;
mod fixtures;
mod recall;

use fixtures::{history, Workshop, CALIBRATION, HOLDOUT, NIGHTS};
use recall::{Arm, Packet, Replay, ARMS};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, env, fs, path::Path};
use tiktoken_rs::{cl100k_base, CoreBPE};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const BUDGETS: [usize; 3] = [96, 128, 173];
const LOADS: [usize; 3] = [0, 16, 64];
const USAGE: &str = "atelier-recall sweep OUT
atelier-recall plan OUT [--budget 128] [--distractors 0] [--repeats 2]
atelier-recall actor OUT --max-http-calls N [--mock]
atelier-recall score OUT RATINGS.json";

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let file = fs::File::create(&temporary)?;
    serde_json::to_writer_pretty(&file, value)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn fresh_directory(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(path)?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct Cell {
    workshop: String,
    mirror: usize,
    commission: usize,
    brief: String,
    distractors: usize,
    budget: usize,
    arm: Arm,
    packet: Packet,
}

fn collect(
    out: &Path,
    bpe: &CoreBPE,
    workshop: &Workshop,
    distractors: usize,
    budgets: &[usize],
) -> Result<Vec<Cell>> {
    let mut cells = Vec::new();
    for mirror in 0..2 {
        let episodes = history(workshop, mirror, distractors);
        let mut replay = Replay::new(&episodes)?;
        save(
            &out.join(format!(
                "history-{}-{distractors}-{mirror}.json",
                workshop.id
            )),
            &serde_json::json!({
                "medium":workshop.medium, "nights":NIGHTS, "episodes":episodes, "audit":replay.audit,
                "two_complete_trial_reports_tokens":recall::trial_report_tokens(bpe, &episodes),
            }),
        )?;
        for (commission, &brief) in workshop.briefs.iter().enumerate() {
            for (budget, arm, packet) in replay.packets(bpe, brief, budgets) {
                cells.push(Cell {
                    workshop: workshop.id.into(),
                    mirror,
                    commission,
                    brief: brief.into(),
                    distractors,
                    budget,
                    arm,
                    packet,
                });
            }
        }
    }
    Ok(cells)
}

fn sweep(out: &Path) -> Result<()> {
    fresh_directory(out)?;
    let bpe = cl100k_base()?;
    let mut cells = Vec::new();
    for distractors in LOADS {
        cells.extend(collect(out, &bpe, &CALIBRATION, distractors, &BUDGETS)?);
    }
    save(&out.join("coverage.json"), &cells)?;
    let mut summary = String::from(
        "distractors,budget,arm,zero_intact,one_intact,two_intact,paired_two_intact,pairs\n",
    );
    for distractors in LOADS {
        for budget in BUDGETS {
            for arm in ARMS {
                let matching = cells
                    .iter()
                    .filter(|c| (c.distractors, c.budget, c.arm) == (distractors, budget, arm));
                let mut count = [0; 3];
                let mut by_brief = BTreeMap::new();
                for cell in matching {
                    let intact = cell.packet.intact_trials.len();
                    count[intact] += 1;
                    if intact == 2 {
                        *by_brief.entry(cell.commission).or_insert(0) += 1;
                    }
                }
                let paired = by_brief.values().filter(|&&n| n == 2).count();
                let label = if arm == Arm::Selmem {
                    "selmem"
                } else {
                    "bm25_snippets"
                };
                summary.push_str(&format!(
                    "{distractors},{budget},{label},{},{},{},{paired},{}\n",
                    count[0],
                    count[1],
                    count[2],
                    CALIBRATION.briefs.len(),
                ));
            }
        }
    }
    fs::write(out.join("sweep.csv"), &summary)?;
    print!("{summary}");
    Ok(())
}

fn plan(out: &Path, args: &[String]) -> Result<()> {
    let (mut budget, mut distractors, mut repeats) = (128, 0, 2);
    let mut values = args.chunks_exact(2);
    for pair in &mut values {
        let target = match pair[0].as_str() {
            "--budget" => &mut budget,
            "--distractors" => &mut distractors,
            "--repeats" => &mut repeats,
            _ => return Err(USAGE.into()),
        };
        *target = pair[1].parse()?;
    }
    if !values.remainder().is_empty() {
        return Err(USAGE.into());
    }
    if budget == 0 || repeats == 0 || distractors > 64 {
        return Err("budget and repeats must be positive, and distractors must be 0..=64".into());
    }
    fresh_directory(out)?;
    let bpe = cl100k_base()?;
    let mut cells = Vec::new();
    for workshop in &HOLDOUT {
        cells.extend(collect(out, &bpe, workshop, distractors, &[budget])?);
    }
    actor::prepare(out, cells, repeats)
}

fn run() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => println!("{USAGE}"),
        [help] if help == "--help" || help == "-h" => println!("{USAGE}"),
        [stage, out] if stage == "sweep" => return sweep(Path::new(out)),
        [stage, out, options @ ..] if stage == "plan" => return plan(Path::new(out), options),
        [stage, out, flag, limit, mode @ ..]
            if stage == "actor"
                && flag == "--max-http-calls"
                && (mode.is_empty() || (mode.len() == 1 && mode[0] == "--mock")) =>
        {
            return actor::run(Path::new(out), limit.parse()?, !mode.is_empty());
        }
        [stage, out, ratings] if stage == "score" => {
            return actor::score(Path::new(out), Path::new(ratings));
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("atelier-recall: {error}");
        std::process::exit(1);
    }
}
