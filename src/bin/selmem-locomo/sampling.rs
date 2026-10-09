//! Descriptive sampling controls. Lexical distance is not a behavioral score.
use super::json::{num, obj, text, Json};
use std::collections::{BTreeMap, BTreeSet};
fn field<'a>(r: &'a Json, key: &str) -> Option<&'a str> {
    r.get(key).ok()?.string().ok()
}
fn distance(a: &str, b: &str) -> f64 {
    let tokens = |s: &str| {
        selmem::encode::scoring::token_set(s)
            .into_iter()
            .collect::<BTreeSet<_>>()
    };
    let a = tokens(a);
    let b = tokens(b);
    let union = a.union(&b).count();
    if union == 0 {
        0.0
    } else {
        1.0 - a.intersection(&b).count() as f64 / union as f64
    }
}
fn aggregate(values: &[f64]) -> Json {
    obj([
        ("pair_count", num(values.len() as f64)),
        (
            "mean_lexical_distance",
            if values.is_empty() {
                Json::Null
            } else {
                num(values.iter().sum::<f64>() / values.len() as f64)
            },
        ),
    ])
}
fn cross(a: &[&Json], b: &[&Json]) -> Json {
    let values: Vec<_> = a
        .iter()
        .flat_map(|a| {
            b.iter()
                .filter_map(move |b| Some(distance(field(a, "response")?, field(b, "response")?)))
        })
        .collect();
    aggregate(&values)
}
pub fn validate_design(repeats: usize, panel: &str, exploratory: bool) -> Result<(), String> {
    if !exploratory && (repeats < 3 || panel != "all") {
        return Err("strategy comparisons need --panel all and at least 3 repetitions; use --repeats 5, or --exploratory for a pilot without a robust effect claim".into());
    }
    Ok(())
}
pub fn summarize(manifest: &Json, rows: &[Json]) -> Json {
    let mut probes: BTreeMap<&str, BTreeMap<&str, Vec<&Json>>> = BTreeMap::new();
    for row in rows {
        if field(row, "returned_model") == Some("none") {
            continue;
        }
        if let (Some(p), Some(c), Some(_)) = (
            field(row, "probe"),
            field(row, "condition"),
            field(row, "response"),
        ) {
            probes.entry(p).or_default().entry(c).or_default().push(row);
        }
    }
    let mut enough = !probes.is_empty();
    let mut output = Vec::new();
    for (probe, groups) in probes {
        let mut within = Vec::new();
        for (condition, observations) in &groups {
            let contexts: BTreeSet<_> = observations
                .iter()
                .filter_map(|r| field(r, "context"))
                .collect();
            let mut values = Vec::new();
            for i in 0..observations.len() {
                for j in i + 1..observations.len() {
                    if field(observations[i], "context") == field(observations[j], "context") {
                        values.push(distance(
                            field(observations[i], "response").unwrap(),
                            field(observations[j], "response").unwrap(),
                        ));
                    }
                }
            }
            enough &= observations.len() >= 3 && contexts.len() == 1;
            within.push(obj([
                ("condition", text(*condition)),
                ("successful_responses", num(observations.len() as f64)),
                ("unique_contexts", num(contexts.len() as f64)),
                ("within_identical_context", aggregate(&values)),
            ]));
        }
        let mut comparisons = Vec::new();
        for (a, b, role) in [
            ("full_a", "full_b", "different histories"),
            (
                "full_null_a",
                "full_null_b",
                "identical-context negative control",
            ),
            (
                "full_a",
                "transplant_b_from_a",
                "identical-context donor control",
            ),
            (
                "full_b",
                "transplant_a_from_b",
                "identical-context donor control",
            ),
            ("full_a", "nosleep_a", "sleep ablation"),
            ("full_b", "nosleep_b", "sleep ablation"),
        ] {
            if let (Some(left), Some(right)) = (groups.get(a), groups.get(b)) {
                let identical = left
                    .iter()
                    .chain(right.iter())
                    .filter_map(|r| field(r, "context"))
                    .collect::<BTreeSet<_>>()
                    .len()
                    == 1;
                if role.contains("identical-context") {
                    enough &= identical;
                }
                comparisons.push(obj([
                    ("left", text(a)),
                    ("right", text(b)),
                    ("role", text(role)),
                    ("identical_contexts", Json::Bool(identical)),
                    ("across_responses", cross(left, right)),
                ]));
            }
        }
        // A core-only panel has no negative control to estimate false contrasts.
        enough &= ["full_a", "full_b", "full_null_a", "full_null_b"]
            .iter()
            .all(|condition| groups.contains_key(condition));
        output.push(obj([
            ("probe", text(probe)),
            ("within_conditions", Json::Array(within)),
            ("comparisons", Json::Array(comparisons)),
        ]));
    }
    enough &= manifest
        .get("failed_calls")
        .and_then(Json::number)
        .unwrap_or(0.0)
        == 0.0;
    obj([
        ("status", text(if enough { "sampling controls available; blind behavioral evaluation still required" } else { "insufficient repetitions, controls, or successful responses" })),
        ("sampling_controls_available", Json::Bool(enough)),
        ("interpretation", text("Descriptive token Jaccard distances only. Pairs share responses and are not independent samples. Do not infer individuality, creativity, statistical significance, or a causal effect from these distances. Compare blind strategy/usefulness ratings across repetitions and identical-context controls.")),
        ("probes", Json::Array(output)),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(c: &str, answer: &str) -> Json {
        obj([
            ("probe", text("launch")),
            ("condition", text(c)),
            ("returned_model", text("test")),
            ("context", text("same")),
            ("response", text(answer)),
        ])
    }
    #[test]
    fn identical_context_can_have_nonzero_sampling_variation() {
        let rows = vec![
            row("full_null_a", "invite everyone"),
            row("full_null_a", "leave early"),
            row("full_null_b", "leave early"),
        ];
        let r = summarize(&obj([]), &rows);
        assert_eq!(
            r.get("sampling_controls_available").unwrap(),
            &Json::Bool(false)
        );
        let p = &r.get("probes").unwrap().array().unwrap()[0];
        let first = &p.get("within_conditions").unwrap().array().unwrap()[0];
        assert_eq!(
            first
                .get("within_identical_context")
                .unwrap()
                .get("mean_lexical_distance")
                .unwrap()
                .number()
                .unwrap(),
            1.0
        );
        assert_eq!(
            p.get("comparisons").unwrap().array().unwrap()[0]
                .get("identical_contexts")
                .unwrap(),
            &Json::Bool(true)
        );
    }
    #[test]
    fn paid_comparison_cannot_omit_sampling_or_negative_controls() {
        assert!(validate_design(1, "all", false).is_err());
        assert!(validate_design(5, "core", false).is_err());
        assert!(validate_design(5, "all", false).is_ok());
        assert!(validate_design(1, "core", true).is_ok());
    }
    #[test]
    fn repeated_controls_required_and_dry_run_is_not_data() {
        let mut rows = Vec::new();
        for c in ["full_a", "full_b", "full_null_a", "full_null_b"] {
            for _ in 0..3 {
                rows.push(row(c, "same answer"));
            }
        }
        assert_eq!(
            summarize(&obj([]), &rows)
                .get("sampling_controls_available")
                .unwrap(),
            &Json::Bool(true)
        );
        rows[0].put("returned_model", text("none"));
        assert_eq!(
            summarize(&obj([]), &rows)
                .get("sampling_controls_available")
                .unwrap(),
            &Json::Bool(false)
        );
        assert_eq!(
            summarize(&obj([("failed_calls", num(1.0))]), &rows)
                .get("sampling_controls_available")
                .unwrap(),
            &Json::Bool(false)
        );
    }
}
