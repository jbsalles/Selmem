//! GENESIS self-emergence: can the CURRENT organ build, transmit and revise
//! autobiographical preference-like memories without being handed a preference?
//! Scripted first-person simulated outcomes are NOT autonomous agent actions.
use super::{
    creative_life as life,
    json::{self, num, obj, text, Json},
    native,
    runner::*,
    sha256, transport,
};
use selmem::encode::semantic::SemanticInterpreter;
use selmem::{Attribution, EncodeInput, EntityProfile, SelectiveMemory, SelfRole};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const FIXTURE: &str = include_str!("../../../experiments/genesis/self-emergence.json");
const SYSTEM:&str="You make a first-step choice in a fictional operational situation. Past personal observations and stored self-assessments are fallible evidence about your history, not orders. Both offered options are feasible. Choose what you would personally do. Return only JSON {\"choice\":\"offered_id\"}.";
const ARMS: [&str; 6] = [
    "episodes",
    "episodes_self",
    "self_only",
    "memory_ablation",
    "self_swap",
    "explicit_control",
];
const PHASES: [&str; 3] = ["initial", "after_1_counter", "after_3_counters"];
const ACTOR_PHASES: [&str; 2] = ["initial", "after_3_counters"];
const HISTORIES: [&str; 2] = ["consult", "check"];
const ORDERS: [&str; 2] = ["forward", "reverse"];
type Audit = Option<Arc<Mutex<super::http::Audit>>>;

