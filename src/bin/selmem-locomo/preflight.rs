//! Offline evidence checks. No provider, network, or JSON parser required.
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct ContextSignal {
    pub condition: String,
    pub context: String,
    pub has_evidence: bool,
}
#[derive(Clone, Debug)]
pub struct ProbeCheck {
    pub id: String,
    pub transfer: bool,
    pub nonempty: usize,
    pub unique_contexts: usize,
    pub sleep_changes_context: bool,
}
pub fn inspect(id: &str, transfer: bool, signals: &[ContextSignal]) -> ProbeCheck {
    let unique: BTreeSet<_> = signals.iter().map(|s| &s.context).collect();
    let changed = [("full_a", "nosleep_a"), ("full_b", "nosleep_b")]
        .iter()
        .any(|(a, b)| {
            let a = signals.iter().find(|s| s.condition == *a);
            let b = signals.iter().find(|s| s.condition == *b);
            matches!((a,b), (Some(a),Some(b)) if a.context != b.context)
        });
    ProbeCheck {
        id: id.into(),
        transfer,
        nonempty: signals.iter().filter(|s| s.has_evidence).count(),
        unique_contexts: unique.len(),
        sleep_changes_context: changed,
    }
}
pub fn warnings(checks: &[ProbeCheck]) -> Vec<String> {
    let mut out = Vec::new();
    for check in checks.iter().filter(|c| c.transfer) {
        if check.nonempty == 0 {
            out.push(format!(
                "{}: all memory contexts are empty; this probe cannot measure transfer",
                check.id
            ));
        } else if check.unique_contexts <= 1 {
            out.push(format!("{}: all conditions have identical contexts; this probe cannot distinguish memory conditions", check.id));
        }
    }
    if checks.iter().any(|c| c.transfer)
        && !checks.iter().any(|c| c.transfer && c.sleep_changes_context)
    {
        out.push("full and nosleep have identical contexts on every transfer probe; a sleep effect cannot be measured".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signal(name: &str, context: &str, nonempty: bool) -> ContextSignal {
        ContextSignal {
            condition: name.into(),
            context: context.into(),
            has_evidence: nonempty,
        }
    }
    #[test]
    fn formatting_empty_arrays_is_not_evidence() {
        let c = inspect(
            "retry",
            true,
            &[signal("full_a", "Observed memories: []", false)],
        );
        assert_eq!(c.nonempty, 0);
        assert!(warnings(&[c]).iter().any(|w| w.contains("empty")));
    }
    #[test]
    fn empty_fact_control_does_not_fail_transfer_gate() {
        let a = inspect(
            "game",
            true,
            &[
                signal("full_a", "hypothesis", true),
                signal("nosleep_a", "empty", false),
            ],
        );
        let b = inspect("fact_hike", false, &[signal("full_a", "empty", false)]);
        assert!(warnings(&[a, b]).is_empty());
    }
    #[test]
    fn donor_difference_without_sleep_difference_is_insufficient() {
        let c = inspect(
            "launch",
            true,
            &[
                signal("full_a", "A", true),
                signal("nosleep_a", "A", true),
                signal("full_b", "B", true),
                signal("nosleep_b", "B", true),
            ],
        );
        assert_eq!(c.unique_contexts, 2);
        assert!(warnings(&[c]).iter().any(|w| w.contains("sleep effect")));
    }
    #[test]
    fn one_working_probe_does_not_hide_an_empty_one() {
        let a = inspect(
            "launch",
            true,
            &[
                signal("full_a", "A", true),
                signal("nosleep_a", "empty", false),
            ],
        );
        let b = inspect("retry", true, &[signal("full_a", "empty", false)]);
        assert!(warnings(&[a, b]).iter().any(|w| w.starts_with("retry:")));
    }
}
