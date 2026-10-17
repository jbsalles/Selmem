//! One budget for every HTTP attempt, including scorer calls and retries.
use super::{
    json::{num, obj, text},
    runner::{log, Result},
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
struct Budget {
    out: PathBuf,
    limit: usize,
    used: usize,
}
thread_local! { static BUDGET: RefCell<Option<Budget>> = const { RefCell::new(None) }; }
pub(super) struct Session;
pub(super) fn begin(out: &Path, limit: usize) -> Session {
    enable_semantic_cache(false);
    BUDGET.with(|b| {
        *b.borrow_mut() = Some(Budget {
            out: out.into(),
            limit,
            used: 0,
        })
    });
    Session
}
impl Drop for Session {
    fn drop(&mut self) {
        BUDGET.with(|b| *b.borrow_mut() = None);
        enable_semantic_cache(false);
    }
}
pub(super) fn used() -> usize {
    BUDGET.with(|b| b.borrow().as_ref().map_or(0, |b| b.used))
}
pub(super) fn post(kind: &str, url: &str, key: Option<&str>, body: &str) -> Result<String> {
    for attempt in 1..=3 {
        let slot = BUDGET.with(|b| -> Result<_> {
            let mut b = b.borrow_mut();
            let Some(b) = b.as_mut() else { return Ok(None) };
            if b.used >= b.limit {
                return Err(format!(
                    "HTTP budget exhausted: {}/{} attempts",
                    b.used, b.limit
                ));
            }
            let id = b.used + 1;
            log(
                &b.out.join("http.jsonl"),
                &obj([
                    ("phase", text("start")),
                    ("id", num(id as f64)),
                    ("kind", text(kind)),
                    ("attempt", num(attempt as f64)),
                    ("endpoint", text(url)),
                    ("request_body", text(body)),
                ]),
            )?;
            b.used = id;
            Ok(Some((id, b.out.clone())))
        })?;
        if let Some((id, _)) = &slot {
            super::progress::line(format!("HTTP #{id} START {kind} attempt={attempt}"));
        }
        let started = Instant::now();
        let response = selmem::net::httpx::post_json(url, key, body);
        if let Some((id, out)) = &slot {
            log(
                &out.join("http.jsonl"),
                &obj([
                    ("phase", text("finish")),
                    ("id", num(*id as f64)),
                    ("kind", text(kind)),
                    ("seconds", num(started.elapsed().as_secs_f64())),
                    (
                        "response",
                        response
                            .as_ref()
                            .map(text)
                            .unwrap_or(super::json::Json::Null),
                    ),
                    (
                        "error",
                        response
                            .as_ref()
                            .err()
                            .map(text)
                            .unwrap_or(super::json::Json::Null),
                    ),
                ]),
            )?;
            super::progress::line(format!(
                "HTTP #{id} {} {kind}",
                if response.is_ok() { "DONE" } else { "FAILED" }
            ));
        }
        let transient = response.as_ref().err().is_some_and(|e| transient_error(e));
        let remaining = BUDGET.with(|b| b.borrow().as_ref().is_some_and(|b| b.used < b.limit));
        if transient && remaining && attempt < 3 {
            let wait = 30 * attempt;
            super::progress::line(format!("HTTP retry {kind} in {wait}s"));
            std::thread::sleep(Duration::from_secs(wait));
        } else {
            return response;
        }
    }
    unreachable!()
}
fn transient_error(e:&str)->bool {
    e.starts_with("curl: (28)") || ["HTTP 429:","HTTP 502:","HTTP 503:","HTTP 504:"].iter().any(|prefix|e.starts_with(prefix))
}
#[cfg(test)]
mod tests {
    #[test] fn timeouts_retry_but_permanent_errors_do_not(){
        assert!(super::transient_error("curl: (28) Operation timed out"));
        assert!(super::transient_error("HTTP 429: capacity"));
        assert!(!super::transient_error("HTTP 401: invalid key"));
        assert!(!super::transient_error("HTTP 400: invalid request"));
    }
    use super::*;
    #[test]
    fn zero_budget_never_attempts_network() {
        let _session = begin(Path::new("unused"), 0);
        assert!(post("test", "http://127.0.0.1:1", None, "{}")
            .unwrap_err()
            .contains("budget exhausted"));
        assert_eq!(used(), 0);
    }
    #[test]
    fn failed_connection_is_recorded_and_counts_once() {
        let out = std::env::temp_dir().join(format!("genesis-budget-{}", std::process::id()));
        std::fs::create_dir_all(&out).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let _session = begin(&out, 1);
        assert!(post("test", &url, None, "{}").is_err());
        assert_eq!(used(), 1);
        assert!(post("test", &url, None, "{}")
            .unwrap_err()
            .contains("budget exhausted"));
        let rows = std::fs::read_to_string(out.join("http.jsonl")).unwrap();
        assert_eq!(rows.lines().count(), 2);
        assert!(!rows.lines().last().unwrap().contains("\"error\":null"));
        std::fs::remove_dir_all(out).unwrap();
    }
}

