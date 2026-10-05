use crate::core::model::{MemoryTrace, Mood};
use crate::core::talk::WorkingTalk;

#[derive(Clone, Debug)]
pub struct Interpretation {
    pub valence: f32,
    pub arousal: f32,
    pub disgust: f32,
    pub schema: Option<String>,
    pub self_relevance: f32,
}

/// What to do when the HTTP narrator cannot get a usable reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePolicy {
    /// UI default. Rules speak, and the miss is logged.
    Fallback,
    /// Benchmarks. Do not pretend the model answered.
    Error,
    /// Log the miss, then rules. Reports must still say fallback_used.
    RecordAndFallback,
}

#[derive(Clone, Debug)]
pub struct LlmCallLog {
    pub narrator: String,
    pub fallback_used: bool,
    pub llm_error: Option<String>,
    pub calls: u32,
    pub failures: u32,
}

impl Default for LlmCallLog {
    fn default() -> Self {
        Self {
            narrator: "rules".into(),
            fallback_used: false,
            llm_error: None,
            calls: 0,
            failures: 0,
        }
    }
}

pub trait Narrator: Send + Sync {
    fn failure_log(&self) -> LlmCallLog {
        LlmCallLog::default()
    }
    fn reconstruct(&self, trace: &MemoryTrace, mood: &Mood, query: &str) -> String;
    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String>;
    /// Live event only — never an archive. Default: none (caller uses lexicon + identity).
    fn interpret(&self, event: &str, mood: &Mood, axioms: &[String]) -> Option<Interpretation> {
        let _ = (event, mood, axioms);
        None
    }
    /// After the gate only. Propose a fact-core. Default: none (caller keeps fact_core).
    fn extract_core(&self, event: &str) -> Option<String> {
        let _ = event;
        None
    }
    /// Propose a lossless split of a long paste. Each string must be an
    /// excerpt of `event`. Default: none (caller packs by lines / words).
    fn segment(&self, event: &str) -> Option<Vec<String>> {
        let _ = event;
        None
    }
    /// Réécriture de consolidation : garder la charge, accentuer, jeter le superflu.
    fn rewrite(
        &self,
        trace: &MemoryTrace,
        neighbors: &[&MemoryTrace],
        profile: &crate::core::profile::EntityProfile,
    ) -> Option<String> {
        let _ = neighbors;
        Some(crate::dream::retell(
            &trace.gist,
            profile,
            trace.valence,
            trace.disgust,
        ))
    }
    /// Pull a drifted gist back onto the core. Never receives an archive.
    fn recontextualize(
        &self,
        trace: &MemoryTrace,
        core: &str,
        profile: &crate::core::profile::EntityProfile,
    ) -> String {
        crate::recall::ground::recontextualize_rule(
            core,
            &trace.gist,
            profile,
            trace.valence,
            trace.disgust,
        )
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
    /// The profile is a disposition. Empty means DropStake: the book does not color the mouth.
    fn reply_disposed(
        &self,
        user: &str,
        memories: &[String],
        axioms: &[String],
        mood: &Mood,
        talk: &WorkingTalk,
        profile: &str,
    ) -> String {
        let _ = (mood, talk);
        let mut out = String::new();
        if !profile.is_empty() {
            if profile.contains("valence_bias: -") {
                out.push_str("I measure this before I move. ");
            } else if profile.contains("valence_bias: 0.") || profile.contains("valence_bias: 1") {
                out.push_str("I leave room for this. ");
            }
            if let Some(stake) = profile.split("[recent_stakes:").nth(1) {
                let name = stake.split_whitespace().next().unwrap_or("");
                if !name.is_empty() && !user.to_lowercase().contains(&name.to_lowercase()) {
                    out.push_str(name);
                    out.push(' ');
                }
            }
        } else if let Some(m) = memories.first() {
            out.push_str(&crate::lexicon::rule().reply_recall);
            out.push_str(m);
            out.push(' ');
        }
        if let Some(ax) = axioms.first() {
            if profile.is_empty() {
                out.push_str(ax);
                out.push(' ');
            }
        }
        if out.is_empty() {
            out.push_str(&crate::lexicon::rule().reply_empty);
        }
        out.push('(');
        out.push_str(user);
        out.push(')');
        out
    }
}

#[derive(Default)]
pub struct RuleNarrator;

impl Narrator for RuleNarrator {
    fn reconstruct(&self, trace: &MemoryTrace, mood: &Mood, _query: &str) -> String {
        let _ = mood;
        if trace.status == crate::core::model::TraceStatus::Latent {
            return crate::lexicon::rule().latent.clone();
        }
        if (trace.fidelity < 0.42 || trace.confidence < 0.35) && !trace.core.is_empty() {
            trace.core.clone()
        } else {
            trace.gist.clone()
        }
    }

    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String> {
        if traces.len() < 2 {
            return None;
        }
        Some(compress_belief(traces))
    }
}

/// A belief is what returned, not an order and not a copy of an hour.
fn compress_belief(traces: &[&MemoryTrace]) -> String {
    let mut schemas: Vec<(String, usize)> = Vec::new();
    for t in traces {
        if let Some(s) = &t.schema {
            if let Some(slot) = schemas.iter_mut().find(|(k, _)| k == s) {
                slot.1 += 1;
            } else {
                schemas.push((s.clone(), 1));
            }
        }
    }
    let schema = schemas
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(s, _)| s)
        .unwrap_or_else(|| "unnamed".into());
    let mut acts: Vec<(String, usize)> = Vec::new();
    for t in traces {
        let mut seen = Vec::new();
        for a in &t.semantic.actions {
            let w = a.to_lowercase();
            if !seen.contains(&w) {
                seen.push(w);
            }
        }
        for w in seen {
            if let Some(slot) = acts.iter_mut().find(|(k, _)| k == &w) {
                slot.1 += 1;
            } else {
                acts.push((w, 1));
            }
        }
    }
    let pairs = relation_pairs(traces);
    let mean = traces.iter().map(|t| t.valence).sum::<f32>() / traces.len().max(1) as f32;
    let sense = if mean <= -0.2 {
        "against"
    } else if mean >= 0.2 {
        "for"
    } else {
        "under"
    };
    if pairs.is_empty() {
        format!("The same stake returned {} times {sense} {}.", traces.len(), schema)
    } else {
        format!(
            "The same stake returned {} times {sense} {}: {}.",
            traces.len(),
            schema,
            pairs.join("; ")
        )
    }
}

