use crate::encode::EncodeInput;
use crate::engine::SelectiveMemory;
use crate::core::model::Channel;

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub fn dispatch(mem: &mut SelectiveMemory, method: &str, path: &str, query: &str, body: &str) -> HttpResponse {
    if method == "OPTIONS" {
        return HttpResponse {
            status: 204,
            body: String::new(),
        };
    }
    match (method, path) {
        ("GET", "/health") => ok(format!(
            "{{\"ok\":true,\"name\":\"{}\",\"traces\":{},\"archives\":{},\"axioms\":{}}}",
            json_esc(&mem.profile.name),
            mem.store.traces.len(),
            mem.store.archives.len(),
            mem.store.axioms.len()
        )),
        ("GET", "/profile") => ok(profile_json(mem)),
        ("POST", "/profile") => {
            if let Some(v) = json_str(body, "name") {
                let n = v.trim();
                if !n.is_empty() {
                    mem.profile.name = n.to_string();
                }
            }
            if let Some(v) = json_str(body, "voice") {
                mem.profile.set_voice(&v);
            }
            if let Some(v) = json_f32(body, "encode_threshold") {
                mem.profile.encode_threshold = v.clamp(0.10, 0.80);
            }
            if let Some(v) = json_f32(body, "w_self") {
                mem.profile.w_self = v.clamp(0.0, 0.60);
            }
            if let Some(v) = json_f32(body, "embellish_gain") {
                mem.profile.embellish_gain = v.clamp(0.0, 0.50);
            }
            if let Some(v) = json_f32(body, "disgust_gain") {
                mem.profile.disgust_gain = v.clamp(0.0, 0.50);
            }
            if let Some(v) = json_f32(body, "decay_lambda") {
                mem.profile.decay_lambda = v.clamp(0.01, 0.30);
            }
            if let Some(v) = json_f32(body, "narrator_firmness") {
                mem.profile.narrator_firmness = v.clamp(0.0, 1.0);
            }
            if let Some(v) = json_f32(body, "max_recall") {
                mem.profile.max_recall = v.clamp(1.0, 12.0) as usize;
            }
            if let Some(v) = json_f32(body, "merge_similarity") {
                mem.profile.merge_similarity = v.clamp(0.05, 0.95);
            }
            if let Some(v) = json_f32(body, "ground_min_overlap") {
                mem.profile.ground_min_overlap = v.clamp(0.0, 1.0);
            }
            if let Some(v) = json_f32(body, "ground_strikes") {
                mem.profile.ground_strikes = v.max(1.0) as usize;
            }
            if let Some(v) = json_f32(body, "reconsolidation_eta") {
                mem.profile.reconsolidation_eta = v.clamp(0.0, 1.0);
            }
            if let Some(v) = json_bool(body, "cut_reconsolidate") {
                mem.cut.reconsolidate = v;
            }
            if let Some(v) = json_bool(body, "cut_ground") {
                mem.cut.ground = v;
            }
            if let Some(v) = json_bool(body, "cut_ladder") {
                mem.cut.ladder = v;
            }
            let _ = mem.save();
            ok(profile_json(mem))
        }
        ("GET", "/llm") => ok(llm_json(mem)),
        ("POST", "/llm") => {
            let url = json_str(body, "url").unwrap_or_else(|| mem.llm.url.clone());
            let model = json_str(body, "model").unwrap_or_else(|| mem.llm.model.clone());
            let key = json_str(body, "api_key");
            if json_bool(body, "clear").unwrap_or(false) || url.trim().is_empty() {
                let _ = mem.set_llm("", "", None);
            } else if let Err(e) = mem.set_llm(&url, &model, key) {
                return err(400, &e);
            }
            ok(llm_json(mem))
        }
        ("GET", "/mood") => ok(format!(
            "{{\"valence\":{:.4},\"arousal\":{:.4},\"disgust\":{:.4}}}",
            mem.mood.valence, mem.mood.arousal, mem.mood.disgust
        )),
        ("GET", "/who") => {
            let axioms: Vec<String> = mem
                .who_am_i()
                .into_iter()
                .map(|a| {
                    format!(
                        "{{\"id\":\"{}\",\"layer\":\"{}\",\"strength\":{:.3},\"valence\":{:.3},\"statement\":\"{}\"}}",
                        json_esc(&a.id),
                        match a.layer {
                            crate::core::model::AxiomLayer::Motif => "motif",
                            crate::core::model::AxiomLayer::Belief => "belief",
                            crate::core::model::AxiomLayer::Trait => "trait",
                        },
                        a.strength,
                        a.valence,
                        json_esc(&a.statement)
                    )
                })
                .collect();
            ok(format!("{{\"name\":\"{}\",\"axioms\":[{}]}}", json_esc(&mem.profile.name), axioms.join(",")))
        }
        ("GET", "/lineage") => {
            let schema = query_param(query, "schema").unwrap_or("");
            let items: Vec<String> = mem
                .lineage(schema)
                .into_iter()
                .map(|a| {
                    format!(
                        "{{\"id\":\"{}\",\"statement\":\"{}\",\"superseded_by\":\"{}\"}}",
                        json_esc(&a.id),
                        json_esc(&a.statement),
                        json_esc(a.superseded_by.as_deref().unwrap_or(""))
                    )
                })
                .collect();
            ok(format!("{{\"schema\":\"{}\",\"chain\":[{}]}}", json_esc(schema), items.join(",")))
        }
        ("GET", "/audit") => {
            let id = query_param(query, "id").unwrap_or("");
            match mem.audit(id) {
                Some(v) => ok(format!(
                    "{{\"trace_id\":\"{}\",\"verbatim\":\"{}\"}}",
                    json_esc(id),
                    json_esc(v)
                )),
                None => err(404, "archive introuvable"),
            }
        }
        ("GET", "/book") => ok(book_json(mem)),
        ("GET", "/events") => ok(events_json(mem)),
        ("POST", "/turn") => {
            let text = json_str(body, "text")
                .or_else(|| json_str(body, "event"))
                .unwrap_or_default();
            if text.trim().is_empty() {
                return err(400, "text requis");
            }
            // Chat stays in the sitting. The gate runs at sleep (or pin).
            let reply = mem.speak(&text);
            let _ = mem.save();
            ok(format!(
                "{{\"reply\":\"{}\",\"topic\":\"{}\",\"mood\":{{\"valence\":{:.3},\"arousal\":{:.3},\"disgust\":{:.3}}}}}",
                json_esc(&reply),
                json_esc(mem.talk.topic.as_deref().unwrap_or("")),
                mem.mood.valence,
                mem.mood.arousal,
                mem.mood.disgust
            ))
        }
        ("POST", "/pin") => {
            let from_body = json_str(body, "text").or_else(|| json_str(body, "event"));
            let from_talk = mem
                .talk
                .turns
                .last()
                .map(|t| t.user.clone())
                .or_else(|| mem.talk.topic.clone())
                .unwrap_or_default();
            let text = from_body.filter(|s| !s.trim().is_empty()).unwrap_or(from_talk);
            if text.trim().is_empty() {
                return err(400, "nothing to pin");
            }
            let (v, a, d, schema) = guess_affect(&text);
            let mut input = EncodeInput::new(&text);
            input.valence = v;
            input.arousal = a.max(0.45);
            input.disgust = d;
            input.self_relevance = 1.0;
            input.utility = 0.75;
            input.permanence = 0.85;
            input.schema = mem.talk.schema.clone().or(schema).or(Some("pinned".into()));
            input.channel = Channel::Selfhood;
            let dec = mem.live_with(input);
            let _ = mem.save();
            ok(format!(
                "{{\"kept\":{},\"score\":{:.4},\"reason\":\"{}\",\"pinned\":\"{}\",\"topic\":\"{}\"}}",
                if dec.kept { "true" } else { "false" },
                dec.score,
                json_esc(&dec.reason),
                json_esc(&text),
                json_esc(mem.talk.topic.as_deref().unwrap_or(""))
            ))
        }
        ("POST", "/live") => {
            let event = json_str(body, "event").unwrap_or_default();
            if event.trim().is_empty() {
                return err(400, "event requis");
            }
            let mut input = EncodeInput::new(&event);
            if let Some(v) = json_f32(body, "valence") {
                input.valence = v;
            }
            if let Some(v) = json_f32(body, "arousal") {
                input.arousal = v;
            }
            if let Some(v) = json_f32(body, "disgust") {
                input.disgust = v;
            }
            if let Some(v) = json_f32(body, "self_relevance") {
                input.self_relevance = v;
            }
            if let Some(v) = json_f32(body, "utility") {
                input.utility = v;
            }
            if let Some(v) = json_f32(body, "goal_align") {
                input.goal_align = v;
            }
            if let Some(v) = json_f32(body, "permanence") {
                input.permanence = v;
            }
            if let Some(s) = json_str(body, "schema") {
                if !s.is_empty() {
                    input.schema = Some(s);
                }
            }
            if let Some(ch) = json_str(body, "channel") {
                input.channel = match ch.as_str() {
                    "world" => Channel::World,
                    "log" => Channel::Log,
                    _ => Channel::Selfhood,
                };
            }
            let d = mem.live_with(input);
            let _ = mem.save();
            let tid = d.trace_id.as_deref().unwrap_or("");
            ok(format!(
                "{{\"kept\":{},\"score\":{:.4},\"reason\":\"{}\",\"trace_id\":\"{}\",\"archive_id\":\"{}\"}}",
                if d.kept { "true" } else { "false" },
                d.score,
                json_esc(&d.reason),
                json_esc(tid),
                json_esc(&d.archive_id)
            ))
        }
        ("POST", "/remember") => {
            let query_txt = json_str(body, "query").unwrap_or_default();
            if query_txt.trim().is_empty() {
                return err(400, "query requis");
            }
            let recs = mem.remember(&query_txt);
            let _ = mem.save();
            let items: Vec<String> = recs
                .iter()
                .map(|r| {
                    format!(
                        "{{\"trace_id\":\"{}\",\"fidelity\":{:.3},\"channel\":\"{}\",\"schema\":\"{}\",\"disclaimer\":\"{}\",\"narrative\":\"{}\"}}",
                        json_esc(&r.trace_id),
                        r.fidelity,
                        crate::persist::snapshot::channel_token(r.channel),
                        json_esc(r.schema.as_deref().unwrap_or("")),
                        json_esc(&r.disclaimer),
                        json_esc(&r.narrative)
                    )
                })
                .collect();
            ok(format!("{{\"memories\":[{}]}}", items.join(",")))
        }
        ("POST", "/sleep") => {
            let snap = mem.talk.turns.clone();
            let topic = mem.talk.topic.clone();
            let (sitting, _) = mem.keep_sitting();
            mem.clear_talk();
            let report = mem.sleep();
            mem.fade_sitting();
            for t in &snap {
                mem.talk.record(&t.user, &t.reply);
            }
            mem.talk.topic = topic;
            let _ = mem.save();
            ok(format!(
                "{{\"faded\":{},\"cold\":{},\"myth\":{},\"merged\":{},\"extinguished\":{},\"weathered\":{},\"rewritten\":{},\"released\":{},\"sculpted\":{},\"axioms\":{},\"sitting\":{},\"kind\":\"{}\",\"new_hours\":{},\"charge\":{:.3}}}",
                report.faded,
                report.cold,
                report.myth,
                report.merged,
                report.extinguished,
                report.weathered,
                report.rewritten,
                report.released,
                report.sculpted.len(),
                report.axioms.len(),
                sitting,
                report.kind.as_str(),
                report.new_hours,
                report.charge
            ))
        }
        ("POST", "/speak") => {
            let user = json_str(body, "text")
                .or_else(|| json_str(body, "query"))
                .unwrap_or_default();
            if user.trim().is_empty() {
                return err(400, "text requis");
            }
            let reply = mem.speak(&user);
            let _ = mem.save();
            ok(format!(
                "{{\"reply\":\"{}\",\"topic\":\"{}\"}}",
                json_esc(&reply),
                json_esc(mem.talk.topic.as_deref().unwrap_or(""))
            ))
        }
        ("GET", "/talk") => {
            mem.refresh_talk();
            let turns: Vec<String> = mem
                .talk
                .turns
                .iter()
                .map(|t| {
                    format!(
                        "{{\"user\":\"{}\",\"reply\":\"{}\"}}",
                        json_esc(&t.user),
                        json_esc(&t.reply)
                    )
                })
                .collect();
            ok(format!(
                "{{\"topic\":\"{}\",\"schema\":\"{}\",\"turns\":[{}]}}",
                json_esc(mem.talk.topic.as_deref().unwrap_or("")),
                json_esc(mem.talk.schema.as_deref().unwrap_or("")),
                turns.join(",")
            ))
        }
        ("POST", "/talk/clear") => {
            mem.clear_talk();
            ok("{\"cleared\":true}".into())
        }
        ("POST", "/reset") => {
            mem.reset();
            match mem.save() {
                Ok(()) => ok("{\"reset\":true}".into()),
                Err(e) => err(500, &e.to_string()),
            }
        }
        ("POST", "/save") => match mem.save() {
            Ok(()) => ok("{\"saved\":true}".into()),
            Err(e) => err(500, &e.to_string()),
        },
        _ => err(404, "route inconnue"),
    }
}

