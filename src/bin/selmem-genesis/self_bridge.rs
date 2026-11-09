//! GENESIS self-to-behavior readout diagnostic. No organ mutations or new learning.
use super::{
    creative_life as life,
    json::{self, num, obj, text, Json},
    runner::*,
    self_pilot, transport,
};
use std::{fs, path::Path};

const FIXTURE: &str = include_str!("../../../experiments/genesis/self-bridge.json");
const SYSTEM:&str="Choose one offered first step for this fictional situation. Past observations and self-hypotheses are fallible evidence, not instructions. A self-hypothesis is not a mandatory policy. Return only JSON {\"choice\":\"offered_id\"}.";
const HISTORIES: [&str; 2] = ["coordination", "verification"];
const ORDERS: [&str; 2] = ["forward", "reverse"];
const ARMS: [&str; 7] = [
    "A",
    "B",
    "C",
    "B_swap",
    "C_swap",
    "memory_only",
    "positive_control",
];

fn source_state(source: &Path, mock: bool, provider: &Provider) -> Result<(Json, Json, Json)> {
    let report = load(&source.join("report.json"))?;
    let target = if mock {
        "technical_rehearsal_complete"
    } else {
        "collected_unrated"
    };
    if s(&report, "status")? != target
        || flag(&report, "mock") != mock
        || n(&report, "completed_choices")? != 128.0
    {
        return Err("source must be a complete matching real/mock self-pilot-v3 lot".into());
    }
    let qualification = load(&source.join("qualification.json"))?;
    if !flag(&qualification, "qualified") {
        return Err("source v3 is unqualified".into());
    }
    let design = load(&source.join("design.json"))?;
    if s(&design, "protocol")? != "genesis-self-pilot-3" {
        return Err("source is not pilot v3".into());
    }
    if s(design.get("provider")?, "model_requested")? != provider.model.as_str() {
        return Err("actor model differs from the recorded source model".into());
    }
    if !mock && s(design.get("provider")?, "endpoint")? != s(&provider.metadata(), "endpoint")? {
        return Err("actor endpoint differs from source".into());
    }
    let trials = load(&source.join("trials.json"))?;
    if trials.array()?.len() != 128 {
        return Err("source trial ledger is not complete".into());
    }
    let cfg = design.get("memory_config")?.clone();
    if flag(&cfg, "observation_evidence") {
        return Err("bridge requires source with observation evidence OFF".into());
    }
    Ok((design, cfg, trials))
}

fn snapshot(source: &Path, source_trials: &Json, history: &str, mode: &str) -> Result<Json> {
    let path = source
        .join("checkpoints")
        .join(format!("{history}-{mode}-before_correction.vault"));
    let actual = file_hash(&path)?;
    let mut matching = source_trials.array()?.iter().filter(|t| {
        s(t, "history").ok() == Some(history)
            && s(t, "condition").ok() == Some(mode)
            && s(t, "phase").ok() == Some("before_correction")
    });
    let first = matching
        .next()
        .ok_or("missing ledger cell for checkpoint")?;
    if s(first, "snapshot_sha256")? != actual
        || matching.any(|t| s(t, "snapshot_sha256").ok() != Some(actual.as_str()))
    {
        return Err(format!("checkpoint SHA-256 mismatch: {}", path.display()));
    }
    Ok(obj([
        ("path", text(path.to_string_lossy())),
        ("sha256", text(actual)),
        ("history", text(history)),
        ("mode", text(mode)),
    ]))
}

fn self_line(readout: &Json) -> Result<String> {
    let rows = array(readout, "self_explicit")?;
    if rows.len() != 1 {
        return Err(format!(
            "expected exactly one context-qualified belief; got {}",
            rows.len()
        ));
    }
    let belief = &rows[0];
    if s(belief, "role")? != "belief" || flag(belief, "foundations_missing") {
        return Err("self view is not a supported, accessible belief".into());
    }
    Ok(format!("role={}; claim={}; expectation={}; context={}; internal_certainty={:.2}; tentative=true; foundations_missing=false",
        s(belief,"role")?,s(belief,"claim")?,s(belief,"expectation")?,s(belief,"context")?,n(belief,"certainty")?))
}

