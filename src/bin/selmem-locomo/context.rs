//! Pack whole evidence items; never slice serialized JSON or a caveat.
use super::json::{num, obj, text, Json};

pub struct Packed {
    pub text: String,
    pub audit: Json,
}
fn render(memories: &[Json], axioms: &[Json], disposition: &str) -> String {
    format!(
        "Observed memories:\n{}\nFallible interpretations:\n{}\nReading disposition:\n{}\n",
        Json::Array(memories.to_vec()).encode(),
        Json::Array(axioms.to_vec()).encode(),
        text(disposition).encode()
    )
}
pub fn build(readout: &Json, budget: usize) -> Result<Packed, String> {
    let memories = readout.get("memories")?.array()?;
    let axioms = readout.get("axioms")?.array()?;
    // ReadingProfile::render puts the same hypotheses in an [axioms: ...]
    // suffix. Supply them once, with their full evidence and reservations.
    let profile = readout.get("reading_profile")?.string()?;
    let disposition = profile.split(" [axioms:").next().unwrap_or(profile).trim();
    let mut selected = [Vec::new(), Vec::new()];
    let mut kept_disposition = "";
    if render(&[], &[], "").chars().count() > budget {
        return Err("context budget cannot fit the empty context headers".into());
    }
    // Alternate evidence and interpretations, preserving each section's rank.
    // Unused capacity is shared instead of imposing fixed section quotas.
    for index in 0..memories.len().max(axioms.len()) {
        for (section, items) in [memories, axioms].into_iter().enumerate() {
            if let Some(item) = items.get(index) {
                item.string()?;
                selected[section].push(item.clone());
                if render(&selected[0], &selected[1], "").chars().count() > budget {
                    selected[section].pop();
                }
            }
        }
    }
    if render(&selected[0], &selected[1], disposition)
        .chars()
        .count()
        <= budget
    {
        kept_disposition = disposition;
    }
    let output = render(&selected[0], &selected[1], kept_disposition);
    Ok(Packed {
        audit: obj([
            ("characters", num(output.chars().count() as f64)),
            ("budget", num(budget as f64)),
            ("memories_included", num(selected[0].len() as f64)),
            (
                "memories_omitted",
                num((memories.len() - selected[0].len()) as f64),
            ),
            ("interpretations_included", num(selected[1].len() as f64)),
            (
                "interpretations_omitted",
                num((axioms.len() - selected[1].len()) as f64),
            ),
            (
                "disposition_included",
                Json::Bool(!kept_disposition.is_empty()),
            ),
            (
                "disposition_omitted",
                Json::Bool(!disposition.is_empty() && kept_disposition.is_empty()),
            ),
        ]),
        text: output,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn readout(mem: Vec<Json>, axioms: Vec<Json>, profile: &str) -> Json {
        obj([
            ("memories", Json::Array(mem)),
            ("axioms", Json::Array(axioms)),
            ("reading_profile", text(profile)),
        ])
    }
    #[test]
    fn full_evidence_caveat_and_unicode_survive_shared_budget() {
        let hypothesis = format!(
            "subject=Caroline; evidence={}; This is an analogy, not Claire's own experience.",
            "é\"\\".repeat(400)
        );
        let r = readout(
            vec![text("short memory")],
            vec![text(&hypothesis)],
            &format!("[mood: ] [axioms: {hypothesis}]"),
        );
        let packed = build(&r, 6000).unwrap();
        let block = packed
            .text
            .split("Fallible interpretations:\n")
            .nth(1)
            .unwrap()
            .split("\nReading disposition:")
            .next()
            .unwrap();
        assert_eq!(
            super::super::json::parse(block).unwrap(),
            Json::Array(vec![text(&hypothesis)])
        );
        assert_eq!(packed.text.matches("This is an analogy").count(), 1);
        assert!(packed.text.chars().count() <= 6000);
    }
    #[test]
    fn oversized_items_are_omitted_whole_and_small_later_items_fit() {
        let r = readout(
            vec![text("x".repeat(5000)), text("usable")],
            vec![text("complete caveat")],
            "[mood: neutral]",
        );
        let packed = build(&r, 250).unwrap();
        assert!(packed.text.contains("usable"));
        assert!(packed.text.contains("complete caveat"));
        assert!(!packed.text.contains("xxxxx"));
        assert_eq!(
            packed
                .audit
                .get("memories_omitted")
                .unwrap()
                .number()
                .unwrap(),
            1.0
        );
        assert!(packed.text.chars().count() <= 250);
        assert!(build(&r, 5).is_err());
    }
}
