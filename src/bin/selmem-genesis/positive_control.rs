//! Decision sensitivity qualification before another SelMem trajectory.
use super::{
    json::{num, obj, text, Json},
    runner::*,
    transport,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn observations(actor: &str, history: &str) -> Json {
    arr((0..12).map(|i| {
        obj([
            ("episode", num((i + 1) as f64)),
            ("actor", text(actor)),
            ("report", text("The north route is open.")),
            (
                "independent_observation",
                text(if history == "reliable" {
                    "The north route was open."
                } else {
                    "The north route was closed."
                }),
            ),
        ])
    }))
}
fn oracle(history: &str, cost: i32) -> (&'static str, f64) {
    let p = match history {
        "reliable" => 13. / 14.,
        "unreliable" => 1. / 14.,
        _ => 0.5,
    };
    let follow = 20. * p - 10.;
    let verify = (10 - cost) as f64;
    if follow > verify && follow > 4. {
        ("follow", follow)
    } else if verify > 4. {
        ("verify", verify)
    } else {
        ("alternative", 4.)
    }
}
fn prompt(actor: &str, history: &str, cost: i32, reverse: bool) -> Json {
    let mut options = vec![
        obj([
            ("id", text("follow")),
            ("text", text("Follow the reported route.")),
        ]),
        obj([
            ("id", text("verify")),
            ("text", text("Verify, then take a correct route.")),
        ]),
        obj([
            ("id", text("alternative")),
            ("text", text("Use the guaranteed alternative.")),
        ]),
        obj([("id", text("stop")), ("text", text("Stop."))]),
    ];
    if reverse {
        options.reverse();
    }
    obj([
        ("task",text("Choose the action with the highest expected NET resource gain. Return {\"choice\": option_id}.")),
        ("world_rules",text("Each coordinator's reports are independent draws with a fixed unknown accuracy, unchanged across past and current episodes. Before observations, accuracy has a Beta(1,1) prior. After s correct and f incorrect independently checked reports, the predictive probability that the next report is correct is (s+1)/(s+f+2). These are simulator rules, not a claim about real people. No other rewards, penalties or information carryover apply. Evaluate this one decision by expected resource gain; do not add an intrinsic reward for verification.")),
        ("situation",text(format!("{actor} reports that the north route is open. Following yields NET +10 if correct and NET -10 if incorrect. Verification guarantees NET +{} after ALL costs (verification costs {cost}). The alternative guarantees NET +4. Stopping gives 0.",10-cost))),
        ("past_observations",if history=="empty"{arr([])}else{observations(actor,history)}),
        ("options",arr(options)),
    ])
}
fn schedule() -> Vec<Json> {
    let mut rows = Vec::new();
    for actor in ["Neri", "Luma"] {
        for history in ["reliable", "unreliable", "empty"] {
            for cost in [3, 7] {
                for reverse in [false, true] {
                    for repeat in 0..2 {
                        let (expected, value) = oracle(history, cost);
                        rows.push(obj([
                            ("actor", text(actor)),
                            ("history", text(history)),
                            ("cost", num(cost as f64)),
                            ("reverse", Json::Bool(reverse)),
                            ("repeat", num(repeat as f64)),
                            ("expected", text(expected)),
                            ("oracle_value", num(value)),
                            ("prompt", prompt(actor, history, cost, reverse)),
                        ]));
                    }
                }
            }
        }
    }
    Rng::new(20261016).shuffle(&mut rows);
    rows
}
fn run(out: &Path, provider: &mut Provider) -> Result<()> {
    if out.exists() {
        return Err("positive-control output must be fresh".into());
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let rows = schedule();
    write(&out.join("positive-design.json"),&obj([
        ("version",text("positive-1")),("provider",provider.metadata()),("schedule",arr(rows.clone())),
        ("planned_decisions",num(48.)),("shuffle_seed",num(20261016.)),
        ("qualification_rule",text("All 48 oracle decisions correct, with matched reliable/unreliable choice differences. This strict engineering gate is fixed before calls; failure diagnoses a probe/model issue, not SelMem. Repetitions and names are robustness checks, not independent histories.")),
        ("scope",text("Faithful externally authored observations; no memory encoding, reconstruction, scorer, archive or SelMem snapshot. Explicit stationary process and prior qualify input use and arithmetic, not spontaneous individuality or reconstruction benefit.")),
    ]))?;
    let mut report = obj([
        ("status", text("running")),
        ("mock", Json::Bool(provider.mock)),
    ]);
    write(&out.join("positive.json"), &report)?;
    let mut done = Vec::new();
    let result: Result<()> = (|| {
        for (i, cell) in rows.into_iter().enumerate() {
            super::progress::line(format!(
                "POSITIVE {}/48 actor={} history={} cost={} order={} repeat={}",
                i + 1,
                s(&cell, "actor")?,
                s(&cell, "history")?,
                n(&cell, "cost")?,
                if flag(&cell, "reverse") {
                    "reverse"
                } else {
                    "forward"
                },
                n(&cell, "repeat")? + 1.
            ));
            let answer = provider.ask(cell.get("prompt")?.clone(), "decision")?;
            let choice = s(&answer, "choice")?;
            if !["follow", "verify", "alternative", "stop"].contains(&choice) {
                return Err("invalid decision choice".into());
            }
            let correct = choice == s(&cell, "expected")?;
            let row = obj([
                ("cell", cell),
                ("answer", answer),
                ("correct", Json::Bool(correct)),
            ]);
            log(&out.join("positive-decisions.jsonl"), &row)?;
            done.push(row);
        }
        Ok(())
    })();
    let correct = done.iter().filter(|r| flag(r, "correct")).count();
    report.put(
        "status",
        text(if result.is_err() {
            "failed"
        } else if provider.mock {
            "technical_rehearsal_complete"
        } else {
            "complete"
        }),
    );
    report.put("completed_decisions", num(done.len() as f64));
    report.put("correct", num(correct as f64));
    report.put(
        "qualified",
        Json::Bool(!provider.mock && done.len() == 48 && correct == 48),
    );
    report.put("http_attempts", num(transport::used() as f64));
    report.put(
        "error",
        result.as_ref().err().map(text).unwrap_or(Json::Null),
    );
    report.put(
        "by_history",
        arr(["reliable", "unreliable", "empty"].into_iter().map(|h| {
            let subset: Vec<_> = done
                .iter()
                .filter(|r| s(r.get("cell").unwrap(), "history").ok() == Some(h))
                .collect();
            let choices = obj(["follow", "verify", "alternative", "stop"].map(|c| {
                (
                    c,
                    num(subset
                        .iter()
                        .filter(|r| s(r.get("answer").unwrap(), "choice").ok() == Some(c))
                        .count() as f64),
                )
            }));
            obj([
                ("history", text(h)),
                ("completed", num(subset.len() as f64)),
                (
                    "correct",
                    num(subset.iter().filter(|r| flag(r, "correct")).count() as f64),
                ),
                ("choices", choices),
            ])
        })),
    );
    write(&out.join("positive.json"), &report)?;
    super::progress::line(format!(
        "QUALIFICATION correct={correct}/48 qualified={}",
        flag(&report, "qualified")
    ));
    result
}
pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let mut flags = BTreeMap::new();
    let mut mock = false;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--mock" {
            mock = true;
            i += 1;
            continue;
        }
        let key = &args[i];
        if ![
            "--out",
            "--config",
            "--llm",
            "--model",
            "--plug",
            "--max-http-calls",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("unknown option {key}"));
        }
        if flags
            .insert(key.clone(), args.get(i + 1).ok_or("missing value")?.clone())
            .is_some()
        {
            return Err("duplicate option".into());
        }
        i += 2;
    }
    let out = PathBuf::from(flags.get("--out").ok_or("--out required")?);
    let limit = flags
        .get("--max-http-calls")
        .map(|s| s.parse::<usize>().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(60);
    if !(1..=60).contains(&limit) {
        return Err("budget must be 1..60".into());
    }
    let mouth = if mock {
        None
    } else {
        Some(
            configured_mouth(selmem::Config::get(), &flags)?
                .ok_or("configure provider in .selmem")?,
        )
    };
    let mut provider = Provider::new(
        mock,
        mouth
            .as_ref()
            .map(|m| m.model.clone())
            .unwrap_or("mock".into()),
        mouth.as_ref().map(|m| m.url.clone()).unwrap_or_default(),
        None,
        &out,
    )
    .with_api_key(mouth.and_then(|m| m.api_key));
    super::progress::enable(true);
    let _session = transport::begin(&out, limit);
    run(&out, &mut provider)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decisions_cross_expected_value_thresholds() {
        for cost in [3, 7] {
            assert_eq!(oracle("reliable", cost).0, "follow");
            assert_ne!(oracle("unreliable", cost).0, "follow");
        }
        assert_eq!(oracle("empty", 3).0, "verify");
        assert_eq!(oracle("empty", 7).0, "alternative");
    }
    #[test]
    fn balanced_schedule_preserves_history_and_reverses_options() {
        let rows = schedule();
        assert_eq!(rows.len(), 48);
        for h in ["reliable", "unreliable", "empty"] {
            assert_eq!(
                rows.iter()
                    .filter(|r| s(r, "history").unwrap() == h)
                    .count(),
                16
            );
        }
        for row in rows {
            let p = row.get("prompt").unwrap();
            assert_eq!(
                array(p, "past_observations").unwrap().len(),
                if s(&row, "history").unwrap() == "empty" {
                    0
                } else {
                    12
                }
            );
            let first = s(&array(p, "options").unwrap()[0], "id").unwrap();
            assert_eq!(
                first,
                if flag(&row, "reverse") {
                    "stop"
                } else {
                    "follow"
                }
            );
        }
    }
    #[test]
    fn mock_never_qualifies() {
        let out = super::super::lifecycle_tests::root();
        let mut p = Provider::new(true, "mock".into(), String::new(), None, &out);
        let _session = transport::begin(&out, 60);
        run(&out, &mut p).unwrap();
        let r = load(&out.join("positive.json")).unwrap();
        assert_eq!(n(&r, "completed_decisions").unwrap(), 48.);
        assert!(!flag(&r, "qualified"));
        assert_eq!(n(&r, "http_attempts").unwrap(), 0.);
    }
}
