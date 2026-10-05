use crate::net::httpx::{extract_json_string, json_esc, post_json};
use crate::core::model::{MemoryTrace, Mood};
use crate::core::talk::WorkingTalk;
use crate::recall::narrator::{FailurePolicy, LlmCallLog, Narrator, RuleNarrator};

pub struct HttpNarrator {
    pub url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub policy: FailurePolicy,
    log: std::sync::Mutex<LlmCallLog>,
    fallback: RuleNarrator,
}

impl HttpNarrator {
    pub fn parse(endpoint: &str, model: impl Into<String>, api_key: Option<String>) -> Option<Self> {
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return None;
        }
        let model = model.into();
        Some(Self {
            url: endpoint.to_string(),
            log: std::sync::Mutex::new(LlmCallLog {
                narrator: model.clone(),
                ..LlmCallLog::default()
            }),
            model,
            api_key,
            policy: FailurePolicy::Fallback,
            fallback: RuleNarrator,
        })
    }

    pub fn with_policy(mut self, policy: FailurePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// True when the caller may use RuleNarrator. Error policy never may.
    fn note_fail(&self, err: &str) -> bool {
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.failures = log.failures.saturating_add(1);
        log.llm_error = Some(err.to_string());
        match self.policy {
            FailurePolicy::Error => false,
            FailurePolicy::Fallback | FailurePolicy::RecordAndFallback => {
                log.fallback_used = true;
                true
            }
        }
    }

    fn miss(&self, err: &str) -> String {
        if self.note_fail(err) {
            String::new()
        } else {
            format!("[llm-error] {err}")
        }
    }

    fn chat(&self, system: &str, user: &str) -> Result<String, String> {
        let cfg = crate::config::Config::get();
        let temp = cfg.temp();
        let effort = cfg.reasoning();
        let extra = if effort.is_empty() || effort == "off" {
            String::new()
        } else {
            format!(",\"reasoning_effort\":\"{}\"", json_esc(&effort))
        };
        let cap = token_cap_field(&self.url, &self.model);
        let body = format!(
            "{{\"model\":\"{}\",\"temperature\":{},\"{cap}\":280{} ,\"messages\":[{{\"role\":\"system\",\"content\":\"{}\"}},{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            json_esc(&self.model),
            temp,
            extra,
            json_esc(system),
            json_esc(user)
        );
        if let Ok(mut log) = self.log.lock() {
            log.calls = log.calls.saturating_add(1);
            if log.narrator.is_empty() {
                log.narrator = self.model.clone();
            }
        }
        let raw = post_json(&self.url, self.api_key.as_deref(), &body)?;
        if let Some(msg) = api_error(&raw) {
            return Err(msg);
        }
        extract_json_string(&raw, "content")
            .filter(|s| !s.trim().is_empty())
            .or_else(|| loose_content(&raw))
            .ok_or_else(|| {
                let clip: String = raw.chars().take(180).collect();
                format!("unreadable LLM reply ({} bytes): {clip}", raw.len())
            })
    }
}

fn loose_content(raw: &str) -> Option<String> {
    let key = "\"content\":\"";
    let i = raw.find(key)?;
    let start = i + key.len() - 1;
    crate::net::httpx::parse_json_string(&raw[start..])
        .map(|(s, _)| s)
        .filter(|s| !s.trim().is_empty())
}

/// GPT-5/6 and the o-series reject `max_tokens`. Grok / Ollama still want it.
fn token_cap_field(url: &str, model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    let openai = url.contains("api.openai.com")
        || url.contains("openai.azure.com")
        || m.starts_with("gpt-5")
        || m.starts_with("gpt-6")
        || m.starts_with("o1")
        || m.starts_with("o3")
        || m.starts_with("o4");
    if openai {
        "max_completion_tokens"
    } else {
        "max_tokens"
    }
}

