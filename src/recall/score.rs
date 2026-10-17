//! Outside the organ. Sees the sealed claim and one sentence. Does not write.

use crate::net::httpx::{scorer_answer, json_esc, post_json};

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
        let w = s.trim().to_ascii_lowercase();
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
    fn name(&self) -> &str;
}

/// The organ does not invent a proposition. Absent scorer is unknown.
pub struct NullScorer;

impl PropositionScorer for NullScorer {
    fn score(&self, _claim: &str, _sentence: &str) -> PropositionLabel {
        PropositionLabel::Unknown
    }
    fn name(&self) -> &str {
        "null"
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

impl HttpScorer {
    fn request_body(&self, claim: &str, sentence: &str, effort: &str) -> String {
        let system = "Reply with one word: entail, contradict, or unknown. The claim is sealed. Do not rewrite it.";
        let user = format!("CLAIM:\n{claim}\nSENTENCE:\n{sentence}");
        let effort = crate::config::normalize_reasoning(effort);
        let extra = format!(",\"reasoning_effort\":\"{}\"", json_esc(&effort));
        let cap = super::http::token_cap_field(&self.url, &self.model);
        let model = self.model.to_ascii_lowercase();
        let reasoning_model = ["gpt-5", "gpt-6", "o1", "o3", "o4"].iter().any(|prefix| model.starts_with(prefix));
        let temperature = if reasoning_model { "" } else { "\"temperature\":0," };
        let budget = if effort != "none" && effort != "off" && !effort.is_empty() { 2048 } else { 128 };
        format!(
            "{{\"model\":\"{}\",{temperature}\"{cap}\":{budget}{extra},\"messages\":[{{\"role\":\"system\",\"content\":\"{}\"}},{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            json_esc(&self.model),
            json_esc(system),
            json_esc(&user)
        )
    }
}

impl PropositionScorer for HttpScorer {
    fn score(&self, claim: &str, sentence: &str) -> PropositionLabel {
        let body = self.request_body(claim, sentence, &crate::config::Config::get().reasoning());
        let raw = match post_json(&self.url, self.api_key.as_deref(), &body) {
            Ok(s) => s,
            Err(_) => return PropositionLabel::Unknown,
        };
        let text = scorer_answer(&raw).unwrap_or_default();
        if text.contains(claim) || text.contains(sentence) {
            return PropositionLabel::Unknown;
        }
        PropositionLabel::parse(&text)
    }

    fn name(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    #[test]
    fn requests_use_completion_caps_and_respect_reasoning_policy() {
        let gpt=HttpScorer::parse("https://api.openai.com/v1/chat/completions","gpt-5",None).unwrap();
        let body=gpt.request_body("claim","sentence","low");assert!(body.contains("\"max_completion_tokens\":2048"));assert!(body.contains("\"reasoning_effort\":\"low\""));assert!(!body.contains("temperature"));assert!(!body.contains("\"max_tokens\""));
        let grok=HttpScorer::parse("https://api.x.ai/v1/chat/completions","grok",None).unwrap();let body=grok.request_body("claim","sentence","off");assert!(body.contains("\"max_tokens\":128"));assert!(body.contains("\"temperature\":0"));assert!(body.contains("\"reasoning_effort\":\"none\""));
    }
}
