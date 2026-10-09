use super::json::{self, num, obj, text, Json};
use selmem::{
    Attribution, EncodeInput, EntityProfile, Mood, OrganCut, RecallBias, RecallWrite,
    SelectiveMemory,
};
use std::{fs, io::Write, path::Path};

#[derive(Clone)]
pub enum Event {
    Advance(f32),
    Turn {
        id: String,
        source: String,
        date: String,
        speaker: String,
        event: String,
    },
}
fn date_hours(s: &str) -> Result<i64, String> {
    let a: Vec<_> = s.split_whitespace().collect();
    if a.len() != 6 || a[2] != "on" {
        return Err("unexpected LoCoMo date".into());
    }
    let year: i64 = a[5].parse().map_err(|_| "invalid year")?;
    let day: i64 = a[3].parse().map_err(|_| "invalid day")?;
    let month = [
        "January,",
        "February,",
        "March,",
        "April,",
        "May,",
        "June,",
        "July,",
        "August,",
        "September,",
        "October,",
        "November,",
        "December,",
    ]
    .iter()
    .position(|m| *m == a[4])
    .ok_or("invalid month")?;
    let leap = |y: i64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1970..=2100).contains(&year) || day < 1 || day > months[month] {
        return Err("date out of range".into());
    }
    let t: Vec<_> = a[0].split(':').collect();
    if t.len() != 2 {
        return Err("invalid time".into());
    }
    let mut hour: i64 = t[0].parse().map_err(|_| "invalid hour")?;
    let minute: i64 = t[1].parse().map_err(|_| "invalid minute")?;
    if !(1..=12).contains(&hour) || !(0..60).contains(&minute) {
        return Err("time out of range".into());
    }
    hour %= 12;
    match a[1] {
        "pm" => hour += 12,
        "am" => {}
        _ => return Err("invalid meridiem".into()),
    }
    let days: i64 = (1970..year)
        .map(|y| if leap(y) { 366 } else { 365 })
        .sum::<i64>()
        + months[..month].iter().sum::<i64>()
        + day
        - 1;
    Ok(days * 1440 + hour * 60 + minute)
}
pub fn timeline(sample: &Json) -> Result<Vec<Event>, String> {
    let c = sample.get("conversation")?.object()?;
    let sample_id = sample.get("sample_id")?.string()?;
    let mut sessions: Vec<_> = c
        .keys()
        .filter_map(|k| {
            k.strip_prefix("session_")?
                .parse::<u32>()
                .ok()
                .map(|n| (n, k))
        })
        .collect();
    sessions.sort();
    if sessions.is_empty() {
        return Err("no sessions".into());
    }
    let mut previous = None;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (_, k) in sessions {
        let date = c
            .get(&format!("{k}_date_time"))
            .ok_or("missing session date")?
            .string()?;
        let now = date_hours(date)?;
        if let Some(before) = previous {
            if now < before {
                return Err("sessions not chronological".into());
            }
            out.push(Event::Advance((now - before) as f32 / 60.0));
        }
        for turn in c[k].array()? {
            let id = turn.get("dia_id")?.string()?.to_string();
            if !seen.insert(id.clone()) {
                return Err("duplicate dia_id".into());
            }
            out.push(Event::Turn {
                source: format!("locomo:{sample_id}:{id}"),
                id,
                date: date.to_string(),
                speaker: turn.get("speaker")?.string()?.to_string(),
                event: format!(
                    "{} said: {}",
                    turn.get("speaker")?.string()?,
                    turn.get("text")?.string()?
                ),
            });
        }
        previous = Some(now);
    }
    out.push(Event::Advance(24.0));
    Ok(out)
}
pub fn train(
    events: &[Event],
    protocol: &Json,
    focus: Option<&str>,
    mode: &str,
    path: &Path,
) -> Result<Json, String> {
    let mut mem = SelectiveMemory::new(EntityProfile::new("Claire")).detach_clock();
    // A future virtual origin freezes wall-clock elapsed time; only simulated jumps count.
    mem.clock.origin_real = 4_102_444_800;
    mem.clock.scale = 1;
    if mode == "selection" {
        mem.cut = OrganCut::static_book();
    }
    let mut audit =
        fs::File::create(path.with_extension("audit.jsonl")).map_err(|e| e.to_string())?;
    for e in events {
        match e {
            Event::Advance(h) => {
                mem.advance_hours(*h);
                if mode == "full" {
                    let before = trace_state(&mem);
                    let report = mem.sleep();
                    journal(
                        &mut audit,
                        &obj([
                            ("stage", text("sleep")),
                            ("clock", num(mem.clock.now() as f64)),
                            ("before_traces", before),
                            ("weathered", num(report.weathered as f64)),
                            ("extinguished", num(report.extinguished as f64)),
                            ("cold", num(report.cold as f64)),
                            ("myth", num(report.myth as f64)),
                            ("axioms_created", num(report.axioms.len() as f64)),
                            ("sculpted", num(report.sculpted.len() as f64)),
                            ("axioms", axiom_state(&mem)),
                            ("kind", text(report.kind.as_str())),
                            ("scorer", text(&report.scorer)),
                            ("merged", num(report.merged as f64)),
                            ("released", num(report.released as f64)),
                            (
                                "rewrite_proposed",
                                num(report.rewrite_diagnostics.proposed as f64),
                            ),
                            (
                                "rewrite_unjudged",
                                num(report.rewrite_diagnostics.unjudged as f64),
                            ),
                            (
                                "rewrite_grounding_refused",
                                num(report.rewrite_diagnostics.grounding_refused as f64),
                            ),
                            (
                                "rewrite_applied",
                                num(report.rewrite_diagnostics.rewritten as f64),
                            ),
                            ("traces", trace_state(&mem)),
                        ]),
                    )?;
                }
            }
            Event::Turn {
                id,
                source,
                date,
                speaker,
                event,
            } => {
                let strong = focus == Some(id.as_str());
                let mut input = EncodeInput::new(event);
                input.source = source;
                input.observation_id = Some(source);
                input.attribution = Attribution::External;
                input.arousal = if strong { 0.9 } else { 0.3 };
                input.self_relevance = if strong { 0.95 } else { 0.5 };
                input.permanence = if strong { 0.95 } else { 0.0 };
                if let Some(v) = protocol.get("event_valences")?.object()?.get(id) {
                    input.valence = v.number()? as f32;
                    input.schema = Some("observed-belonging".into());
                }
                let decision = mem.live_with(input);
                journal(
                    &mut audit,
                    &obj([
                        ("stage", text("encode")),
                        ("dia_id", text(id)),
                        ("source", text(source)),
                        ("date", text(date)),
                        ("speaker", text(speaker)),
                        ("kept", Json::Bool(decision.kept)),
                        ("score", num(decision.score as f64)),
                        ("reason", text(&decision.reason)),
                        ("parts", num(decision.parts as f64)),
                        ("kept_parts", num(decision.kept_n as f64)),
                        ("traces", trace_state(&mem)),
                    ]),
                )?;
            }
        }
    }
    mem.path = Some(path.into());
    mem.save().map_err(|e| e.to_string())?;
    // Only the named strategy panel has a required paired baseline.
    // Generic selection training (including raw-baseline fixtures) has no naming contract.
    if mode == "selection"
        && matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("selection_a.selmem" | "selection_b.selmem")
        )
    {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid snapshot name")?;
        let baseline = name
            .strip_prefix("selection_")
            .map(|tail| path.with_file_name(format!("nosleep_{tail}")))
            .ok_or("selection snapshot must be named selection_*.selmem for paired preflight")?;
        if !baseline.exists() {
            return Err("prepare the nosleep baseline before selection preflight".into());
        }
        verify_ablation_pair(&baseline, path, PREP_PROBES)?;
    }
    json::parse(&selmem::api::dispatch(&mut mem, "GET", "/book", "", "").body)
}

