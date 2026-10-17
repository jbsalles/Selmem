use super::json::{num, obj, text, Json};
use selmem::{AxiomLayer, EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory};
use std::{collections::HashSet, path::Path};
fn strings(xs: impl IntoIterator<Item = String>) -> Json {
    Json::Array(xs.into_iter().map(text).collect())
}
fn field<'a>(r: &'a Json, k: &str) -> Result<&'a str, String> {
    r.get(k)?.string()
}
fn flag(r: &Json, k: &str) -> bool {
    matches!(r.get(k), Ok(Json::Bool(true)))
}
fn save(m: &SelectiveMemory) -> Result<(), String> {
    m.save().map_err(|e| e.to_string())?;
    if let Some(path) = &m.path {
        std::fs::File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn inspect(m: &SelectiveMemory) -> Json {
    let mut traces: Vec<_> = m.store.traces.values().collect();
    traces.sort_by(|a, b| a.id.cmp(&b.id));
    let mut axioms: Vec<_> = m.store.axioms.values().collect();
    axioms.sort_by(|a, b| a.id.cmp(&b.id));
    obj([
        ("clock", num(m.clock.now() as f64)),
        (
            "traces",
            Json::Array(
                traces
                    .iter()
                    .map(|t| {
                        obj([
                            ("id", text(&t.id)),
                            (
                                "observation",
                                text(t.observation_id.as_deref().unwrap_or("")),
                            ),
                            ("gist", text(&t.gist)),
                            ("core", text(&t.core)),
                            ("schema", text(t.schema.as_deref().unwrap_or(""))),
                            ("fidelity", num(t.fidelity as f64)),
                            ("attribution",text(format!("{:?}",t.attribution))),
                            ("access",num(selmem::encode::scoring::access_value(t,&m.profile) as f64)),
                            ("permanence",num(t.permanence as f64)),
                            ("source",text(&t.source)),
                            ("suppressed",Json::Bool(t.suppressed)),
                            ("valence", num(t.valence as f64)),
                            ("arousal", num(t.arousal as f64)),
                            ("self_relevance", num(t.self_relevance as f64)),
                            ("status", text(format!("{:?}", t.status))),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "axioms",
            Json::Array(
                axioms
                    .iter()
                    .map(|a| {
                        obj([
                            ("id", text(&a.id)),
                            ("statement", text(&a.statement)),
                            ("layer", text(format!("{:?}", a.layer))),
                            ("supports", strings(a.support_trace_ids.clone())),
                            ("strength",num(a.strength as f64)),
                            ("schema",a.schema.as_ref().map(text).unwrap_or(Json::Null)),
                            ("superseded_by",a.superseded_by.as_ref().map(text).unwrap_or(Json::Null)),
                            ("distinct_supports",num(a.support_trace_ids.iter().filter_map(|id|m.store.traces.get(id)).map(|t|t.observation_id.as_deref().unwrap_or(&t.id)).collect::<HashSet<_>>().len() as f64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("centers", num(m.store.centers.len() as f64)),
        ("associations", num(m.store.associations.pairs.len() as f64)),
        ("observation_evidence_enabled", Json::Bool(m.store.observation_evidence.enabled)),
        ("observation_evidence", Json::Array(m.store.observation_evidence.groups.iter().map(|(id,g)|obj([
            ("trace_id",text(id)),("source",text(&g.source)),("observations",num(g.observations.len() as f64)),
            ("observation_ids",strings(g.observations.keys().cloned())),
            ("claims",Json::Array(g.counts().into_iter().map(|(claim,count)|obj([("claim",text(claim)),("count",num(count as f64))])).collect())),
        ])).collect())),
        ("associations_learn", Json::Bool(m.store.associations.learn)),
        (
            "associations_reconstruct",
            Json::Bool(m.store.associations.reconstruct),
        ),
        (
            "mood",
            strings([
                m.mood.valence.to_string(),
                m.mood.arousal.to_string(),
                m.mood.disgust.to_string(),
            ]),
        ),
    ])
}
fn graph(m: &SelectiveMemory) -> Json {
    Json::Array(
        m.store
            .associations
            .pairs
            .iter()
            .map(|((a, b), p)| {
                obj([
                    ("a", text(a)),
                    ("b", text(b)),
                    ("weight", num(p.weight as f64)),
                    ("episodes", num(p.co_recall_episodes as f64)),
                    ("updated_at", num(p.updated_at as f64)),
                    ("last_episode", num(p.last_episode as f64)),
                ])
            })
            .collect(),
    )
}
fn readout(m: &mut SelectiveMemory, query: &str, live: bool, transfer: bool) -> Json {
    let graph_before = graph(m);
    let (memories, dump) = m.remember_with(
        query,
        if live {
            RecallWrite::Live
        } else {
            RecallWrite::ReadOnly
        },
        if transfer { RecallBias::Transfer } else { RecallBias::Observed },
        &[],
    );
    let _guard = selmem::core::model::ClockGuard::push(m.clock.clone());
    let mut reading = selmem::recall::reading::ReadingProfile::for_query(
        &m.store,
        &m.mood,
        query,
        &dump.selected,
    );
    for (i, mark) in reading.salient.iter_mut().enumerate() {
        mark.id = format!("salient_{}", i + 1);
    }
    obj([
        (
            "memories",
            strings(memories.iter().map(|x| x.narrative.clone())),
        ),
        ("axioms", strings(reading.axioms.clone())),
        ("reading", text(reading.render())),
        ("selected_ids", strings(dump.selected)),
        (
            "recalled_traces",
            Json::Array(
                memories
                    .iter()
                    .map(|x| {
                        obj([
                            ("id", text(&x.trace_id)),
                            ("narrative", text(&x.narrative)),
                            ("pulled_toward_core", Json::Bool(x.pulled_toward_core)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "candidates",
            Json::Array(
                dump.candidates
                    .iter()
                    .map(|c| {
                        obj([
                            ("id", text(&c.trace_id)),
                            ("score", num(c.score as f64)),
                            ("base_score", num(c.base_score as f64)),
                            ("anchor", num(c.anchor as f64)),
                            ("status", text(format!("{:?}", c.status))),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "associations_used",
            Json::Array(
                dump.associations_used
                    .iter()
                    .map(|a| {
                        obj([
                            ("target", text(&a.target)),
                            ("accepted", Json::Bool(a.accepted)),
                            (
                                "cues",
                                Json::Array(
                                    a.cues
                                        .iter()
                                        .map(|c| {
                                            obj([
                                                ("trace_id", text(&c.trace_id)),
                                                ("gist", text(&c.gist)),
                                                (
                                                    "attribution",
                                                    text(format!("{:?}", c.attribution)),
                                                ),
                                                ("weight", num(c.weight as f64)),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "associations_reinforced",
            Json::Array(
                dump.associations_reinforced
                    .iter()
                    .map(|a| {
                        obj([
                            ("a", text(&a.a)),
                            ("b", text(&a.b)),
                            ("before", num(a.before as f64)),
                            ("after", num(a.after as f64)),
                            ("episode", num(a.episode as f64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("associations_learn", Json::Bool(m.store.associations.learn)),
        (
            "associations_reconstruct",
            Json::Bool(m.store.associations.reconstruct),
        ),
        (
            "supports_associations",
            Json::Bool(m.narrator_arc().supports_associations()),
        ),
        ("graph_before", graph_before),
        ("graph_after", graph(m)),
        ("scorer", text(dump.scorer)),
    ])
}
fn lesion(m: &mut SelectiveMemory, kind: &str, roots: &[String]) -> Result<Json, String> {
    let root_set: HashSet<_> = roots.iter().cloned().collect();
    // Association closure, NOT an oracle for causal descendants.
    let closure: HashSet<_> = m.store.lineage(roots).into_iter().collect();
    let before = inspect(m);
    let mut removed = HashSet::new();
    match kind {
        "intact" => (),
        "mood" => m.mood=selmem::Mood::default(),
        "latent" => removed=m.store.traces.iter().filter(|(_,t)|t.status==selmem::TraceStatus::Latent).map(|(id,_)|id.clone()).collect(),
        "residual_only" => removed=m.store.traces.iter().filter(|(_,t)|t.status!=selmem::TraceStatus::Latent).map(|(id,_)|id.clone()).collect(),
        "episode" => removed = root_set,
        "associated" => removed = closure.difference(&root_set).cloned().collect(),
        "beliefs" => m.store.axioms.retain(|_, a| a.layer != AxiomLayer::Belief),
        "schemas" => {
            m.store.centers.clear();
            m.store.axioms.clear();
        }
        "co_recall" => m.store.associations.pairs.clear(),
        "observation_evidence" => m.store.observation_evidence.groups.clear(),
        "traces" => removed = m.store.traces.keys().cloned().collect(),
        "closure" => {
            removed = closure;
            let schemas: HashSet<_> = removed
                .iter()
                .filter_map(|id| m.store.traces.get(id))
                .filter_map(|t| t.schema.clone())
                .collect();
            m.store
                .axioms
                .retain(|_, a| !a.support_trace_ids.iter().any(|s| removed.contains(s)));
            let axiom_ids: HashSet<_> = m.store.axioms.keys().cloned().collect();
            m.store.centers.retain(|s, c| {
                !schemas.contains(s)
                    && !c.hub_id.as_ref().is_some_and(|id| removed.contains(id))
                    && !c
                        .axiom_id
                        .as_ref()
                        .is_some_and(|id| !axiom_ids.contains(id))
            });
        }
        "schemas_co_recall" => {
            m.store.centers.clear();
            m.store.axioms.clear();
            m.store.associations.pairs.clear();
        }
        "empty" => {
            m.store = selmem::MemoryStore::new();
            m.mood = selmem::Mood::default();
        }
        _ => return Err(format!("unsupported lesion {kind}")),
    }
    m.store.traces.retain(|id, _| !removed.contains(id));
    m.store.observation_evidence.groups.retain(|id, _| !removed.contains(id));
    m.store.edges.retain(|id, _| !removed.contains(id));
    for edges in m.store.edges.values_mut() {
        edges.retain(|id| !removed.contains(id));
    }
    m.store.pending_night.retain(|id| !removed.contains(id));
    m.store
        .associations
        .pairs
        .retain(|(a, b), _| !removed.contains(a) && !removed.contains(b));
    m.clear_talk();
    let mut ids: Vec<_> = removed.into_iter().collect();
    ids.sort();
    Ok(obj([("before", before), ("after", inspect(m)), ("removed_trace_ids", strings(ids)),
        ("scope", text("structural lesion; closure includes schema siblings, not proven causal descendants"))]))
}
pub(super) fn execute(r: &Json) -> Result<Json, String> {
    if field(r,"op")?=="recall" && flag(r,"creative_transfer"){return Err("transfer writes require explicit reflect operation".into());}
    let path = Path::new(field(r, "path")?);
    let op = field(r, "op")?;
    if op != "init" && !path.is_file() {
        return Err("snapshot missing; refusing implicit empty organ".into());
    }
    if op == "init" && path.exists() {
        return Err("refusing to overwrite existing snapshot".into());
    }
    let mut m =
        SelectiveMemory::open(path, EntityProfile::new("observer")).map_err(|e| e.to_string())?;
    if op == "init" {
        match r.get("memory_config").and_then(|c|c.get("observation_evidence")) {
            Ok(Json::Bool(enabled)) => m.set_observation_evidence(*enabled),
            Ok(_) => return Err("observation_evidence must be boolean".into()),
            Err(_) => (),
        }
        m.clock.origin_real = 4_102_444_800;
        m.clock.scale = 1;
        m.clock.jump = 0;
        m.clock.detached = true;
        match field(r, "architecture")? {
            "selmem_full" | "selmem_no_sleep" => (),
            "selmem_no_recon" => {
                m.cut.reconstruct = false;
                m.cut.reconsolidate = false;
            }
            arch => return Err(format!("unsupported native architecture {arch}")),
        }
        m.set_associations(
            flag(r, "associations_learn"),
            flag(r, "associations_reconstruct"),
        );
        save(&m)?;
        return Ok(inspect(&m));
    }
    if let Ok(n) = r.get("id_origin") {
        selmem::core::model::set_next_id(n.number()? as u64);
    }
    if op == "probe" {
        if let Ok(Json::Bool(use_context)) = r
            .get("memory_config")
            .and_then(|c| c.get("probe_associations_reconstruct"))
        {
            m.set_associations(false, *use_context);
        }
    }
    // Transient probe-only ablation: the saved common life is never rewritten.
    if op == "probe" {
        if let Ok(value) = r.get("reconstruct") {
            let Json::Bool(enabled) = value else { return Err("reconstruct must be boolean".into()); };
            m.cut.reconstruct = *enabled;
            m.cut.reconsolidate = false;
        }
    }
    let mut http_audit = None;
    let lifecycle = r
        .get("memory_config")
        .and_then(|c| c.get("lifecycle_backend"))
        .and_then(Json::string)
        .unwrap_or("rules");
    if !["rules", "http"].contains(&lifecycle) {
        return Err("lifecycle_backend must be rules or http".into());
    }
    if lifecycle == "http"
        && r.get("memory_config")
            .and_then(|c| c.get("memory_backend"))
            .and_then(Json::string)
            != Ok("http")
    {
        return Err("HTTP lifecycle requires HTTP memory backend".into());
    }
    if matches!(op, "recall" | "reflect" | "probe") || (lifecycle == "http" && matches!(op, "live" | "sleep")) {
        if let Ok(config) = r.get("memory_config") {
            if config.get("memory_backend").and_then(Json::string) == Ok("http") {
                let (bound, audit) = super::http::bind(m, config)?;
                m = bound;
                http_audit = Some(audit);
            }
        }
    }
    let checked_readout = |m: &mut SelectiveMemory, query: &str, live| -> Result<Json, String> {
        let transfer = flag(r, "creative_transfer");
        if transfer && live && op != "reflect" { return Err("live creative transfer requires explicit reflect operation".into()); }
        let mut out = readout(m, query, live, transfer);
        out.put("recall_intent", text(if transfer { "transfer" } else { "factual" }));
        let calls = m.narrator_arc().failure_log();
        out.put("narrator_calls", num(calls.calls as f64));
        out.put("narrator_failures", num(calls.failures as f64));
        out.put(
            "recall_backend",
            text(if http_audit.is_some() {
                "http"
            } else {
                "rules"
            }),
        );
        if calls.failures > 0 || calls.fallback_used {
            return Err(format!(
                "memory reconstruction failed: {:?}",
                calls.llm_error
            ));
        }
        if let Some(audit) = &http_audit {
            let audit = audit.lock().unwrap();
            if let Some(error) = &audit.error {
                return Err(format!("memory scorer failed: {error}"));
            }
            out.put("scorer_calls", Json::Array(audit.calls.clone()));
            out.put(
                "reconstruction_checks",
                Json::Array(audit.reconstructions.clone()),
            );
            // Core fallback is not evidence that the offered associative framing was accepted.
            let mut used = out.get("associations_used")?.array()?.to_vec();
            for association in &mut used {
                if audit.reconstructions.iter().any(|c| {
                    flag(c, "enforced")
                        && flag(c, "associated")
                        && c.get("target") == association.get("target")
                        && !flag(c, "accepted")
                }) {
                    association.put("accepted", Json::Bool(false));
                }
            }
            out.put("associations_used", Json::Array(used));
            out.put(
                "memory_calls",
                Json::Array(
                    audit
                        .narrator
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|call| {
                            obj([
                                ("request_body", text(&call.request_body)),
                                (
                                    "raw_response",
                                    call.response.as_ref().map(text).unwrap_or(Json::Null),
                                ),
                                (
                                    "error",
                                    call.response.as_ref().err().map(text).unwrap_or(Json::Null),
                                ),
                            ])
                        })
                        .collect(),
                ),
            );
            out.put("memory_temperature", num(0.));
            out.put("memory_reasoning", text(selmem::Config::get().reasoning()));
        }
        Ok(out)
    };
    let lifecycle_audit = |m: &SelectiveMemory| -> Result<Json, String> {
        let calls = m.narrator_arc().failure_log();
        if calls.failures > 0 || calls.fallback_used {
            return Err(format!("lifecycle narrator failed: {:?}", calls.llm_error));
        }
        let (semantic_calls, reconstructions, narrator_exchanges, scorer_calls) =
            if let Some(audit) = &http_audit {
                let audit = audit.lock().unwrap();
                if let Some(error) = &audit.error {
                    return Err(format!("lifecycle backend failed: {error}"));
                }
                let exchanges = audit
                    .narrator
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|c| {
                        obj([
                            ("request_body", text(&c.request_body)),
                            (
                                "raw_response",
                                c.response.as_ref().map(text).unwrap_or(Json::Null),
                            ),
                            (
                                "error",
                                c.response.as_ref().err().map(text).unwrap_or(Json::Null),
                            ),
                        ])
                    })
                    .collect();
                (
                    audit.semantic_calls.clone(),
                    audit.reconstructions.clone(),
                    exchanges,
                    audit.calls.clone(),
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };
        Ok(obj([
            ("op", text(op)),
            ("configured_backend", text(lifecycle)),
            (
                "bound_backend",
                text(if http_audit.is_some() {
                    "http"
                } else {
                    "rules"
                }),
            ),
            ("semantic_calls", Json::Array(semantic_calls)),
            ("narrator_calls", num(calls.calls as f64)),
            ("narrator_exchanges", Json::Array(narrator_exchanges)),
            ("scorer_calls", Json::Array(scorer_calls)),
            ("reconstruction_checks", Json::Array(reconstructions)),
        ]))
    };
    match op {
        "live" => {
            let event = r.get("event")?;
            let mut input = EncodeInput::new(field(event, "text")?);
            input.observation_id = Some(field(event,"observation_id").unwrap_or(field(event,"id")?));
            input.source = "genesis_fixture";
            if flag(event, "external") {
                input.attribution = selmem::Attribution::External;
            }
            // Predeclared, symmetric task stakes. The native gate still decides.
            if flag(event, "critical") {
                input.self_relevance = 0.9;
                input.utility = 0.9;
                input.goal_align = 0.9;
            }
            // No supplied schema, valence, permanence or behavioral instruction.
            let d = m.live_with(input);
            let out = obj([
                ("kept", Json::Bool(d.kept)),
                ("reason", text(&d.reason)),
                ("score", num(d.score as f64)),
                ("new_trace", Json::Bool(d.kept_n > 0)),
                ("salience_evaluated", Json::Bool(!d.reason.contains("salience not evaluated"))),
                ("trace_id", d.trace_id.map(text).unwrap_or(Json::Null)),
                ("state", inspect(&m)),
                ("lifecycle_audit", lifecycle_audit(&m)?),
            ]);
            save(&m)?;
            Ok(out)
        }
        "recall" | "reflect" => {
            m.begin_association_episode();
            let out = checked_readout(&mut m, field(r, "query")?, true)?;
            save(&m)?;
            Ok(out)
        }
        "probe" => checked_readout(&mut m, field(r, "query")?, false),
        "tick" => {
            m.advance_hours(r.get("hours")?.number()? as f32);
            save(&m)?;
            Ok(inspect(&m))
        }
        "sleep" => {
            let report = if flag(r, "deep") { m.sleep_deep() } else { m.sleep() };
            let audit = lifecycle_audit(&m)?;
            save(&m)?;
            let mut out = inspect(&m);
            out.put("night", obj([
                ("kind", text(format!("{:?}", report.kind))),
                ("merged", num(report.merged as f64)),
                ("released", num(report.released as f64)),
                ("rewritten", num(report.rewritten as f64)),
                ("weathered", num(report.weathered as f64)),
                ("axioms_created", num(report.axioms.len() as f64)),
            ]));
            out.put("lifecycle_audit", audit);
            Ok(out)
        }
        "recall_cut" => {m.cut.reconstruct=flag(r,"enabled");m.cut.reconsolidate=flag(r,"enabled");save(&m)?;Ok(inspect(&m))},
        "inspect" => Ok(inspect(&m)),
        "lesion" => {
            let roots = r
                .get("roots")?
                .array()?
                .iter()
                .map(|v| v.string().map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?;
            let out = lesion(&mut m, field(r, "kind")?, &roots)?;
            save(&m)?;
            Ok(out)
        }
        _ => Err(format!("unknown operation {op}")),
    }
}