/// Restore attempted calls, including a started request without a returned result.
pub(super) fn resume(out:&Path,limit:usize)->Result<Session>{
 enable_semantic_cache(false);
 let path=out.join("http.jsonl");let mut used=0;
 if path.exists(){let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;let end=bytes.iter().rposition(|b|*b==b'\n').map_or(0,|p|p+1);
  if end<bytes.len(){let f=std::fs::OpenOptions::new().write(true).open(&path).map_err(|e|e.to_string())?;f.set_len(end as u64).map_err(|e|e.to_string())?;f.sync_all().map_err(|e|e.to_string())?;log(&out.join("http-recovery.jsonl"),&obj([("trailing_bytes",num((bytes.len()-end) as f64))]))?;}
  for line in std::str::from_utf8(&bytes[..end]).map_err(|e|e.to_string())?.lines(){let r=super::json::parse(line)?;if r.get("phase")?.string()?=="start"{used=used.max(r.get("id")?.number()? as usize);}}
 }
 BUDGET.with(|b|*b.borrow_mut()=Some(Budget{out:out.into(),limit,used}));Ok(Session)
}

// Only stateless semantic annotations may be reused. Memory reconstruction,
// choices and creative generations remain independent calls.
#[derive(Default)]
struct SemanticCache { enabled: bool, hits: usize, misses: usize }
thread_local! { static SEMANTIC_CACHE: RefCell<SemanticCache> = RefCell::new(SemanticCache::default()); }
pub(super) fn enable_semantic_cache(enabled: bool) {
    SEMANTIC_CACHE.with(|c| *c.borrow_mut() = SemanticCache { enabled, ..Default::default() });
}
pub(super) fn semantic_cache_stats() -> super::json::Json {
    SEMANTIC_CACHE.with(|c| { let c=c.borrow(); obj([
        ("enabled",super::json::Json::Bool(c.enabled)),
        ("hits_this_session",num(c.hits as f64)),
        ("misses_this_session",num(c.misses as f64)),
    ]) })
}
fn semantic_cache_path(url:&str,body:&str)->Option<(String,PathBuf)> {
    let enabled=SEMANTIC_CACHE.with(|c|c.borrow().enabled);
    if !enabled {return None;}
    let key=super::sha256::hex(obj([("policy",text("validated-semantic-1")),("url",text(url)),("request",text(body))]).encode().as_bytes());
    BUDGET.with(|b|b.borrow().as_ref().map(|b|(key.clone(),b.out.join("semantic-cache").join(format!("{key}.json")))))
}
pub(super) fn semantic_lookup(url:&str,body:&str)->Result<Option<String>> {
    let Some((key,path))=semantic_cache_path(url,body) else {return Ok(None)};
    if !path.exists(){SEMANTIC_CACHE.with(|c|c.borrow_mut().misses+=1);return Ok(None);}
    let cached=super::runner::load(&path)?;
    let raw=super::runner::s(&cached,"raw_response")?;
    if super::runner::s(&cached,"request_sha256")?!=key || super::runner::s(&cached,"response_sha256")?!=super::sha256::hex(raw.as_bytes()) {
        return Err("semantic cache integrity mismatch; preserve the cache and journal".into());
    }
    SEMANTIC_CACHE.with(|c|c.borrow_mut().hits+=1);
    BUDGET.with(|b|->Result<()>{if let Some(b)=b.borrow().as_ref(){log(&b.out.join("semantic-cache-audit.jsonl"),&obj([("request_sha256",text(&key)),("kind",text("semantic_cache_hit"))]))?;}Ok(())})?;
    super::progress::line("SEMANTIC CACHE HIT: no HTTP request");
    Ok(Some(raw.into()))
}
/// Call only after parsing and validating the annotations successfully.
pub(super) fn semantic_store_validated(url:&str,body:&str,raw:&str)->Result<()> {
    let Some((key,path))=semantic_cache_path(url,body) else {return Ok(())};
    super::runner::write(&path,&obj([("request_sha256",text(key)),("response_sha256",text(super::sha256::hex(raw.as_bytes()))),("raw_response",text(raw))]))
}

#[cfg(test)]mod cache_tests{
 use super::*;
 #[test]fn cache_keys_include_complete_request_and_endpoint_and_detect_corruption(){
  let out=super::super::lifecycle_tests::root();std::fs::create_dir_all(&out).unwrap();let _session=begin(&out,1);enable_semantic_cache(true);
  semantic_store_validated("endpoint-a","model=one;reasoning=none;event=one","validated reply").unwrap();
  assert_eq!(semantic_lookup("endpoint-a","model=one;reasoning=none;event=one").unwrap().as_deref(),Some("validated reply"));
  for body in ["model=two;reasoning=none;event=one","model=one;reasoning=low;event=one","model=one;reasoning=none;event=two"]{assert!(semantic_lookup("endpoint-a",body).unwrap().is_none());}
  assert!(semantic_lookup("endpoint-b","model=one;reasoning=none;event=one").unwrap().is_none());assert_eq!(used(),0);
  let (_,path)=semantic_cache_path("endpoint-a","model=one;reasoning=none;event=one").unwrap();let mut r=super::super::runner::load(&path).unwrap();r.put("raw_response",text("altered reply"));super::super::runner::write(&path,&r).unwrap();assert!(semantic_lookup("endpoint-a","model=one;reasoning=none;event=one").is_err());std::fs::remove_dir_all(out).unwrap();
 }
}