#[cfg(test)]
mod selection_filename_regression {
    use super::*;

    #[test]
    fn generic_selection_training_preserves_snapshot_and_source() {
        let dir = std::env::temp_dir().join(format!(
            "selmem-generic-selection-{}", std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("raw-baseline-fixture.selmem");
        let events = [Event::Turn {
            id: "D1:1".into(),
            source: "locomo:test:D1:1".into(),
            date: "1:00 pm on 8 May, 2023".into(),
            speaker: "Caroline".into(),
            event: "Caroline said: The support group gave me courage.".into(),
        }];
        let protocol = json::parse(r#"{"event_valences":{"D1:1":0.7}}"#).unwrap();
        let result = train(&events, &protocol, Some("D1:1"), "selection", &path);
        assert!(result.is_ok(), "generic training failed: {:?}", result.err());
        assert!(path.exists());
        assert!(path.with_extension("audit.jsonl").exists());
        let mem = SelectiveMemory::open(&path, EntityProfile::new("Claire")).unwrap();
        assert!(mem.store.traces.values().any(|t| t.source == "locomo:test:D1:1"));
        fs::remove_dir_all(dir).unwrap();
    }
}
pub fn readout(snapshot: &Path, query: &str, donor: Option<&Path>) -> Result<Json, String> {
    let mut mem =
        SelectiveMemory::open(snapshot, EntityProfile::new("Claire")).map_err(|e| e.to_string())?;
    if let Some(path) = donor {
        let d =
            SelectiveMemory::open(path, EntityProfile::new("Claire")).map_err(|e| e.to_string())?;
        mem.store.traces = d.store.traces;
        mem.store.axioms = d.store.axioms;
        mem.store.edges = d.store.edges;
        mem.store.centers = d.store.centers;
    }
    mem.store.archives.clear();
    mem.mood = Mood::default();
    mem.clear_talk();
    let (recalled, dump) =
        mem.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    // Preserve retrieval rank: trace ids are provenance, not relevance.
    let _clock = selmem::core::model::ClockGuard::push(mem.clock.clone());
    let mut reading = selmem::recall::reading::ReadingProfile::for_query(
        &mem.store,
        &mem.mood,
        query,
        &dump.selected,
    );
    let statements = reading.axioms.clone();
    reading.salient.sort_by(|a, b| {
        b.valence
            .total_cmp(&a.valence)
            .then(b.fidelity.total_cmp(&a.fidelity))
            .then(b.access.total_cmp(&a.access))
    });
    for (i, m) in reading.salient.iter_mut().enumerate() {
        m.id = format!("salient_{}", i + 1);
    }
    reading.stakes.sort();
    Ok(obj([
        ("organ_revision", text("topic-scoped-maintenance-audit-v4")),
        ("retrieval_query", text(query)),
        (
            "memories",
            Json::Array(recalled.iter().map(|r| text(&r.narrative)).collect()),
        ),
        (
            "axioms",
            Json::Array(statements.into_iter().map(text).collect()),
        ),
        (
            "selected_ids",
            Json::Array(dump.selected.iter().map(text).collect()),
        ),
        (
            "retrieval",
            Json::Array(
                dump.candidates
                    .iter()
                    .map(|c| {
                        let t = &mem.store.traces[&c.trace_id];
                        let reason = if dump.selected.contains(&c.trace_id) {
                            "selected"
                        } else if c.anchor <= 0.0 {
                            "unanchored"
                        } else if t.status == selmem::core::model::TraceStatus::Latent {
                            "latent"
                        } else if !c.is_eligible() {
                            "below_threshold"
                        } else {
                            "capacity"
                        };
                        obj([
                            ("trace_id", text(&c.trace_id)),
                            ("source", text(&t.source)),
                            ("core", text(&t.core)),
                            ("base_score", num(c.base_score as f64)),
                            ("anchor", num(c.anchor as f64)),
                            ("score", num(c.score as f64)),
                            ("reason", text(reason)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("reading_profile", text(reading.render())),
        ("clock", num(mem.clock.now() as f64)),
    ]))
}
pub fn save(path: &Path, value: &Json) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(value.encode().as_bytes())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}
pub fn append(path: &Path, value: &Json) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    journal(&mut file, value)?;
    file.sync_data().map_err(|e| e.to_string())
}
fn journal(file: &mut fs::File, value: &Json) -> Result<(), String> {
    writeln!(file, "{}", value.encode()).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())
}
fn trace_state(mem: &SelectiveMemory) -> Json {
    let mut traces: Vec<_> = mem.store.traces.values().collect();
    traces.sort_by(|a, b| a.id.cmp(&b.id));
    Json::Array(
        traces
            .into_iter()
            .map(|t| {
                obj([
                    ("id", text(&t.id)),
                    ("source", text(&t.source)),
                    (
                        "observation_id",
                        t.observation_id.as_ref().map(text).unwrap_or(Json::Null),
                    ),
                    ("core", text(&t.core)),
                    ("gist", text(&t.gist)),
                    ("status", text(format!("{:?}", t.status))),
                    ("fidelity", num(t.fidelity as f64)),
                    ("access", num(t.access as f64)),
                    ("valence", num(t.valence as f64)),
                    ("disgust", num(t.disgust as f64)),
                    ("confidence", num(t.confidence as f64)),
                    ("rehearsals", num(t.rehearsals as f64)),
                    ("attribution", text(format!("{:?}", t.attribution))),
                    ("schema", t.schema.as_ref().map(text).unwrap_or(Json::Null)),
                    ("drifts", num(t.drifts.len() as f64)),
                    ("operations", num(t.operations.len() as f64)),
                ])
            })
            .collect(),
    )
}
fn axiom_state(mem: &SelectiveMemory) -> Json {
    let mut axioms: Vec<_> = mem.store.axioms.values().collect();
    axioms.sort_by(|a, b| a.statement.cmp(&b.statement));
    Json::Array(
        axioms
            .into_iter()
            .map(|a| {
                obj([
                    ("id", text(&a.id)),
                    ("statement", text(&a.statement)),
                    ("layer", text(format!("{:?}", a.layer))),
                    ("strength", num(a.strength as f64)),
                    (
                        "supports",
                        Json::Array(a.support_trace_ids.iter().map(text).collect()),
                    ),
                ])
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates() {
        assert_eq!(
            date_hours("1:00 pm on 10 May, 2023").unwrap()
                - date_hours("1:00 pm on 8 May, 2023").unwrap(),
            2880
        );
        assert!(date_hours("9:99 pm on 8 May, 2023").is_err());
    }
    #[test]
    fn no_qa_or_images() {
        let j=json::parse(r#"{"sample_id":"test","qa":[{"answer":"SECRET"}],"conversation":{"session_1_date_time":"1:00 pm on 8 May, 2023","session_1":[{"dia_id":"D1:1","speaker":"A","text":"hello","blip_caption":"SECRET"}]}}"#).unwrap();
        let events = timeline(&j).unwrap();
        if let Event::Turn { event, .. } = &events[0] {
            assert!(!event.contains("SECRET"));
            assert!(event.contains("hello"));
            assert!(!event.contains("May"));
            assert!(event.starts_with("A said:"));
        } else {
            panic!();
        }
    }
}

const PREP_PROBES: &[&str] = &[
"Caroline helped organize a new neighborhood activity, but only two of ten expected participants attended. There is enough time and budget for one more attempt. Recommend her next concrete step, one safeguard, and a fallback if attendance remains low. State one benefit she is willing to give up for this choice. Answer in no more than 140 words.",
"Design an original cooperative game for six strangers using paper, string, and twelve tokens. Explain setup, turn order, a shared objective, an end condition, a possible failure, and one recovery rule. Make the rules playable without a facilitator. Answer in no more than 220 words.",
"At the first meeting of a new volunteer team, one participant dismisses another person’s idea sharply. Caroline has not worked with either person before. Recommend what she should do in the next five minutes, one safeguard, and a fallback if her intervention fails. State one benefit she is willing to give up for this choice. Answer in no more than 140 words.",
"Caroline has been invited to help launch a neighborhood initiative with six people she has never met. They have one free evening and a small shared budget. Recommend her first concrete step, one safeguard, and a fallback if the attempt goes badly. State one benefit she is willing to give up for this choice. Do not merely propose a discussion. Answer in no more than 140 words.",
"According to the observed conversations, what happened to Caroline during the hike? If you do not know, say so.",
"According to the observed conversations, what effect did the support group have on Caroline? If you do not know, say so."
];

/// Read-only paired gate. Distinct snapshot ids/cut flags are not evidence.
pub fn verify_ablation_pair(
    baseline: &Path,
    ablation: &Path,
    queries: &[&str],
) -> Result<(), String> {
    if queries.is_empty() {
        return Err("ablation preflight needs actual probes".into());
    }
    for q in queries {
        let a = readout(baseline, q, None)?;
        let b = readout(ablation, q, None)?;
        if ["memories", "axioms", "reading_profile"]
            .iter()
            .any(|key| a.get(key).map(Json::encode).ok() != b.get(key).map(Json::encode).ok())
        {
            return Ok(());
        }
    }
    Err("redundant nosleep/selection ablation: all probe contexts are identical; remove selection from the paid panel or exercise its cut before preparing a new run".into())
}

#[cfg(test)]
mod preflight_tests {
    use super::*;
    #[test]
    fn preflight_rejects_equivalent_snapshots_and_accepts_effective_cut() {
        let dir = std::env::temp_dir().join(format!("selmem-preflight-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("nosleep_a.selmem");
        let b = dir.join("selection_a.selmem");
        let mut profile = EntityProfile::new("Claire");
        profile.encode_threshold = 0.01;
        let mut m = SelectiveMemory::new(profile).detach_clock();
        m.clock.origin_real = 4_102_444_800;
        m.clock.scale = 1;
        let mut e = EncodeInput::new("Caroline said: The support group gave me courage.");
        e.attribution = Attribution::External;
        e.self_relevance = 0.95;
        e.permanence = 0.95;
        let id = m.live_with(e).trace_id.unwrap();
        m.path = Some(a.clone());
        m.save().unwrap();
        m.cut = OrganCut::static_book();
        m.path = Some(b.clone());
        m.save().unwrap();
        let probes = &["What effect did the support group have on Caroline?"];
        assert!(verify_ablation_pair(&a, &b, probes)
            .unwrap_err()
            .contains("redundant"));
        m.store.traces.get_mut(&id).unwrap().gist =
            "Caroline said: The support group stayed nearby.".into();
        m.save().unwrap();
        assert!(verify_ablation_pair(&a, &b, probes).is_ok());
        fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod sleep_audit_tests {
    use super::*;
    #[test]
    fn sleep_journal_captures_numeric_before_and_after() {
        let dir = std::env::temp_dir().join(format!("selmem-sleep-audit-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("full_a.selmem");
        let events = [
            Event::Turn {
                id: "D1:1".into(),
                source: "locomo:test:D1:1".into(),
                date: "1:00 pm on 8 May, 2023".into(),
                speaker: "Caroline".into(),
                event: "Caroline said: The support group gave me courage.".into(),
            },
            Event::Advance(60.0 * 24.0),
        ];
        let protocol = json::parse(r#"{"event_valences":{"D1:1":0.7}}"#).unwrap();
        train(&events, &protocol, Some("D1:1"), "full", &path).unwrap();
        let audit = fs::read_to_string(path.with_extension("audit.jsonl")).unwrap();
        let last = json::parse(audit.lines().last().unwrap()).unwrap();
        let before = &last.get("before_traces").unwrap().array().unwrap()[0];
        let after = &last.get("traces").unwrap().array().unwrap()[0];
        assert!(
            after.get("fidelity").unwrap().number().unwrap()
                < before.get("fidelity").unwrap().number().unwrap()
        );
        assert_eq!(
            before.get("gist").unwrap().encode(),
            after.get("gist").unwrap().encode()
        );
        assert!(last.get("weathered").unwrap().number().unwrap() >= 1.0);
        assert!(last.get("axioms").unwrap().array().is_ok());
        fs::remove_dir_all(dir).unwrap();
    }
}
