//! External strings. Rust applies them; it does not author them.
use std::sync::OnceLock;

pub struct Weighted {
    pub stem: String,
    pub w: f32,
}

pub struct AffectLex {
    pub neg: Vec<Weighted>,
    pub pos: Vec<Weighted>,
    pub intensifiers: Vec<String>,
    pub schema_neg: String,
    pub schema_pos: String,
    pub schema_mix: String,
}

pub struct Prompts {
    pub reconstruct: String,
    pub distill: String,
    pub interpret: String,
    pub extract_core: String,
    pub segment: String,
    pub rewrite: String,
    pub recontextualize: String,
    pub reply: String,
    pub voice_tender: String,
    pub voice_austere: String,
    pub voice_neutral: String,
}

pub struct RuleCopy {
    pub reply_recall: String,
    pub reply_empty: String,
    pub axiom_neg: String,
    pub axiom_pos: String,
    pub axiom_mid: String,
    pub latent: String,
}

pub fn affect() -> &'static AffectLex {
    static T: OnceLock<AffectLex> = OnceLock::new();
    T.get_or_init(|| parse_affect(include_str!("../data/affect.txt")))
}

pub fn prompts() -> &'static Prompts {
    static T: OnceLock<Prompts> = OnceLock::new();
    T.get_or_init(|| parse_prompts(include_str!("../data/prompts.txt")))
}

pub fn rule() -> &'static RuleCopy {
    static T: OnceLock<RuleCopy> = OnceLock::new();
    T.get_or_init(|| parse_rule(include_str!("../data/rule.txt")))
}

fn parse_affect(raw: &str) -> AffectLex {
    let mut lex = AffectLex {
        neg: Vec::new(),
        pos: Vec::new(),
        intensifiers: Vec::new(),
        schema_neg: "wound".into(),
        schema_pos: "bond".into(),
        schema_mix: "ambivalence".into(),
    };
    let mut sec = String::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            sec = s.to_string();
            continue;
        }
        match sec.as_str() {
            "neg" | "pos" => {
                if let Some((w, n)) = line.split_once('=') {
                    if let Ok(weight) = n.trim().parse::<f32>() {
                        let item = Weighted {
                            stem: w.trim().to_string(),
                            w: weight,
                        };
                        if sec == "neg" {
                            lex.neg.push(item);
                        } else {
                            lex.pos.push(item);
                        }
                    }
                }
            }
            "intensifiers" => lex.intensifiers.push(line.to_string()),
            "schema" => {
                if let Some((k, v)) = line.split_once('=') {
                    match k.trim() {
                        "neg" => lex.schema_neg = v.trim().into(),
                        "pos" => lex.schema_pos = v.trim().into(),
                        "mix" => lex.schema_mix = v.trim().into(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    lex
}

fn parse_prompts(raw: &str) -> Prompts {
    let mut map = std::collections::HashMap::new();
    let mut sec = String::new();
    let mut buf = String::new();
    let flush = |sec: &str, buf: &str, map: &mut std::collections::HashMap<String, String>| {
        if !sec.is_empty() {
            map.insert(sec.to_string(), buf.trim().to_string());
        }
    };
    for line in raw.lines() {
        if let Some(s) = line
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.trim().strip_suffix(']'))
        {
            flush(&sec, &buf, &mut map);
            sec = s.to_string();
            buf.clear();
            continue;
        }
        if !sec.is_empty() {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
        }
    }
    flush(&sec, &buf, &mut map);
    Prompts {
        reconstruct: map.remove("reconstruct").unwrap_or_default(),
        distill: map.remove("distill").unwrap_or_default(),
        interpret: map.remove("interpret").unwrap_or_default(),
        extract_core: map.remove("extract_core").unwrap_or_default(),
        segment: map.remove("segment").unwrap_or_default(),
        rewrite: map.remove("rewrite").unwrap_or_default(),
        recontextualize: map.remove("recontextualize").unwrap_or_default(),
        reply: map.remove("reply").unwrap_or_default(),
        voice_tender: map.remove("voice_tender").unwrap_or_default(),
        voice_austere: map.remove("voice_austere").unwrap_or_default(),
        voice_neutral: map.remove("voice_neutral").unwrap_or_default(),
    }
}

fn parse_rule(raw: &str) -> RuleCopy {
    let mut r = RuleCopy {
        reply_recall: String::new(),
        reply_empty: String::new(),
        axiom_neg: String::new(),
        axiom_pos: String::new(),
        axiom_mid: String::new(),
        latent: String::new(),
    };
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim() {
            "reply_recall" => r.reply_recall = v.to_string(),
            "reply_empty" => r.reply_empty = v.to_string(),
            "axiom_neg" => r.axiom_neg = v.to_string(),
            "axiom_pos" => r.axiom_pos = v.to_string(),
            "axiom_mid" => r.axiom_mid = v.to_string(),
            "latent" => r.latent = v.to_string(),
            _ => {}
        }
    }
    r
}
