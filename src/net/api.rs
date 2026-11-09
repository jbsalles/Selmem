use crate::core::model::{Attribution, Channel};
use crate::encode::EncodeInput;
use crate::engine::SelectiveMemory;

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub fn is_mouth(method: &str, path: &str) -> bool {
    method == "POST" && (path == "/turn" || path == "/speak")
}

pub fn mouth_user(path: &str, body: &str) -> Result<String, HttpResponse> {
    let text = json_str(body, "text")
        .or_else(|| json_str(body, "event"))
        .or_else(|| json_str(body, "query"))
        .unwrap_or_default();
    if text.trim().is_empty() {
        Err(err(400, "text requis"))
    } else {
        let _ = path;
        Ok(text)
    }
}

pub fn mouth_body(path: &str, reply: &str, mem: &SelectiveMemory) -> HttpResponse {
    if path == "/turn" {
        ok(format!(
            "{{\"reply\":\"{}\",\"topic\":\"{}\",\"mood\":{{\"valence\":{:.3},\"arousal\":{:.3},\"disgust\":{:.3}}}}}",
            json_esc(reply),
            json_esc(mem.talk.topic.as_deref().unwrap_or("")),
            mem.mood.valence,
            mem.mood.arousal,
            mem.mood.disgust
        ))
    } else {
        ok(format!(
            "{{\"reply\":\"{}\",\"topic\":\"{}\"}}",
            json_esc(reply),
            json_esc(mem.talk.topic.as_deref().unwrap_or(""))
        ))
    }
}

pub fn dispatch(
    mem: &mut SelectiveMemory,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
) -> HttpResponse {
    dispatch_for(mem, method, path, query, body, false)
}

/// Browser visitors never inherit daemon-wide LLM credentials or custom endpoints.
pub fn dispatch_public(
    mem: &mut SelectiveMemory,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
) -> HttpResponse {
    dispatch_for(mem, method, path, query, body, true)
}

mod routes;
use routes::dispatch_for;

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
        Confabulate => "confab",
        Suppress => "suppress",
    }
}

fn book_json(mem: &SelectiveMemory) -> String {
    let mut traces: Vec<_> = mem.store.traces.values().collect();
    traces.sort_by_key(|t| std::cmp::Reverse(t.created_at));
    let tjson: Vec<String> = traces
        .into_iter()
        .map(|t| {
            format!(
                "{{\"id\":\"{}\",\"status\":\"{}\",\"channel\":\"{}\",\"schema\":\"{}\",\"gist\":\"{}\",\"core\":\"{}\",\"fidelity\":{:.3},\"anchor\":{:.3},\"valence\":{:.3},\"created_at\":{},\"archive_id\":\"{}\",\"drift\":\"{}\"}}",
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
                json_esc(t.archive_id.as_deref().unwrap_or("")),
                json_esc(t.drifts.last().map(|d| d.note.as_str()).unwrap_or(""))
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

fn llm_json(mem: &SelectiveMemory, visitor: bool) -> String {
    let attached = !mem.llm.url.is_empty();
    let key = mem.llm.key.as_deref().unwrap_or("");
    let cfg = crate::config::Config::get();
    let mouths = if visitor {
        ["grok", "gpt"]
            .iter()
            .filter_map(|plug| {
                crate::config::expand_plug(plug).map(|(url, model)| crate::config::Mouth {
                    plug: (*plug).into(),
                    url: url.into(),
                    model: model.into(),
                    api_key: None,
                })
            })
            .collect()
    } else {
        cfg.plugs()
    };
    let plugs: Vec<String> = mouths
        .into_iter()
        .map(|p| {
            format!(
                "{{\"name\":\"{}\",\"model\":\"{}\",\"url\":\"{}\",\"has_key\":{}}}",
                json_esc(&p.plug),
                json_esc(&p.model),
                json_esc(&p.url),
                if p.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
            )
        })
        .collect();
    let default = if visitor {
        String::new()
    } else {
        cfg.default_llm().unwrap_or_default()
    };
    format!(
        "{{\"ok\":true,\"attached\":{},\"plug\":\"{}\",\"default\":\"{}\",\"url\":\"{}\",\"model\":\"{}\",\"has_key\":{},\"key_hint\":\"{}\",\"plugs\":[{}]}}",
        if attached { "true" } else { "false" },
        json_esc(&mem.llm.plug),
        json_esc(&default),
        json_esc(&mem.llm.url),
        json_esc(&mem.llm.model),
        if mem.llm.key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
            "true"
        } else {
            "false"
        },
        json_esc(&if key.is_empty() { String::new() } else if visitor { "••••".into() } else { mask_key(key) }),
        plugs.join(","),
    )
}

fn memory_time_scale(mem: &SelectiveMemory) -> u32 {
    mem.clock.scale
}

fn profile_json(mem: &SelectiveMemory) -> String {
    format!(
        "{{\"ok\":true,\"name\":\"{}\",\"voice\":\"{}\",\"encode_threshold\":{:.4},\"w_self\":{:.4},\"embellish_gain\":{:.4},\"disgust_gain\":{:.4},\"decay_lambda\":{:.4},\"narrator_firmness\":{:.4},\"max_recall\":{},\"merge_similarity\":{:.4},\"ground_min_overlap\":{:.4},\"ground_strikes\":{},\"reconsolidation_eta\":{:.4},\"time_scale\":{},\"cut_reconsolidate\":{},\"cut_ground\":{},\"cut_ladder\":{},\"associations_learn\":{},\"associations_reconstruct\":{}}}",
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
        memory_time_scale(mem),
        if mem.cut.reconsolidate { "true" } else { "false" },
        if mem.cut.ground { "true" } else { "false" },
        if mem.cut.ladder { "true" } else { "false" },
        if mem.store.associations.learn { "true" } else { "false" },
        if mem.store.associations.reconstruct { "true" } else { "false" },
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

/// Last user line that is not "remember this / pin this".
fn sitting_pin_text(talk: &crate::core::talk::WorkingTalk) -> String {
    for t in talk.turns.iter().rev() {
        let s = t.user.trim();
        if s.is_empty() || pin_command(s) {
            continue;
        }
        return s.to_string();
    }
    talk.topic.clone().unwrap_or_default()
}

fn pin_command(s: &str) -> bool {
    let n = s.split_whitespace().count();
    if n > 8 {
        return false;
    }
    let l = s.to_lowercase();
    (l.contains("remember") || l.contains("souviens") || l.contains("pin "))
        && (l.contains("this") || l.contains("ça") || l.contains("ca") || l.contains("ok"))
}