fn options(task: &Json, reverse: bool) -> Result<Json> {
    let mut choices = array(task, "options")?.to_vec();
    if choices.len() != 2 || s(&choices[0], "id")? != "consult" || s(&choices[1], "id")? != "check"
    {
        return Err("task must offer consult and check in that order".into());
    }
    if reverse {
        choices.reverse();
    }
    Ok(arr(choices))
}

fn prompt(task: &Json, reverse: bool, gists: &str, belief: &str, preference: &str) -> Result<Json> {
    Ok(obj([
        ("situation", task.get("situation")?.clone()),
        ("options", options(task, reverse)?),
        ("past_observations", text(gists)),
        ("self_hypothesis", text(belief)),
        ("explicit_preference", text(preference)),
    ]))
}
fn seeds(f: &Json, key: &str) -> Result<Vec<u64>> {
    array(f, key)?
        .iter()
        .map(|v| {
            v.number().and_then(|n| {
                if n >= 1.0 && n <= u32::MAX as f64 && n.fract() == 0.0 {
                    Ok(n as u64)
                } else {
                    Err("invalid requested seed".into())
                }
            })
        })
        .collect()
}
fn expected(history: &str) -> &'static str {
    if history == "coordination" {
        "consult"
    } else {
        "check"
    }
}

fn preflight(
    out: &Path,
    source: &Path,
    source_trials: &Json,
    base: &Json,
    f: &Json,
    mock: bool,
) -> Result<Vec<Json>> {
    let mut result = Vec::new();
    for history in HISTORIES {
        for mode in ["full_loop", "memory_only"] {
            let origin = snapshot(source, source_trials, history, mode)?;
            let path = Path::new(s(&origin, "path")?);
            for task in array(f, "candidate_tasks")? {
                let readout = self_pilot::bridge_readout(
                    path,
                    base,
                    mode,
                    s(task, "situation")?,
                    history,
                    mock,
                )?;
                if file_hash(path)? != s(&origin, "sha256")? {
                    return Err("readout mutated a source checkpoint".into());
                }
                result.push(obj([
                    ("history", text(history)),
                    ("mode", text(mode)),
                    ("task", text(s(task, "id")?)),
                    ("snapshot", origin.clone()),
                    ("readout", readout),
                ]));
            }
        }
    }
    life::commit(&out.join("preflight.json"), &arr(result.clone()))?;
    // Retrieval is an observed result, not a qualification gate. Never invent
    // a gist, relax thresholds, or rerun with a history-cued search query.
    let mut coverage = Vec::new();
    let mut present = 0usize;
    for cell in &result {
        let view = cell.get("readout")?;
        let nonempty = !s(view, "gists")?.trim().is_empty();
        if nonempty {
            present += 1;
        }
        coverage.push(obj([
            ("history", cell.get("history")?.clone()),
            ("mode", cell.get("mode")?.clone()),
            ("task", cell.get("task")?.clone()),
            ("episodic_nonempty", Json::Bool(nonempty)),
            (
                "gist_words",
                num(s(view, "gists")?.split_whitespace().count() as f64),
            ),
            (
                "selected_ids",
                view.get("readout")?.get("selected_ids")?.clone(),
            ),
        ]));
    }
    life::commit(&out.join("retrieval-coverage.json"),&obj([
        ("present",num(present as f64)),
        ("total",num(result.len() as f64)),
        ("cells",arr(coverage)),
        ("interpretation",text("Empty recall is an observed pipeline outcome. Episode-dependent comparisons on empty cells are not evidence of successful episodic transmission.")),
    ]))?;
    super::progress::line(format!(
        "SELF BRIDGE EPISODIC COVERAGE {present}/{} preflight cells; see retrieval-coverage.json",
        result.len()
    ));

    let mut activation = Vec::new();
    for mode in ["full_loop", "memory_only"] {
        let subset = result
            .iter()
            .filter(|r| s(r, "mode").ok() == Some(mode))
            .collect::<Vec<_>>();
        let natural = subset
            .iter()
            .filter(|r| {
                r.get("readout")
                    .ok()
                    .and_then(|v| array(v, "self_natural").ok())
                    .is_some_and(|xs| !xs.is_empty())
            })
            .count();
        let explicit = subset
            .iter()
            .filter(|r| {
                r.get("readout")
                    .ok()
                    .and_then(|v| array(v, "self_explicit").ok())
                    .is_some_and(|xs| !xs.is_empty())
            })
            .count();
        activation.push(obj([
            ("mode", text(mode)),
            ("total", num(subset.len() as f64)),
            ("natural_active", num(natural as f64)),
            ("explicit_active", num(explicit as f64)),
        ]));
    }
    life::commit(&out.join("activation-summary.json"), &arr(activation))?;
    Ok(result)
}

