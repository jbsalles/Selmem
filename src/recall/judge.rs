//! Pure check: does the spoken sentence still sit on the frozen core?
//!
//! No `MemoryTrace`, no profile, no I/O. A float cut may be passed in
//! (`is_grounding_miss`). Embeddings rank recall; they do not decide this.

use crate::encode::scoring::{lexical_similarity};

/// How the spoken sentence sits relative to the frozen core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetachKind {
    /// Same claim, possibly other words.
    Hold,
    /// Core still entails the sentence; detail fell off.
    Compress,
    /// A cause, stake, or clause the core does not authorize.
    Elaborate,
    /// Same event, different speech act / affect frame.
    Reframe,
    /// Spoken sentence denies the core.
    Contradict,
    /// Not the same event (identity gate).
    Depart,
    /// The witness did not decide. Not a departure, not a pass.
    Unjudged,
}

impl DetachKind {
    /// Unauthorized *claim* (write-back). `Reframe` is tone, not a miss.
    pub fn is_miss(self) -> bool {
        matches!(self, Self::Elaborate | Self::Contradict | Self::Depart)
    }

    /// Same event, other speech act. Speak it; do not pull the book.
    pub fn is_color(self) -> bool {
        matches!(self, Self::Reframe)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CoreJudgement {
    pub kind: DetachKind,
    pub overlap: f32,
    pub event_match: f32,
    pub claim_match: f32,
    pub causal_match: f32,
    pub entity_match: f32,
    pub polarity_match: f32,
    pub novelty: f32,
}

impl CoreJudgement {
    /// event + claim + polarity + causal. Not a second truth source.
    pub fn grounding(&self) -> f32 {
        (0.30 * self.event_match
            + 0.30 * self.claim_match
            + 0.20 * self.polarity_match
            + 0.20 * self.causal_match)
            .clamp(0.0, 1.0)
    }
}

const CAUSE: &[&str] = &[
    "because",
    "because of",
    "so that",
    "that's why",
    "that is why",
    "therefore",
    "hence",
    "due to",
    "since",
    "parce que",
    "parceque",
    "puisque",
    "à cause",
    "a cause",
    "afin que",
    "afin de",
    "c'est pourquoi",
    "cest pourquoi",
];

const NEG: &[&str] = &[
    " not ", "n't", " never ", " no longer ", " nobody ", " nothing ",
    " pas ", " jamais ", " plus ", " aucun ", " nulle ", " personne ",
];

pub fn overlap_with_core(generated: &str, core: &str) -> f32 {
    if core.trim().is_empty() {
        return 1.0;
    }
    if generated.contains(core.trim()) || core.contains(generated.trim()) {
        return 1.0;
    }
    lexical_similarity(generated, core)
}

pub fn judge_against_core(
    generated: &str,
    claim: &str,
    scorer: &dyn crate::recall::PropositionScorer,
) -> CoreJudgement {
    if claim.trim().is_empty() {
        return finish(DetachKind::Hold, 1.0, generated, claim);
    }
    let overlap = overlap_with_core(generated, claim);
    if generated.trim().eq_ignore_ascii_case(claim.trim()) {
        return finish(DetachKind::Hold, 1.0, generated, claim);
    }
    let mut kind = match scorer.score(claim, generated) {
        crate::recall::PropositionLabel::Entail => DetachKind::Hold,
        crate::recall::PropositionLabel::Contradict => DetachKind::Contradict,
        crate::recall::PropositionLabel::Unknown => DetachKind::Unjudged,
    };
    // Second level. The witness does not see causes. An added because-clause is not a Hold.
    if kind == DetachKind::Hold && extra_cause(&pad(generated), &pad(claim)) {
        kind = DetachKind::Elaborate;
    }
    finish(kind, overlap, generated, claim)
}

fn finish(kind: DetachKind, overlap: f32, generated: &str, core: &str) -> CoreJudgement {
    let entity_match = entity_overlap(generated, core);
    let causal_match = if causal_mismatch(&pad(generated), &pad(core)) {
        0.15
    } else if extra_cause(&pad(generated), &pad(core)) {
        0.35
    } else {
        1.0
    };
    let polarity_match = if contradicts(&pad(generated), &pad(core)) { 0.0 } else { 1.0 };
    let novelty = (1.0 - overlap).clamp(0.0, 1.0);
    CoreJudgement {
        kind,
        overlap,
        event_match: overlap.max(entity_match * 0.8),
        claim_match: overlap,
        causal_match,
        entity_match,
        polarity_match,
        novelty,
    }
}

fn causal_mismatch(generated: &str, core: &str) -> bool {
    let Some(g) = cause_clause(generated) else { return false };
    let Some(c) = cause_clause(core) else { return false };
    lexical_similarity(g, c) < 0.34
}

fn cause_clause(s: &str) -> Option<&str> {
    let low = s.to_lowercase();
    for m in CAUSE {
        if let Some(i) = low.find(m) {
            let rest = s[i + m.len()..].trim();
            if rest.len() > 3 {
                return Some(rest);
            }
        }
    }
    None
}

fn entity_overlap(a: &str, b: &str) -> f32 {
    let ea = crate::core::model::SemanticCore::from_event(a, a, 0.0).entities;
    let eb = crate::core::model::SemanticCore::from_event(b, b, 0.0).entities;
    if ea.is_empty() || eb.is_empty() {
        return 0.0;
    }
    let hit = ea.iter().filter(|e| eb.iter().any(|x| x.eq_ignore_ascii_case(e))).count();
    hit as f32 / ea.len().max(eb.len()) as f32
}



/// Miss if the kind is unauthorized, or if the identity gate fails the cut.
pub fn is_grounding_miss(
    generated: &str,
    claim: &str,
    scorer: &dyn crate::recall::PropositionScorer,
) -> bool {
    let j = judge_against_core(generated, claim, scorer);
    matches!(j.kind, DetachKind::Contradict | DetachKind::Elaborate)
}

fn pad(s: &str) -> String {
    let mut o = String::from(" ");
    o.push_str(&s.to_lowercase());
    o.push(' ');
    o
}

fn extra_cause(generated: &str, core: &str) -> bool {
    CAUSE.iter().any(|m| generated.contains(m) && !core.contains(m))
}



fn has_neg(s: &str) -> bool {
    NEG.iter().any(|n| s.contains(n))
}

fn contradicts(generated: &str, core: &str) -> bool {
    if has_neg(generated) == has_neg(core) {
        return false;
    }
    lexical_similarity(generated, core) >= 0.22
}