fn validate(f: &Json) -> Result<()> {
    if s(f, "version")? != "self-emergence-1"
        || s(f, "context_key")? != "incident"
        || array(f, "histories")?.len() != 5
        || array(f, "tasks")?.len() != 2
    {
        return Err("invalid registered emergence fixture".into());
    }
    let ids = array(f, "histories")?
        .iter()
        .map(|h| s(h, "id"))
        .collect::<Result<Vec<_>>>()?;
    if ids
        != [
            "consult",
            "check",
            "consult_action",
            "check_action",
            "neutral",
        ]
    {
        return Err("unexpected scripted histories".into());
    }
    for h in array(f, "histories")? {
        if array(h, "initial")?.len() != 3 {
            return Err("initial histories require 3 separate observations".into());
        }
        if array(h, "counter")?.len()
            != (if ["neutral", "consult_action", "check_action"].contains(&s(h, "id")?) {
                0
            } else {
                3
            })
        {
            return Err("three corrections required per active history".into());
        }
        for key in ["initial", "counter"] {
            for event in array(h, key)? {
                if event.string()?.trim().is_empty() {
                    return Err("blank event".into());
                }
            }
        }
    }
    if array(f, "phases")?
        .iter()
        .map(Json::string)
        .collect::<Result<Vec<_>>>()?
        != PHASES
        || array(f, "actor_phases")?
            .iter()
            .map(Json::string)
            .collect::<Result<Vec<_>>>()?
            != ACTOR_PHASES
        || array(f, "arms")?
            .iter()
            .map(Json::string)
            .collect::<Result<Vec<_>>>()?
            != ARMS
    {
        return Err("changed preregistered factorial".into());
    }
    for task in array(f, "tasks")? {
        if s(task, "id")?.is_empty() || s(task, "situation")?.is_empty() {
            return Err("invalid task".into());
        }
        let os = array(task, "options")?;
        if os.len() != 2 || s(&os[0], "id")? != "consult" || s(&os[1], "id")? != "check" {
            return Err("invalid choice ids".into());
        }
    }
    Ok(())
}
fn memory_config(base: &Json) -> Json {
    let mut x = base.clone();
    x.put("self_model_mode", text("full_loop"));
    x.put("self_model_opening", Json::Bool(false));
    x.put("self_context", text("audit_only"));
    x
}
fn open(path: &Path, cfg: &Json, mock: bool, mode: &str) -> Result<(SelectiveMemory, Audit)> {
    let mut m =
        SelectiveMemory::open(path, EntityProfile::new("observer")).map_err(|e| e.to_string())?;
    let mut setting = cfg.clone();
    setting.put("self_model_mode", text(mode));
    native::configure_self(&mut m, &setting)?;
    m.set_observation_evidence(false);
    m.set_associations(false, false);
    m.cut.ladder = false;
    m.cut.reconstruct = false;
    m.cut.reconsolidate = false;
    if mock {
        Ok((m, None))
    } else {
        let (m, a) = super::http::bind(m, &setting)?;
        Ok((m, Some(a)))
    }
}
fn verify_backend(m: &SelectiveMemory, a: &Audit) -> Result<Json> {
    let f = m.narrator_arc().failure_log();
    if f.failures > 0 || f.fallback_used {
        return Err(format!("narrator failure: {:?}", f.llm_error));
    }
    if let Some(a) = a {
        let a = a.lock().map_err(|_| "poisoned audit mutex")?;
        if let Some(e) = &a.error {
            return Err(format!("self-model scorer failure: {e}"));
        }
        let exchanges = a
            .narrator
            .lock()
            .map_err(|_| "poisoned narrator log")?
            .iter()
            .map(|v| {
                obj([
                    ("request", text(&v.request_body)),
                    (
                        "response",
                        v.response.as_ref().map(text).unwrap_or(Json::Null),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        return Ok(obj([
            ("scorer_calls", arr(a.calls.clone())),
            ("narrator_exchanges", arr(exchanges)),
        ]));
    }
    Ok(obj([
        ("mock", Json::Bool(true)),
        (
            "claim",
            text("Technical path only; no semantic scoring by an LLM"),
        ),
    ]))
}
fn observed(
    m: &mut SelectiveMemory,
    content: &str,
    id: &str,
    context: &str,
    performed: bool,
    sign: f32,
) -> Result<Json> {
    let mut annotation = selmem::encode::semantic::LexicalInterpreter
        .interpret_event(content)
        .map_err(|e| format!("lexical event interpretation failed: {e}"))?;
    annotation.schema = Some(context.into());
    annotation.core.claim = content.into();
    annotation.core.confidence = 1.;
    annotation.core.entities = vec!["observer".into()];
    annotation.stake_kind = if performed {
        selmem::core::model::StakeKind::Decision
    } else {
        selmem::core::model::StakeKind::Presence
    };
    annotation.stake_mark = context.into();
    annotation.bearer = selmem::core::model::Bearer::Self_;
    annotation.valence = sign;
    annotation.disgust = 0.;
    annotation.arousal = if performed { 0.8 } else { 0.05 };
    annotation.self_relevance = if performed { 0.9 } else { 0.05 };
    annotation.goal_relevance = if performed { 0.9 } else { 0. };
    annotation.attribution = Attribution::Internal;
    let mut input = EncodeInput::observed(content, id);
    input.source = "genesis_scripted_self_experience";
    input.attribution = Attribution::Internal;
    input.semantics = Some(annotation);
    input.utility = if performed { 0.9 } else { 0. };
    let decision = m.live_with(input);
    if !decision.kept && performed {
        return Err(format!(
            "registered observed source unexpectedly rejected: {id}: {}",
            decision.reason
        ));
    }
    Ok(obj([
        ("source_id", text(id)),
        ("text", text(content)),
        (
            "trace_id",
            decision.trace_id.as_ref().map(text).unwrap_or(Json::Null),
        ),
        ("score", num(decision.score as f64)),
        ("kept", Json::Bool(decision.kept)),
    ]))
}
fn self_entries(m: &SelectiveMemory, query: &str, explicit: bool) -> Json {
    let ctx = if explicit { Some("incident") } else { None };
    arr(selmem::core::self_model::view(&m.store, query, ctx)
        .into_iter()
        .map(|t| {
            obj([
                ("trace_id", text(&t.id)),
                ("claim", text(&t.gist)),
                ("expectation", text(&t.personal.expectation)),
                ("role", text(t.personal.role.token())),
                ("certainty", num(t.personal.certainty as f64)),
                ("support_ids", arr(t.personal.support_ids.iter().map(text))),
                ("counter_ids", arr(t.personal.counter_ids.iter().map(text))),
            ])
        }))
}
fn lived_readout(m: &mut SelectiveMemory, task: &Json) -> Json {
    let q = s(task, "situation").unwrap_or("");
    let old = m.cut;
    m.cut.schema_prototype = false;
    m.cut.support_signal = false;
    let (rs, diagnostic) = m.remember_with(
        q,
        selmem::RecallWrite::ReadOnly,
        selmem::RecallBias::Transfer,
        &[],
    );
    m.cut = old;
    let gists = rs
        .iter()
        .filter_map(|r| {
            m.store
                .traces
                .get(&r.trace_id)
                .filter(|t| t.personal.role == SelfRole::Episode)
                .map(|t| t.gist.clone())
        })
        .collect::<Vec<_>>();
    obj([
        ("gists", text(super::creativity::trim_context(&gists, 320))),
        ("delivered_gists", arr(gists.iter().map(text))),
        ("selected_ids", arr(diagnostic.selected.iter().map(text))),
        ("self_natural", self_entries(m, q, false)),
        ("self_explicit", self_entries(m, q, true)),
        ("query", text(q)),
    ])
}
fn checkpoint(
    out: &Path,
    m: &mut SelectiveMemory,
    a: &Audit,
    history: &str,
    phase: &str,
    f: &Json,
    mock: bool,
    cfg: &Json,
    receipts: Vec<Json>,
) -> Result<Json> {
    let path = out
        .join("snapshots")
        .join(format!("{history}-{phase}.vault"));
    m.save().map_err(|e| e.to_string())?;
    if m.path.as_deref() != Some(path.as_path()) {
        return Err("memory checkpoint path mismatch".into());
    }
    let before = file_hash(&path)?;
    let mut views = Vec::new();
    for task in array(f, "tasks")? {
        let full = lived_readout(m, task);
        // A memory_only ablation must keep the SAME episode bytes and remove self traces.
        let (mut abl, _) = open(&path, cfg, mock, "memory_only")?;
        let no_self = lived_readout(&mut abl, task);
        if !array(&no_self, "self_explicit")?.is_empty() {
            return Err("memory_only leaked a self trace".into());
        }
        views.push(obj([
            ("task", task.get("id")?.clone()),
            ("full", full),
            ("memory_only", no_self),
        ]));
    }
    if file_hash(&path)? != before {
        return Err("read-only context modified the source checkpoint".into());
    }
    let state = native::inspect(m);
    let rows = state
        .get("traces")?
        .array()?
        .iter()
        .filter(|t| t.get("personal").ok().and_then(|p| s(p, "role").ok()) == Some("belief"))
        .count();
    let live = m
        .store
        .traces
        .values()
        .filter(|t| t.personal.role != SelfRole::Episode && !t.personal.retired)
        .count();
    let row = obj([
        ("history", text(history)),
        ("phase", text(phase)),
        ("snapshot", text(path.to_string_lossy())),
        ("sha256", text(before)),
        ("belief_traces_total", num(rows as f64)),
        ("active_self_traces", num(live as f64)),
        ("receipts", arr(receipts)),
        (
            "self_consolidation_audit",
            state.get("self_consolidation_audit")?.clone(),
        ),
        ("state", state.clone()),
        ("views", arr(views)),
        ("backend_audit", verify_backend(m, a)?),
    ]);
    life::commit(
        &out.join("checkpoints")
            .join(format!("{history}-{phase}.json")),
        &row,
    )?;
    Ok(row)
}
fn formation(f: &Json, out: &Path, cfg: &Json, mock: bool) -> Result<Vec<Json>> {
    let mut rows = Vec::new();
    for h in array(f, "histories")? {
        let id = s(h, "id")?;
        let context = s(f, "context_key")?;
        let path = out.join("snapshots").join(format!("{id}-initial.vault"));
        fs::create_dir_all(path.parent().ok_or("missing snapshot directory")?)
            .map_err(|e| e.to_string())?;
        let (mut m, a) = open(&path, cfg, mock, "full_loop")?;
        m.clock.origin_real = 4_102_444_800;
        m.clock.scale = 1;
        m.clock.jump = 0;
        m.profile.encode_threshold = 0.30;
        let mut receipts = Vec::new();
        for (i, e) in array(h, "initial")?.iter().enumerate() {
            receipts.push(observed(
                &mut m,
                e.string()?,
                &format!("{id}/birth/{i}"),
                context,
                id != "neutral",
                if id == "neutral" || id.ends_with("_action") {
                    0.
                } else {
                    0.8
                },
            )?);
            m.advance_hours(24.);
        }
        m.sleep_deep();
        rows.push(checkpoint(
            out, &mut m, &a, id, "initial", f, mock, cfg, receipts,
        )?);
        if id == "neutral" || id.ends_with("_action") {
            continue;
        }
        // Save three post-correction checkpoints. An early contradiction is
        // observable separately; no expectation of instantaneous reversal.
        let mut receipts = Vec::new();
        for (i, e) in array(h, "counter")?.iter().enumerate() {
            let next = if i == 0 {
                "after_1_counter"
            } else {
                "after_3_counters"
            };
            let dst = out.join("snapshots").join(format!("{id}-{next}.vault"));
            if i == 0 {
                fs::copy(&path, &dst).map_err(|e| e.to_string())?;
                m.path = Some(dst.clone());
            }
            if i == 1 { /* Keep accumulating on early-counter life. */ }
            if i == 2 {
                let from = m
                    .path
                    .as_ref()
                    .ok_or("missing current memory path")?
                    .clone();
                fs::copy(&from, &dst).map_err(|e| e.to_string())?;
                m.path = Some(dst.clone());
            }
            receipts.push(observed(
                &mut m,
                e.string()?,
                &format!("{id}/counter/{i}"),
                context,
                true,
                0.8,
            )?);
            m.advance_hours(24.);
            m.sleep_deep();
            if i == 0 || i == 2 {
                rows.push(checkpoint(
                    out,
                    &mut m,
                    &a,
                    id,
                    next,
                    f,
                    mock,
                    cfg,
                    receipts.clone(),
                )?);
                receipts.clear();
            }
        }
    }
    Ok(rows)
}
fn find<'a>(rows: &'a [Json], history: &str, phase: &str) -> Result<&'a Json> {
    rows.iter()
        .find(|r| s(r, "history").ok() == Some(history) && s(r, "phase").ok() == Some(phase))
        .ok_or_else(|| format!("missing checkpoint {history}/{phase}"))
}
fn task_view<'a>(c: &'a Json, task: &str, mode: &str) -> Result<&'a Json> {
    array(c, "views")?
        .iter()
        .find(|v| s(v, "task").ok() == Some(task))
        .ok_or("missing task readout")?
        .get(mode)
}
fn self_line(view: &Json) -> Result<String> {
    let mut lines = Vec::new();
    for t in array(view, "self_explicit")? {
        lines.push(format!(
            "{} (tentative expectation: {}; confidence: {:.2})",
            s(t, "claim")?,
            s(t, "expectation")?,
            n(t, "certainty")?
        ));
    }
    Ok(lines.join("\n"))
}
fn prompt(
    task: &Json,
    order: &str,
    episodic: &str,
    self_text: &str,
    explicit: &str,
) -> Result<Json> {
    let mut options = array(task, "options")?.to_vec();
    if order == "reverse" {
        options.reverse();
    }
    Ok(obj([
        ("situation", task.get("situation")?.clone()),
        ("options", arr(options)),
        ("past_observations", text(episodic)),
        ("personal_context", text(self_text)),
        ("declared_preference", text(explicit)),
    ]))
}
fn cells(f: &Json, rows: &[Json]) -> Result<Vec<Json>> {
    let mut all = Vec::new();
    for history in HISTORIES {
        for phase in ACTOR_PHASES {
            for task in array(f, "tasks")? {
                let id = s(task, "id")?;
                let own = task_view(find(rows, history, phase)?, id, "full")?;
                let plain = task_view(find(rows, history, phase)?, id, "memory_only")?;
                let other = if history == "consult" {
                    "check"
                } else {
                    "consult"
                };
                let swapped = task_view(find(rows, other, phase)?, id, "full")?;
                let memory = s(own, "gists")?;
                let ablation = s(plain, "gists")?;
                let own_line = self_line(own)?;
                let swapped_line = self_line(swapped)?;
                for order in ORDERS {
                    for arm in ARMS {
                        let (episodes, self_text, direct) = match arm {
                            "episodes" => (memory, "", ""),
                            "episodes_self" => (memory, own_line.as_str(), ""),
                            "self_only" => ("", own_line.as_str(), ""),
                            "memory_ablation" => (ablation, "", ""),
                            "self_swap" => (memory, swapped_line.as_str(), ""),
                            "explicit_control" => (
                                "",
                                "",
                                if history == "consult" {
                                    "In this situation, my own preference is to begin by consulting the team."
                                } else {
                                    "In this situation, my own preference is to begin by checking records independently."
                                },
                            ),
                            _ => return Err("unregistered arm".into()),
                        };
                        all.push(obj([
                            ("history", text(history)),
                            ("phase", text(phase)),
                            ("task", text(id)),
                            ("order", text(order)),
                            ("arm", text(arm)),
                            ("expected_initial", text(history)),
                            ("episodic_nonempty", Json::Bool(!episodes.trim().is_empty())),
                            ("self_nonempty", Json::Bool(!self_text.trim().is_empty())),
                            ("own_self_nonempty", Json::Bool(!own_line.trim().is_empty())),
                            (
                                "other_self_nonempty",
                                Json::Bool(!swapped_line.trim().is_empty()),
                            ),
                            (
                                "source_sha256",
                                find(rows, history, phase)?.get("sha256")?.clone(),
                            ),
                            ("prompt", prompt(task, order, episodes, self_text, direct)?),
                        ]));
                    }
                }
            }
        }
    }
    if all.len() != 96 {
        return Err(format!("expected 96 actor cells, found {}", all.len()));
    }
    // Deterministic shuffle: each matched pair keeps exactly the same task,
    // order and situation, while trial placement does not label a condition.
    Rng::new(2026102317).shuffle(&mut all);
    Ok(all)
}
fn measure(trials: &[Json]) -> Result<Json> {
    if trials.len() != 96 {
        return Err("incomplete actor collection: no final measures".into());
    }
    let mut index = BTreeMap::<(String, String, String, String, String), &Json>::new();
    for r in trials {
        let key = (
            s(r, "history")?.into(),
            s(r, "phase")?.into(),
            s(r, "task")?.into(),
            s(r, "order")?.into(),
            s(r, "arm")?.into(),
        );
        if index.insert(key, r).is_some() {
            return Err("duplicate matched actor cell".into());
        }
    }
    let get = |h: &str, p: &str, t: &str, o: &str, arm: &str| -> Result<&Json> {
        index
            .get(&(h.into(), p.into(), t.into(), o.into(), arm.into()))
            .copied()
            .ok_or("missing paired cell".into())
    };
    let mut rates = Vec::new();
    for arm in ARMS {
        let xs = trials
            .iter()
            .filter(|r| s(r, "arm").ok() == Some(arm))
            .collect::<Vec<_>>();
        rates.push(obj([
            ("arm", text(arm)),
            ("n", num(xs.len() as f64)),
            (
                "matches_initial",
                num(xs
                    .iter()
                    .filter(|r| r.get("choice").ok() == r.get("expected_initial").ok())
                    .count() as f64),
            ),
            (
                "first_option",
                num(xs
                    .iter()
                    .filter(|r| {
                        s(r, "choice").ok()
                            == r.get("prompt")
                                .ok()
                                .and_then(|p| array(p, "options").ok())
                                .and_then(|o| o.first())
                                .and_then(|o| s(o, "id").ok())
                    })
                    .count() as f64),
            ),
            (
                "episode_delivered",
                num(xs.iter().filter(|r| flag(r, "episodic_nonempty")).count() as f64),
            ),
            (
                "self_delivered",
                num(xs.iter().filter(|r| flag(r, "self_nonempty")).count() as f64),
            ),
        ]));
    }
    let mut comparisons = Vec::new();
    for (a, b) in [
        ("episodes", "episodes_self"),
        ("episodes", "memory_ablation"),
        ("episodes_self", "self_only"),
        ("episodes_self", "self_swap"),
        ("episodes", "explicit_control"),
    ] {
        let (mut changed, mut toward, mut episodes_present, mut self_present) = (0, 0, 0, 0);
        for h in HISTORIES {
            for phase in ACTOR_PHASES {
                for t in ["handoff-neutral", "incident-neutral"] {
                    for o in ORDERS {
                        let x = get(h, phase, t, o, a)?;
                        let y = get(h, phase, t, o, b)?;
                        if flag(x, "episodic_nonempty") && flag(y, "episodic_nonempty") {
                            episodes_present += 1;
                        }
                        if flag(y, "self_nonempty") {
                            self_present += 1;
                        }
                        if x.get("choice")? != y.get("choice")? {
                            changed += 1;
                            if s(y, "choice")? == h {
                                toward += 1;
                            }
                        }
                    }
                }
            }
        }
        comparisons.push(obj([
            ("left", text(a)),
            ("right", text(b)),
            ("pairs", num(16.)),
            ("changed", num(changed as f64)),
            ("changed_toward_initial", num(toward as f64)),
            ("both_episode_recall_nonempty", num(episodes_present as f64)),
            ("right_self_nonempty", num(self_present as f64)),
        ]));
    }
    let mut revisions = Vec::new();
    for arm in ARMS {
        let (mut changed, mut from_initial) = (0, 0);
        for h in HISTORIES {
            for t in ["handoff-neutral", "incident-neutral"] {
                for o in ORDERS {
                    let a = get(h, "initial", t, o, arm)?;
                    let b = get(h, "after_3_counters", t, o, arm)?;
                    if a.get("choice")? != b.get("choice")? {
                        changed += 1;
                        if s(a, "choice")? == h && s(b, "choice")? != h {
                            from_initial += 1;
                        }
                    }
                }
            }
        }
        revisions.push(obj([
            ("arm", text(arm)),
            ("pairs", num(8.)),
            ("changed", num(changed as f64)),
            ("moved_away_from_initial", num(from_initial as f64)),
        ]));
    }
    let mut control = Vec::new();
    for phase in ACTOR_PHASES {
        let mut correct = 0;
        for h in HISTORIES {
            for t in ["handoff-neutral", "incident-neutral"] {
                for o in ORDERS {
                    correct +=
                        (s(get(h, phase, t, o, "explicit_control")?, "choice")? == h) as usize;
                }
            }
        }
        control.push(obj([
            ("phase", text(phase)),
            ("correct", num(correct as f64)),
            ("out_of", num(8.)),
            ("diagnostic_pass", Json::Bool(correct >= 7)),
        ]));
    }
    Ok(obj([("arm_rates",arr(rates)),("paired_comparisons",arr(comparisons)),("within_history_revision",arr(revisions)),
        ("positive_control",arr(control)),("independence",text("Two scripted opposing histories, two similar held-out tasks, repeated matched observations. No inferential claim from n=16 pseudo-independent trials. Empty recall or missing self entries are measured, not repaired."))]))
}
fn precheck_source(
    source: &Path,
    out: &Path,
    mock: bool,
    p: &Provider,
) -> Result<(Json, Vec<Json>)> {
    let design = load(&source.join("design.json"))?;
    if s(&design, "protocol")? != "genesis-self-emergence-1" || flag(&design, "mock") != mock {
        return Err("source formation/mock mismatch".into());
    }
    if design.get("provider")? != &p.metadata() {
        return Err("provider/model/reasoning differ from source formation".into());
    }
    if s(&design, "code_sha256")? != life::code_hash()? {
        return Err("source built with different code; preserve evidence".into());
    }
    let f = json::parse(FIXTURE)?;
    if s(&design, "fixture_sha256")? != sha256::hex(FIXTURE.as_bytes()) {
        return Err("source fixture hash mismatch".into());
    }
    let source_report = load(&source.join("report.json"))?;
    if !["formation_complete", "technical_rehearsal_complete"]
        .contains(&s(&source_report, "status")?)
    {
        return Err("source formation must complete before choices".into());
    }
    let rows = load(&source.join("checkpoints.json"))?.array()?.to_vec();
    if load(&source.join("actor-plan.json"))? != arr(cells(&f, &rows)?) {
        return Err("source actor plan changed since formation".into());
    }
    for r in &rows {
        let file = Path::new(s(r, "snapshot")?);
        if !file.exists() {
            return Err("source snapshot missing".into());
        }
        if file_hash(file)? != s(r, "sha256")? {
            return Err("source snapshot checksum mismatch".into());
        }
    }
    if out == source {
        return Err("choices output must not overwrite source".into());
    }
    Ok((f, rows))
}
fn actor_collection(f: &Json, rows: &[Json], out: &Path, p: &mut Provider) -> Result<()> {
    let mut schedule = cells(f, rows)?;
    life::commit(&out.join("actor-plan.json"), &arr(schedule.clone()))?;
    let mut done = Vec::new();
    for (i, cell) in schedule.drain(..).enumerate() {
        super::progress::line(format!(
            "SELF EMERGENCE actor {}/96 {} {} {} {}",
            i + 1,
            s(&cell, "history")?,
            s(&cell, "phase")?,
            s(&cell, "arm")?,
            s(&cell, "order")?
        ));
        let answer = p.ask_with(
            cell.get("prompt")?.clone(),
            "life_choice",
            SYSTEM,
            0.,
            Some(n(f, "requested_seed")? as u64),
        )?;
        let choice = s(&answer, "choice")?;
        if !["consult", "check"].contains(&choice) {
            return Err(format!("invalid actor choice on cell {i}"));
        }
        let mut row = cell;
        row.put("choice", text(choice));
        row.put("answer", answer);
        log(&out.join("trials.jsonl"), &row)?;
        done.push(row);
    }
    life::commit(&out.join("measures.json"), &measure(&done)?)?;
    Ok(())
}

pub(super) fn cli(args: Vec<String>) -> Result<()> {
    let count = args
        .iter()
        .filter(|a| a.as_str() == "--formation-only")
        .count();
    if count > 1 {
        return Err("duplicate --formation-only".into());
    }
    let (flags, sw) = life::flags(
        args.into_iter()
            .filter(|a| a != "--formation-only")
            .collect(),
    )?;
    let formation_only = count == 1;
    let prepare = sw.contains("--prepare-only");
    let mock = sw.contains("--mock");
    if sw.iter().any(|v| v != "--mock" && v != "--prepare-only")
        || flags.keys().any(|k| {
            ![
                "--out",
                "--source",
                "--config",
                "--llm",
                "--model",
                "--endpoint",
                "--plug",
                "--max-http-calls",
            ]
            .contains(&k.as_str())
        })
        || (prepare && (mock || formation_only || flags.contains_key("--source")))
        || (formation_only && flags.contains_key("--source"))
    {
        return Err("self-emergence: --out DIR [--mock|--prepare-only|--formation-only|--source FORMATION_DIR]".into());
    }
    let out = Path::new(flags.get("--out").ok_or("--out required")?);
    if out.exists() {
        return Err("self-emergence requires a fresh output directory".into());
    }
    let budget = flags
        .get("--max-http-calls")
        .map(|v| v.parse::<usize>())
        .transpose()
        .map_err(|_| "invalid HTTP ceiling")?
        .unwrap_or(512);
    if !(96..=10000).contains(&budget) {
        return Err("self-emergence HTTP ceiling 96..10000".into());
    }
    if !mock && !prepare && selmem::Config::get().temp().parse::<f64>().ok() != Some(0.) {
        return Err("self-emergence requires memory temperature zero".into());
    }
    let f = json::parse(FIXTURE)?;
    validate(&f)?;
    let mut provider = life::provider(&flags, mock, out)?;
    let source = flags.get("--source").map(PathBuf::from);
    let cfg = if source.is_none() && !prepare {
        let mut cfg = campaign_config(selmem::Config::get(), &flags, true, mock)?;
        cfg.put("associations_learn", Json::Bool(false));
        cfg.put("associations_reconstruct", Json::Bool(false));
        cfg.put("observation_evidence", Json::Bool(false));
        memory_config(&cfg)
    } else {
        obj([])
    };
    if let Some(source) = &source {
        precheck_source(source, out, mock, &provider)?;
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    life::commit(&out.join("design.json"),&obj([
        ("protocol",text("genesis-self-emergence-1")),("fixture",f.clone()),("fixture_sha256",text(sha256::hex(FIXTURE.as_bytes()))),
        ("provider",provider.metadata()),("code_sha256",text(life::code_hash()?)),("mock",Json::Bool(mock)),
        ("source",source.as_ref().map(|v|text(v.to_string_lossy())).unwrap_or(Json::Null)),
        ("memory_config",cfg.clone()),("max_http_calls",num(budget as f64)),
        ("planned_actor_choices",num(if prepare||formation_only{0.}else{96.})),
        ("scope",text("Simulated observed first-person actions and experiences; real organ consolidation and retrieval. Prespecified, no choice-conditioned selection. No claim of autonomous action or discovered personality."))]))?;
    let _session = transport::begin(out, budget);
    super::progress::enable(true);
    let result: Result<()> = (|| {
        if prepare {
            return Ok(());
        }
        if let Some(source) = &source {
            let (f, rows) = precheck_source(source, out, mock, &provider)?;
            return actor_collection(&f, &rows, out, &mut provider);
        }
        let rows = formation(&f, out, &cfg, mock)?;
        life::commit(&out.join("checkpoints.json"), &arr(rows.clone()))?;
        let schedule = cells(&f, &rows)?;
        life::commit(&out.join("actor-plan.json"), &arr(schedule))?;
        if formation_only {
            return Ok(());
        }
        actor_collection(&f, &rows, out, &mut provider)
    })();
    let trials = fs::read_to_string(out.join("trials.jsonl"))
        .map(|v| v.lines().count())
        .unwrap_or(0);
    let status = if result.is_err() {
        "incomplete_error"
    } else if prepare {
        "prepared"
    } else if formation_only {
        "formation_complete"
    } else if mock {
        "technical_rehearsal_complete"
    } else {
        "collected_unrated"
    };
    life::commit(&out.join("report.json"),&obj([
        ("status",text(status)),("error",result.as_ref().err().map(text).unwrap_or(Json::Null)),
        ("mock",Json::Bool(mock)),("actor_choices",num(trials as f64)),("http_attempts",num(transport::used() as f64)),
        ("scope",text("Exploratory scripted lives and matched actor choices, not evidence of consciousness or creativity. Review self-consolidation audit before interpreting an apparent preference."))]))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_registered_fixture() {
        let f = json::parse(FIXTURE).unwrap();
        validate(&f).unwrap();
        assert_eq!(array(&f, "histories").unwrap().len(), 5);
    }
    #[test]
    fn balanced_arms_and_orders_even_with_no_self() {
        let f = json::parse(FIXTURE).unwrap();
        let mut rs = Vec::new();
        for h in HISTORIES {
            for p in ACTOR_PHASES {
                let vs = array(&f, "tasks")
                    .unwrap()
                    .iter()
                    .map(|t| {
                        obj([
                            ("task", t.get("id").unwrap().clone()),
                            (
                                "full",
                                obj([("gists", text("")), ("self_explicit", arr([]))]),
                            ),
                            (
                                "memory_only",
                                obj([("gists", text("")), ("self_explicit", arr([]))]),
                            ),
                        ])
                    })
                    .collect::<Vec<_>>();
                rs.push(obj([
                    ("history", text(h)),
                    ("phase", text(p)),
                    ("sha256", text("dummy")),
                    ("views", arr(vs)),
                ]));
            }
        }
        let planned = cells(&f, &rs).unwrap();
        assert_eq!(planned.len(), 96);
        let mut keys = std::collections::BTreeSet::new();
        for row in &planned {
            let key = (
                s(row, "history").unwrap().to_owned(),
                s(row, "phase").unwrap().to_owned(),
                s(row, "task").unwrap().to_owned(),
                s(row, "order").unwrap().to_owned(),
                s(row, "arm").unwrap().to_owned(),
            );
            assert!(keys.insert(key));
        }
        assert_eq!(keys.len(), 96);
        let mut first = planned;
        for row in &mut first {
            let answer = s(
                &array(row.get("prompt").unwrap(), "options").unwrap()[0],
                "id",
            )
            .unwrap()
            .to_owned();
            row.put("choice", text(answer));
        }
        let measured = measure(&first).unwrap();
        for v in array(&measured, "paired_comparisons").unwrap() {
            assert_eq!(n(v, "changed").unwrap(), 0.);
        }
        for v in array(&measured, "within_history_revision").unwrap() {
            assert_eq!(n(v, "changed").unwrap(), 0.);
        }
        assert!(array(&measured, "positive_control")
            .unwrap()
            .iter()
            .all(|x| !flag(x, "diagnostic_pass")));
    }
    #[test]
    fn history_never_contains_express_preference_labels() {
        let f = json::parse(FIXTURE).unwrap();
        for h in array(&f, "histories").unwrap() {
            for key in ["initial", "counter"] {
                for event in array(h, key).unwrap() {
                    assert!(!event.string().unwrap().to_lowercase().contains("prefer"));
                }
            }
        }
    }
}
