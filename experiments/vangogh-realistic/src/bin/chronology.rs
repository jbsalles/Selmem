//! Build a dated editorial timeline and task-blind historical adaptation requests.
//! Raw modern dialogs are a draft, never silently certified as historical scenes.
use roxmltree::Document;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use tiktoken_rs::cl100k_base;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn node_text(n: roxmltree::Node<'_, '_>) -> String {
    normalized(
        &n.descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect::<String>(),
    )
}
fn day_number(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146097 + yo * 365 + yo / 4 - yo / 100 + doy
}
#[derive(Clone, Serialize)]
struct Anchor {
    id: String,
    machine_date: String,
    date_text: String,
    last_reported_place: String,
    reference_day: i64,
    month_corrected: bool,
    uncertain: bool,
}
fn anchor(dir: &Path, id: &str) -> Result<Anchor> {
    let xml = fs::read_to_string(dir.join(format!("{id}.xml")))?;
    let doc = Document::parse(&xml)?;
    let sent = doc
        .descendants()
        .find(|n| n.has_tag_name("correspAction") && n.attribute("type") == Some("sent"))
        .ok_or("missing sent metadata")?;
    let date = sent
        .children()
        .find(|n| n.has_tag_name("date"))
        .ok_or("missing date")?;
    let machine_date = date.attribute("when").ok_or("missing when")?.to_string();
    let date_text = node_text(date);
    let pieces = machine_date
        .split('-')
        .map(str::parse::<i64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if pieces.len() < 2 {
        return Err(format!("unsupported date {machine_date}").into());
    }
    let mut y = pieces[0];
    let mut m = pieces[1];
    let mut d = *pieces.get(2).unwrap_or(&15);
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut mentioned: Vec<_> = months
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            date_text
                .find(s)
                .map(|position| (position, i as i64 + 1, s.len()))
        })
        .collect();
    mentioned.sort_by_key(|x| x.0);
    let month_corrected = mentioned.first().is_some_and(|x| x.1 != m);
    let mut textual_day = false;
    if let Some(&(position, month, length)) = mentioned.first() {
        m = month;
        let numbers = |s: &str| {
            s.split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.parse::<i64>().ok())
                .collect::<Vec<_>>()
        };
        if let Some(year) = numbers(&date_text[position + length..])
            .into_iter()
            .find(|n| *n >= 1800 && *n <= 1900)
        {
            y = year;
        }
        if let Some(day) = numbers(&date_text[..position])
            .into_iter()
            .find(|n| (1..=31).contains(n))
        {
            d = day;
            textual_day = true;
        } else {
            let before = date_text[..position].to_lowercase();
            d = if before.contains("beginning") || before.contains("early") {
                1
            } else if before.contains("end") || before.contains("late") {
                25
            } else {
                15
            };
        }
    }
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err("invalid date".into());
    }
    let lower = date_text.to_lowercase();
    let uncertain = !textual_day
        || [
            "about", "between", " or ", " and ", "early", "mid-", "late", "probably", "possibly",
        ]
        .iter()
        .any(|s| lower.contains(s));
    let last_reported_place = sent
        .children()
        .find(|n| n.has_tag_name("placeName"))
        .map(node_text)
        .unwrap_or_default();
    Ok(Anchor {
        id: id.into(),
        machine_date,
        date_text,
        last_reported_place,
        reference_day: day_number(y, m, d),
        month_corrected,
        uncertain,
    })
}
#[derive(Clone)]
struct Block {
    id: String,
    rows: Vec<Value>,
}
fn read_blocks(dir: &Path, stream: &str, audit: &str, kind: &str) -> Result<Vec<Block>> {
    let s = fs::read_to_string(dir.join(stream))?;
    let a = fs::read_to_string(dir.join(audit))?;
    let s = s.lines().collect::<Vec<_>>();
    let a = a.lines().collect::<Vec<_>>();
    if s.len() != a.len() {
        return Err("stream/audit length mismatch".into());
    }
    let mut result: Vec<Block> = vec![];
    for (s, a) in s.into_iter().zip(a) {
        let row: Value = serde_json::from_str(s)?;
        let label: Value = serde_json::from_str(a)?;
        if row["event_id"] != label["event_id"] {
            return Err("stream/audit ID mismatch".into());
        }
        if label["source_kind"] != kind {
            continue;
        }
        let id = label["source_block"]
            .as_str()
            .ok_or("missing source block")?;
        if result.last().is_some_and(|b| b.id == id) {
            result.last_mut().unwrap().rows.push(row)
        } else {
            result.push(Block {
                id: id.into(),
                rows: vec![row],
            })
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for b in &result {
        if !seen.insert(&b.id) {
            return Err("source block reused or split".into());
        }
    }
    Ok(result)
}
// Preserve edition order. For uncertain/inverted anchors, one ordinal unit is
// used rather than inventing negative time or silently sorting letters.
fn weights(anchors: &[Anchor]) -> Vec<u64> {
    anchors
        .windows(2)
        .map(|p| (p[1].reference_day - p[0].reference_day).max(1) as u64)
        .collect()
}
fn allocate(noise_tokens: &[usize], weights: &[u64]) -> Result<Vec<Vec<usize>>> {
    if weights.is_empty() {
        return Err("at least two letters required for inter-letter reconstruction".into());
    }
    let total: u128 = weights.iter().map(|w| *w as u128).sum();
    let noise_total: usize = noise_tokens.iter().sum();
    let mut out = vec![vec![]; weights.len()];
    let mut elapsed = 0u128;
    let mut used = 0;
    let mut j = 0;
    for (i, w) in weights.iter().enumerate() {
        elapsed += *w as u128;
        let due = (noise_total as u128 * elapsed / total) as usize;
        while j < noise_tokens.len() && used + noise_tokens[j] <= due {
            out[i].push(j);
            used += noise_tokens[j];
            j += 1
        }
    }
    if j != noise_tokens.len() {
        return Err("incomplete allocation".into());
    }
    Ok(out)
}
fn emit(out: &mut String, v: &impl Serialize) -> Result<()> {
    out.push_str(&serde_json::to_string(v)?);
    out.push('\n');
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("CHRONOLOGY: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        println!("chronology LETTER_XML_DIR PREPARED_CORPUS FRESH_OUT\nProduces an English timeline draft and historical-scene rewrite requests. No LLM calls.");
        return Ok(());
    }
    let xml = Path::new(&args[0]);
    let input = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    if out.exists() {
        return Err("output exists".into());
    }
    let manifest: Value = serde_json::from_slice(&fs::read(input.join("manifest.json"))?)?;
    for (name, expected) in manifest["files"].as_object().ok_or("missing checksums")? {
        if hash(&fs::read(input.join(name))?) != expected.as_str().ok_or("invalid checksum")? {
            return Err(format!("checksum mismatch: {name}").into());
        }
    }
    let letters = read_blocks(input, "stream-clean.jsonl", "audit-clean.jsonl", "letter")?;
    let noise = read_blocks(input, "stream-mixed.jsonl", "audit-mixed.jsonl", "locomo")?;
    let anchors = letters
        .iter()
        .map(|l| anchor(xml, &l.id))
        .collect::<Result<Vec<_>>>()?;
    // Bind XML metadata to exactly the edition used for corpus preparation.
    let mut files = fs::read_dir(xml)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    files.retain(|p| {
        p.extension().is_some_and(|x| x == "xml")
            && p.file_stem()
                .is_some_and(|x| x.to_string_lossy().starts_with("let"))
    });
    files.sort();
    let mut source_hashes = String::new();
    for p in files {
        let id = p.file_stem().unwrap().to_string_lossy();
        source_hashes.push_str(&format!("{id}:{}\n", hash(&fs::read(&p)?)))
    }
    if hash(source_hashes.as_bytes())
        != manifest["letters_sha256"]
            .as_str()
            .ok_or("missing source digest")?
    {
        return Err("XML metadata differs from the prepared corpus edition".into());
    }
    let bpe = cl100k_base()?;
    let counts: Vec<_> = noise
        .iter()
        .map(|s| {
            s.rows
                .iter()
                .map(|e| bpe.encode_ordinary(e["text"].as_str().unwrap_or("")).len())
                .sum::<usize>()
        })
        .collect();
    let gap_weights = weights(&anchors);
    let slots = allocate(&counts, &gap_weights)?;
    let mut timeline = String::new();
    let mut requests = String::new();
    let mut audit = String::new();
    let mut sequence = 0;
    let mut previous_scenes = BTreeMap::<String, String>::new();
    for (i, l) in letters.iter().enumerate() {
        for row in &l.rows {
            sequence += 1;
            let mut v = row.clone();
            v["sequence"] = json!(sequence);
            v["language"] = json!("en");
            v["chronology"] =
                json!({"anchor_date":anchors[i].date_text,"date_uncertain":anchors[i].uncertain});
            emit(&mut timeline, &v)?;
            emit(
                &mut audit,
                &json!({"sequence":sequence,"source_kind":"letter","source_block":l.id}),
            )?;
        }
        if i + 1 == letters.len() {
            continue;
        }
        for (position, &j) in slots[i].iter().enumerate() {
            let n = &noise[j];
            let scene_id = format!("scene-{:04}", j + 1);
            let interval = json!({"after_letter":l.id,"before_letter":letters[i+1].id,"after_date":anchors[i].date_text,"before_date":anchors[i+1].date_text,"uncertain":anchors[i].uncertain||anchors[i+1].uncertain||anchors[i+1].reference_day<=anchors[i].reference_day,"position":position+1,"scenes_in_gap":slots[i].len()});
            for row in &n.rows {
                sequence += 1;
                let mut v = row.clone();
                v["sequence"] = json!(sequence);
                v["period"] = json!(i + 1);
                v["language"] = json!("en");
                v["chronology"] = interval.clone();
                emit(&mut timeline, &v)?;
                emit(
                    &mut audit,
                    &json!({"sequence":sequence,"source_kind":"unadapted_locomo_draft","source_block":n.id,"scene_id":scene_id,"interval":interval}),
                )?;
            }
            let past = l
                .rows
                .iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(" ");
            let past_tokens = bpe.encode_ordinary(&past);
            let mut end = past_tokens.len().min(2000);
            let past_excerpt = loop {
                if let Ok(s) = bpe.decode(past_tokens[..end].to_vec()) {
                    break s;
                }
                end -= 1;
            };
            let thread =
                n.id.rsplit_once('/')
                    .ok_or("invalid noise block ID")?
                    .0
                    .to_string();
            let previous_scene = previous_scenes.get(&thread).cloned();
            let last_vincent_place = (0..=i)
                .rev()
                .find(|&k| letters[k].rows[0]["sender"] == "Vincent van Gogh")
                .map(|k| anchors[k].last_reported_place.clone());
            let request = json!({"scene_id":scene_id,"language":"en","status":"PENDING_REWRITE_AND_REVIEW","after_date":anchors[i].date_text,"before_date":anchors[i+1].date_text,"date_uncertain":interval["uncertain"],"last_reported_place":last_vincent_place,"place_is_not_a_claim_of_continued_residence":true,"preceding_letter_excerpt":past_excerpt,"source_dialogue":n.rows.iter().map(|r|json!({"sender":r["sender"],"recipient":r["recipient"],"text":r["text"]})).collect::<Vec<_>>(),"reference_body_token_budget":counts[j],"previous_scene_id":previous_scene,"instructions":"Write an English fictional everyday scene suitable for the interval in nineteenth-century Europe. Use the source dialogue only for ordinary social dynamics and variety. Preserve plausible recurring relationships and tensions; do not force every scene to be bland or creative. Replace or omit modern technology, institutions, dates and terminology; do not merely rename objects. Do not invent documented meetings, major biographical events, new facts about real named people, paintings or artistic revelations. Use fictional participants, keep their identity separate from Vincent and his correspondents, and do not write their experiences as Vincent's own memories. Do not use knowledge of later letters or famous future works. The excerpt is historical context, not a template to quote or repeat. If the interval or location is uncertain, use no precise invented date or asserted location. All dialogue and narrative must be in English. Return JSON with scene_id, English events [{sender,recipient,text}], changes_from_source, uncertainties. The scene is experimental fiction, not a recovered historical event. Before each request, the runner must supply only the generated past scene/state (still subject to review) referenced by previous_scene_id. Recount with the reference tokenizer after rewriting; never inflate to the target by repeating text."});
            emit(&mut requests, &request)?;
            previous_scenes.insert(thread, scene_id);
        }
    }
    let warnings:Vec<_>=anchors.windows(2).filter(|w|w[1].reference_day<w[0].reference_day).map(|w|json!({"after":w[0].id,"before":w[1].id,"action":"edition order retained; gap is ordinal, not a verified duration"})).collect();
    let stage = out.with_extension("preparing");
    if let Some(p) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?
    }
    fs::create_dir(&stage)?;
    let mut hashes = BTreeMap::new();
    for (name, text) in [
        ("timeline-draft.jsonl", timeline),
        ("timeline-audit.jsonl", audit),
        ("scene-requests.jsonl", requests),
    ] {
        hashes.insert(name, hash(text.as_bytes()));
        fs::write(stage.join(name), text)?
    }
    let report = json!({"version":"interletter-v1","status":"DRAFT_REQUIRES_HISTORICAL_REWRITING_AND_REVIEW","language":"en","language_basis":"published English letter translation and English LoCoMo text; quotations and names may remain in other languages","letters":letters.len(),"scene_requests":noise.len(),"gaps":slots.len(),"occupied_gaps":slots.iter().filter(|x|!x.is_empty()).count(),"corrected_month_anchors":anchors.iter().filter(|a|a.month_corrected).count(),"uncertain_anchors":anchors.iter().filter(|a|a.uncertain).count(),"date_order_warnings":warnings,"first_date":anchors.first().unwrap().date_text,"last_date":anchors.last().unwrap().date_text,"allocation":"whole sessions weighted by estimated elapsed days between letters; uncertain machine dates are scheduling estimates only; no scenes before first or after last letter","draft_noise_tokens":counts.iter().sum::<usize>(),"requested_final_noise_percent":manifest["requested_noise_percent"],"final_noise_fraction_after_rewriting":Value::Null,"anchors":anchors,"files":hashes,"model_calls":0});
    fs::write(
        stage.join("manifest.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::rename(stage, out)?;
    println!("{} letters, {} ordered scene requests across {} occupied intervals; historical rewriting pending. {}",letters.len(),noise.len(),slots.iter().filter(|x|!x.is_empty()).count(),out.display());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textual_dates_correct_cross_year_and_partial_machine_dates() {
        let dir = std::env::temp_dir().join(format!("vg-dates-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for (id, when, text, expected) in [
            (
                "let001",
                "1889-01-31",
                "Tuesday, 31 December 1889 or Wednesday, 1 January 1890",
                day_number(1889, 12, 31),
            ),
            (
                "let002",
                "1888-09",
                "9 September 1888",
                day_number(1888, 9, 9),
            ),
            (
                "let003",
                "1881-10",
                "beginning of October 1881",
                day_number(1881, 10, 1),
            ),
        ] {
            fs::write(dir.join(format!("{id}.xml")),format!("<TEI><correspAction type=\"sent\"><date when=\"{when}\">{text}</date></correspAction></TEI>")).unwrap();
            assert_eq!(anchor(&dir, id).unwrap().reference_day, expected);
        }
        assert!(anchor(&dir, "let001").unwrap().uncertain);
        assert!(!anchor(&dir, "let002").unwrap().uncertain);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn calendar_intervals() {
        assert_eq!(day_number(1888, 3, 1) - day_number(1888, 2, 28), 2);
        assert_eq!(day_number(1889, 3, 1) - day_number(1889, 2, 28), 1);
    }
    #[test]
    fn allocation_preserves_sessions_and_time_weight() {
        let slots = allocate(&[10, 10, 10, 10, 10, 10, 10, 10, 10, 10], &[1, 9]).unwrap();
        assert_eq!(slots[0], vec![0]);
        assert_eq!(slots[1], (1..10).collect::<Vec<_>>());
        assert!(allocate(&[1], &[]).is_err());
    }
    #[test]
    fn all_sessions_stay_inside_intervals() {
        let slots = allocate(&[70, 5, 25], &[1, 1, 1]).unwrap();
        assert!(slots[0].is_empty());
        assert_eq!(
            slots.into_iter().flatten().collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
}
