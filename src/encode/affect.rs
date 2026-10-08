/// Lexical affect when the caller sends no valence. Table: data/affect.txt
use crate::lexicon;

pub fn guess(text: &str) -> (f32, f32, f32, Option<String>) {
    let t = text.to_lowercase();
    let lex = lexicon::affect();
    let n: f32 = lex
        .neg
        .iter()
        .filter(|w| matches_pattern(&t, &w.stem))
        .map(|w| w.w)
        .sum();
    let p: f32 = lex
        .pos
        .iter()
        .filter(|w| matches_pattern(&t, &w.stem))
        .map(|w| w.w)
        .sum();
    let bang = t.matches('!').count() as f32 * 0.08;
    let intens = if lex.intensifiers.iter().any(|w| matches_pattern(&t, w)) {
        0.12
    } else {
        0.0
    };

    if n > p + 0.15 {
        let arousal = (0.45 + 0.15 * n + bang + intens).min(0.95);
        let disgust = (0.22 + 0.18 * n).min(0.85);
        (
            -(0.35 + 0.2 * n).min(0.95),
            arousal,
            disgust,
            Some(lex.schema_neg.clone()),
        )
    } else if p > n + 0.15 {
        let arousal = (0.32 + 0.12 * p + bang).min(0.9);
        (
            (0.35 + 0.18 * p).min(0.95),
            arousal,
            0.0,
            Some(lex.schema_pos.clone()),
        )
    } else if n > 0.0 && p > 0.0 {
        (
            (p - n) * 0.25,
            0.4 + bang,
            (0.15 * n).min(0.4),
            Some(lex.schema_mix.clone()),
        )
    } else {
        (0.0, (0.18 + bang).min(0.5), 0.0, None)
    }
}

/// Whole token/phrase match. A trailing '*' explicitly permits a stem.
fn matches_pattern(text: &str, pattern: &str) -> bool {
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let parts: Vec<_> = pattern.split_whitespace().collect();
    if parts.is_empty() || words.len() < parts.len() {
        return false;
    }
    words.windows(parts.len()).any(|window| {
        window.iter().zip(&parts).all(|(word, part)| {
            part.strip_suffix('*')
                .map_or(*word == *part, |stem| word.starts_with(stem))
        })
    })
}
