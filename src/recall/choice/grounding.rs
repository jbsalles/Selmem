//! Grounding in the delivered, possibly reconstructed packet; no archive access.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Evidence {
    pub source_id: String,
    pub quote: String,
}
impl Evidence {
    pub fn to_json(&self) -> Json {
        obj([
            ("source_id", text(&self.source_id)),
            ("quote", text(&self.quote)),
        ])
    }
}
#[derive(Clone, Debug)]
pub struct SourceReview {
    pub source_id: String,
    pub decision: String,
    pub reason: String,
    pub orientations: Vec<String>,
}
impl SourceReview {
    pub fn to_json(&self) -> Json {
        obj([
            ("source_id", text(&self.source_id)),
            ("decision", text(&self.decision)),
            ("reason", text(&self.reason)),
            ("orientation_ids", arr(self.orientations.iter().map(text))),
        ])
    }
}
fn normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn source_texts(input: &Json) -> Result<BTreeMap<String, String>, String> {
    let memory = input.get("memory")?;
    let mut texts = BTreeMap::new();
    for (field, id, body) in [
        ("episodes", "trace_id", "narrative"),
        ("dispositions", "axiom_id", "statement"),
    ] {
        for source in memory.get(field)?.array()? {
            texts.insert(string(source, id)?, string(source, body)?);
        }
    }
    Ok(texts)
}
pub(super) fn parse_evidence(
    row: &Json,
    input: &Json,
    sources: &[String],
) -> Result<Vec<Evidence>, String> {
    let texts = source_texts(input)?;
    let rows = row.get("evidence")?.array()?;
    require(
        !rows.is_empty() && rows.len() <= 8,
        "one evidence excerpt per supporting source required",
    )?;
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    for row in rows {
        let id = string(row, "source_id")?;
        require(
            sources.contains(&id) && seen.insert(id.clone()),
            "unknown or duplicate evidence source",
        )?;
        let quote = bounded(row, "quote", 100)?;
        // Whitespace may differ in a JSON response, words and punctuation may not.
        // Exact excerpts prove presence, not entailment: the evaluator checks meaning.
        let body = texts.get(&id).ok_or("evidence source absent")?;
        require(
            normalized(&quote).chars().count() >= 8
                && normalized(body).contains(&normalized(&quote)),
            "evidence quote must be an exact excerpt of its delivered source",
        )?;
        out.push(Evidence {
            source_id: id,
            quote,
        });
    }
    require(
        seen.len() == sources.len(),
        "every supporting source needs an evidence excerpt",
    )?;
    Ok(out)
}
pub(super) fn parse_source_reviews(
    v: &Json,
    allowed: &BTreeSet<String>,
    orientations: &[Orientation],
) -> Result<Vec<SourceReview>, String> {
    let rows = v.get("source_reviews")?.array()?;
    require(
        rows.len() == allowed.len(),
        "review every delivered episode and disposition exactly once",
    )?;
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    for row in rows {
        let id = string(row, "source_id")?;
        require(
            allowed.contains(&id) && seen.insert(id.clone()),
            "unknown or duplicate source review",
        )?;
        let decision = string(row, "decision")?;
        require(
            ["used", "irrelevant", "uncertain", "capacity"].contains(&decision.as_str()),
            "invalid source decision",
        )?;
        let refs = strings(row, "orientation_ids")?;
        let expected = orientations
            .iter()
            .filter(|o| o.sources.contains(&id))
            .map(|o| o.id.clone())
            .collect::<BTreeSet<_>>();
        require(
            refs.len() == expected.len()
                && refs.iter().collect::<BTreeSet<_>>().len() == refs.len()
                && refs.iter().all(|id| expected.contains(id)),
            "source review must list its actual orientations",
        )?;
        require(
            (decision == "used") == !refs.is_empty(),
            "used source must support an orientation; abstentions must not",
        )?;
        if decision == "capacity" {
            require(
                orientations.len() == 3,
                "capacity abstention requires three orientations",
            )?;
        }
        out.push(SourceReview {
            source_id: id,
            decision,
            reason: bounded(row, "reason", 50)?,
            orientations: refs,
        });
    }
    Ok(out)
}
#[derive(Clone, Debug)]
pub struct OrientationReview {
    pub id: String,
    pub verdict: String,
    pub reason: String,
    pub premise: String,
    pub lesson: String,
    pub scope: String,
    pub limit_polarity: String,
}
#[derive(Clone, Debug)]
pub struct Judgment {
    pub raw: Vec<Evaluation>,
    pub effective: Vec<Evaluation>,
    pub reviews: Vec<OrientationReview>,
    pub activation: Json,
}
impl Judgment {
    pub fn reviews_json(&self) -> Json {
        Json::Object(
            self.reviews
                .iter()
                .map(|r| {
                    (
                        r.id.clone(),
                        obj([
                            ("verdict", text(&r.verdict)),
                            ("reason", text(&r.reason)),
                            ("premise", text(&r.premise)),
                            ("lesson", text(&r.lesson)),
                            ("scope", text(&r.scope)),
                            ("limit_polarity", text(&r.limit_polarity)),
                        ]),
                    )
                })
                .collect(),
        )
    }
}
/// Require explicit semantic reviews, then gate the model's fallible scores in Rust.
pub fn parse_judgment(raw: &str, c: &Candidates) -> Result<Judgment, String> {
    let evaluations = parse_evaluations(raw, c)?;
    let root = json::parse(raw)?;
    let rows = root.get("orientation_reviews")?.object()?;
    require(
        rows.len() == c.orientations.len(),
        "review every orientation exactly once",
    )?;
    let mut reviews = vec![];
    let mut blocked = BTreeSet::new();
    for o in &c.orientations {
        let row = rows.get(&o.id).ok_or("missing orientation review")?;
        let premise = string(row, "premise")?;
        let lesson = string(row, "lesson")?;
        let scope = string(row, "scope")?;
        let limit_polarity = string(row, "limit_polarity")?;
        require(
            ["faithful", "contradicted", "uncertain"].contains(&premise.as_str()),
            "invalid premise review",
        )?;
        require(
            ["plausible", "unsupported", "uncertain"].contains(&lesson.as_str()),
            "invalid lesson review",
        )?;
        require(
            ["applicable", "inapplicable", "uncertain"].contains(&scope.as_str()),
            "invalid scope review",
        )?;
        require(
            ["exception", "requirement", "unclear"].contains(&limit_polarity.as_str()),
            "invalid limit polarity",
        )?;
        let verdict = if premise == "contradicted" {
            "unsupported"
        } else if premise == "faithful"
            && lesson == "plausible"
            && scope == "applicable"
            && limit_polarity != "unclear"
        {
            "supported"
        } else {
            "uncertain"
        }
        .to_string();
        if verdict != "supported" {
            blocked.insert(o.id.clone());
        }
        reviews.push(OrientationReview {
            id: o.id.clone(),
            verdict,
            premise,
            lesson,
            scope,
            limit_polarity,
            reason: bounded(row, "reason", 70)?,
        });
    }
    let activation = root.get("activation_review")?;
    let verdict = string(activation, "verdict")?;
    require(
        ["adequate", "missed_opportunity", "uncertain"].contains(&verdict.as_str()),
        "invalid activation verdict",
    )?;
    let reason = bounded(activation, "reason", 70)?;
    let activation = obj([("verdict", text(verdict)), ("reason", text(reason))]);
    let mut effective = evaluations.clone();
    for (e, candidate) in effective.iter_mut().zip(&c.candidates) {
        for (index, orientation) in c.orientations.iter().enumerate() {
            if blocked.contains(&orientation.id) {
                e.alignment[index] = 0.0;
            }
        }
        // Only an explicit contradicted historical premise excludes reliance.
        for id in candidate.orientations.iter().filter(|id| {
            reviews
                .iter()
                .any(|r| &r.id == *id && r.premise == "contradicted")
        }) {
            e.violations.push(format!("unverified_orientation:{id}"));
        }
    }
    Ok(Judgment {
        raw: evaluations,
        effective,
        reviews,
        activation,
    })
}

#[cfg(test)]
mod tests;