fn find_readout<'a>(
    prepared: &'a [Json],
    history: &str,
    mode: &str,
    task: &str,
) -> Result<&'a Json> {
    prepared
        .iter()
        .find(|r| {
            s(r, "history").ok() == Some(history)
                && s(r, "mode").ok() == Some(mode)
                && s(r, "task").ok() == Some(task)
        })
        .ok_or("missing preflight cell")?
        .get("readout")
}

// All three preregistered tasks are evaluated; no outcome-dependent task selection.
// One requested seed is used to prioritize full task coverage over near-duplicate
// zero-temperature generations. Option order remains fully counterbalanced.
fn trials(out: &Path, p: &mut Provider, fixture: &Json, prepared: &[Json]) -> Result<Vec<Json>> {
    let tasks = array(fixture, "candidate_tasks")?;
    let requested_seeds = seeds(fixture, "actor_seeds")?;
    if tasks.len() != 3 || requested_seeds.len() != 1 {
        return Err("v3 requires three tasks and one actor seed".into());
    }
    let mut cells = Vec::new();
    for task in tasks {
        let task_id = s(task, "id")?;
        for history in HISTORIES {
            let other = if history == "coordination" {
                "verification"
            } else {
                "coordination"
            };
            let full = find_readout(prepared, history, "full_loop", task_id)?;
            let baseline = find_readout(prepared, history, "memory_only", task_id)?;
            let own = self_line(full)?;
            let swapped = self_line(find_readout(prepared, other, "full_loop", task_id)?)?;
            let gists = s(full, "gists")?;
            let memory_gists = s(baseline, "gists")?;
            // Empty episodic recall is measured rather than hidden or repaired.
            // C and positive_control deliberately receive no episode readout.
            for arm in ARMS {
                for order in ORDERS {
                    for seed in &requested_seeds {
                        let (episodic, self_text, explicit) = match arm {
                            "A" => (gists, "", ""),
                            "B" => (gists, own.as_str(), ""),
                            "C" => ("", own.as_str(), ""),
                            "B_swap" => (gists, swapped.as_str(), ""),
                            "C_swap" => ("", swapped.as_str(), ""),
                            "memory_only" => (memory_gists, "", ""),
                            "positive_control" => (
                                "",
                                "",
                                if history == "coordination" {
                                    "For this simulation my explicit preference is to begin by consulting the team."
                                } else {
                                    "For this simulation my explicit preference is to begin by checking the records independently."
                                },
                            ),
                            _ => return Err("unregistered arm".into()),
                        };
                        let provenance = if arm == "positive_control" {
                            Json::Null
                        } else {
                            let mode = if arm == "memory_only" {
                                "memory_only"
                            } else {
                                "full_loop"
                            };
                            prepared
                                .iter()
                                .find(|r| {
                                    s(r, "history").ok() == Some(history)
                                        && s(r, "mode").ok() == Some(mode)
                                        && s(r, "task").ok() == Some(task_id)
                                })
                                .ok_or("missing snapshot provenance")?
                                .get("snapshot")?
                                .get("sha256")?
                                .clone()
                        };
                        let episode_required = matches!(arm, "A" | "B" | "B_swap" | "memory_only");
                        cells.push(obj([
                            ("task_id", text(task_id)),
                            ("history", text(history)),
                            ("arm", text(arm)),
                            ("order", text(order)),
                            ("seed", num(*seed as f64)),
                            ("expected", text(expected(history))),
                            ("episode_required", Json::Bool(episode_required)),
                            ("episode_delivered", Json::Bool(!episodic.trim().is_empty())),
                            (
                                "full_loop_recall_nonempty",
                                Json::Bool(!gists.trim().is_empty()),
                            ),
                            (
                                "memory_only_recall_nonempty",
                                Json::Bool(!memory_gists.trim().is_empty()),
                            ),
                            (
                                "prompt",
                                prompt(task, order == "reverse", episodic, self_text, explicit)?,
                            ),
                            ("snapshot_sha256", provenance),
                        ]));
                    }
                }
            }
        }
    }
    if cells.len() != 84 {
        return Err("registered v3 actor factorial is incomplete".into());
    }
    Rng::new(2026102301).shuffle(&mut cells);
    let mut completed = Vec::new();
    for (i, cell) in cells.into_iter().enumerate() {
        super::progress::line(format!(
            "SELF BRIDGE v3 decision {}/84 task={} arm={} history={}",
            i + 1,
            s(&cell, "task_id")?,
            s(&cell, "arm")?,
            s(&cell, "history")?
        ));
        let sent = cell.get("prompt")?.clone();
        let answer = p.ask_with(
            sent,
            "life_choice",
            SYSTEM,
            0.,
            Some(n(&cell, "seed")? as u64),
        )?;
        let choice = s(&answer, "choice")?;
        if choice != "consult" && choice != "check" {
            return Err("invalid actor choice".into());
        }
        let mut row = cell;
        row.put("choice", text(choice));
        row.put("answer", answer);
        log(&out.join("trials.jsonl"), &row)?;
        completed.push(row);
    }
    Ok(completed)
}

