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

pub trait Narrator: Send + Sync {
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
        let mut out = String::new();
        if let Some(ax) = axioms.first() {
            out.push_str(ax);
            out.push(' ');
        }
        if let Some(m) = memories.first() {
            out.push_str(&crate::lexicon::rule().reply_recall);
            out.push_str(m);
            out.push(' ');
        }
        if out.is_empty() {
            out.push_str(&crate::lexicon::rule().reply_empty);
        }
        out.push_str("(");
        out.push_str(user);
        out.push_str(")");
        let _ = (mood, talk);
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
        if trace.fidelity < 0.42 && !trace.core.is_empty() {
            trace.core.clone()
        } else {
            trace.gist.clone()
        }
    }

    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String> {
        if traces.len() < 2 {
            return None;
        }
        let mut counts: Vec<(String, usize)> = Vec::new();
        for t in traces {
            if let Some(s) = &t.schema {
                if let Some(slot) = counts.iter_mut().find(|(k, _)| k == s) {
                    slot.1 += 1;
                } else {
                    counts.push((s.clone(), 1));
                }
            }
        }
        let dominant = counts
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map(|(s, _)| s)
            .unwrap_or_else(|| "self".into());
        let mean_v: f32 = traces.iter().map(|t| t.valence).sum::<f32>() / traces.len() as f32;
        let copy = crate::lexicon::rule();
        let tmpl = if mean_v < -0.2 {
            &copy.axiom_neg
        } else if mean_v > 0.2 {
            &copy.axiom_pos
        } else {
            &copy.axiom_mid
        };
        Some(tmpl.replace("{schema}", &dominant))
    }
}


