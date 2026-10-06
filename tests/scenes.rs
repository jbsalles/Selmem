//! Data-driven narrative cases from tests/cases/*.json
use selmem::encode::scoring::lexical_similarity;
use selmem::{
    AxiomLayer, Channel, EntityProfile, IdentityAxiom, SelectiveMemory, fingerprint,
    singularity_distance, EncodeInput,
};

#[test]
fn json_narrative_cases() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no tests/cases/*.json");
    for path in files {
        let raw = std::fs::read_to_string(&path).unwrap();
        run_case(&path.file_name().unwrap().to_string_lossy(), &raw);
    }
}

fn run_case(name: &str, raw: &str) {
    let nights = field_u32(raw, "nights").unwrap_or(0);
    let compare = field_bool(raw, "compare_clones").unwrap_or(false);
    let scenes = object_array(raw, "scenes");
    assert!(!scenes.is_empty(), "{name}: scenes vides");
    let expect = object_field(raw, "expect").unwrap_or_else(|| raw.to_string());
    let threshold = field_f32(raw, "encode_threshold");

    if compare {
        let mut claire = make_mem("tender", "Claire", threshold);
        let mut silas = make_mem("austere", "Silas", threshold);
        seed_axioms(raw, &mut claire);
        seed_axioms(raw, &mut silas);
        play(&mut claire, &scenes, nights);
        play(&mut silas, &scenes, nights);
        check(name, &expect, &scenes, &mut claire, Some(&mut silas));
    } else {
        let kind = field_str(raw, "profile").unwrap_or_else(|| "tender".into());
        let label = if kind == "austere" { "Silas" } else { "Claire" };
        let mut mem = make_mem(&kind, label, threshold);
        seed_axioms(raw, &mut mem);
        play(&mut mem, &scenes, nights);
        check(name, &expect, &scenes, &mut mem, None);
    }
}

fn make_mem(kind: &str, name: &str, threshold: Option<f32>) -> SelectiveMemory {
    let mut profile = match kind {
        "austere" => EntityProfile::austere(name),
        "plain" => EntityProfile::new(name),
        _ => EntityProfile::tender(name),
    };
    if let Some(t) = threshold {
        profile.encode_threshold = t;
    }
    SelectiveMemory::new(profile)
}

fn seed_axioms(raw: &str, mem: &mut SelectiveMemory) {
    for obj in object_array(raw, "seed_axioms") {
        let layer = match field_str(&obj, "layer").as_deref() {
            Some("motif") => AxiomLayer::Motif,
            Some("trait") => AxiomLayer::Trait,
            _ => AxiomLayer::Belief,
        };
        mem.store.add_axiom(IdentityAxiom {
            id: field_str(&obj, "id").unwrap_or_else(|| "ax_seed".into()),
            statement: field_str(&obj, "statement").unwrap_or_default(),
            support_trace_ids: Vec::new(),
            valence: field_f32(&obj, "valence").unwrap_or(0.0),
            strength: field_f32(&obj, "strength").unwrap_or(0.3),
            created_at: 1,
            superseded_by: None,
            schema: field_str(&obj, "schema"),
            layer,
            stake_kind: selmem::core::model::StakeKind::None,
            bearer: selmem::core::model::Bearer::World,
            loss_kind: selmem::core::model::LossKind::None,
            stake_mark: String::new(),
        });
    }
}

fn play(mem: &mut SelectiveMemory, scenes: &[String], nights: u32) {
    for obj in scenes {
        let event = field_str(obj, "event").expect("event");
        let mut ev = EncodeInput::new(&event);
        if let Some(v) = field_f32(obj, "valence") {
            ev.valence = v;
        }
        if let Some(v) = field_f32(obj, "arousal") {
            ev.arousal = v;
        }
        if let Some(v) = field_f32(obj, "disgust") {
            ev.disgust = v;
        }
        if let Some(v) = field_f32(obj, "self_relevance") {
            ev.self_relevance = v;
        }
        if let Some(s) = field_str(obj, "attribution") {
            ev.attribution = selmem::Attribution::parse(&s);
        }
        if let Some(v) = field_f32(obj, "utility") {
            ev.utility = v;
        }
        if let Some(v) = field_f32(obj, "permanence") {
            ev.permanence = v;
        }
        ev.schema = field_str(obj, "schema");
        if field_str(obj, "channel").as_deref() == Some("world") {
            ev.channel = Channel::World;
        }
        mem.live_with(ev);
    }
    for _ in 0..nights {
        mem.sleep();
    }
}

fn check(
    name: &str,
    expect: &str,
    scenes: &[String],
    mem: &mut SelectiveMemory,
    other: Option<&mut SelectiveMemory>,
) {
    if let Some(kept) = field_bool(expect, "last_kept") {
        let last = scenes.last().unwrap();
        let event = field_str(last, "event").unwrap();
        let found = mem.store.archives.values().any(|a| a.verbatim == event);
        if kept {
            assert!(
                found || !mem.store.traces.is_empty(),
                "{name}: expected the last scene to be kept"
            );
        } else {
            assert!(!found, "{name}: last scene should have been dropped");
        }
    }
    if let Some(n) = field_u32(expect, "archives") {
        assert_eq!(mem.store.archives.len(), n as usize, "{name}: archives");
    }
    if field_bool(expect, "speak_nonempty").unwrap_or(false) {
        let q = field_str(expect, "speak").unwrap_or_else(|| "tu te souviens".into());
        let reply = mem.speak(&q);
        assert!(!reply.trim().is_empty(), "{name}: empty speak");
    }
    if let Some(layer) = field_str(expect, "who_has_layer") {
        let hit = mem.who_am_i().iter().any(|a| match layer.as_str() {
            "motif" => a.layer == AxiomLayer::Motif,
            "belief" => a.layer == AxiomLayer::Belief,
            "trait" => a.layer == AxiomLayer::Trait,
            _ => false,
        });
        assert!(hit, "{name}: missing layer {layer}");
    }
    if let Some(layer) = field_str(expect, "who_not_layer") {
        let hit = mem.who_am_i().iter().any(|a| match layer.as_str() {
            "motif" => a.layer == AxiomLayer::Motif,
            "belief" => a.layer == AxiomLayer::Belief,
            "trait" => a.layer == AxiomLayer::Trait,
            _ => false,
        });
        assert!(!hit, "{name}: unexpected layer {layer}");
    }

    let rec = if let Some(ask) = field_str(expect, "ask") {
        mem.remember(&ask)
            .into_iter()
            .map(|r| r.narrative)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    };

    if let Some(idx) = field_u32(expect, "not_verbatim_index") {
        let src = field_str(&scenes[idx as usize], "event").unwrap();
        assert!(!rec.is_empty(), "{name}: empty recall");
        assert_ne!(rec, src, "{name}: reconstruction == log");
        assert!(
            lexical_similarity(&rec, &src) < 1.0,
            "{name}: reconstruction stuck to the log"
        );
        assert!(
            mem.store
                .archives
                .values()
                .any(|a| a.verbatim == src),
            "{name}: the log must still sit in the seal or 'not verbatim' is an empty-store pass"
        );
    }
    if let Some(needles) = field_str_array(expect, "recall_contains_any") {
        let world = if let Some(q) = field_str(expect, "world_ask") {
            mem.remember(&q)
                .into_iter()
                .map(|r| r.narrative)
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        } else {
            rec.to_lowercase()
        };
        assert!(
            needles.iter().any(|n| world.contains(&n.to_lowercase())),
            "{name}: missing world fact in {world}"
        );
    }

    if field_bool(expect, "clones_differ").unwrap_or(false) {
        let other = other.expect("clones_differ needs compare_clones");
        let d = singularity_distance(&fingerprint(mem), &fingerprint(other));
        let rec_b = if let Some(ask) = field_str(expect, "ask") {
            other
                .remember(&ask)
                .into_iter()
                .map(|r| r.narrative)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };
        let texts_differ = rec.is_empty() || rec_b.is_empty() || lexical_similarity(&rec, &rec_b) < 0.99;
        assert!(
            d > 0.0 || texts_differ,
            "{name}: clones interchangeable d={d:.4}"
        );
    }
}

fn field_str(json: &str, key: &str) -> Option<String> {
    selmem::net::httpx::extract_json_string(json, key)
}

fn field_f32(json: &str, key: &str) -> Option<f32> {
    let pat = format!("\"{key}\"");
    let i = json.find(&pat)?;
    let after = json[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num.parse().ok()
}

fn field_bool(json: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{key}\"");
    let i = json.find(&pat)?;
    let after = json[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn field_u32(json: &str, key: &str) -> Option<u32> {
    field_f32(json, key).map(|n| n as u32)
}

fn field_str_array(json: &str, key: &str) -> Option<Vec<String>> {
    let pat = format!("\"{key}\"");
    let i = json.find(&pat)?;
    let after = json[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    let start = after.find('[')?;
    let rest = &after[start + 1..];
    let end = rest.find(']')?;
    let inner = &rest[..end];
    let vals: Vec<String> = inner
        .split(',')
        .filter_map(|s| {
            let s = s.trim().trim_matches('"');
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals)
    }
}

fn object_field(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = json.find(&pat)?;
    let after = json[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !after.starts_with('{') {
        return None;
    }
    Some(slice_balanced(after, '{', '}')?.to_string())
}

fn object_array(json: &str, key: &str) -> Vec<String> {
    let pat = format!("\"{key}\"");
    let Some(i) = json.find(&pat) else {
        return Vec::new();
    };
    let after = match json[i + pat.len()..].trim_start().strip_prefix(':') {
        Some(s) => s.trim_start(),
        None => return Vec::new(),
    };
    let Some(arr) = slice_balanced(after, '[', ']') else {
        return Vec::new();
    };
    let inner = &arr[1..arr.len().saturating_sub(1)];
    let mut out = Vec::new();
    let mut rest = inner.trim();
    while let Some(obj) = slice_balanced(rest, '{', '}') {
        out.push(obj.to_string());
        rest = rest[obj.len()..].trim_start().trim_start_matches(',').trim_start();
        if rest.is_empty() {
            break;
        }
    }
    out
}

fn slice_balanced(s: &str, open: char, close: char) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = s.find(open)?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in s[start..].char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        if c == '"' {
            in_str = true;
            continue;
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(&s[start..start + i + c.len_utf8()]);
            }
        }
    }
    let _ = bytes;
    None
}