fn ok(body: String) -> HttpResponse {
    HttpResponse { status: 200, body }
}

fn err(status: u16, msg: &str) -> HttpResponse {
    HttpResponse {
        status,
        body: format!("{{\"error\":\"{}\"}}", json_esc(msg)),
    }
}

pub fn json_esc(s: &str) -> String {
    crate::net::httpx::json_esc(s)
}

fn status_name(s: crate::core::model::TraceStatus) -> &'static str {
    use crate::core::model::TraceStatus::*;
    match s {
        Active => "active",
        Cold => "cold",
        Myth => "myth",
        Latent => "latent",
    }
}

fn drift_name(k: crate::core::model::DriftKind) -> &'static str {
    use crate::core::model::DriftKind::*;
    match k {
        Embellish => "embellish",
        AmplifyDisgust => "amplify",
        Fade => "fade",
        Merge => "merge",
        Weather => "weather",
        Rewrite => "rewrite",
        Reinterpret => "reinterpret",
        Ground => "ground",
        Color => "color",
    }
}

fn book_json(mem: &SelectiveMemory) -> String {
    let mut traces: Vec<_> = mem.store.traces.values().collect();
    traces.sort_by_key(|t| std::cmp::Reverse(t.created_at));
    let tjson: Vec<String> = traces
        .into_iter()
        .map(|t| {
            format!(
                "{{\"id\":\"{}\",\"status\":\"{}\",\"channel\":\"{}\",\"schema\":\"{}\",\"gist\":\"{}\",\"core\":\"{}\",\"fidelity\":{:.3},\"anchor\":{:.3},\"valence\":{:.3},\"created_at\":{},\"archive_id\":\"{}\"}}",
                json_esc(&t.id),
                status_name(t.status),
                crate::persist::snapshot::channel_token(t.channel),
                json_esc(t.schema.as_deref().unwrap_or("")),
                json_esc(&t.gist),
                json_esc(&t.core),
                t.fidelity,
                t.anchor,
                t.valence,
                t.created_at,
                json_esc(t.archive_id.as_deref().unwrap_or(""))
            )
        })
        .collect();
    let mut axioms: Vec<_> = mem.store.axioms.values().collect();
    axioms.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    let ajson: Vec<String> = axioms
        .into_iter()
        .map(|a| {
            format!(
                "{{\"id\":\"{}\",\"layer\":\"{}\",\"strength\":{:.3},\"valence\":{:.3},\"statement\":\"{}\",\"schema\":\"{}\",\"superseded\":{},\"created_at\":{}}}",
                json_esc(&a.id),
                match a.layer {
                    crate::core::model::AxiomLayer::Motif => "motif",
                    crate::core::model::AxiomLayer::Belief => "belief",
                    crate::core::model::AxiomLayer::Trait => "trait",
                },
                a.strength,
                a.valence,
                json_esc(&a.statement),
                json_esc(a.schema.as_deref().unwrap_or("")),
                if a.superseded_by.is_some() { "true" } else { "false" },
                a.created_at
            )
        })
        .collect();
    let mut archives: Vec<_> = mem.store.archives.values().collect();
    archives.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    let rjson: Vec<String> = archives
        .into_iter()
        .map(|a| {
            format!(
                "{{\"id\":\"{}\",\"source\":\"{}\",\"created_at\":{},\"verbatim\":\"{}\"}}",
                json_esc(&a.id),
                json_esc(&a.source),
                a.created_at,
                json_esc(&a.verbatim)
            )
        })
        .collect();
    format!(
        "{{\"traces\":[{}],\"axioms\":[{}],\"archives\":[{}]}}",
        tjson.join(","),
        ajson.join(","),
        rjson.join(",")
    )
}

