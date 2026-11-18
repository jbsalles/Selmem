use roxmltree::{Document, Node};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
};
use tiktoken_rs::{cl100k_base, CoreBPE};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn tokens(b: &CoreBPE, s: &str) -> usize {
    b.encode_ordinary(s).len()
}
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn required<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing nonempty {k}").into())
}
#[derive(Clone, Serialize)]
struct Event {
    sender: String,
    recipient: String,
    text: String,
}
#[derive(Clone)]
struct Block {
    id: String,
    events: Vec<Event>,
    tokens: usize,
    partial: bool,
}

fn ends_sentence(word: &str) -> bool {
    let word = word.trim_end_matches(['"', '\'', '”', '’', ')', ']', '»']);
    if word.ends_with(['!', '?', '…']) {
        return true;
    }
    let Some(stem) = word.strip_suffix('.') else {
        return false;
    };
    let stem = stem.trim_start_matches(['"', '\'', '“', '‘', '(', '[', '«']);
    // Keep initials and common abbreviations with the following word.
    !stem.is_empty()
        && !stem.chars().all(|c| c == '.')
        && !(stem.chars().filter(|&c| c != '.').count() == 1
            && stem.chars().any(char::is_alphabetic))
        && !matches!(
            stem.to_ascii_lowercase().as_str(),
            "mr" | "mrs" | "ms" | "dr" | "prof" | "st" | "m" | "mme" | "e.g" | "i.e"
        )
}

