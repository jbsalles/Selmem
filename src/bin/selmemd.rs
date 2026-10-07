#[path = "selmemd/sessions.rs"]
mod sessions;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use selmem::{api, Config, EntityProfile, HttpEmbedder, SelectiveMemory};

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg = Config::get();
    let bind = cfg.resolve_or(flag(&args, "--bind"), "bind", "127.0.0.1:7420");
    let path = cfg.resolve_or(flag(&args, "--path"), "path", "entity.db");
    let name = cfg.resolve_or(flag(&args, "--name"), "name", "");
    let kind = cfg.resolve_or(flag(&args, "--profile"), "profile", "tender");
    let key = cfg.resolve(flag(&args, "--api-key"), "api_key");
    let embed_url = cfg.resolve(flag(&args, "--embed"), "embed");
    let public_demo = cfg.resolve(None, "public_demo").map(|s| matches!(s.as_str(), "1" | "true" | "yes")).unwrap_or(false);
    let token = if public_demo { None } else { cfg.resolve(flag(&args, "--token"), "token") };
    let visitors = public_demo.then(|| Arc::new(sessions::Sessions::new(profile_for_demo(&name, &kind))));

    let profile = match kind.as_str() {
        "austere" => EntityProfile::austere(name),
        _ => EntityProfile::tender(name),
    };

    let mut mem = if public_demo { SelectiveMemory::new(profile) } else { SelectiveMemory::open(&path, profile).expect("failed to open memory") };
    if let Some(v) = cfg
        .resolve(flag(&args, "--ground-overlap"), "ground_overlap")
        .and_then(|s| s.parse().ok())
    {
        mem.profile.ground_min_overlap = v;
    }
    if let Some(v) = cfg
        .resolve(flag(&args, "--ground-strikes"), "ground_strikes")
        .and_then(|s| s.parse().ok())
    {
        mem.profile.ground_strikes = v;
    }
    if let Some(v) = cfg
        .resolve(flag(&args, "--narrator-firmness"), "narrator_firmness")
        .and_then(|s| s.parse().ok())
    {
        mem.profile.narrator_firmness = v;
    }
    if !public_demo {
    let mouth = match cfg.mouth(flag(&args, "--llm"), flag(&args, "--model"), flag(&args, "--plug"), "llama3") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    if let Some(mouth) = mouth {
        match mem.set_plug(&mouth.plug, &mouth.url, &mouth.model, mouth.api_key) {
            Ok(()) => eprintln!("HTTP narrator attached  {} {}", mouth.plug, mouth.model),
            Err(e) => eprintln!("{e}; repli RuleNarrator"),
        }
    }
    if let Some(url) = flag(&args, "--scorer").or(cfg.resolve(None, "scorer")) {
        let model = cfg.resolve_or(flag(&args, "--scorer-model"), "scorer_model", "llama3");
        let key = cfg.resolve(None, "scorer_key").or(key.clone());
        if let Some(s) = selmem::HttpScorer::parse(&url, model, key) {
            mem = mem.with_scorer(Box::new(s));
            eprintln!("scorer attached  {url}");
        }
    } else {
        eprintln!("scorer absent");
    }
    if let Some(url) = embed_url {
        let emodel = cfg.resolve_or(flag(&args, "--embed-model"), "embed_model", "text-embedding-3-small");
        if let Some(e) = HttpEmbedder::parse(&url, emodel, key.clone()) {
            mem = mem.with_embedder(Box::new(e));
            eprintln!("HTTP embeddings attached");
        }
    }

    }

    let mem = Arc::new(Mutex::new(Some(mem)));
    let listener = TcpListener::bind(&bind).expect("bind");
    eprintln!("selmemd sur http://{bind}  fichier={path}");
    if let Some(p) = cfg.path.as_ref() {
        eprintln!("config {}", p.display());
    }
    if token.is_some() {
        eprintln!("auth: Authorization: Bearer requis (sauf /health et /)");
    }
    eprintln!("UI  http://{bind}/");
    eprintln!("POST /turn /live /remember /sleep /speak /talk/clear   GET /who /lineage /mood /talk /health");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let mem = Arc::clone(&mem);
                let token = token.clone();
                let visitors = visitors.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_conn(s, &mem, token.as_deref(), visitors.as_deref()) {
                        eprintln!("req: {e}");
                    }
                });
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
}

