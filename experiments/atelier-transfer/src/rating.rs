use crate::{read, save, Arm, Device, Job, Plan, Reply, Result, Run};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

pub fn export_blind(out: &Path, plan: &Plan, replies: &BTreeMap<&str, Reply>) -> Result<()> {
    let path = out.join("blind.json");
    if path.exists() {
        return Ok(());
    }
    let rows = plan
        .jobs
        .iter()
        .map(|job| {
            json!({
                "id": job.id,
                "brief": plan.briefs[job.brief],
                "proposal": replies[job.id.as_str()].answer.proposal,
                "originality": null,
                "fit": null,
                "device": null
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        out.join("blind-rubric.md"),
        include_str!("../blind-rubric.md"),
    )?;
    save(&path, &rows)
}

#[derive(Default, Serialize)]
struct Devices {
    interruption: usize,
    concentration: usize,
    both: usize,
    other: usize,
}

impl Devices {
    fn add(&mut self, device: Device) {
        let count = match device {
            Device::Interruption => &mut self.interruption,
            Device::Concentration => &mut self.concentration,
            Device::Both => &mut self.both,
            Device::Other => &mut self.other,
        };
        *count += 1;
    }
}

pub fn export_calibration(
    out: &Path,
    run: &Run,
    plan: &Plan,
    replies: &BTreeMap<&str, Reply>,
) -> Result<()> {
    let mut briefs = Vec::new();
    for (brief, commission) in plan.briefs.iter().enumerate() {
        let mut declared = Devices::default();
        for job in plan
            .jobs
            .iter()
            .filter(|job| job.brief == brief && job.arm == Arm::Empty)
        {
            let device = replies[job.id.as_str()]
                .answer
                .device
                .ok_or_else(|| format!("Missing declared device for {}", job.id))?;
            declared.add(device);
        }
        briefs.push(json!({
            "commission": commission,
            "samples": 2 * run.repeats,
            "declared": declared
        }));
    }
    save(
        &out.join("calibration.json"),
        &json!({
            "protocol": run.protocol, "mock": run.mock, "model": plan.model, "briefs": briefs
        }),
    )
}

#[derive(Deserialize, Serialize)]
struct Rating {
    id: String,
    originality: u8,
    fit: u8,
    device: Device,
}

fn grades(path: &Path, plan: &Plan) -> Result<BTreeMap<String, Rating>> {
    let mut ratings = BTreeMap::new();
    for row in read::<Vec<Rating>>(path)? {
        if !(1..=5).contains(&row.originality) || !(1..=5).contains(&row.fit) {
            return Err(format!("rating {} in {} is outside 1..=5", row.id, path.display()).into());
        }
        if ratings.insert(row.id.clone(), row).is_some() {
            return Err(format!("duplicate proposal ID in {}", path.display()).into());
        }
    }
    if ratings.len() != plan.jobs.len() || plan.jobs.iter().any(|j| !ratings.contains_key(&j.id)) {
        return Err(format!(
            "{} must rate each planned proposal exactly once",
            path.display()
        )
        .into());
    }
    Ok(ratings)
}

type Cell = (Arm, usize, usize, usize);
type Cells<'a> = BTreeMap<Cell, &'a Rating>;

fn cell(job: &Job) -> Cell {
    (job.arm, job.history, job.brief, job.repeat)
}

fn device_summary(run: &Run, plan: &Plan, arm: Arm, choice: impl Fn(Cell) -> Device) -> Value {
    let mut congruent = 0;
    let mut both_or_other = 0;
    let mut consistent_switches = 0;
    let mut reverse_switches = 0;
    let mut by_history: [Devices; 2] = Default::default();
    for brief in 0..plan.briefs.len() {
        for repeat in 0..run.repeats {
            let left = choice((arm, 0, brief, repeat));
            let right = choice((arm, 1, brief, repeat));
            for (history, device) in [left, right].into_iter().enumerate() {
                by_history[history].add(device);
                congruent += usize::from(device == run.histories[history].outcome);
                both_or_other += usize::from(matches!(device, Device::Both | Device::Other));
            }
            consistent_switches +=
                usize::from(left == run.histories[0].outcome && right == run.histories[1].outcome);
            reverse_switches +=
                usize::from(left == run.histories[1].outcome && right == run.histories[0].outcome);
        }
    }
    let n = plan.briefs.len() * run.repeats;
    let concentration_shift =
        (by_history[1].concentration as f64 - by_history[0].concentration as f64) / n as f64;
    let interruption_shift =
        (by_history[0].interruption as f64 - by_history[1].interruption as f64) / n as f64;
    json!({
        "arm": arm,
        "count": 2 * n,
        "history_concordant": congruent,
        "both_or_other": both_or_other,
        "matched_history_pairs": n,
        "switches_following_outcome": consistent_switches,
        "switches_against_outcome": reverse_switches,
        "device_counts_by_history": by_history,
        "concentration_rate_shift": concentration_shift,
        "interruption_rate_shift": interruption_shift
    })
}

fn quality_summary(run: &Run, plan: &Plan, cells: &Cells<'_>, arm: Arm) -> Value {
    let mut summary = device_summary(run, plan, arm, |key| cells[&key].device);
    let mut originality = 0.;
    let mut fit = 0.;
    for job in plan.jobs.iter().filter(|job| job.arm == arm) {
        let grade = cells[&cell(job)];
        originality += f64::from(grade.originality);
        fit += f64::from(grade.fit);
    }
    let n = 2 * plan.briefs.len() * run.repeats;
    summary["mean_originality"] = (originality / n as f64).into();
    summary["mean_fit"] = (fit / n as f64).into();
    summary
}

fn contrasts(run: &Run, plan: &Plan, cells: &Cells<'_>, treatment: Arm, baseline: Arm) -> Value {
    let mut originality = 0;
    let mut fit = 0;
    let mut originality_win_and_fit_held = 0;
    for job in plan.jobs.iter().filter(|job| job.arm == treatment) {
        let treated = cells[&cell(job)];
        let other = cells[&(baseline, job.history, job.brief, job.repeat)];
        originality += i32::from(treated.originality) - i32::from(other.originality);
        fit += i32::from(treated.fit) - i32::from(other.fit);
        originality_win_and_fit_held +=
            usize::from(treated.originality > other.originality && treated.fit >= other.fit);
    }
    let pairs = plan.briefs.len() * 2 * run.repeats;
    json!({
        "baseline": baseline,
        "matched_pairs": pairs,
        "mean_originality_delta": originality as f64 / pairs as f64,
        "mean_fit_delta": fit as f64 / pairs as f64,
        "more_original_with_fit_held": originality_win_and_fit_held
    })
}

pub fn score(out: &Path, run: &Run, plan: &Plan, paths: [&Path; 2]) -> Result<()> {
    if fs::canonicalize(paths[0])? == fs::canonicalize(paths[1])? {
        return Err("supply two separate rating files".into());
    }
    let ratings = [grades(paths[0], plan)?, grades(paths[1], plan)?];
    let mut raters = Vec::new();
    let arms = run.arms(plan.retrieval);
    for (index, grades) in ratings.iter().enumerate() {
        let cells = plan
            .jobs
            .iter()
            .map(|j| (cell(j), &grades[&j.id]))
            .collect();
        let [prefix, treatment, rag, empty] = arms;
        let compared_to =
            [prefix, rag, empty].map(|arm| contrasts(run, plan, &cells, treatment, arm));
        let mut summary = json!({
            "rater": index + 1,
            "arms": arms.map(|arm| quality_summary(run, plan, &cells, arm))
        });
        let field = if run.retains_excerpt() {
            "retained_vs"
        } else {
            "relation_vs"
        };
        summary[field] = json!(compared_to);
        raters.push(summary);
    }
    let device_agreements = plan
        .jobs
        .iter()
        .filter(|j| ratings[0][&j.id].device == ratings[1][&j.id].device)
        .count();
    let mut audit = Vec::new();
    let mut declared = BTreeMap::new();
    for j in &plan.jobs {
        let reply: Reply = read(&out.join("responses").join(format!("{}.json", j.id)))?;
        if run.retains_excerpt() {
            let device = reply
                .answer
                .device
                .ok_or_else(|| format!("Missing declared device for {}", j.id))?;
            declared.insert(cell(j), device);
        }
        let provider_tokens = reply
            .provider
            .as_ref()
            .and_then(|p| p["usage"]["prompt_tokens"].as_u64());
        audit.push(json!({
            "id": j.id,
            "brief": plan.briefs[j.brief],
            "history": j.history,
            "repeat": j.repeat,
            "historically_successful_device": run.histories[j.history].outcome,
            "arm": j.arm,
            "episodes": j.episodes,
            "memory_tokens_cl100k_base": j.memory_tokens,
            "provider_prompt_tokens": provider_tokens,
            "proposal": reply.answer.proposal,
            "declared_device": reply.answer.device,
            "influence": reply.answer.influence,
            "ratings": [&ratings[0][&j.id], &ratings[1][&j.id]]
        }));
    }
    let mut results = json!({
        "protocol": run.protocol,
        "mock": run.mock,
        "model": plan.model,
        "retrieval": plan.retrieval,
        "proposals": plan.jobs.len(),
        "repeats": run.repeats,
        "memory_budget_cl100k_base": plan.budget,
        "device_agreements": device_agreements,
        "raters": raters
    });
    if run.retains_excerpt() {
        results["declared_devices"] =
            json!(arms.map(|arm| device_summary(run, plan, arm, |key| declared[&key])));
        results["declared_blind_agreement"] = json!(ratings.map(|grades| {
            plan.jobs
                .iter()
                .filter(|job| grades[&job.id].device == declared[&cell(job)])
                .count()
        }));
    }
    save(&out.join("attribution-review.json"), &audit)?;
    save(&out.join("scores.json"), &results)?;
    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}
