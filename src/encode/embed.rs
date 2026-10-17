use crate::net::httpx::{extract_json_array_f32, json_esc, post_json};
use crate::encode::scoring::token_set;

pub const EMBED_DIM: usize = 96;

#[derive(Clone, Debug)]
pub enum EmbeddingResult {
    External(Vec<f32>),
    Fallback(Vec<f32>),
}

#[derive(Clone, Debug)]
pub struct EmbedLog {
    pub backend: String,
    pub fallback_used: bool,
    pub error: Option<String>,
    pub calls: u32,
}

impl Default for EmbedLog {
    fn default() -> Self {
        Self {
            backend: "hash".into(),
            fallback_used: false,
            error: None,
            calls: 0,
        }
    }
}

pub trait Embedder: Send + Sync {
    /// Dense hash sketches are lexical fallbacks, not evidence of semantic proximity.
    fn semantic_similarity(&self) -> bool { true }
    fn space_id(&self) -> String { std::any::type_name::<Self>().to_string() }
    fn embed(&self, text: &str) -> Vec<f32> {
        match self.embed_result(text) {
            EmbeddingResult::External(v) | EmbeddingResult::Fallback(v) => v,
        }
    }
    fn embed_result(&self, text: &str) -> EmbeddingResult;
    fn embed_log(&self) -> EmbedLog {
        EmbedLog::default()
    }
}

#[derive(Default, Clone)]
pub struct HashEmbedder;

pub struct HttpEmbedder {
    pub url: String,
    pub model: String,
    pub api_key: Option<String>,
    log: std::sync::Mutex<EmbedLog>,
}

impl HttpEmbedder {
    pub fn parse(endpoint: &str, model: impl Into<String>, api_key: Option<String>) -> Option<Self> {
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return None;
        }
        let model = model.into();
        Some(Self {
            url: endpoint.to_string(),
            log: std::sync::Mutex::new(EmbedLog {
                backend: model.clone(),
                ..EmbedLog::default()
            }),
            model,
            api_key,
        })
    }
}

impl Embedder for HttpEmbedder {
    fn space_id(&self) -> String { format!("http:{}:{}", self.url, self.model) }
    fn embed_result(&self, text: &str) -> EmbeddingResult {
        let body = format!(
            "{{\"model\":\"{}\",\"input\":\"{}\"}}",
            json_esc(&self.model),
            json_esc(text)
        );
        if let Ok(mut log) = self.log.lock() {
            log.calls = log.calls.saturating_add(1);
        }
        match post_json(&self.url, self.api_key.as_deref(), &body) {
            Ok(raw) => match extract_json_array_f32(&raw, "embedding") {
                Some(v) if !v.is_empty() && v.iter().all(|x| x.is_finite()) && v.iter().any(|x| *x != 0.0) => EmbeddingResult::External(v),
                _ => self.fell_back(text, "unreadable embedding"),
            },
            Err(e) => self.fell_back(text, &e),
        }
    }

    fn embed_log(&self) -> EmbedLog {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl HttpEmbedder {
    fn fell_back(&self, _text: &str, err: &str) -> EmbeddingResult {
        if let Ok(mut log) = self.log.lock() {
            log.fallback_used = true;
            log.backend = "lexical".into();
            log.error = Some(err.to_string());
        }
        eprintln!("selmem embed fallback: {err}");
        // Empty vectors force lexical retrieval; hash and remote spaces must never mix.
        EmbeddingResult::Fallback(Vec::new())
    }
}

impl Embedder for HashEmbedder {
    fn semantic_similarity(&self) -> bool { false }
    fn space_id(&self) -> String { "hash96-v1".into() }
    fn embed_result(&self, text: &str) -> EmbeddingResult {
        EmbeddingResult::External(self.embed_hash(text))
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        self.embed_hash(text)
    }
}

impl HashEmbedder {
    fn embed_hash(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; EMBED_DIM];
        let lower = text.to_lowercase();
        let chars: Vec<char> = lower.chars().filter(|c| c.is_alphanumeric()).collect();
        for n in 2..=3 {
            if chars.len() >= n {
                for w in chars.windows(n) {
                    let s: String = w.iter().collect();
                    bump(&mut v, &s, 1.0);
                }
            }
        }
        for tok in token_set(&lower) {
            bump(&mut v, &tok, 1.4);
        }
        l2_normalize(&mut v);
        v
    }
}

fn bump(v: &mut [f32], s: &str, w: f32) {
    let h = fnv(s);
    let i = (h as usize) % v.len();
    v[i] += w;
    let j = ((h >> 17) as usize) % v.len();
    if j != i {
        v[j] += w * 0.5;
    }
}

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn l2_normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 1e-8 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let d = na.sqrt() * nb.sqrt();
    if d < 1e-8 {
        0.0
    } else {
        (dot / d).clamp(-1.0, 1.0)
    }
}

pub fn novelty_emb(query: &[f32], existing: &[Vec<f32>]) -> f32 {
    if existing.is_empty() {
        return 1.0;
    }
    let nearest = existing
        .iter()
        .map(|e| cosine(query, e))
        .fold(0.0_f32, f32::max);
    (1.0 - nearest).clamp(0.0, 1.0)
}

#[cfg(test)]
mod consistency_tests {
    use super::*;
    #[test]
    fn provider_failure_cannot_smuggle_hash_vectors_into_remote_space() {
        let e=HttpEmbedder::parse("http://localhost:1/embeddings","remote",None).unwrap();
        match e.fell_back("some text","injected outage") {EmbeddingResult::Fallback(v)=>assert!(v.is_empty()),_=>panic!("expected fallback")}
        assert!(e.embed_log().fallback_used);assert_eq!(e.embed_log().backend,"lexical");assert!(e.space_id().contains("remote"));
    }
}