fn chunks(b: &CoreBPE, s: &str) -> Result<Vec<String>> {
    let words: Vec<_> = s.split_whitespace().collect();
    let mut start = 0;
    let mut out = vec![];
    while start < words.len() {
        if tokens(b, words[start]) > 256 {
            return Err("word exceeds 256 tokens".into());
        }
        let mut lo = start + 1;
        let mut hi = (start + 256).min(words.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if tokens(b, &words[start..mid].join(" ")) <= 256 {
                lo = mid
            } else {
                hi = mid - 1
            }
        }
        let end = if lo == words.len() {
            lo
        } else {
            words[start..lo]
                .iter()
                .rposition(|word| ends_sentence(word))
                .map_or(lo, |index| start + index + 1)
        };
        out.push(words[start..end].join(" "));
        start = end;
    }
    Ok(out)
}
fn add_events(
    b: &CoreBPE,
    out: &mut Vec<Event>,
    sender: &str,
    recipient: &str,
    text: &str,
) -> Result<()> {
    for text in chunks(b, text)? {
        out.push(Event {
            sender: sender.into(),
            recipient: recipient.into(),
            text,
        });
    }
    Ok(())
}
fn block(b: &CoreBPE, id: String, events: Vec<Event>) -> Block {
    let n = events.iter().map(|e| tokens(b, &e.text)).sum();
    Block {
        id,
        events,
        tokens: n,
        partial: false,
    }
}
fn text_node(n: Node<'_, '_>, out: &mut String) {
    if n.is_text() {
        out.push_str(n.text().unwrap_or(""));
        return;
    }
    if !n.is_element() {
        return;
    }
    match n.tag_name().name() {
        "note" | "ptr" | "figure" | "del" | "fw" => return,
        "lb" | "pb" | "space" => {
            out.push(' ');
            return;
        }
        "choice" => {
            if let Some(c) = n
                .children()
                .find(|c| c.has_tag_name("corr") || c.has_tag_name("reg"))
                .or_else(|| n.children().find(Node::is_element))
            {
                text_node(c, out)
            }
            return;
        }
        _ => {}
    }
    for c in n.children() {
        text_node(c, out)
    }
    if matches!(n.tag_name().name(), "p" | "l" | "opener" | "closer") {
        out.push(' ')
    }
}
fn plain(n: Node<'_, '_>) -> String {
    let mut s = String::new();
    text_node(n, &mut s);
    normalize(&s)
}
fn letters(dir: &Path, b: &CoreBPE) -> Result<(Vec<Block>, String)> {
    let mut paths = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| {
        p.extension().is_some_and(|e| e == "xml")
            && p.file_stem()
                .is_some_and(|n| n.to_string_lossy().starts_with("let"))
    });
    paths.sort();
    let mut out = vec![];
    let mut hashes = String::new();
    for p in paths {
        let xml = fs::read_to_string(&p)?;
        let id = p.file_stem().unwrap().to_string_lossy().to_string();
        hashes.push_str(&format!("{id}:{}\n", hash(xml.as_bytes())));
        let doc = Document::parse(&xml)?;
        let person = |kind| -> Result<String> {
            let a = doc
                .descendants()
                .find(|n| n.has_tag_name("correspAction") && n.attribute("type") == Some(kind))
                .ok_or("missing correspondence metadata")?;
            let p = a
                .children()
                .find(|n| n.has_tag_name("rs"))
                .ok_or("missing correspondent")?;
            Ok(plain(p))
        };
        let body = |kind| {
            doc.descendants()
                .filter(|n| {
                    n.has_tag_name("div")
                        && n.attribute("type") == Some(kind)
                        && n.attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
                            == Some("en")
                })
                .collect::<Vec<_>>()
        };
        let mut nodes = body("translation");
        if nodes.is_empty() {
            nodes = body("original")
        }
        if nodes.len() != 1 {
            return Err(format!("{id}: expected one English body").into());
        }
        let mut events = vec![];
        add_events(
            b,
            &mut events,
            &person("sent")?,
            &person("received")?,
            &plain(nodes[0]),
        )?;
        if events.is_empty() {
            return Err(format!("empty letter {id}").into());
        }
        out.push(block(b, id, events));
    }
    if out.is_empty() {
        return Err("no letters".into());
    }
    Ok((out, hash(hashes.as_bytes())))
}
fn conversations(data: &Value, b: &CoreBPE) -> Result<(Vec<VecDeque<Block>>, Value)> {
    let samples = data.as_array().ok_or("expected LoCoMo array")?;
    let mut queues = vec![];
    let mut seen_sessions = BTreeSet::new();
    let mut repeated_sessions = 0;
    let mut turns = 0;
    let mut counts = BTreeMap::<String, usize>::new();
    let mut ids = BTreeSet::new();
    for (i, sample) in samples.iter().enumerate() {
        let c = sample["conversation"]
            .as_object()
            .ok_or("missing conversation")?;
        let a = required(&sample["conversation"], "speaker_a")?;
        let z = required(&sample["conversation"], "speaker_b")?;
        if a == z {
            return Err("identical speakers".into());
        }
        let mut sessions = vec![];
        for (k, v) in c {
            if let Some(n) = k
                .strip_prefix("session_")
                .and_then(|n| n.parse::<usize>().ok())
            {
                sessions.push((n, v));
            }
        }
        sessions.sort_by_key(|(n, _)| *n);
        let mut queue = VecDeque::new();
        for (n, v) in sessions {
            let mut events = vec![];
            for turn in v.as_array().ok_or("invalid session")? {
                let dia = required(turn, "dia_id")?;
                if !ids.insert(format!("{i}:{dia}")) {
                    return Err("duplicate source turn id".into());
                }
                let sender = required(turn, "speaker")?;
                let recipient = if sender == a {
                    z
                } else if sender == z {
                    a
                } else {
                    return Err("unknown speaker".into());
                };
                let text = normalize(required(turn, "text")?);
                turns += 1;
                *counts
                    .entry(hash(text.to_lowercase().as_bytes()))
                    .or_default() += 1;
                add_events(b, &mut events, sender, recipient, &text)?;
            }
            if events.is_empty() {
                return Err("empty session".into());
            }
            let fingerprint = hash(&serde_json::to_vec(&events)?);
            if !seen_sessions.insert(fingerprint) {
                repeated_sessions += 1;
                continue;
            }
            queue.push_back(block(b, format!("locomo/{i}/session_{n}"), events));
        }
        if queue.is_empty() {
            return Err("conversation has no unique sessions".into());
        }
        queues.push(queue);
    }
    let stats = json!({"conversations":samples.len(),"raw_turns":turns,"exact_duplicate_turn_occurrences":counts.values().map(|n|n-1).sum::<usize>(),"duplicate_sessions_removed":repeated_sessions,"unique_sessions":seen_sessions.len(),"available_body_tokens":queues.iter().flatten().map(|s|s.tokens).sum::<usize>()});
    Ok((queues, stats))
}
struct Rng(u64);
impl Rng {
    fn index(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}
// Only the next session of a conversation is eligible: chronological order stays intact.
fn order(mut queues: Vec<VecDeque<Block>>, seed: u64) -> Vec<Block> {
    let mut r = Rng(seed);
    let mut out = vec![];
    loop {
        let active: Vec<_> = queues
            .iter()
            .enumerate()
            .filter(|(_, q)| !q.is_empty())
            .map(|(i, _)| i)
            .collect();
        if active.is_empty() {
            return out;
        }
        let i = active[r.index(active.len())];
        out.push(queues[i].pop_front().unwrap());
    }
}
fn prefix(b: &CoreBPE, source: &Block, cap: usize) -> Block {
    let mut events = vec![];
    let mut left = cap;
    for e in &source.events {
        let n = tokens(b, &e.text);
        if n <= left {
            events.push(e.clone());
            left -= n
        } else {
            let mut s = e.text.clone();
            while tokens(b, &s) > left {
                if let Some(i) = s.rfind(' ') {
                    s.truncate(i)
                } else {
                    s.clear();
                    break;
                }
            }
            if !s.is_empty() {
                events.push(Event {
                    text: s,
                    ..e.clone()
                })
            }
            break;
        }
    }
    let mut result = block(b, source.id.clone(), events);
    result.partial = result.tokens < source.tokens;
    result
}
fn select(b: &CoreBPE, ordered: Vec<Block>, target: usize) -> Vec<Block> {
    let mut out = vec![];
    let mut remaining = target;
    for s in ordered {
        if remaining == 0 {
            break;
        }
        let take = if s.tokens <= remaining {
            s
        } else {
            prefix(b, &s, remaining)
        };
        remaining -= take.tokens;
        if take.tokens > 0 {
            let partial = take.partial;
            out.push(take);
            if partial {
                break;
            }
        }
    }
    out
}
fn write_rows(
    out: &mut String,
    events: &Block,
    period: usize,
    id: &mut usize,
    audit: &mut String,
    kind: &str,
) -> Result<()> {
    for (part, e) in events.events.iter().enumerate() {
        *id += 1;
        let event_id = format!("e{:08}", id);
        out.push_str(&serde_json::to_string(&json!({"event_id":event_id,"period":period,"sender":e.sender,"recipient":e.recipient,"text":e.text}))?);
        out.push('\n');
        audit.push_str(&serde_json::to_string(&json!({"event_id":event_id,"source_block":events.id,"part":part,"source_kind":kind,"boundary_prefix":events.partial}))?);
        audit.push('\n');
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("REALISTIC NOISE: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    let b = cl100k_base()?;
    if a.len() == 2 && a[0] == "inspect" {
        let data = fs::read(&a[1])?;
        let (_, stats) = conversations(&serde_json::from_slice(&data)?, &b)?;
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }
    if a.len() < 3 || a.iter().any(|s| s == "--help") {
        println!("selmem-vangogh-realistic inspect LOCOMO.json\nselmem-vangogh-realistic LETTERS_DIR LOCOMO.json FRESH_OUT [--mode full|pilot|available] [--noise-percent 10] [--seed 42]\nfull: require the requested noise fraction without replacement; pilot: shorten to whole-letter prefix; available: use all available noise, ignoring the requested fraction.");
        return Ok(());
    }
    let mut mode = "full";
    let mut seed = 42;
    let mut noise_percent = 10usize;
    let mut i = 3;
    while i < a.len() {
        let key = &a[i];
        i += 1;
        let val = a.get(i).ok_or("missing option value")?;
        match key.as_str() {
            "--mode" => mode = val,
            "--seed" => seed = val.parse()?,
            "--noise-percent" => noise_percent = val.parse()?,
            _ => return Err("unknown option".into()),
        }
        i += 1
    }
    if !["full", "pilot", "available"].contains(&mode) {
        return Err("unknown mode".into());
    }
    if noise_percent >= 100 {
        return Err("noise-percent must be an integer from 0 to 99".into());
    }
    // The percentage is a fraction of the FINAL corpus, not of the letters.
    let required_noise = |source_tokens: usize| -> usize {
        ((source_tokens as u128 * noise_percent as u128) / (100 - noise_percent) as u128) as usize
    };
    let data = fs::read(&a[1])?;
    let (queues, stats) = conversations(&serde_json::from_slice(&data)?, &b)?;
    let available: usize = queues.iter().flatten().map(|s| s.tokens).sum();
    let (mut source, source_hash) = letters(Path::new(&a[0]), &b)?;
    let full_letters = source.len();
    let full_tokens: usize = source.iter().map(|s| s.tokens).sum();
    if mode == "full" && available < required_noise(full_tokens) {
        return Err(format!("insufficient unique noise: available {available} tokens, required {}, shortfall {}. No output written; no repetition allowed. Use --mode pilot or --mode available explicitly.",required_noise(full_tokens),required_noise(full_tokens)-available).into());
    }
    if mode == "pilot" {
        let mut used = 0;
        source = source
            .into_iter()
            .take_while(|s| {
                if required_noise(used + s.tokens) <= available {
                    used += s.tokens;
                    true
                } else {
                    false
                }
            })
            .collect();
        if source.is_empty() {
            return Err("not enough noise for even the first whole letter".into());
        }
    }
    let signal: usize = source.iter().map(|s| s.tokens).sum();
    let target = if mode == "available" {
        available
    } else {
        required_noise(signal)
    };
    let noise = select(&b, order(queues, seed), target);
    let noise_tokens: usize = noise.iter().map(|s| s.tokens).sum();
    let fraction = noise_tokens as f64 / (signal + noise_tokens) as f64;
    if mode != "available" && (fraction - noise_percent as f64 / 100.0).abs() > 0.001 {
        return Err("boundary token ratio outside tolerance".into());
    }
    let out = Path::new(&a[2]);
    if out.exists() {
        return Err("output exists; use a fresh directory".into());
    }
    let mut clean = String::new();
    let mut mixed = String::new();
    let mut audit = String::new();
    let mut clean_audit = String::new();
    let mut clean_id = 0;
    let mut mixed_id = 0;
    let mut consumed = 0;
    let mut index = 0;
    let mut signal_seen = 0;
    let mut checkpoints = vec![];
    for (idx, l) in source.iter().enumerate() {
        signal_seen += l.tokens;
        let due = noise_tokens * signal_seen / signal;
        while index < noise.len() && consumed + noise[index].tokens <= due {
            write_rows(
                &mut mixed,
                &noise[index],
                idx + 1,
                &mut mixed_id,
                &mut audit,
                "locomo",
            )?;
            consumed += noise[index].tokens;
            index += 1
        }
        write_rows(&mut mixed, l, idx + 1, &mut mixed_id, &mut audit, "letter")?;
        write_rows(
            &mut clean,
            l,
            idx + 1,
            &mut clean_id,
            &mut clean_audit,
            "letter",
        )?;
        for pct in [25, 50, 75, 100] {
            if idx + 1 == (source.len() * pct).div_ceil(100) {
                checkpoints.push(json!({"percent":pct,"letters":idx+1,"last_letter":l.id,"clean_events":clean_id,"mixed_events":mixed_id}));
            }
        }
    }
    if index != noise.len() {
        return Err("internal scheduling error".into());
    }
    let stage = out.with_extension("preparing");
    if let Some(p) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?
    }
    fs::create_dir(&stage)?;
    let mut hashes = BTreeMap::new();
    for (name, content) in [
        ("stream-clean.jsonl", clean),
        ("stream-mixed.jsonl", mixed),
        ("audit-mixed.jsonl", audit),
        ("audit-clean.jsonl", clean_audit),
    ] {
        hashes.insert(name, hash(content.as_bytes()));
        fs::write(stage.join(name), content)?
    }
    let manifest = json!({"version":"realistic-noise-v2","requested_noise_percent":noise_percent,"target_noise_tokens":target,"mode":mode,"seed":seed,"tokenizer":"cl100k_base/tiktoken-rs-0.6.0","token_accounting":"sum of independent <=256-token event bodies; metadata excluded","locomo_sha256":hash(&data),"letters_sha256":source_hash,"noise_inventory":stats,"full_letter_count":full_letters,"selected_letters":source.len(),"last_selected_letter":source.last().unwrap().id,"full_source_tokens":full_tokens,"source_tokens":signal,"noise_tokens":noise_tokens,"noise_fraction":fraction,"selected_sessions":noise.len(),"partial_boundary_sessions":noise.iter().filter(|n|n.partial).count(),"selection":"without replacement; session and turn order preserved within each conversation; final boundary may be a prefix","checkpoints":checkpoints,"files":hashes,"status":"CORPUS_ONLY_NOT_AN_ACTOR_RESULT"});
    fs::write(
        stage.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(stage, out)?;
    println!(
        "{} letters, {} source tokens, {} noise tokens ({:.5}%), {} sessions, no resampling. {}",
        source.len(),
        signal,
        noise_tokens,
        fraction * 100.,
        noise.len(),
        out.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!([{"qa":[{"answer":"FORBIDDEN"}],"conversation":{"speaker_a":"A","speaker_b":"B","session_10":[{"dia_id":"D10:1","speaker":"A","text":"Later day"}],"session_2":[{"dia_id":"D2:1","speaker":"B","text":"Earlier day"}],"session_2_date_time":"NEVER INJECT"},"observation":"FORBIDDEN"}])
    }
    #[test]
    fn raw_only_and_numeric_order() {
        let b = cl100k_base().unwrap();
        let (q, _) = conversations(&fixture(), &b).unwrap();
        let s = order(q, 42);
        assert!(s[0].id.ends_with("session_2"));
        assert!(s[1].id.ends_with("session_10"));
        assert_eq!(s[0].events[0].sender, "B");
        assert!(!serde_json::to_string(&s[0].events)
            .unwrap()
            .contains("FORBIDDEN"));
    }
    #[test]
    fn duplicated_dataset_rejected_or_deduplicated() {
        let b = cl100k_base().unwrap();
        let mut f = fixture();
        let clone = f[0].clone();
        f.as_array_mut().unwrap().push(clone);
        assert!(conversations(&f, &b).is_err());
    }
    #[test]
    fn boundary_prefix_not_repeated() {
        let b = cl100k_base().unwrap();
        let (q, _) = conversations(&fixture(), &b).unwrap();
        let s = select(&b, order(q, 42), 3);
        assert!(!s.is_empty());
        assert!(s.iter().map(|x| x.tokens).sum::<usize>() <= 3);
        assert_eq!(
            s.iter().map(|x| &x.id).collect::<BTreeSet<_>>().len(),
            s.len()
        );
        assert!(s.iter().filter(|x| x.partial).count() <= 1);
    }
    #[test]
    fn no_annotation_in_letter_body() {
        let xml = r#"<div><p>Keep <hi>this</hi>.</p><note>SECRET</note><figure>CAPTION</figure><p>Next.</p></div>"#;
        let d = Document::parse(xml).unwrap();
        assert_eq!(plain(d.root_element()), "Keep this. Next.");
    }
    #[test]
    fn unicode_chunks_preserve_text() {
        let b = cl100k_base().unwrap();
        let s = "Un café pour André. ".repeat(300);
        let c = chunks(&b, &s).unwrap();
        assert_eq!(c.join(" "), normalize(&s));
        assert!(c.iter().all(|s| tokens(&b, s) <= 256));
    }

    #[test]
    fn chunks_keep_the_next_sentence_together_when_it_fits() {
        let b = cl100k_base().unwrap();
        let previous = format!("{} weak.", "word ".repeat(248).trim_end());
        let next = "I had just the same thing this summer with a weaver’s interior.";
        assert!(tokens(&b, &format!("{previous} I had just the")) <= 256);
        assert!(tokens(&b, &format!("{previous} {next}")) > 256);
        assert_eq!(
            chunks(&b, &format!("{previous} {next}")).unwrap(),
            [previous, next.to_owned()]
        );
        assert!(!ends_sentence("J."));
        assert!(ends_sentence("Breton.”"));
    }
}
