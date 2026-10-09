//! Lossless semantic split, else line / sentence / word pack.

/// True when a paste is too long to compress as one hour.
pub fn needs_split(event: &str) -> bool {
    let text = event.trim();
    if text.is_empty() {
        return false;
    }
    let lines = text.lines().map(str::trim).filter(|l| !l.is_empty()).count();
    let words = text.split_whitespace().count();
    lines > 10 || words > 80
}

/// Semantic cut if `proposed` is a lossless partition of `event`.
/// Otherwise pack by lines, then by sentences / words.
pub fn split_event(event: &str, proposed: Option<&[String]>) -> Vec<String> {
    if let Some(p) = proposed {
        if let Some(parts) = lossless_parts(event, p) {
            return frame_parts(event, parts);
        }
    }
    segment_facts(event)
}

fn frame_parts(source: &str, parts: Vec<String>) -> Vec<String> {
    let Some((speaker, _)) = super::core::reported_speech(source) else { return parts; };
    parts.into_iter().map(|part| {
        if super::core::reported_speech(&part).is_some_and(|(s, _)| s == speaker) { part }
        else { format!("{speaker} said: {part}") }
    }).collect()
}

/// Ask the narrator only when the hour is long enough to need a cut.
pub fn propose(event: &str, narrator: &dyn crate::recall::narrator::Narrator) -> Option<Vec<String>> {
    if needs_split(event) {
        narrator.segment(event)
    } else {
        None
    }
}

/// Each unit must be a contiguous excerpt of `source`, in order, covering
/// almost the whole text. Paraphrase is rejected.
pub fn lossless_parts(source: &str, proposed: &[String]) -> Option<Vec<String>> {
    let tokens = tokens_with_spans(source);
    if tokens.is_empty() {
        return None;
    }
    let mut ti = 0usize;
    let mut out = Vec::new();
    for p in proposed {
        let words: Vec<&str> = p.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        let mut found = None;
        let mut j = ti;
        while j + words.len() <= tokens.len() {
            let hit = tokens[j..j + words.len()]
                .iter()
                .zip(words.iter())
                .all(|(tok, w)| tok.2.eq_ignore_ascii_case(w));
            if hit {
                found = Some((j, j + words.len()));
                break;
            }
            j += 1;
        }
        let (a, b) = found?;
        if a < ti {
            return None;
        }
        let start = tokens[a].0;
        let end = tokens[b - 1].1;
        out.push(source[start..end].to_string());
        ti = b;
    }
    if out.len() < 2 {
        return None;
    }
    if tokens.len() - ti > 6 {
        return None;
    }
    Some(out)
}

fn tokens_with_spans(source: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, ch) in source.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, i, &source[s..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, source.len(), &source[s..]));
    }
    out
}

/// Read a JSON string array out of a model reply. Anything else is ignored.
pub fn parse_segment_reply(raw: &str) -> Option<Vec<String>> {
    let start = raw.find('[')?;
    let bytes = raw[start..].as_bytes();
    let mut depth = 0i32;
    let mut end = None;
    let mut in_str = false;
    let mut esc = false;
    for (i, b) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if *b == b'\\' {
                esc = true;
            } else if *b == b'"' {
                in_str = false;
            }
            continue;
        }
        match *b {
            b'"' => in_str = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let slice = &raw[start..start + end?];
    let parts = parse_json_strings(slice);
    if parts.len() >= 2 {
        Some(parts)
    } else {
        None
    }
}

fn parse_json_strings(s: &str) -> Vec<String> {
    let mut body = s.trim_start();
    if !body.starts_with('[') {
        return Vec::new();
    }
    body = body[1..].trim_start();
    let mut out = Vec::new();
    loop {
        body = body.trim_start();
        if body.is_empty() || body.starts_with(']') {
            break;
        }
        if body.starts_with(',') {
            body = body[1..].trim_start();
            continue;
        }
        if body.starts_with('"') {
            if let Some((v, n)) = crate::net::httpx::parse_json_string(body) {
                out.push(v);
                body = &body[n..];
                continue;
            }
        }
        break;
    }
    out
}

