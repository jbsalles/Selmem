//! Semantic boundary. Only interpreters inspect words to infer meaning.
//! The lexical backend is a reproducible baseline, not a learned psychology.
use crate::core::model::{AbsenceKind, Attribution, Bearer, LossKind, SemanticCore, StakeKind};

#[derive(Clone, Debug)]
pub struct EventSemantics {
    pub core: SemanticCore,
    pub stake_kind: StakeKind,
    pub bearer: Bearer,
    pub loss_kind: LossKind,
    pub stake_mark: String,
    pub absence: Option<AbsenceKind>,
    pub valence: f32,
    pub arousal: f32,
    pub disgust: f32,
    pub self_relevance: f32,
    pub goal_relevance: f32,
    pub attribution: Attribution,
    pub schema: Option<String>,
}

impl EventSemantics {
    /// Reject malformed backend output before any state is written.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min) in [
            ("polarity", self.core.polarity, -1.0),
            ("valence", self.valence, -1.0),
            ("confidence", self.core.confidence, 0.0),
            ("arousal", self.arousal, 0.0),
            ("disgust", self.disgust, 0.0),
            ("self_relevance", self.self_relevance, 0.0),
            ("goal_relevance", self.goal_relevance, 0.0),
        ] {
            if !value.is_finite() || !(min..=1.0).contains(&value) {
                return Err(format!("invalid semantic {name}"));
            }
        }
        Ok(())
    }
}

/// Independent of the narrator, mood, axioms and sealed archive.
/// Implement with a learned model, external annotations, or a deterministic baseline.
pub trait SemanticInterpreter: Send + Sync {
    fn interpret_event(&self, event: &str) -> Result<EventSemantics, String>;
}

impl<T: SemanticInterpreter + ?Sized> SemanticInterpreter for std::sync::Arc<T> {
    fn interpret_event(&self, event: &str) -> Result<EventSemantics, String> {
        (**self).interpret_event(event)
    }
}

#[derive(Default)]
pub struct LexicalInterpreter;
impl SemanticInterpreter for LexicalInterpreter {
    fn interpret_event(&self, event: &str) -> Result<EventSemantics, String> {
        let (valence, arousal, disgust, schema) = super::affect::guess(event);
        let (stake_kind, bearer, loss_kind, stake_mark, absence) = legacy_stake(event);
        Ok(EventSemantics {
            core: legacy_core(event, &super::core::extractive_core(event), valence),
            stake_kind,
            bearer,
            loss_kind,
            stake_mark,
            absence,
            valence,
            arousal,
            disgust,
            schema,
            self_relevance: 0.5,
            goal_relevance: 0.3,
            attribution: Attribution::None,
        })
    }
}

pub fn legacy_core(event: &str, claim: &str, valence: f32) -> SemanticCore {
    SemanticCore {
        claim: claim.to_string(),
        entities: extract_entities(event),
        actions: extract_actions(event),
        polarity: valence.clamp(-1.0, 1.0),
        confidence: 0.55,
    }
}

fn extract_entities(event: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in event.split_whitespace() {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if w.len() < 2 {
            continue;
        }
        let low = w.to_lowercase();
        let pronoun = matches!(
            low.as_str(),
            "i" | "me" | "she" | "he" | "they" | "we" | "him" | "her" | "you"
        );
        let grammatical = matches!(low.as_str(),
            "a"|"an"|"the"|"this"|"that"|"these"|"those"|"first"|"second"|"third"|
            "yesterday"|"today"|"hello"|"hi"|"hey"|"observation");
        let named = !grammatical && w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        if pronoun || named {
            if !out.iter().any(|e: &String| e.eq_ignore_ascii_case(w)) {
                out.push(w.to_string());
            }
        }
    }
    out.truncate(8);
    out
}

fn extract_actions(event: &str) -> Vec<String> {
    const VERBS: &[&str] = &[
        "left",
        "said",
        "walked",
        "abandoned",
        "told",
        "asked",
        "stayed",
        "opened",
        "closed",
        "went",
        "came",
        "took",
        "gave",
        "kept",
        "broke",
        "loved",
        "hated",
        "waited",
        "lied",
        "withdrawn",
        "renewed",
    ];
    let mut out = Vec::new();
    for raw in event.split_whitespace() {
        let w = raw
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if VERBS.contains(&w.as_str()) || (w.len() > 4 && w.ends_with("ed")) {
            if !out.contains(&w) {
                out.push(w);
            }
        }
    }
    out.truncate(6);
    out
}

