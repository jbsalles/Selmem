//! Bounded memory-guided response selection. No store or archive access.
pub use crate::util::json;
mod assessment;
mod grounding;
pub use assessment::{
    assess, quality_input, validate_assessment, validate_quality_audit, GROUNDING, QUALITY,
};
pub use grounding::{parse_judgment, Evidence, Judgment, OrientationReview, SourceReview};
use json::{num, obj, text, Json};
use std::collections::BTreeSet;

pub const VERSION: &str = "selmem-response-choice-5";
pub const GENERATE: &str = include_str!("../../../data/response-candidates-contextual.txt");
pub const EVALUATE: &str = include_str!("../../../data/response-evaluate-transfer.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceMode {
    Auto,
    Direct,
    Always,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Influence {
    None,
    Moderate,
    Strong,
}
impl Influence {
    pub fn weight(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Moderate => 0.5,
            Self::Strong => 1.0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ChoiceOptions {
    pub mode: ChoiceMode,
    pub influence: Influence,
    pub max_words: usize,
}
impl Default for ChoiceOptions {
    fn default() -> Self {
        Self {
            mode: ChoiceMode::Auto,
            influence: Influence::Moderate,
            max_words: 180,
        }
    }
}
impl ChoiceOptions {
    pub fn validate(&self) -> Result<(), String> {
        require(
            (32..=600).contains(&self.max_words),
            "candidate_words must be in 32..600",
        )
    }
    pub fn from_config(c: &crate::Config) -> Result<Self, String> {
        let mode = match c.resolve_or(None, "response_selection", "auto").as_str() {
            "auto" => ChoiceMode::Auto,
            "direct" => ChoiceMode::Direct,
            "always" => ChoiceMode::Always,
            _ => return Err("response_selection must be auto, direct or always".into()),
        };
        let influence = match c.resolve_or(None, "memory_influence", "moderate").as_str() {
            "none" => Influence::None,
            "moderate" => Influence::Moderate,
            "strong" => Influence::Strong,
            _ => return Err("memory_influence must be none, moderate or strong".into()),
        };
        let max_words = c
            .resolve_or(None, "candidate_words", "180")
            .parse()
            .map_err(|_| "invalid candidate_words")?;
        if !(32..=600).contains(&max_words) {
            return Err("candidate_words must be in 32..600".into());
        }
        Ok(Self {
            mode,
            influence,
            max_words,
        })
    }
    pub fn active(&self, user: &str) -> bool {
        if self.mode != ChoiceMode::Auto {
            return self.mode == ChoiceMode::Always;
        }
        // Deliberately explicit, deterministic and cost-free. Other languages can use always.
        let words: BTreeSet<_> = user
            .split(|c: char| !c.is_alphabetic())
            .map(str::to_lowercase)
            .collect();
        ["design", "propose", "brainstorm", "invent", "imagine"]
            .iter()
            .any(|w| words.contains(*w))
    }
    pub fn to_json(&self) -> Json {
        obj([
            (
                "mode",
                text(match self.mode {
                    ChoiceMode::Auto => "auto",
                    ChoiceMode::Direct => "direct",
                    ChoiceMode::Always => "always",
                }),
            ),
            ("weight", num(self.influence.weight())),
            ("max_words", num(self.max_words as u32)),
        ])
    }
}
fn require(ok: bool, why: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn arr(xs: impl IntoIterator<Item = Json>) -> Json {
    Json::Array(xs.into_iter().collect())
}
fn string(v: &Json, k: &str) -> Result<String, String> {
    Ok(v.get(k)?.string()?.into())
}
fn bounded(v: &Json, k: &str, limit: usize) -> Result<String, String> {
    let s = string(v, k)?;
    let count = s.split_whitespace().count();
    let label = v
        .get("id")
        .ok()
        .and_then(|id| id.string().ok())
        .map(|id| format!("{id}.{k}"))
        .unwrap_or_else(|| k.into());
    require(count>0&&count<=limit,&format!("{label}: expected 1..={limit} whitespace-separated words; received {count}. Shorten or fill this field and return the complete object; do not truncate the answer or evidence silently."))?;
    Ok(s)
}
fn strings(v: &Json, k: &str) -> Result<Vec<String>, String> {
    v.get(k)?
        .array()?
        .iter()
        .map(|x| x.string().map(str::to_string))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Orientation {
    pub id: String,
    pub criterion: String,
    pub consequence: String,
    pub limit: String,
    pub sources: Vec<String>,
    pub evidence: Vec<Evidence>,
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: String,
    pub kind: String,
    pub mechanism: String,
    pub answer: String,
    pub orientations: Vec<String>,
    pub sources: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Candidates {
    pub orientations: Vec<Orientation>,
    pub candidates: Vec<Candidate>,
    pub source_reviews: Vec<SourceReview>,
}
#[derive(Clone, Debug)]
pub struct Evaluation {
    pub id: String,
    pub utility: f64,
    pub feasibility: f64,
    pub originality: f64,
    pub alignment: Vec<f64>,
    pub violations: Vec<String>,
    pub reason: String,
}
impl Evaluation {
    pub fn quality(&self) -> f64 {
        (self.utility + self.feasibility + self.originality) / 3.0
    }
}

fn sources(input: &Json) -> Result<(BTreeSet<String>, Vec<(String, String)>), String> {
    let m = input.get("memory")?;
    let mut ids = BTreeSet::new();
    for e in m.get("episodes")?.array()? {
        require(ids.insert(string(e, "trace_id")?), "duplicate episode ID")?;
    }
    for a in m.get("dispositions")?.array()? {
        require(ids.insert(string(a, "axiom_id")?), "duplicate source ID")?;
    }
    let mut edges = vec![];
    for a in m.get("associations")?.array()? {
        let pair = (string(a, "a")?, string(a, "b")?);
        require(
            ids.contains(&pair.0) && ids.contains(&pair.1),
            "association endpoint absent",
        )?;
        edges.push(pair);
    }
    Ok((ids, edges))
}
pub fn parse_candidates(raw: &str, input: &Json, max_words: usize) -> Result<Candidates, String> {
    require(raw.len() <= 100_000, "candidate response too large")?;
    let v = json::parse(raw)?;
    let (allowed, edges) = sources(input)?;
    let rows = v.get("orientations")?.array()?;
    require(rows.len() <= 3, "at most three orientations")?;
    let mut orientations = vec![];
    let mut oid = BTreeSet::new();
    for row in rows {
        let id = string(row, "id")?;
        require(
            ["O1", "O2", "O3"].contains(&id.as_str()) && oid.insert(id.clone()),
            "invalid orientation ID",
        )?;
        let support = strings(row, "source_ids")?;
        require(
            !support.is_empty()
                && support.len() <= 8
                && support.iter().all(|s| allowed.contains(s)),
            "orientation has unknown or absent sources",
        )?;
        require(
            support.iter().collect::<BTreeSet<_>>().len() == support.len(),
            "duplicate orientation source",
        )?;
        let evidence = grounding::parse_evidence(row, input, &support)?;
        orientations.push(Orientation {
            id,
            criterion: bounded(row, "criterion", 40)?,
            consequence: bounded(row, "consequence", 50)?,
            limit: bounded(row, "limit", 40)?,
            sources: support,
            evidence,
        });
    }
    let rows = v.get("candidates")?.array()?;
    require(rows.len() == 3, "exactly three candidates required")?;
    let mut candidates = vec![];
    let mut mechanisms = std::collections::BTreeMap::new();
    for (i, row) in rows.iter().enumerate() {
        let id = string(row, "id")?;
        require(id == ["A", "B", "C"][i], "candidate order must be A B C")?;
        let kind = string(row, "kind")?;
        let expected = match i {
            0 => "direct",
            1 if !orientations.is_empty() => "oriented",
            2 if !edges.is_empty() => "associative",
            _ => "exploratory",
        };
        require(kind==expected || (i==2 && kind=="exploratory"),&format!("candidate {id}: kind={kind:?} is invalid; expected {}. Generated orientations: {}; delivered association edges: {}. With zero orientations B.kind must be exploratory and orientation_ids must be []; C.kind must be exploratory without a relevant delivered edge. Correct the kind field and return all three complete candidates.",if i==2&&!edges.is_empty(){"associative with a cited existing edge, or exploratory"}else{expected},orientations.len(),edges.len()))?;
        let refs = strings(row, "orientation_ids")?;
        require(
            refs.len() <= 3
                && refs.iter().collect::<BTreeSet<_>>().len() == refs.len()
                && refs.iter().all(|s| oid.contains(s)),
            "unknown or duplicate orientation reference",
        )?;
        if kind == "oriented" {
            require(!refs.is_empty(), "oriented candidate needs an orientation")?;
        }
        let support = strings(row, "source_ids")?;
        require(
            support.len() <= 8 && support.iter().all(|s| allowed.contains(s)),
            "unknown candidate source",
        )?;
        require(
            orientations
                .iter()
                .filter(|o| refs.contains(&o.id))
                .all(|o| o.sources.iter().all(|s| support.contains(s))),
            "candidate must cite its orientation sources",
        )?;
        if kind == "associative" {
            require(
                edges
                    .iter()
                    .any(|(a, b)| support.contains(a) && support.contains(b)),
                "associative candidate must cite an existing edge",
            )?;
        }
        // Metadata may restate an otherwise valid answer. Never impose a hidden
        // lower cap than the answer budget (retain the previous minimum of 60).
        let mechanism = bounded(row, "mechanism", max_words.max(60))?;
        let key = mechanism
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        if let Some(first) = mechanisms.insert(key, id.clone()) {
            return Err(format!("duplicate candidate mechanism: candidate {id} duplicates candidate {first} after case/punctuation normalization. Replace candidate {id} with a substantively different workflow and rewrite BOTH its mechanism and its complete standalone answer. Change roles, steps, resource allocation, incentives or assessment, not just the title or wording. Preserve the original task constraints, source validity and required kind. Return all three complete candidates, not a partial patch."));
        }
        let answer = bounded(row, "answer", max_words)?;
        require(
            !candidates.iter().any(|c: &Candidate| c.answer == answer),
            "duplicate candidate answer",
        )?;
        candidates.push(Candidate {
            id,
            kind,
            mechanism,
            answer,
            orientations: refs,
            sources: support,
        });
    }
    let source_reviews = grounding::parse_source_reviews(&v, &allowed, &orientations)?;
    Ok(Candidates {
        orientations,
        candidates,
        source_reviews,
    })
}
impl Candidates {
    pub fn to_json(&self) -> Json {
        obj([
            (
                "source_reviews",
                arr(self.source_reviews.iter().map(SourceReview::to_json)),
            ),
            (
                "orientations",
                arr(self.orientations.iter().map(|o| {
                    obj([
                        ("id", text(&o.id)),
                        ("criterion", text(&o.criterion)),
                        ("consequence", text(&o.consequence)),
                        ("limit", text(&o.limit)),
                        ("source_ids", arr(o.sources.iter().map(text))),
                        ("evidence", arr(o.evidence.iter().map(Evidence::to_json))),
                    ])
                })),
            ),
            (
                "candidates",
                arr(self.candidates.iter().map(|c| {
                    obj([
                        ("id", text(&c.id)),
                        ("kind", text(&c.kind)),
                        ("mechanism", text(&c.mechanism)),
                        ("answer", text(&c.answer)),
                        ("orientation_ids", arr(c.orientations.iter().map(text))),
                        ("source_ids", arr(c.sources.iter().map(text))),
                    ])
                })),
            ),
        ])
    }
}
fn score(v: &Json, k: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let n = v.get(k)?.number()?;
    require(
        n.is_finite() && n >= lo && n <= hi,
        "choice score out of bounds",
    )?;
    Ok(n)
}
pub fn parse_evaluations(raw: &str, c: &Candidates) -> Result<Vec<Evaluation>, String> {
    require(raw.len() <= 50_000, "evaluation response too large")?;
    let v = json::parse(raw)?;
    require(
        c.candidates.len() == 3
            && c.candidates
                .iter()
                .map(|x| x.id.as_str())
                .eq(["A", "B", "C"]),
        "invalid candidate catalog",
    )?;
    let rows = match v.get("evaluations")? {
        Json::Array(a) => {
            require(
                a.len() == 3,
                &format!(
                    "expected evaluations for A, B and C; received {} array entries",
                    a.len()
                ),
            )?;
            let mut by_id = std::collections::BTreeMap::new();
            for row in a {
                let id = string(row, "id")?;
                require(
                    ["A", "B", "C"].contains(&id.as_str())
                        && by_id.insert(id, row.clone()).is_none(),
                    "unknown or duplicate evaluation ID",
                )?;
            }
            ["A", "B", "C"]
                .iter()
                .map(|id| {
                    by_id
                        .remove(*id)
                        .ok_or_else(|| format!("missing evaluation {id}"))
                })
                .collect::<Result<Vec<_>, String>>()?
        }
        Json::Object(map) => {
            require(
                map.len() == 3,
                &format!(
                    "expected evaluation keys A, B and C; received {} entries",
                    map.len()
                ),
            )?;
            let mut rows = vec![];
            for id in ["A", "B", "C"] {
                let mut row = map
                    .get(id)
                    .ok_or_else(|| format!("missing evaluation {id}"))?
                    .clone();
                if let Json::Object(fields) = &mut row {
                    if let Some(found) = fields.get("id") {
                        require(found.string()? == id, "evaluation key/ID mismatch")?;
                    }
                    fields.insert("id".into(), text(id));
                } else {
                    return Err("evaluation entry must be an object".into());
                }
                rows.push(row);
            }
            rows
        }
        _ => return Err("evaluations must be a keyed object or complete legacy array".into()),
    };
    let mut out = vec![];
    for row in &rows {
        let id = string(row, "id")?;
        let a = row.get("alignment")?.array()?;
        require(
            a.len() == c.orientations.len(),
            "one alignment per orientation required",
        )?;
        let alignment = a
            .iter()
            .map(|v| {
                let n = v.number()?;
                require(
                    n.is_finite() && (-2.0..=2.0).contains(&n),
                    "alignment out of bounds",
                )?;
                Ok(n)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let violations = strings(row, "violations")?;
        require(
            violations.len() <= 12 && violations.iter().all(|x| x.len() <= 500),
            "oversized violation list",
        )?;
        out.push(Evaluation {
            id,
            utility: score(row, "utility", 0.0, 4.0)?,
            feasibility: score(row, "feasibility", 0.0, 4.0)?,
            originality: score(row, "originality", 0.0, 4.0)?,
            alignment,
            violations,
            reason: bounded(row, "reason", 100)?,
        });
    }
    Ok(out)
}
/// A quality floor and hard exclusions apply before memory-weighted selection.
pub fn select(e: &[Evaluation], influence: Influence) -> Result<usize, String> {
    let eligible =
        |e: &&Evaluation| e.violations.is_empty() && e.utility >= 2.0 && e.feasibility >= 2.0;
    let best = e
        .iter()
        .filter(eligible)
        .map(Evaluation::quality)
        .fold(f64::NEG_INFINITY, f64::max);
    require(
        best.is_finite(),
        "no candidate satisfies task constraints and quality floor",
    )?;
    let mut chosen = None;
    let mut top = f64::NEG_INFINITY;
    for (i, e) in e.iter().enumerate() {
        if !eligible(&e) || e.quality() < best - 0.75 {
            continue;
        }
        let alignment = if e.alignment.is_empty() {
            0.0
        } else {
            e.alignment.iter().sum::<f64>() / e.alignment.len() as f64
        };
        let s = e.quality() + influence.weight() * alignment;
        if s > top {
            top = s;
            chosen = Some(i);
        }
    }
    chosen.ok_or_else(|| "no eligible candidate".into())
}
pub fn generation_system(base: &str, max_words: usize) -> String {
    format!("{base}\n{GENERATE}\nEach candidate answer must have at most {max_words} whitespace-separated words. The selection schema replaces any other requested output envelope; task constraints apply to candidate answers.")
}
/// Attach a request-specific catalog rather than a misleading fixed oriented B.
pub fn generation_system_for(base: &str, input: &Json, max_words: usize) -> Result<String, String> {
    let (ids, edges) = sources(input)?;
    let contract = if ids.is_empty() {
        "THIS REQUEST HAS NO DELIVERED EPISODES OR DISPOSITIONS. Return source_reviews: [] and orientations: []. Candidate kinds MUST be A=direct, B=exploratory, C=exploratory. Every candidate has orientation_ids: [] and source_ids: []. B must not have kind oriented. Do not invent an orientation or source to justify that label."
    } else if edges.is_empty() {
        "THIS REQUEST HAS SOURCES BUT NO ASSOCIATION EDGES. A.kind=direct; C.kind=exploratory. B.kind=oriented only if you generate at least one supported orientation; otherwise B.kind=exploratory and orientation_ids=[]. If B is oriented, cite at least one generated orientation and all its source IDs. No candidate may have kind associative."
    } else {
        "THIS REQUEST HAS SOURCES AND ASSOCIATION EDGES. A.kind=direct. B.kind=oriented only if you generate at least one supported orientation; otherwise B.kind=exploratory and orientation_ids=[]. C.kind=associative only when its mechanism uses a relevant delivered edge and cites both endpoints; otherwise C.kind=exploratory."
    };
    Ok(format!(
        "{}\nREQUEST-SPECIFIC CANDIDATE CONTRACT:\n{contract}",
        generation_system(base, max_words)
    ))
}
pub fn evaluation_input(input: &Json, c: &Candidates) -> Json {
    let entry = || {
        obj([
            ("utility", Json::Null),
            ("feasibility", Json::Null),
            ("originality", Json::Null),
            ("alignment", arr(c.orientations.iter().map(|_| Json::Null))),
            ("violations", arr([])),
            ("reason", text("")),
        ])
    };
    obj([
        ("request", input.clone()),
        ("generation", c.to_json()),
        (
            "response_contract",
            obj([
                ("candidate_ids", arr([text("A"), text("B"), text("C")])),
                (
                    "orientation_ids",
                    arr(c.orientations.iter().map(|o| text(&o.id))),
                ),
                ("alignment_length", num(c.orientations.len() as u32)),
                (
                    "orientation_reviews_template",
                    Json::Object(
                        c.orientations
                            .iter()
                            .map(|o| {
                                (
                                    o.id.clone(),
                                    obj([
                                        ("premise", Json::Null),
                                        ("lesson", Json::Null),
                                        ("scope", Json::Null),
                                        ("limit_polarity", Json::Null),
                                        ("reason", text("")),
                                    ]),
                                )
                            })
                            .collect(),
                    ),
                ),
                (
                    "activation_review_template",
                    obj([("verdict", Json::Null), ("reason", text(""))]),
                ),
                (
                    "evaluations_template",
                    obj([("A", entry()), ("B", entry()), ("C", entry())]),
                ),
            ]),
        ),
    ])
}
pub fn evaluation_json(e: &[Evaluation]) -> Json {
    obj([(
        "evaluations",
        arr(e.iter().map(|e| {
            obj([
                ("id", text(&e.id)),
                ("utility", num(e.utility)),
                ("feasibility", num(e.feasibility)),
                ("originality", num(e.originality)),
                ("alignment", arr(e.alignment.iter().map(|n| num(*n)))),
                ("violations", arr(e.violations.iter().map(text))),
                ("reason", text(&e.reason)),
            ])
        })),
    )])
}
#[derive(Clone, Debug)]
pub struct ChoiceResult {
    pub answer: String,
    pub audit: Json,
}
pub fn run(
    input: &Json,
    base: &str,
    options: &ChoiceOptions,
    mut call: impl FnMut(&str, &str, &str, u32) -> Result<String, String>,
) -> Result<ChoiceResult, String> {
    options.validate()?;
    let generated = call(
        "candidates",
        &generation_system_for(base, input, options.max_words)?,
        &input.encode(),
        (options.max_words as u32 * 7 + 1900).min(6500),
    )?;
    let c = parse_candidates(&generated, input, options.max_words)?;
    assess(input, &c, options, call)
}
// Shared by live selection and frozen-pool reevaluation.
pub fn finish_choice(
    c: &Candidates,
    judgment: &Judgment,
    options: &ChoiceOptions,
) -> Result<ChoiceResult, String> {
    options.validate()?;
    let e = &judgment.effective;
    let selected = select(e, options.influence)?;
    let sweep = arr([Influence::None, Influence::Moderate, Influence::Strong]
        .iter()
        .map(|w| {
            obj([
                ("weight", num(w.weight())),
                (
                    "selected",
                    text(&c.candidates[select(&e, *w).expect("same eligibility")].id),
                ),
            ])
        }));
    Ok(ChoiceResult {
        answer: c.candidates[selected].answer.clone(),
        audit: obj([
            ("version", text(VERSION)),
            ("options", options.to_json()),
            ("generation", c.to_json()),
            ("evaluation", evaluation_json(&judgment.raw)),
            ("orientation_reviews", judgment.reviews_json()),
            ("activation_review", judgment.activation.clone()),
            ("effective_evaluation", evaluation_json(e)),
            ("selected", text(&c.candidates[selected].id)),
            ("weight_sweep", sweep),
        ]),
    })
}