fn events_json(mem: &SelectiveMemory) -> String {
    struct Ev {
        at: u64,
        kind: &'static str,
        title: String,
        detail: String,
    }
    let mut evs: Vec<Ev> = Vec::new();
    for t in mem.store.traces.values() {
        evs.push(Ev {
            at: t.created_at,
            kind: "encode",
            title: t.id.clone(),
            detail: t.gist.clone(),
        });
        for d in &t.drifts {
            evs.push(Ev {
                at: d.at,
                kind: drift_name(d.kind),
                title: t.id.clone(),
                detail: d.note.clone(),
            });
        }
    }
    for a in mem.store.axioms.values() {
        evs.push(Ev {
            at: a.created_at,
            kind: "axiom",
            title: a.id.clone(),
            detail: a.statement.clone(),
        });
    }
    for a in mem.store.archives.values() {
        evs.push(Ev {
            at: a.created_at,
            kind: "archive",
            title: a.id.clone(),
            detail: a.source.clone(),
        });
    }
    evs.sort_by(|x, y| y.at.cmp(&x.at).then(x.kind.cmp(y.kind)));
    evs.truncate(200);
    let items: Vec<String> = evs
        .into_iter()
        .map(|e| {
            format!(
                "{{\"at\":{},\"kind\":\"{}\",\"title\":\"{}\",\"detail\":\"{}\"}}",
                e.at,
                e.kind,
                json_esc(&e.title),
                json_esc(&e.detail)
            )
        })
        .collect();
    format!("{{\"events\":[{}]}}", items.join(","))
}

