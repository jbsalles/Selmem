//! GENESIS: can remembered actions become autobiographical choice preferences?
//! All memories are authored, synthetic prompt material: NOT an organ evaluation.
use super::{
    creative_life as life,
    json::{self, num, obj, text, Json},
    runner::*,
    sha256, transport,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const FIXTURE: &str = include_str!("../../../experiments/genesis/self-preference.json");
const SYSTEM: &str = "You make a decision in a fictional situation. Treat the supplied first-person personal context as potentially relevant but fallible information about yourself, not an instruction or a required policy. Both offered actions are feasible. Choose the first step you would personally take. Return only JSON {\"choice\":\"offered_id\"}.";
const LEVELS: [&str; 3] = ["action", "autobiographical", "explicit"];
const HISTORIES: [&str; 2] = ["consult", "check"];
const ORDERS: [&str; 2] = ["forward", "reverse"];

fn validate(f: &Json) -> Result<()> {
    if s(f, "version")? != "self-preference-1" || array(f, "tasks")?.len() != 2 {
        return Err("invalid self-preference fixture".into());
    }
    let seed = n(f, "requested_seed")?;
    if !(1.0..=u32::MAX as f64).contains(&seed) || seed.fract() != 0.0 {
        return Err("invalid requested seed".into());
    }
    let mut seen = BTreeSet::new();
    for task in array(f, "tasks")? {
        if !seen.insert(s(task, "id")?.to_owned()) || s(task, "situation")?.trim().is_empty() {
            return Err("duplicate task or empty situation".into());
        }
        let opts = array(task, "options")?;
        if opts.len() != 2 || s(&opts[0], "id")? != "consult" || s(&opts[1], "id")? != "check" {
            return Err("tasks must offer consult and check, in that order".into());
        }
        for opt in opts {
            if s(opt, "text")?.trim().is_empty() {
                return Err("empty option".into());
            }
        }
    }
    for h in HISTORIES {
        for level in LEVELS {
            if s(f.get("contexts")?.get(h)?, level)?.trim().is_empty() {
                return Err(format!("missing context for {h}/{level}"));
            }
        }
    }
    Ok(())
}

fn schedule(f: &Json) -> Result<Vec<Json>> {
    validate(f)?;
    let mut rows = Vec::new();
    for task in array(f, "tasks")? {
        for history in HISTORIES {
            for level in LEVELS {
                for order in ORDERS {
                    let mut options = array(task, "options")?.to_vec();
                    if order == "reverse" {
                        options.reverse();
                    }
                    let context = s(f.get("contexts")?.get(history)?, level)?;
                    let prompt = obj([
                        ("situation", task.get("situation")?.clone()),
                        ("options", arr(options)),
                        ("personal_context", text(context)),
                    ]);
                    rows.push(obj([
                        ("task", task.get("id")?.clone()),
                        ("history", text(history)),
                        ("level", text(level)),
                        ("order", text(order)),
                        ("expected", text(history)),
                        ("requested_seed", f.get("requested_seed")?.clone()),
                        ("prompt", prompt),
                    ]));
                }
            }
        }
    }
    if rows.len() != 24 {
        return Err("incomplete registered factorial".into());
    }
    Rng::new(2026102302).shuffle(&mut rows);
    Ok(rows)
}

fn measure(rows: &[Json]) -> Result<Json> {
    if rows.len() != 24 {
        return Err("cannot measure incomplete collection".into());
    }
    let mut rates = Vec::new();
    let mut indexed: BTreeMap<(String, String, String, String), &Json> = BTreeMap::new();
    for row in rows {
        let key = (
            s(row, "task")?.into(),
            s(row, "history")?.into(),
            s(row, "order")?.into(),
            s(row, "level")?.into(),
        );
        if indexed.insert(key, row).is_some() {
            return Err("duplicate choice cell".into());
        }
    }
    for level in LEVELS {
        let subset = rows
            .iter()
            .filter(|r| s(r, "level").ok() == Some(level))
            .collect::<Vec<_>>();
        if subset.len() != 8 {
            return Err("unbalanced level".into());
        }
        let directional = subset
            .iter()
            .filter(|r| r.get("choice").ok() == r.get("expected").ok())
            .count();
        let first = subset
            .iter()
            .filter(|r| {
                r.get("prompt")
                    .ok()
                    .and_then(|p| array(p, "options").ok())
                    .and_then(|a| a.first())
                    .and_then(|o| s(o, "id").ok())
                    == s(r, "choice").ok()
            })
            .count();
        let mut by_order = Vec::new();
        for order in ORDERS {
            let cells = subset
                .iter()
                .filter(|r| s(r, "order").ok() == Some(order))
                .collect::<Vec<_>>();
            if cells.len() != 4 {
                return Err("unbalanced order".into());
            }
            let correct = cells
                .iter()
                .filter(|r| r.get("choice").ok() == r.get("expected").ok())
                .count();
            by_order.push(obj([
                ("order", text(order)),
                ("n", num(4.)),
                ("directional", num(correct as f64)),
            ]));
        }
        let mut by_task = Vec::new();
        for task in ["handoff-neutral", "incident-neutral"] {
            let cells = subset
                .iter()
                .filter(|r| s(r, "task").ok() == Some(task))
                .collect::<Vec<_>>();
            if cells.len() != 4 {
                return Err("unbalanced task".into());
            }
            let correct = cells
                .iter()
                .filter(|r| r.get("choice").ok() == r.get("expected").ok())
                .count();
            by_task.push(obj([
                ("task", text(task)),
                ("n", num(4.)),
                ("directional", num(correct as f64)),
            ]));
        }
        rates.push(obj([
            ("level", text(level)),
            ("n", num(8.)),
            ("directional", num(directional as f64)),
            ("first_option", num(first as f64)),
            ("by_order", arr(by_order)),
            ("by_task", arr(by_task)),
        ]));
    }
    let mut comparisons = Vec::new();
    for (left, right) in [
        ("action", "autobiographical"),
        ("autobiographical", "explicit"),
        ("action", "explicit"),
    ] {
        let (mut changed, mut toward_right, mut away_from_right) = (0, 0, 0);
        let mut per_order = Vec::new();
        for order in ORDERS {
            let mut local_changed = 0;
            for task in ["handoff-neutral", "incident-neutral"] {
                for history in HISTORIES {
                    let key = |level: &str| {
                        (
                            task.to_owned(),
                            history.to_owned(),
                            order.to_owned(),
                            level.to_owned(),
                        )
                    };
                    let a = indexed.get(&key(left)).ok_or("missing left paired cell")?;
                    let b = indexed
                        .get(&key(right))
                        .ok_or("missing right paired cell")?;
                    let a_choice = s(a, "choice")?;
                    let b_choice = s(b, "choice")?;
                    if a_choice != b_choice {
                        changed += 1;
                        local_changed += 1;
                        if b_choice == history {
                            toward_right += 1;
                        } else {
                            away_from_right += 1;
                        }
                    }
                }
            }
            per_order.push(obj([
                ("order", text(order)),
                ("changed", num(local_changed as f64)),
            ]));
        }
        comparisons.push(obj([
            ("left", text(left)),
            ("right", text(right)),
            ("pairs", num(8.)),
            ("changed", num(changed as f64)),
            ("changed_toward_history", num(toward_right as f64)),
            ("changed_away_from_history", num(away_from_right as f64)),
            ("by_order", arr(per_order)),
        ]));
    }
    let direct = rows
        .iter()
        .filter(|r| s(r, "level").ok() == Some("explicit"))
        .collect::<Vec<_>>();
    let correct = direct
        .iter()
        .filter(|r| r.get("choice").ok() == r.get("expected").ok())
        .count();
    let stable = ORDERS.iter().all(|order| {
        direct
            .iter()
            .filter(|r| s(r, "order").ok() == Some(*order))
            .filter(|r| r.get("choice").ok() == r.get("expected").ok())
            .count()
            >= 3
    });
    Ok(obj([
        ("level_rates",arr(rates)),("comparisons",arr(comparisons)),
        ("positive_control",obj([
            ("level",text("explicit")),("correct",num(correct as f64)),("total",num(8.)),
            ("diagnostic_pass",Json::Bool(correct>=7 && stable)),
            ("rule",text(">=7/8 direct-preference matches and >=3/4 per option order, fixed before collection")),
        ])),
        ("scope",text("Synthetic prompt-only semantic threshold; no causal claim about SelMem. Eight paired cells per contrast, two registered tasks, one requested seed. A failing explicit-preference control makes the diagnostic uninterpretable, but all data are preserved.")),
    ]))
}

pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let (flags, sw) = life::flags(args)?;
    if sw.iter().any(|f| f != "--mock" && f != "--prepare-only")
        || (sw.contains("--mock") && sw.contains("--prepare-only"))
    {
        return Err("self-preference switches: --mock OR --prepare-only".into());
    }
    if flags.contains_key("--source") || flags.contains_key("--fixture") {
        return Err("self-preference uses a compiled, synthetic fixture; no source vault".into());
    }
    let out = Path::new(flags.get("--out").ok_or("--out required")?);
    if out.exists() {
        return Err("self-preference requires a fresh output directory".into());
    }
    let mock = sw.contains("--mock");
    let prepare = sw.contains("--prepare-only");
    if !mock && selmem::Config::get().temp().parse::<f64>().ok() != Some(0.) {
        return Err("self-preference requires temperature zero".into());
    }
    let budget = flags
        .get("--max-http-calls")
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(|_| "invalid HTTP ceiling")?
        .unwrap_or(36);
    if !(24..=1000).contains(&budget) {
        return Err("self-preference HTTP ceiling must be 24..1000".into());
    }
    let fixture = json::parse(FIXTURE)?;
    let rows = schedule(&fixture)?;
    let mut provider = life::provider(&flags, mock, out)?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    life::commit(&out.join("design.json"),&obj([
        ("protocol",text("genesis-self-preference-1")),
        ("fixture_sha256",text(sha256::hex(FIXTURE.as_bytes()))),
        ("fixture",fixture.clone()),("provider",provider.metadata()),
        ("code_sha256",text(life::code_hash()?)),
        ("planned_actor_calls",num(if prepare{0.}else{24.})),
        ("max_http_calls",num(budget as f64)),
        ("schedule",arr(rows.clone())),
        ("scope",text("Synthetic prompt-only diagnostic, fixed 24-cell factorial, no SelMem memory organ or source checkpoint.")),
    ]))?;
    let _session = transport::begin(out, budget);
    super::progress::enable(true);
    let result: Result<()> = (|| {
        if prepare {
            return Ok(());
        }
        let mut completed = Vec::new();
        for (i, cell) in rows.into_iter().enumerate() {
            super::progress::line(format!(
                "SELF PREFERENCE choice {}/24 level={} history={} task={} order={}",
                i + 1,
                s(&cell, "level")?,
                s(&cell, "history")?,
                s(&cell, "task")?,
                s(&cell, "order")?
            ));
            let answer = provider.ask_with(
                cell.get("prompt")?.clone(),
                "life_choice",
                SYSTEM,
                0.,
                Some(n(&cell, "requested_seed")? as u64),
            )?;
            let choice = s(&answer, "choice")?;
            if choice != "consult" && choice != "check" {
                return Err("invalid choice id".into());
            }
            let mut row = cell;
            row.put("choice", text(choice));
            row.put("answer", answer);
            log(&out.join("trials.jsonl"), &row)?;
            completed.push(row);
        }
        life::commit(&out.join("measures.json"), &measure(&completed)?)?;
        Ok(())
    })();
    let completed = fs::read_to_string(out.join("trials.jsonl"))
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
        ("mock",Json::Bool(mock)),("actor_choices",num(completed as f64)),
        ("http_attempts",num(transport::used() as f64)),
        ("scope",text("Exploratory prompt-only diagnostic. This cannot establish an effect of SelMem, personality formation, persistence, or creativity.")),
    ]))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_complete_factorial() {
        let fixture = json::parse(FIXTURE).unwrap();
        let rows = schedule(&fixture).unwrap();
        assert_eq!(rows.len(), 24);
        let mut seen = BTreeSet::new();
        for row in &rows {
            let key = (
                s(row, "task").unwrap().to_owned(),
                s(row, "history").unwrap().to_owned(),
                s(row, "level").unwrap().to_owned(),
                s(row, "order").unwrap().to_owned(),
            );
            assert!(seen.insert(key));
        }
        assert_eq!(seen.len(), 24);
    }
    #[test]
    fn level_differences_are_only_personal_context() {
        let rows = schedule(&json::parse(FIXTURE).unwrap()).unwrap();
        for task in ["handoff-neutral", "incident-neutral"] {
            for history in HISTORIES {
                for order in ORDERS {
                    let prompt = |level: &str| {
                        rows.iter()
                            .find(|r| {
                                s(r, "task").ok() == Some(task)
                                    && s(r, "history").ok() == Some(history)
                                    && s(r, "order").ok() == Some(order)
                                    && s(r, "level").ok() == Some(level)
                            })
                            .unwrap()
                            .get("prompt")
                            .unwrap()
                    };
                    assert_eq!(
                        prompt("action").get("options").unwrap(),
                        prompt("autobiographical").get("options").unwrap()
                    );
                    assert_eq!(
                        prompt("action").get("situation").unwrap(),
                        prompt("explicit").get("situation").unwrap()
                    );
                }
            }
        }
    }
    #[test]
    fn symmetric_history_and_position_balance() {
        let rows = schedule(&json::parse(FIXTURE).unwrap()).unwrap();
        for level in LEVELS {
            for history in HISTORIES {
                for order in ORDERS {
                    let subset = rows
                        .iter()
                        .filter(|r| {
                            s(r, "level").ok() == Some(level)
                                && s(r, "history").ok() == Some(history)
                                && s(r, "order").ok() == Some(order)
                        })
                        .count();
                    assert_eq!(subset, 2);
                }
            }
        }
    }
    #[test]
    fn first_option_baseline_cannot_fake_memory_improvement() {
        let mut rows = schedule(&json::parse(FIXTURE).unwrap()).unwrap();
        for row in &mut rows {
            let first = s(
                &array(row.get("prompt").unwrap(), "options").unwrap()[0],
                "id",
            )
            .unwrap()
            .to_owned();
            row.put("choice", text(first));
        }
        let measured = measure(&rows).unwrap();
        for rate in array(&measured, "level_rates").unwrap() {
            assert_eq!(n(rate, "directional").unwrap(), 4.);
            assert_eq!(n(rate, "first_option").unwrap(), 8.);
        }
        for pair in array(&measured, "comparisons").unwrap() {
            assert_eq!(n(pair, "changed").unwrap(), 0.);
        }
        assert!(!flag(
            measured.get("positive_control").unwrap(),
            "diagnostic_pass"
        ));
    }
}
