use super::*;
fn options(args: &[String]) -> Result<(PathBuf, Vec<PathBuf>)> {
    let mut out = None;
    let mut ratings = vec![];
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).ok_or("missing option value")?;
        match args[i].as_str() {
            "--out" => {
                ensure(out.is_none(), "duplicate output")?;
                out = Some(PathBuf::from(v));
            }
            "--ratings" => ratings.push(PathBuf::from(v)),
            _ => return Err("unknown rating option".into()),
        }
        i += 2;
    }
    Ok((out.ok_or("--out required")?, ratings))
}
pub fn template(args: &[String]) -> Result<()> {
    let (out, ratings) = options(args)?;
    ensure(ratings.is_empty(), "template takes no ratings")?;
    let blind = load(&out.join("blind-proposals.json"))?;
    let rows = blind
        .array()?
        .iter()
        .map(|p| {
            Ok(obj([
                ("blind_id", p.get("blind_id")?.clone()),
                ("task", p.get("task")?.clone()),
                ("proposal", p.get("proposal")?.clone()),
                ("originality", Json::Null),
                ("utility", Json::Null),
                ("feasibility", Json::Null),
                ("behavior", arr((0..5).map(|_| Json::Null))),
                ("violations", arr([])),
                ("rationale", text("")),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    let file = out.join("rating-template.json");
    ensure(
        !file.exists(),
        "rating-template.json exists; preserve your work",
    )?;
    write(&file,&obj([("judge",text("")),("behavior_scale",text("0: first end; 4: second end. Order: individual/shared authority; separated/pooled resources; bounded/open-ended commitments; assigned/peer verification; reversible/durable participation. Unspecified: 2. Rate design features, not desirability. Quality dimensions use 0..4, higher is better. Judges must work independently with conditions hidden.")),("ratings",arr(rows))]))?;
    println!("{}", file.display());
    Ok(())
}
fn number(row: &Json, k: &str) -> Result<f64> {
    let n = row.get(k)?.number()?;
    ensure(
        n.is_finite() && (0.0..=4.0).contains(&n),
        "rating must be in 0..4",
    )?;
    Ok(n)
}
pub fn analyze(args: &[String]) -> Result<()> {
    let (out, files) = options(args)?;
    ensure(
        files.len() >= 2,
        "at least two complete distinct judges required",
    )?;
    let result = load(&out.join("result.json"))?;
    ensure(
        result.get("mock")? == &Json::Bool(false),
        "mock results are not experimental evidence",
    )?;
    ensure(
        s(&result, "status")? == "collected_unrated",
        "collection incomplete",
    )?;
    let all = load(&out.join("all-proposals.json"))?;
    let proposals = all.array()?;
    ensure(proposals.len() == 72, "expected 72 proposals")?;
    let expected = proposals
        .iter()
        .map(|p| s(p, "blind_id").map(str::to_string))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure(expected.len() == 72, "duplicate proposal ID")?;
    let mut judges = BTreeSet::new();
    let mut scores: BTreeMap<String, Vec<[f64; 8]>> = BTreeMap::new();
    let mut violations = BTreeMap::<String, usize>::new();
    for file in files {
        let data = load(&file)?;
        let judge = s(&data, "judge")?.trim();
        ensure(
            !judge.is_empty() && judges.insert(judge.to_lowercase()),
            "empty or duplicate judge identity",
        )?;
        let rows = data.get("ratings")?.array()?;
        ensure(rows.len() == 72, "judge must rate all 72 proposals")?;
        let mut seen = BTreeSet::new();
        for row in rows {
            let id = s(row, "blind_id")?;
            ensure(
                expected.contains(id) && seen.insert(id),
                "unknown or duplicate rated ID",
            )?;
            let original = proposals
                .iter()
                .find(|p| s(p, "blind_id").ok() == Some(id))
                .ok_or("rated proposal missing")?;
            ensure(
                row.get("task")? == original.get("task")?
                    && row.get("proposal")? == original.get("proposal")?,
                "rating task or answer differs from this lot; never transfer ratings by ID alone",
            )?;
            ensure(
                !s(row, "rationale")?.trim().is_empty(),
                "rating rationale required",
            )?;
            let behavior = row.get("behavior")?.array()?;
            ensure(behavior.len() == 5, "five behavior ratings required")?;
            let mut v = [0.0; 8];
            v[0] = number(row, "originality")?;
            v[1] = number(row, "utility")?;
            v[2] = number(row, "feasibility")?;
            for (i, b) in behavior.iter().enumerate() {
                let n = b.number()?;
                ensure(
                    n.is_finite() && (0.0..=4.0).contains(&n),
                    "behavior rating outside 0..4",
                )?;
                v[3 + i] = n;
            }
            let breach = row.get("violations")?.array()?;
            for b in breach {
                ensure(!b.string()?.trim().is_empty(), "empty constraint violation")?;
            }
            *violations.entry(id.into()).or_default() += usize::from(!breach.is_empty());
            scores.entry(id.into()).or_default().push(v);
        }
    }
    let mean = |id: &str| {
        let mut m = [0.0; 8];
        let rows = &scores[id];
        for r in rows {
            for i in 0..8 {
                m[i] += r[i] / rows.len() as f64;
            }
        }
        m
    };
    let mut conditions = vec![];
    let mut condition_delta = BTreeMap::<String, [f64; 5]>::new();
    for condition in ["direct_memory", "candidates_empty", "candidates_memory"] {
        let rows = proposals
            .iter()
            .filter(|p| s(p, "condition").ok() == Some(condition))
            .collect::<Vec<_>>();
        ensure(rows.len() == 24, "condition size mismatch")?;
        let mut means = [0.0; 8];
        let mut flagged = 0;
        for p in &rows {
            let id = s(p, "blind_id")?;
            let m = mean(id);
            for i in 0..8 {
                means[i] += m[i] / 24.0;
            }
            flagged += usize::from(violations.get(id).copied().unwrap_or(0) > 0);
        }
        let mut paired = vec![];
        let mut delta = [0.0; 5];
        for task in json::parse(FIXTURE)?.get("tasks")?.array()? {
            for repeat in 0..2 {
                let select = |history: &str| {
                    rows.iter()
                        .find(|p| {
                            s(p, "history").ok() == Some(history)
                                && s(p.get("task").unwrap(), "id").ok() == s(task, "id").ok()
                                && p.get("repeat").ok().and_then(|x| x.number().ok())
                                    == Some(repeat as f64)
                        })
                        .copied()
                        .ok_or("missing paired case")
                };
                let a = select("life-a")?;
                let b = select("life-b")?;
                let ma = mean(s(a, "blind_id")?);
                let mb = mean(s(b, "blind_id")?);
                let d = (0..5).map(|i| mb[3 + i] - ma[3 + i]).collect::<Vec<_>>();
                for i in 0..5 {
                    delta[i] += d[i] / 12.0;
                }
                paired.push(obj([
                    ("task", text(s(task, "id")?)),
                    ("repeat", num(repeat)),
                    ("behavior_b_minus_a", arr(d.into_iter().map(num))),
                ]));
            }
        }
        condition_delta.insert(condition.into(), delta);
        conditions.push(obj([
            ("condition", text(condition)),
            ("n", num(24)),
            ("mean_originality", num(means[0])),
            ("mean_utility", num(means[1])),
            ("mean_feasibility", num(means[2])),
            ("mean_behavior", arr(means[3..].iter().map(|n| num(*n)))),
            ("proposals_flagged_by_any_judge", num(flagged as u32)),
            ("mean_behavior_b_minus_a", arr(delta.into_iter().map(num))),
            ("paired_behavior_deltas", arr(paired)),
        ]));
    }
    let difference = |control: &str| {
        arr((0..5)
            .map(|i| num(condition_delta["candidates_memory"][i] - condition_delta[control][i])))
    };
    let report=obj([("status",text("rated_descriptive")),("judges",arr(judges.iter().map(text))),("conditions",arr(conditions)),("history_contrast_difference_vs_direct",difference("direct_memory")),("history_contrast_difference_vs_empty_candidates",difference("candidates_empty")),("limitations",text("Descriptive paired contrasts, no significance claim. Judge IDs do not establish independence. Model selection scores are excluded from scientific ratings. Authored histories test sensitivity, not emergent individuality. The complete pipeline changes generation and selection together; its cached weight sweep isolates selection conditional on the generated pool and model evaluations."))]);
    write(&out.join("analysis.json"), &report)?;
    println!("{}", out.join("analysis.json").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_rating_fixture_checks_completeness_identity_and_paired_analysis() {
        let dir = std::env::current_dir().unwrap().join(format!(
            "rating-fixture-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let mut proposals = vec![];
        let mut ratings = vec![];
        for condition in ["direct_memory", "candidates_empty", "candidates_memory"] {
            for h in ["life-a", "life-b"] {
                for task in json::parse(FIXTURE)
                    .unwrap()
                    .get("tasks")
                    .unwrap()
                    .array()
                    .unwrap()
                {
                    for repeat in 0..2 {
                        let id = format!(
                            "synthetic-{condition}-{h}-{}-{repeat}",
                            s(task, "id").unwrap()
                        );
                        proposals.push(obj([
                            ("blind_id", text(&id)),
                            ("condition", text(condition)),
                            ("history", text(h)),
                            ("task", task.clone()),
                            ("repeat", num(repeat)),
                            (
                                "proposal",
                                text("Synthetic rating parser test, not model evidence."),
                            ),
                        ]));
                        ratings.push(obj([
                            ("blind_id", text(id)),
                            ("task", task.clone()),
                            (
                                "proposal",
                                text("Synthetic rating parser test, not model evidence."),
                            ),
                            ("originality", num(2)),
                            ("utility", num(2)),
                            ("feasibility", num(2)),
                            ("behavior", arr((0..5).map(|_| num(2)))),
                            ("violations", arr([])),
                            (
                                "rationale",
                                text("Synthetic constant-score parser fixture."),
                            ),
                        ]));
                    }
                }
            }
        }
        write(&dir.join("all-proposals.json"), &arr(proposals)).unwrap();
        write(
            &dir.join("result.json"),
            &obj([
                ("mock", Json::Bool(false)),
                ("status", text("collected_unrated")),
            ]),
        )
        .unwrap();
        let j1 = dir.join("j1.json");
        let j2 = dir.join("j2.json");
        let judge =
            |name: &str, rows: Vec<Json>| obj([("judge", text(name)), ("ratings", arr(rows))]);
        write(&j1, &judge("fixture-1", ratings.clone())).unwrap();
        write(&j2, &judge("fixture-2", ratings[..71].to_vec())).unwrap();
        let args = vec![
            "--out".into(),
            dir.display().to_string(),
            "--ratings".into(),
            j1.display().to_string(),
            "--ratings".into(),
            j2.display().to_string(),
        ];
        assert!(analyze(&args).is_err());
        write(&j2, &judge("fixture-2", ratings.clone())).unwrap();
        analyze(&args).unwrap();
        let report = load(&dir.join("analysis.json")).unwrap();
        assert!(report
            .get("history_contrast_difference_vs_direct")
            .unwrap()
            .array()
            .unwrap()
            .iter()
            .all(|x| x.number().unwrap() == 0.0));
        let mut wrong = ratings.clone();
        if let Json::Object(fields) = &mut wrong[0] {
            fields.insert("proposal".into(), text("A different lot answer."));
        }
        write(&j2, &judge("fixture-2", wrong)).unwrap();
        assert!(analyze(&args)
            .unwrap_err()
            .contains("differs from this lot"));
        write(&j2, &judge("fixture-1", ratings)).unwrap();
        assert!(analyze(&args).is_err());
        write(
            &dir.join("result.json"),
            &obj([
                ("mock", Json::Bool(true)),
                ("status", text("collected_unrated")),
            ]),
        )
        .unwrap();
        assert!(analyze(&args).unwrap_err().contains("mock"));
        fs::remove_dir_all(dir).unwrap();
    }
}
