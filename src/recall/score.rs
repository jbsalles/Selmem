//! Outside the organ. Sees the sealed claim and one sentence. Does not write.

use crate::net::httpx::{extract_json_string, json_esc, post_json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropositionLabel {
    Entail,
    Contradict,
    Unknown,
}

impl PropositionLabel {
    pub fn token(self) -> &'static str {
        match self {
            Self::Entail => "entail",
            Self::Contradict => "contradict",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        let w = s.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
        match w.as_str() {
            "entail" | "entails" | "same" => Self::Entail,
            "contradict" | "contradicts" | "contradiction" => Self::Contradict,
            _ => Self::Unknown,
        }
    }
}

/// Frozen labeler. No store, no archive, no write.
pub trait PropositionScorer: Send + Sync {
    fn score(&self, claim: &str, sentence: &str) -> PropositionLabel;
}

/// The organ does not invent a proposition. Absent scorer is unknown.
pub struct NullScorer;

impl PropositionScorer for NullScorer {
    fn score(&self, _claim: &str, _sentence: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
}

/// Temperature 0. The prompt is the two strings and nothing else.
pub struct HttpScorer {
    pub url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl HttpScorer {
    pub fn parse(url: &str, model: impl Into<String>, api_key: Option<String>) -> Option<Self> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return None;
        }
        Some(Self {
            url: url.to_string(),
            model: model.into(),
            api_key,
        })
    }
}

impl PropositionScorer for HttpScorer {
    fn score(&self, claim: &str, sentence: &str) -> PropositionLabel {
        let system = "Reply with one word: entail, contradict, or unknown. The claim is sealed. Do not rewrite it.";
        let user = format!("CLAIM:\n{claim}\nSENTENCE:\n{sentence}");
        let body = format!(
            "{{\"model\":\"{}\",\"temperature\":0,\"max_tokens\":8,\"messages\":[{{\"role\":\"system\",\"content\":\"{}\"}},{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            json_esc(&self.model),
            json_esc(system),
            json_esc(&user)
        );
        let raw = match post_json(&self.url, self.api_key.as_deref(), &body) {
            Ok(s) => s,
            Err(_) => return PropositionLabel::Unknown,
        };
        let text = extract_json_string(&raw, "content").unwrap_or_default();
        PropositionLabel::parse(&text)
    }
}
