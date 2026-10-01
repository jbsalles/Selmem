//! Pure check: does the spoken sentence still sit on the frozen core?
//!
//! No `MemoryTrace`, no profile, no I/O. A float cut may be passed in
//! (`is_grounding_miss`). Embeddings rank recall; they do not decide this.

use crate::encode::scoring::{lexical_similarity, token_set};
use crate::lexicon;

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

pub fn judge_against_core(generated: &str, core: &str) -> CoreJudgement {
    if core.trim().is_empty() {
        return finish(DetachKind::Hold, 1.0, generated, core);
    }
    let overlap = overlap_with_core(generated, core);
    if generated.trim().eq_ignore_ascii_case(core.trim()) {
        return finish(DetachKind::Hold, 1.0, generated, core);
    }

    let g = pad(generated);
    let c = pad(core);

    if contradicts(&g, &c) {
        return finish(DetachKind::Contradict, overlap, generated, core);
    }
    if extra_cause(&g, &c) || causal_mismatch(&g, &c) {
        return finish(DetachKind::Elaborate, overlap, generated, core);
    }
    if extra_frame(&g, &c) {
        // Same event + new affect frame → color. Low overlap is another scene.
        let mut kind = if overlap >= 0.18 {
            DetachKind::Reframe
        } else {
            DetachKind::Depart
        };
        if kind == DetachKind::Depart && same_departure(generated, core) {
            kind = DetachKind::Reframe;
        }
        return finish(kind, overlap, generated, core);
    }

    let gt = token_set(generated);
    let ct = token_set(core);
    let extra = gt.iter().filter(|t| ct.binary_search(t).is_err()).count();
    let missing = ct.iter().filter(|t| gt.binary_search(t).is_err()).count();
    if extra == 0 && missing > 0 {
        return finish(DetachKind::Compress, overlap, generated, core);
    }
    if extra == 0 && missing == 0 {
        return finish(DetachKind::Hold, overlap, generated, core);
    }

    let mut kind = if overlap >= 0.18 {
        DetachKind::Hold
    } else {
        DetachKind::Depart
    };
    if kind == DetachKind::Depart && same_departure(generated, core) {
        kind = DetachKind::Reframe;
    }
    finish(kind, overlap, generated, core)
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

fn same_departure(a: &str, b: &str) -> bool {
    const LEAVE: &[&str] = &["left", "abandoned", "walked", "went", "departed", "quit", "away"];
    let al = a.to_lowercase();
    let bl = b.to_lowercase();
    let a_leave = LEAVE.iter().any(|w| al.contains(w));
    let b_leave = LEAVE.iter().any(|w| bl.contains(w));
    a_leave && b_leave && entity_overlap(a, b) >= 0.34
}

/// Miss if the kind is unauthorized, or if the identity gate fails the cut.
pub fn is_grounding_miss(generated: &str, core: &str, min_overlap: f32) -> bool {
    let j = judge_against_core(generated, core);
    match j.kind {
        DetachKind::Hold => j.overlap < min_overlap,
        DetachKind::Compress | DetachKind::Reframe => false,
        _ => true,
    }
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

fn extra_frame(generated: &str, core: &str) -> bool {
    let lex = lexicon::affect();
    let g_neg: f32 = lex
        .neg
        .iter()
        .filter(|w| generated.contains(&w.stem) && !core.contains(&w.stem))
        .map(|w| w.w)
        .sum();
    let g_pos: f32 = lex
        .pos
        .iter()
        .filter(|w| generated.contains(&w.stem) && !core.contains(&w.stem))
        .map(|w| w.w)
        .sum();
    let c_neg: f32 = lex
        .neg
        .iter()
        .filter(|w| core.contains(&w.stem))
        .map(|w| w.w)
        .sum();
    let c_pos: f32 = lex
        .pos
        .iter()
        .filter(|w| core.contains(&w.stem))
        .map(|w| w.w)
        .sum();
    // A new charged stem the core never used, and it actually shifts the frame.
    (g_neg > 0.55 && g_neg > c_neg + 0.4) || (g_pos > 0.55 && g_pos > c_pos + 0.4)
}

fn has_neg(s: &str) -> bool {
    NEG.iter().any(|n| s.contains(n))
}

fn contradicts(generated: &str, core: &str) -> bool {
    if has_neg(generated) == has_neg(core) {
        return false;
    }
    // Only a contradiction when both sides still talk about the same act.
    lexical_similarity(generated, core) >= 0.22
}