pub fn legacy_stake(event: &str) -> (StakeKind, Bearer, LossKind, String, Option<AbsenceKind>) {
    let low = event.to_lowercase();
    let absence = if low.contains("no reply") || low.contains("unanswered") {
        Some(AbsenceKind::Unanswered)
    } else if low.contains("didn't come") || low.contains("did not come") || low.contains("missed")
    {
        Some(AbsenceKind::Missed)
    } else if low.contains("never arrived") || low.contains("unmet") {
        Some(AbsenceKind::Unmet)
    } else {
        None
    };
    let kind = if absence.is_some() {
        StakeKind::Absence
    } else if low.contains("promise") || low.contains("vow") {
        StakeKind::Promise
    } else if low.contains("rule") || low.contains("must") {
        StakeKind::Rule
    } else if low.contains("limit")
        || low.contains("boundary")
        || low.contains("withdrawn")
        || low.contains("cancelled")
    {
        StakeKind::Limit
    } else if low.contains("decision") || low.contains("choose") {
        StakeKind::Decision
    } else if low.contains("present") || low.contains("stayed") || low.contains("remained") {
        StakeKind::Presence
    } else if low.contains("felt") || low.contains("mood") {
        StakeKind::Mood
    } else {
        StakeKind::None
    };
    let words: Vec<_> = low.split_whitespace()
        .map(|w|w.trim_matches(|c:char|!c.is_alphanumeric())).collect();
    let bearer = if words.contains(&"i") {
        Bearer::Self_
    } else if words.iter().any(|w|matches!(*w,"she"|"he"|"they"|"you")) {
        Bearer::Other
    } else {
        Bearer::World
    };
    let loss = if kind == StakeKind::Limit {
        LossKind::Status
    } else if kind == StakeKind::Absence {
        LossKind::Access
    } else if kind == StakeKind::Promise || kind == StakeKind::Rule {
        LossKind::Coherence
    } else if kind == StakeKind::Presence {
        LossKind::Time
    } else {
        LossKind::None
    };
    let mark = stake_mark(&low);
    (kind, bearer, loss, mark, absence)
}

fn stake_mark(low: &str) -> String {
    const SKIP: &[&str] = &[
        "that", "this", "with", "from", "after", "before", "into", "your", "their", "been", "were",
        "was", "have", "has", "had", "them", "they", "what", "the", "and", "for",
    ];
    let tokens: Vec<&str> = low
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 4 && !SKIP.contains(w))
        .collect();
    const PREFER: &[&str] = &["withdrawn", "cancelled", "extended", "renewed"];
    if let Some(act) = tokens.iter().find(|w| PREFER.contains(w)) {
        return (*act).to_string();
    }
    tokens
        .into_iter()
        .max_by_key(|w| w.len())
        .unwrap_or("")
        .to_string()
}

/// Strict, line-oriented protocol shared by HTTP and external adapters.
/// Missing/unknown enum values and non-finite numbers are errors, not defaults.
pub fn parse_semantics(raw: &str, event: &str) -> Result<EventSemantics, String> {
    let mut fields = std::collections::BTreeMap::new();
    for line in raw.lines().filter(|s| !s.trim().is_empty()) {
        let (key, value) = line.split_once('=').ok_or("expected key=value")?;
        if fields.insert(key.trim(), value.trim()).is_some() {
            return Err(format!("duplicate semantic field: {}", key.trim()));
        }
    }
    let get = |key: &str| {
        fields
            .get(key)
            .copied()
            .ok_or_else(|| format!("missing {key}"))
    };
    let number = |key: &str| -> Result<f32, String> {
        get(key)?.parse().map_err(|_| format!("invalid {key}"))
    };
    let list = |key: &str| -> Result<Vec<String>, String> {
        Ok(get(key)?
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "-")
            .map(str::to_string)
            .collect())
    };
    let stake = get("event_type")?;
    if ![
        "none", "decision", "presence", "rule", "promise", "limit", "mood", "absence",
    ]
    .contains(&stake)
    {
        return Err("invalid event_type".into());
    }
    let bearer = get("bearer")?;
    if !["self", "other", "world"].contains(&bearer) {
        return Err("invalid bearer".into());
    }
    let loss = get("loss")?;
    if !["none", "status", "access", "coherence", "time"].contains(&loss) {
        return Err("invalid loss".into());
    }
    let agency = get("agency")?;
    if !["none", "internal", "external"].contains(&agency) {
        return Err("invalid agency".into());
    }
    let absence = match get("absence")? {
        "none" => None,
        value => Some(AbsenceKind::parse(value).ok_or("invalid absence")?),
    };
    let schema = get("schema")?;
    let mark = get("stake_mark")?;
    let semantics = EventSemantics {
        core: SemanticCore {
            claim: match fields.get("core_claim") {
                Some(claim) => super::core::accept_core(claim, event)
                    .ok_or("core_claim is not grounded in the event")?,
                None => super::core::extractive_core(event),
            },
            entities: list("entities")?,
            actions: list("actions")?,
            polarity: number("valence")?,
            confidence: number("confidence")?,
        },
        stake_kind: StakeKind::parse(stake),
        bearer: Bearer::parse(bearer),
        loss_kind: LossKind::parse(loss),
        stake_mark: if mark == "-" {
            String::new()
        } else {
            mark.into()
        },
        absence,
        valence: number("valence")?,
        arousal: number("arousal")?,
        disgust: number("disgust")?,
        self_relevance: number("self_relevance")?,
        goal_relevance: number("goal_relevance")?,
        attribution: Attribution::parse(agency),
        schema: if schema == "-" {
            None
        } else {
            Some(schema.into())
        },
    };
    semantics.validate()?;
    Ok(semantics)
}