fn relation_pairs(traces: &[&MemoryTrace]) -> Vec<String> {
    // The act is whatever the hour already marked. No stake list.
    let mut out = Vec::new();
    for t in traces {
        let words: Vec<String> = t.core.split_whitespace().map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric()).to_string()
        }).filter(|w| !w.is_empty()).collect();
        let lows: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
        for act in &t.semantic.actions {
            let Some(i) = lows.iter().position(|w| w == act) else { continue };
            let after = ((i + 1)..words.len()).find_map(|j| {
                let w = &words[j];
                if w.len() > 3 && !is_glue(w) { Some(w.to_lowercase()) } else { None }
            });
            let before = (0..i).rev().find_map(|j| {
                let w = &words[j];
                if w.len() > 3 && !is_glue(w) { Some(w.to_lowercase()) } else { None }
            });
            let pair = match (after, before) {
                (Some(o), _) => format!("{act} {o}"),
                (_, Some(o)) => format!("{o} {act}"),
                _ => act.clone(),
            };
            if !out.contains(&pair) {
                out.push(pair);
            }
            if out.len() == 3 {
                return out;
            }
        }
    }
    out
}

fn is_glue(w: &str) -> bool {
    matches!(w.to_lowercase().as_str(),
        "that" | "this" | "with" | "from" | "after" | "before" | "into" | "your" | "their"
        | "been" | "were" | "was" | "have" | "has" | "had" | "them" | "they" | "what")
}