fn handle_conn(
    mut stream: TcpStream,
    mem: &Mutex<Option<SelectiveMemory>>,
    token: Option<&str>,
    visitors: Option<&sessions::Sessions>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(90))).ok();
    let mut buf = vec![0u8; 8192];
    let mut data = Vec::new();
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(pos) = find_headers_end(&data) {
            let header = String::from_utf8_lossy(&data[..pos]);
            let mut lines = header.split("\r\n");
            let start = lines.next().unwrap_or("");
            let mut parts = start.split_whitespace();
            let method = parts.next().unwrap_or("GET").to_string();
            let target = parts.next().unwrap_or("/").to_string();
            let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
            let mut content_len = 0usize;
            let mut auth = String::new();
            let mut audit_role = false;
            let mut cookies = String::new();
            let mut secure = false;
            for line in lines {
                let l = line.to_ascii_lowercase();
                if let Some(v) = l.strip_prefix("content-length:") {
                    content_len = v.trim().parse().unwrap_or(0);
                }
                if l.starts_with("authorization:") {
                    auth = line.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
                }
                if l.starts_with("cookie:") {
                    cookies = line.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
                }
                if l.starts_with("x-forwarded-proto:") { secure = l.split_once(':').map(|(_, v)| v.trim() == "https").unwrap_or(false); }
                if l.starts_with("x-selmem-audit:") {
                    audit_role = l.split_once(':').map(|(_, v)| v.trim() == "1").unwrap_or(false);
                }
            }
            let content_len = content_len.min(1_000_000);
            let body_start = pos + 4;
            while data.len() < body_start + content_len {
                let n = stream.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..n]);
            }
            let body = String::from_utf8_lossy(&data[body_start..body_start + content_len.min(data.len().saturating_sub(body_start))]).to_string();
            if method == "GET" && (path == "/" || path == "/ui" || path == "/index.html") {
                write_http_ct(&mut stream, 200, "text/html; charset=utf-8", UI)?;
                return Ok(());
            }
            if method == "GET" && path == "/session" {
                if let Some(visitors) = visitors {
                    let Some((id, _)) = visitors.resolve(sessions::cookie_id(&cookies), true)? else {
                        return write_http(&mut stream, 503, "{\"error\":\"demo full; try again later\"}");
                    };
                    let cookie = format!("Set-Cookie: selmem_session={id}; HttpOnly; SameSite=Strict; Path=/{}\r\n", if secure { "; Secure" } else { "" });
                    return write_http_extra(&mut stream, 200, "application/json", "{\"public_demo\":true}", &cookie);
                }
                return write_http(&mut stream, 200, "{\"public_demo\":false}");
            }
            if visitors.is_some() && method == "GET" && path == "/health" {
                return write_http(&mut stream, 200, "{\"ok\":true,\"public_demo\":true}");
            }
            if let Some(tok) = token {
                let allowed = path == "/health" || method == "OPTIONS";
                let ok_auth = auth.len() > 7
                    && auth[..7].eq_ignore_ascii_case("bearer ")
                    && auth[7..].trim() == tok;
                if !allowed && !ok_auth {
                    write_http(&mut stream, 401, "{\"error\":\"unauthorized\"}")?;
                    return Ok(());
                }
            }
            if path == "/audit" && !audit_role {
                write_http(&mut stream, 403, "{\"error\":\"audit actor required\"}")?;
                return Ok(());
            }
            let session;
            let organ = if let Some(visitors) = visitors {
                let Some((_, selected)) = visitors.resolve(sessions::cookie_id(&cookies), false)? else {
                    return write_http(&mut stream, 401, "{\"error\":\"session expired\"}");
                };
                session = selected;
                session.as_ref()
            } else { mem };
            let res = dispatch_unlocked(organ, &method, path, query, &body, visitors.is_some())?;
            write_http(&mut stream, res.status, &res.body)?;
            return Ok(());
        }
        if data.len() > 1_000_000 {
            break;
        }
    }
    Ok(())
}