impl Narrator for HttpNarrator {
    fn failure_log(&self) -> LlmCallLog {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn reconstruct(&self, trace: &MemoryTrace, mood: &Mood, query: &str) -> String {
        if trace.status == crate::core::model::TraceStatus::Latent {
            return crate::lexicon::rule().latent.clone();
        }
        let system = &crate::lexicon::prompts().reconstruct;
        let user = format!(
            "gist: {}\nschema: {}\nvalence: {:.2} arousal: {:.2} disgust: {:.2} fidelity: {:.2}\ncurrent mood: v={:.2} a={:.2} d={:.2}\nrecall cue: {}",
            trace.gist,
            trace.schema.as_deref().unwrap_or("-"),
            trace.valence,
            trace.arousal,
            trace.disgust,
            trace.fidelity,
            mood.valence,
            mood.arousal,
            mood.disgust,
            query
        );
        match self.chat(system, &user) {
            Ok(s) if !s.trim().is_empty() => s,
            Ok(_) => self.fallback.reconstruct(trace, mood, query),
            Err(e) => {
                let marker = self.miss(&e);
                if marker.is_empty() {
                    self.fallback.reconstruct(trace, mood, query)
                } else {
                    marker
                }
            }
        }
    }

    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String> {
        let mut block = String::new();
        for t in traces {
            block.push_str(&format!(
                "- [{}] v={:.2} d={:.2} {}\n",
                t.schema.as_deref().unwrap_or("-"),
                t.valence,
                t.disgust,
                t.gist
            ));
        }
        let system = &crate::lexicon::prompts().distill;
        match self.chat(system, &block) {
            Ok(s) => {
                let s = s.trim().to_string();
                if s.is_empty() {
                    self.fallback.distill_axiom(traces)
                } else {
                    Some(s)
                }
            }
            Err(e) => {
                if self.note_fail(&e) {
                    self.fallback.distill_axiom(traces)
                } else {
                    None
                }
            }
        }
    }

    fn segment(&self, event: &str) -> Option<Vec<String>> {
        let system = &crate::lexicon::prompts().segment;
        if system.is_empty() || event.trim().is_empty() {
            return None;
        }
        match self.chat(system, event) {
            Ok(raw) => crate::encode::parse_segment_reply(&raw),
            Err(e) => {
                let _ = self.note_fail(&e);
                None
            }
        }
    }

    fn extract_core(&self, event: &str) -> Option<String> {
        let system = &crate::lexicon::prompts().extract_core;
        if system.is_empty() {
            return None;
        }
        match self.chat(system, event) {
            Ok(s) => {
                let s = s.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            Err(_) => None,
        }
    }

    fn interpret(
        &self,
        event: &str,
        mood: &Mood,
        axioms: &[String],
    ) -> Option<crate::recall::narrator::Interpretation> {
        let system = &crate::lexicon::prompts().interpret;
        let user = format!(
            "event: {}\nmood: v={:.2} a={:.2} d={:.2}\naxioms: {}",
            event,
            mood.valence,
            mood.arousal,
            mood.disgust,
            axioms.iter().take(5).cloned().collect::<Vec<_>>().join(" | ")
        );
        match self.chat(system, &user) {
            Ok(raw) => parse_interp(&raw).or_else(|| self.fallback.interpret(event, mood, axioms)),
            Err(e) => {
                if self.note_fail(&e) {
                    self.fallback.interpret(event, mood, axioms)
                } else {
                    None
                }
            }
        }
    }

    fn rewrite(
        &self,
        trace: &MemoryTrace,
        neighbors: &[&MemoryTrace],
        profile: &crate::core::profile::EntityProfile,
    ) -> Option<String> {
        let gild = profile.embellish_gain - profile.disgust_gain;
        let p = crate::lexicon::prompts();
        let voice = if gild >= 0.04 {
            p.voice_tender.as_str()
        } else if gild <= -0.04 {
            p.voice_austere.as_str()
        } else {
            p.voice_neutral.as_str()
        };
        let mut block = format!(
            "core: {}\ngist: {}\nvalence: {:.2} disgust: {:.2} fidelity: {:.2} schema: {}\nvoix: {}\n",
            trace.core,
            trace.gist,
            trace.valence,
            trace.disgust,
            trace.fidelity,
            trace.schema.as_deref().unwrap_or("-"),
            voice
        );
        for n in neighbors.iter().take(3) {
            block.push_str(&format!("proche: {}\n", n.gist));
        }
        let system = &crate::lexicon::prompts().rewrite;
        match self.chat(system, &block) {
            Ok(s) => {
                let s = s.trim().to_string();
                if s.is_empty() {
                    self.fallback.rewrite(trace, neighbors, profile)
                } else {
                    Some(s)
                }
            }
            Err(e) => {
                if self.note_fail(&e) {
                    self.fallback.rewrite(trace, neighbors, profile)
                } else {
                    None
                }
            }
        }
    }

    fn recontextualize(
        &self,
        trace: &MemoryTrace,
        core: &str,
        profile: &crate::core::profile::EntityProfile,
    ) -> String {
        let system = &crate::lexicon::prompts().recontextualize;
        let user = format!(
            "core: {}\ngist actuel: {}\nvalence: {:.2} disgust: {:.2} schema: {}",
            core,
            trace.gist,
            trace.valence,
            trace.disgust,
            trace.schema.as_deref().unwrap_or("-")
        );
        match self.chat(system, &user) {
            Ok(s) if !s.trim().is_empty() => s.trim().chars().take(280).collect(),
            Ok(_) => self.fallback.recontextualize(trace, core, profile),
            Err(e) => {
                let marker = self.miss(&e);
                if marker.is_empty() {
                    self.fallback.recontextualize(trace, core, profile)
                } else {
                    marker
                }
            }
        }
    }

    fn reply(
        &self,
        user: &str,
        memories: &[String],
        axioms: &[String],
        mood: &Mood,
        talk: &WorkingTalk,
    ) -> String {
        self.reply_disposed(user, memories, axioms, mood, talk, "")
    }

    fn reply_disposed(
        &self,
        user: &str,
        memories: &[String],
        axioms: &[String],
        mood: &Mood,
        talk: &WorkingTalk,
        profile: &str,
    ) -> String {
        let mut ctx = String::new();
        ctx.push_str(&talk.render());
        if !profile.is_empty() {
            ctx.push_str("reading: ");
            ctx.push_str(profile);
            ctx.push('\n');
        }
        for (i, m) in memories.iter().take(4).enumerate() {
            if i == 0 {
                ctx.push_str("this hour: ");
            } else {
                ctx.push_str("- memory: ");
            }
            ctx.push_str(m);
            ctx.push('\n');
        }
        for a in axioms.iter().take(4) {
            ctx.push_str("- axiom: ");
            ctx.push_str(a);
            ctx.push('\n');
        }
        let system = format!(
            "{}\n{}",
            crate::lexicon::prompts().reply,
            crate::net::httpx::READING_CONSTRAINT
        );
        let user_p = format!(
            "mood v={:.2} a={:.2} d={:.2}\n{ctx}\nhuman: {user}",
            mood.valence, mood.arousal, mood.disgust
        );
        match self.chat(&system, &user_p) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("selmem LLM reply failed: {e}");
                let marker = self.miss(&e);
                if marker.is_empty() {
                    self.fallback.reply_disposed(user, memories, axioms, mood, talk, profile)
                } else {
                    marker
                }
            }
        }
    }
}