fn mask_key(key: &str) -> String {
    let t = key.trim();
    if t.len() <= 8 {
        return "••••".into();
    }
    format!("{}…{}", &t[..4], &t[t.len() - 4..])
}

fn llm_json(mem: &SelectiveMemory) -> String {
    let attached = !mem.llm.url.is_empty();
    let key = mem.llm.key.as_deref().unwrap_or("");
    format!(
        "{{\"ok\":true,\"attached\":{},\"url\":\"{}\",\"model\":\"{}\",\"has_key\":{},\"key_hint\":\"{}\"}}",
        if attached { "true" } else { "false" },
        json_esc(&mem.llm.url),
        json_esc(&mem.llm.model),
        if mem.llm.key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
            "true"
        } else {
            "false"
        },
        json_esc(&if key.is_empty() { String::new() } else { mask_key(key) }),
    )
}

fn profile_json(mem: &SelectiveMemory) -> String {
    format!(
        "{{\"ok\":true,\"name\":\"{}\",\"voice\":\"{}\",\"encode_threshold\":{:.4},\"w_self\":{:.4},\"embellish_gain\":{:.4},\"disgust_gain\":{:.4},\"decay_lambda\":{:.4},\"narrator_firmness\":{:.4},\"max_recall\":{},\"merge_similarity\":{:.4},\"ground_min_overlap\":{:.4},\"ground_strikes\":{},\"reconsolidation_eta\":{:.4},\"cut_reconsolidate\":{},\"cut_ground\":{},\"cut_ladder\":{}}}",
        json_esc(&mem.profile.name),
        mem.profile.voice_kind(),
        mem.profile.encode_threshold,
        mem.profile.w_self,
        mem.profile.embellish_gain,
        mem.profile.disgust_gain,
        mem.profile.decay_lambda,
        mem.profile.narrator_firmness,
        mem.profile.max_recall,
        mem.profile.merge_similarity,
        mem.profile.ground_min_overlap,
        mem.profile.ground_strikes,
        mem.profile.reconsolidation_eta,
        if mem.cut.reconsolidate { "true" } else { "false" },
        if mem.cut.ground { "true" } else { "false" },
        if mem.cut.ladder { "true" } else { "false" },
    )
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            Some(v)
        } else {
            None
        }
    })
}

fn json_str(body: &str, key: &str) -> Option<String> {
    crate::net::httpx::first_string_field(body, key)
}

fn json_f32(body: &str, key: &str) -> Option<f32> {
    crate::net::httpx::first_number_field(body, key)
}

fn json_bool(body: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{}\"", key);
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let rest = rest.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn guess_affect(text: &str) -> (f32, f32, f32, Option<String>) {
    crate::encode::affect::guess(text)
}
