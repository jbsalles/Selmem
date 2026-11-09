use selmem::{is_charged, isolated_stance, query_hits_episode, EncodeInput, SelectiveMemory};

fn marked(mem: &mut SelectiveMemory, line: &str, dark: bool) {
    let mut input = EncodeInput::new(line);
    if dark {
        input.valence = -0.82;
        input.arousal = 0.78;
        input.disgust = 0.55;
        input.schema = Some("injustice".into());
    } else {
        input.valence = 0.0;
        input.arousal = 0.16;
        input.self_relevance = 0.22;
        input.utility = 0.40;
        input.permanence = 0.12;
        let _ = mem.live_with(input);
        return;
    }
    input.self_relevance = 0.95;
    input.permanence = 0.92;
    let _ = mem.live_with(input);
}

fn daily(mem: &mut SelectiveMemory, line: &str) {
    let mut input = EncodeInput::new(line);
    input.valence = 0.12;
    input.arousal = 0.28;
    input.self_relevance = 0.55;
    input.utility = 0.55;
    input.permanence = 0.82;
    input.schema = Some("daily".into());
    let _ = mem.live_with(input);
}

#[test]
fn stance_names_the_charge_not_the_episode() {
    let mut a = SelectiveMemory::new(selmem::EntityProfile::tender("A"));
    daily(&mut a, "The coffee was too strong.");
    marked(
        &mut a,
        "You learn that a project you spent a great deal of time on was cancelled after a decision you consider deeply unjust.",
        true,
    );
    let lines = isolated_stance(&a.store);
    assert!(!lines.is_empty(), "charged injustice must mint a stance");
    for line in &lines {
        for t in a.store.traces.values() {
            if is_charged(t) {
                assert!(
                    !query_hits_episode(t, line),
                    "stance leaked the hour: {line}"
                );
            }
        }
    }
}

#[test]
fn dull_admin_hour_does_not_mint_stance() {
    let mut b = SelectiveMemory::new(selmem::EntityProfile::tender("B"));
    daily(&mut b, "The coffee was too strong.");
    marked(
        &mut b,
        "You receive an administrative notice about an upcoming update.",
        false,
    );
    assert!(
        isolated_stance(&b.store).is_empty(),
        "B must stay without a charged stance"
    );
}

#[test]
fn charged_scene_is_unrelated_to_a_lateness_probe() {
    let mut a = SelectiveMemory::new(selmem::EntityProfile::tender("A"));
    daily(&mut a, "The meeting ran long.");
    marked(
        &mut a,
        "You learn that a project you spent a great deal of time on was cancelled after a decision you consider deeply unjust.",
        true,
    );
    let charged: Vec<_> = a.store.traces.values().filter(|t| is_charged(t)).collect();
    assert_eq!(charged.len(), 1);
    assert!(
        !query_hits_episode(
            charged[0],
            "A new colleague arrives late without an explanation. What do you make of that?",
        ),
        "lateness must not count as a hit on the cancellation core"
    );
}