fn api_error(raw: &str) -> Option<String> {
    if !raw.contains("\"error\"") || raw.contains("\"choices\"") {
        return None;
    }
    crate::net::httpx::first_string_field(raw, "message")
        .or_else(|| crate::net::httpx::first_string_field(raw, "error"))
        .or_else(|| Some(raw.chars().take(200).collect()))
}

/// LLM only writes the spoken answer. Reconstruction stays on the rules
/// so a bifurcation run does not spend a call per trace.
pub struct SpeakOnlyHttp {
    pub http: HttpNarrator,
    rules: RuleNarrator,
}

impl SpeakOnlyHttp {
    pub fn parse(endpoint: &str, model: impl Into<String>, api_key: Option<String>) -> Option<Self> {
        Some(Self {
            http: HttpNarrator::parse(endpoint, model, api_key)?,
            rules: RuleNarrator,
        })
    }

    pub fn with_policy(mut self, policy: FailurePolicy) -> Self {
        self.http.policy = policy;
        self
    }
}

impl Narrator for SpeakOnlyHttp {
    fn failure_log(&self) -> LlmCallLog {
        self.http.failure_log()
    }

    fn reconstruct(&self, trace: &MemoryTrace, mood: &Mood, query: &str) -> String {
        self.rules.reconstruct(trace, mood, query)
    }
    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String> {
        self.rules.distill_axiom(traces)
    }
    fn segment(&self, event: &str) -> Option<Vec<String>> {
        self.http.segment(event)
    }
    fn reply(
        &self,
        user: &str,
        memories: &[String],
        axioms: &[String],
        mood: &Mood,
        talk: &WorkingTalk,
    ) -> String {
        self.reply_disposed(user, memories, axioms, mood, talk, "")
    }
    fn reply_disposed(
        &self,
        user: &str,
        memories: &[String],
        axioms: &[String],
        mood: &Mood,
        talk: &WorkingTalk,
        profile: &str,
    ) -> String {
        self.http.reply_disposed(user, memories, axioms, mood, talk, profile)
    }
}

fn parse_interp(raw: &str) -> Option<crate::recall::narrator::Interpretation> {
    let t = raw.to_lowercase();
    let grab = |k: &str| {
        t.split_whitespace().find_map(|w| {
            w.strip_prefix(&format!("{k}="))
                .and_then(|x| x.trim_matches(|c| c == ';' || c == ',').parse::<f32>().ok())
        })
    };
    let v = grab("v")?;
    let a = grab("a").unwrap_or(0.35);
    let d = grab("d").unwrap_or(0.0);
    let r = grab("r").unwrap_or(0.55);
    let schema = t.split_whitespace().find_map(|w| {
        w.strip_prefix("s=")
            .map(|s| s.trim_matches(|c| c == ';' || c == ',').to_string())
    });
    let schema = schema.filter(|s| s != "-" && s.len() > 1);
    Some(crate::recall::narrator::Interpretation {
        valence: v.clamp(-1.0, 1.0),
        arousal: a.clamp(0.0, 1.0),
        disgust: d.clamp(0.0, 1.0),
        schema,
        self_relevance: r.clamp(0.0, 1.0),
    })
}