/// A short hour stays one fact. A long paste is packed into 5–10 line slices
/// so compress(28) does not throw away everything after the first paragraph.
pub fn segment_facts(event: &str) -> Vec<String> {
    let text = event.trim();
    if text.is_empty() {
        return Vec::new();
    }
    // A quoted turn can contain an incident and a different response to it.
    // Keep the attribution on every slice; do not turn quoted "I" into self.
    if let Some((speaker, body)) = super::core::reported_speech(text) {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut sign = 0i8;
        for sentence in body.split_inclusive(['.', '!', '?', ';', '\n']) {
            let (v, _, _, _) = super::affect::guess(sentence);
            let next = if v > 0.15 { 1 } else if v < -0.15 { -1 } else { 0 };
            if sign != 0 && next != 0 && sign != next && !current.is_empty() {
                parts.push(format!("{speaker} said: {}", current.trim()));
                current.clear();
            }
            current.push_str(sentence);
            if next != 0 { sign = next; }
        }
        if !current.trim().is_empty() { parts.push(format!("{speaker} said: {}", current.trim())); }
        let mut bounded = Vec::new();
        for part in parts {
            let (_, body) = super::core::reported_speech(&part).unwrap();
            if part.split_whitespace().count() <= 80 {
                bounded.push(part);
            } else {
                bounded.extend(pack_sentences(body, 40, 60).into_iter()
                    .map(|p| format!("{speaker} said: {p}")));
            }
        }
        return bounded;
    }
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let words = text.split_whitespace().count();
    if lines.len() <= 10 && words <= 80 {
        return vec![text.to_string()];
    }
    if lines.len() <= 1 {
        return pack_sentences(text, 40, 70);
    }
    pack_lines(&lines, 5, 10)
}

fn pack_lines(lines: &[&str], min: usize, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let target = ((min + max) / 2).max(1);
    for line in lines {
        cur.push(*line);
        if cur.len() >= target {
            out.push(cur.join("\n"));
            cur.clear();
        }
    }
    if !cur.is_empty() {
        if let Some(last) = out.last_mut() {
            let last_n = last.lines().count();
            if last_n + cur.len() <= max {
                last.push('\n');
                last.push_str(&cur.join("\n"));
            } else {
                out.push(cur.join("\n"));
            }
        } else {
            out.push(cur.join("\n"));
        }
    }
    out
}

fn pack_sentences(text: &str, min_words: usize, max_words: usize) -> Vec<String> {
    let mut sentences: Vec<String> = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if matches!(ch, '.' | '!' | '?' | '。' | '…') {
            let s = buf.trim();
            if !s.is_empty() {
                sentences.push(s.to_string());
            }
            buf.clear();
        }
    }
    let tail = buf.trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
    if sentences.len() <= 1 {
        return pack_words(text, max_words);
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut n = 0usize;
    for s in sentences {
        let w = s.split_whitespace().count();
        if n > 0 && n + w > max_words {
            out.push(cur.trim().to_string());
            cur.clear();
            n = 0;
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(&s);
        n += w;
        if n >= min_words && n >= max_words / 2 {
            out.push(cur.trim().to_string());
            cur.clear();
            n = 0;
        }
    }
    if !cur.trim().is_empty() {
        if let Some(last) = out.last_mut() {
            let last_n = last.split_whitespace().count();
            if last_n + n <= max_words {
                last.push(' ');
                last.push_str(cur.trim());
            } else {
                out.push(cur.trim().to_string());
            }
        } else {
            out.push(cur.trim().to_string());
        }
    }
    if out.is_empty() {
        vec![text.to_string()]
    } else {
        out
    }
}

fn pack_words(text: &str, max_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return vec![text.to_string()];
    }
    words
        .chunks(max_words)
        .map(|c| c.join(" "))
        .collect()
}