fn dispatch_unlocked(
    mem: &Mutex<Option<SelectiveMemory>>,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
    public_demo: bool,
) -> std::io::Result<selmem::api::HttpResponse> {
    let busy = || selmem::api::HttpResponse {
        status: 503,
        body: "{\"error\":\"memory busy\"}".into(),
    };
    if api::is_mouth(method, path) {
        let text = match api::mouth_user(path, body) {
            Ok(t) => t,
            Err(res) => return Ok(res),
        };
        let (draft, narrator) = {
            let mut slot = mem.lock().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "memory locked")
            })?;
            let Some(g) = slot.as_mut() else {
                return Ok(busy());
            };
            if g.mouth_held() {
                return Ok(busy());
            }
            if public_demo && !g.llm.url.is_empty() && g.llm.key.as_ref().map(|key| key.trim().is_empty()).unwrap_or(true) {
                return Ok(selmem::api::HttpResponse { status: 400, body: "{\"error\":\"Enter your provider API key or choose rules\"}".into() });
            }
            let draft = g.open_mouth(&text);
            let narrator = g.narrator_arc();
            (draft, narrator)
        };
        let reply = selmem::Narrator::reply(narrator.as_ref(), &draft.user, &draft.memories, &draft.axioms, &draft.mood, &draft.talk);
        let mut slot = mem.lock().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "memory locked")
        })?;
        let Some(g) = slot.as_mut() else {
            return Ok(busy());
        };
        g.close_mouth(&draft, &reply);
        let _ = g.save();
        return Ok(api::mouth_body(path, &reply, g));
    }
    let external = method == "POST";
    if external {
        let mut organ = {
            let mut slot = mem.lock().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "memory locked")
            })?;
            let Some(organ) = slot.as_mut() else {
                return Ok(busy());
            };
            if organ.mouth_held() {
                return Ok(busy());
            }
            let organ = slot.take().unwrap();
            organ
        };
        let res = if public_demo { api::dispatch_public(&mut organ, method, path, query, body) } else { api::dispatch(&mut organ, method, path, query, body) };
        let mut slot = mem.lock().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "memory locked")
        })?;
        *slot = Some(organ);
        return Ok(res);
    }
    let mut slot = mem.lock().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::Other, "memory locked")
    })?;
    let Some(g) = slot.as_mut() else {
        return Ok(busy());
    };
    Ok(if public_demo { api::dispatch_public(g, method, path, query, body) } else { api::dispatch(g, method, path, query, body) })
}

fn find_headers_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

const UI: &str = include_str!("../net/ui.html");

fn write_http(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    write_http_ct(stream, status, "application/json; charset=utf-8", body)
}

fn write_http_ct(stream: &mut TcpStream, status: u16, ctype: &str, body: &str) -> std::io::Result<()> {
    write_http_extra(stream, status, ctype, body, "")
}

fn write_http_extra(stream: &mut TcpStream, status: u16, ctype: &str, body: &str, extra: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nCache-Control: no-store\r\n{extra}Access-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find_map(|w| {
        if w[0] == name {
            Some(w[1].clone())
        } else {
            None
        }
    })
}

fn profile_for_demo(name: &str, kind: &str) -> EntityProfile {
    if kind == "austere" { EntityProfile::austere(name) } else { EntityProfile::tender(name) }
}
