//! HTTP endpoint dispatch, separate from wire format and rendering.
use super::*;
pub(super) fn dispatch_for(
    mem: &mut SelectiveMemory,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
    visitor: bool,
) -> HttpResponse {
    if method == "OPTIONS" {
        return HttpResponse {
            status: 204,
            body: String::new(),
        };
    }
    match (method, path) {
        ("GET", "/health") | ("GET", "/stats") => ok(format!(
            "{{\"ok\":true,\"name\":\"{}\",\"traces\":{},\"archives\":{},\"axioms\":{},\"scorer\":\"{}\"}}",
            json_esc(&mem.profile.name),
            mem.store.traces.len(),
            mem.store.archives.len(),
            mem.store.axioms.len(),
            json_esc(&mem.scorer_name())
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
            if let Some(v) = json_f32(body, "time_scale") {
                mem.clock.set_scale(v.round() as u32);
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
            let learn = json_bool(body, "associations_learn").unwrap_or(mem.store.associations.learn);
            let reconstruct = json_bool(body, "associations_reconstruct").unwrap_or(mem.store.associations.reconstruct);
            mem.set_associations(learn, reconstruct);
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
            ok(profile_json(mem))
        }
        ("GET", "/llm") => ok(llm_json(mem, visitor)),
        ("POST", "/llm") => {
            if let Some(plug) = json_str(body, "plug") {
                let plug = plug.trim();
                if plug.is_empty() || plug == "rules" || json_bool(body, "clear").unwrap_or(false) {
                    let _ = mem.set_llm("", "", None);
                    return ok(llm_json(mem, visitor));
                }
                let mouth = if visitor {
                    crate::config::expand_plug(plug).map(|(url, model)| crate::config::Mouth {
                        plug: plug.into(), url: url.into(), model: model.into(), api_key: None,
                    }).ok_or_else(|| "choose grok or gpt".to_string()).map(Some)
                } else {
                    crate::config::Config::get().mouth_named(plug, None, None, "gpt-4o-mini")
                };
                if visitor { let _ = mem.set_llm("", "", None); }
                match mouth {
                    Ok(Some(mouth)) => {
                        if let Err(e) = mem.set_plug(&mouth.plug, &mouth.url, &mouth.model, json_str(body, "api_key").or(mouth.api_key))
                        {
                            return err(400, &e);
                        }
                    }
                    Ok(None) => return err(400, "plug has no endpoint"),
                    Err(e) => return err(400, &e),
                }
                return ok(llm_json(mem, visitor));
            }
            let url = json_str(body, "url").unwrap_or_else(|| mem.llm.url.clone());
            let model = json_str(body, "model").unwrap_or_else(|| mem.llm.model.clone());
            let key = json_str(body, "api_key");
            if visitor && !url.is_empty() && !["grok", "gpt"].iter().any(|plug| {
                crate::config::expand_plug(plug).map(|(endpoint, _)| endpoint == url).unwrap_or(false)
            }) { return err(400, "public demo supports grok and gpt endpoints only"); }
            if visitor && url != mem.llm.url { let _ = mem.set_llm("", "", None); }
            if json_bool(body, "clear").unwrap_or(false) || url.trim().is_empty() {
                let _ = mem.set_llm("", "", None);
            } else {
                let result = if visitor {
                    let plug = if crate::config::expand_plug("gpt").map(|(endpoint, _)| endpoint == url).unwrap_or(false) { "gpt" } else { "grok" };
                    mem.set_plug(plug, &url, &model, key)
                } else { mem.set_llm(&url, &model, key) };
                if let Err(e) = result { return err(400, &e); }
            }
            ok(llm_json(mem, visitor))
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
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
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
            let from_talk = sitting_pin_text(&mem.talk);
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
            if let Some(id) = dec.trace_id.as_deref() {
                let _ = mem.pin(id);
            }
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
            ok(format!(
                "{{\"kept\":{},\"score\":{:.4},\"reason\":\"{}\",\"pinned\":\"{}\",\"topic\":\"{}\"}}",
                if dec.kept { "true" } else { "false" },
                dec.score,
                json_esc(&dec.reason),
                json_esc(&text),
                json_esc(mem.talk.topic.as_deref().unwrap_or(""))
            ))
        }
        ("POST", "/suppress") => {
            let id = json_str(body, "id").unwrap_or_default();
            if id.trim().is_empty() {
                return err(400, "id requis");
            }
            let ok_flag = mem.suppress(&id);
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
            ok(format!("{{\"ok\":{},\"id\":\"{}\"}}", if ok_flag { "true" } else { "false" }, json_esc(&id)))
        }
        ("POST", "/unsuppress") => {
            let id = json_str(body, "id").unwrap_or_default();
            if id.trim().is_empty() {
                return err(400, "id requis");
            }
            let ok_flag = mem.unsuppress(&id);
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
            ok(format!("{{\"ok\":{},\"id\":\"{}\"}}", if ok_flag { "true" } else { "false" }, json_esc(&id)))
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
            if let Some(v) = json_f32(body, "self_congruence") {
                input.self_congruence = v.clamp(0.0, 1.0);
            }
            if let Some(s) = json_str(body, "attribution") {
                input.attribution = Attribution::parse(&s);
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
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
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
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
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
            let hours = memory_time_scale(mem);
            mem.advance_hours(hours as f32);
            let report = mem.sleep();
            mem.fade_sitting();
            // Sleep is offered as an ordinary reconstructible lived episode.
            let _ = mem.record_sleep_memory();
            for t in &snap {
                mem.talk.record(&t.user, &t.reply);
            }
            mem.talk.topic = topic;
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
            ok(format!(
                "{{\"faded\":{},\"cold\":{},\"myth\":{},\"merged\":{},\"extinguished\":{},\"weathered\":{},\"rewritten\":{},\"released\":{},\"sculpted\":{},\"axioms\":{},\"sitting\":{},\"kind\":\"{}\",\"new_hours\":{},\"charge\":{:.3},\"hours_advanced\":{},\"time_scale\":{},\"now\":{}}}",
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
                report.charge,
                hours,
                memory_time_scale(mem),
                mem.clock.now()
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
            if let Err(e) = mem.save() { return err(500, &format!("memory persistence failed: {e}")); }
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