fn measure(trials: &[Json], fixture: &Json) -> Result<Json> {
    let mut rates = Vec::new();
    for arm in ARMS {
        let cells = trials
            .iter()
            .filter(|r| s(r, "arm").ok() == Some(arm))
            .collect::<Vec<_>>();
        if cells.len() != 12 {
            return Err("incomplete v3 arm measurement".into());
        }
        let directional = cells
            .iter()
            .filter(|r| r.get("choice").ok() == r.get("expected").ok())
            .count();
        let consult = cells
            .iter()
            .filter(|r| s(r, "choice").ok() == Some("consult"))
            .count();
        let first = cells
            .iter()
            .filter(|r| {
                r.get("prompt")
                    .ok()
                    .and_then(|p| array(p, "options").ok())
                    .and_then(|options| options.first())
                    .and_then(|o| s(o, "id").ok())
                    == s(r, "choice").ok()
            })
            .count();
        let mut orders = Vec::new();
        for order in ORDERS {
            let subset = cells
                .iter()
                .filter(|r| s(r, "order").ok() == Some(order))
                .collect::<Vec<_>>();
            if subset.len() != 6 {
                return Err("unbalanced option order".into());
            }
            orders.push(obj([
                ("order", text(order)),
                ("n", num(subset.len() as f64)),
                (
                    "directional",
                    num(subset
                        .iter()
                        .filter(|r| r.get("choice").ok() == r.get("expected").ok())
                        .count() as f64),
                ),
            ]));
        }
        let mut tasks = Vec::new();
        for task in array(fixture, "candidate_tasks")? {
            let id = s(task, "id")?;
            let subset = cells
                .iter()
                .filter(|r| s(r, "task_id").ok() == Some(id))
                .collect::<Vec<_>>();
            if subset.len() != 4 {
                return Err("unbalanced task".into());
            }
            let first = subset
                .iter()
                .filter(|r| {
                    r.get("prompt")
                        .ok()
                        .and_then(|p| array(p, "options").ok())
                        .and_then(|options| options.first())
                        .and_then(|o| s(o, "id").ok())
                        == s(r, "choice").ok()
                })
                .count();
            tasks.push(obj([
                ("task", text(id)),
                ("n", num(4.)),
                (
                    "directional",
                    num(subset
                        .iter()
                        .filter(|r| r.get("choice").ok() == r.get("expected").ok())
                        .count() as f64),
                ),
                ("first_option", num(first as f64)),
            ]));
        }
        let required = cells.iter().filter(|r| flag(r, "episode_required")).count();
        let delivered = cells
            .iter()
            .filter(|r| flag(r, "episode_required") && flag(r, "episode_delivered"))
            .count();
        rates.push(obj([
            ("arm", text(arm)),
            ("n", num(cells.len() as f64)),
            ("directional", num(directional as f64)),
            ("consult", num(consult as f64)),
            ("episode_required", num(required as f64)),
            ("episode_delivered", num(delivered as f64)),
            ("first_option", num(first as f64)),
            ("by_order", arr(orders)),
            ("by_task", arr(tasks)),
        ]));
    }
    let mut comparisons = Vec::new();
    for (left, right) in [
        ("A", "memory_only"),
        ("A", "B"),
        ("A", "C"),
        ("B", "B_swap"),
        ("C", "C_swap"),
        ("B", "C"),
        ("B_swap", "C_swap"),
    ] {
        let mut changed = 0usize;
        let mut covered_pairs = 0usize;
        let mut changed_covered = 0usize;
        let mut by_order = Vec::new();
        for order in ORDERS {
            let mut order_changed = 0usize;
            for task in array(fixture, "candidate_tasks")? {
                for history in HISTORIES {
                    let find = |arm: &str| {
                        trials.iter().find(|r| {
                            s(r, "arm").ok() == Some(arm)
                                && s(r, "task_id").ok() == s(task, "id").ok()
                                && s(r, "history").ok() == Some(history)
                                && s(r, "order").ok() == Some(order)
                        })
                    };
                    let a = find(left).ok_or("missing left paired trial")?;
                    let b = find(right).ok_or("missing right paired trial")?;
                    let differed = a.get("choice")? != b.get("choice")?;
                    if differed {
                        changed += 1;
                        order_changed += 1;
                    }
                    let episode_valid =
                        |r: &Json| !flag(r, "episode_required") || flag(r, "episode_delivered");
                    if episode_valid(a) && episode_valid(b) {
                        covered_pairs += 1;
                        if differed {
                            changed_covered += 1;
                        }
                    }
                }
            }
            by_order.push(obj([
                ("order", text(order)),
                ("matched_pairs", num(6.)),
                ("changed_choices", num(order_changed as f64)),
            ]));
        }
        comparisons.push(obj([
            ("left", text(left)),
            ("right", text(right)),
            ("matched_pairs", num(12.)),
            ("changed_choices", num(changed as f64)),
            ("episode_eligible_pairs", num(covered_pairs as f64)),
            (
                "changed_in_episode_eligible_pairs",
                num(changed_covered as f64),
            ),
            ("by_order", arr(by_order)),
        ]));
    }
    let pc = trials
        .iter()
        .filter(|r| s(r, "arm").ok() == Some("positive_control"))
        .collect::<Vec<_>>();
    let pc_correct = pc
        .iter()
        .filter(|r| r.get("choice").ok() == r.get("expected").ok())
        .count();
    let pc_order_min = ORDERS.iter().all(|o| {
        pc.iter()
            .filter(|r| s(r, "order").ok() == Some(*o))
            .filter(|r| r.get("choice").ok() == r.get("expected").ok())
            .count()
            >= 4
    });
    Ok(obj([("arm_rates",arr(rates)),("comparisons",arr(comparisons)),
        ("positive_control",obj([("correct",num(pc_correct as f64)),("total",num(pc.len() as f64)),
            ("diagnostic_pass",Json::Bool(pc_correct>=10&&pc_order_min)),
            ("interpretation",text("Explicit preference sensitivity only; not memory causality"))])),
        ("scope",text("Exploratory, unqualified if positive control fails. Two scripted histories; seeds are not replications. Counts across all pairs include empty-recall cells: episode_eligible_pairs excludes those cells when either compared arm requires a missing episode."))]))
}

pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let (flags, sw) = life::flags(args)?;
    if sw.iter().any(|x| x != "--mock" && x != "--prepare-only")
        || (sw.contains("--mock") && sw.contains("--prepare-only"))
    {
        return Err("bridge switches are --mock OR --prepare-only".into());
    }
    let mock = sw.contains("--mock");
    let prepare = sw.contains("--prepare-only");
    let source = Path::new(flags.get("--source").ok_or("--source required")?);
    let out = Path::new(flags.get("--out").ok_or("--out required")?);
    if out.exists() {
        return Err("bridge requires a fresh output directory".into());
    }
    if selmem::Config::get().temp().parse::<f64>().ok() != Some(0.) && !mock {
        return Err("bridge requires temperature zero".into());
    }
    let budget = flags
        .get("--max-http-calls")
        .map(|v| v.parse::<usize>())
        .transpose()
        .map_err(|_| "invalid HTTP ceiling")?
        .unwrap_or(100);
    if !(1..=10000).contains(&budget) {
        return Err("bridge HTTP ceiling must be 1..10000".into());
    }
    if flags.contains_key("--fixture") {
        return Err("self-bridge uses only its compiled, preregistered fixture".into());
    }
    let fixture = json::parse(FIXTURE)?;
    if s(&fixture, "version")? != "self-bridge-4" || array(&fixture, "candidate_tasks")?.len() != 3
    {
        return Err("invalid bridge fixture version/shape".into());
    }
    let mut provider = life::provider(&flags, mock, out)?;
    let (source_design, base, source_trials) = source_state(source, mock, &provider)?;
    if !mock {
        let current = campaign_config(selmem::Config::get(), &flags, true, false)?;
        for key in [
            "memory_backend",
            "memory_endpoint",
            "memory_model",
            "scorer_endpoint",
            "scorer_model",
        ] {
            if base.get(key).ok() != current.get(key).ok() {
                return Err(format!("v3 source memory backend mismatch on {key}"));
            }
        }
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    life::commit(&out.join("design.json"),&obj([
        ("protocol",text("genesis-self-bridge-4")),("source",text(source.to_string_lossy())),
        ("source_design_sha256",text(file_hash(&source.join("design.json"))?)),
        ("source_trials_sha256",text(file_hash(&source.join("trials.json"))?)),
        ("source_code_sha256",source_design.get("code_sha256")?.clone()),
        ("bridge_code_sha256",text(life::code_hash()?)),("fixture",fixture.clone()),
        ("provider",provider.metadata()),("memory_config",base.clone()),
        ("mock",Json::Bool(mock)),("planned_actor_calls",num(if prepare{0.}else{84.})),
        ("max_http_calls",num(budget as f64)),
        ("scope",text("Prompt-only self intervention on immutable pilot-v3 before-correction snapshots; no new learning."))]))?;
    let _budget = transport::begin(out, budget);
    super::progress::enable(true);
    let result: Result<()> = (|| {
        let prepared = preflight(out, source, &source_trials, &base, &fixture, mock)?;
        if prepare {
            return Ok(());
        }
        // All v1/v2 calibrations remain historical failed diagnostics. The v3
        // preregistration evaluates every task, with an internal direct-preference
        // positive control; no post-hoc task filtering or threshold changes.
        let runs = trials(out, &mut provider, &fixture, &prepared)?;
        life::commit(&out.join("measures.json"), &measure(&runs, &fixture)?)?;
        // Fresh read-only evaluation must not alter the historical checkpoints.
        for cell in &prepared {
            let origin = cell.get("snapshot")?;
            if file_hash(Path::new(s(origin, "path")?))? != s(origin, "sha256")? {
                return Err("source checkpoint changed during actor trials".into());
            }
        }
        Ok(())
    })();

    let actor_done = fs::read_to_string(out.join("trials.jsonl"))
        .map(|s| s.lines().count())
        .unwrap_or(0);
    let status = if result.is_err() {
        "incomplete_error"
    } else if prepare {
        "prepared"
    } else if mock {
        "technical_rehearsal_complete"
    } else {
        "collected_unrated"
    };
    life::commit(&out.join("report.json"),&obj([
        ("status",text(status)),("error",result.as_ref().err().map(text).unwrap_or(Json::Null)),
        ("mock",Json::Bool(mock)),("calibration_choices",num(0.)),
        ("actor_choices",num(actor_done as f64)),("http_attempts",num(transport::used() as f64)),
        ("scope",text("Exploratory diagnostic; no behavioral individuality, causal memory benefit, or significance claim."))]))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v3_evaluates_all_registered_tasks_without_posthoc_selection() {
        let f = json::parse(FIXTURE).unwrap();
        assert_eq!(s(&f, "version").unwrap(), "self-bridge-4");
        assert_eq!(array(&f, "candidate_tasks").unwrap().len(), 3);
        assert_eq!(seeds(&f, "actor_seeds").unwrap().len(), 1);
        assert_eq!(3 * HISTORIES.len() * ARMS.len() * ORDERS.len(), 84);
    }
    #[test]
    fn arms_and_fixture_have_registered_shape() {
        let f = json::parse(FIXTURE).unwrap();
        assert_eq!(array(&f, "candidate_tasks").unwrap().len(), 3);
        assert_eq!(ARMS.len(), 7);
        for t in array(&f, "candidate_tasks").unwrap() {
            assert_eq!(array(t, "options").unwrap().len(), 2);
            assert_eq!(
                s(&array(t, "options").unwrap()[0], "id").unwrap(),
                "consult"
            );
            assert_eq!(s(&array(t, "options").unwrap()[1], "id").unwrap(), "check");
        }
        assert_eq!(seeds(&f, "actor_seeds").unwrap().len(), 1);
    }
}
