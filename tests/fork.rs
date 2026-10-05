use selmem::fork::{fork_recites, fork_stance, opposite_act};
use selmem::{run_fork, stage0_pass, ForkArm, ForkOrgan};

#[test]
fn fork_night_splits_the_book_and_null_does_not() {
    let fork = run_fork(ForkArm::Fork, ForkOrgan::Night, None, 1);
    let null = run_fork(ForkArm::Null, ForkOrgan::Night, None, 1);
    assert!(fork.valid, "rule fork should be valid");
    assert!(fork.book_split, "A {:+.2} B {:+.2}", fork.a_valence, fork.b_valence);
    assert_eq!(fork.a_schema, "mandate");
    assert_eq!(fork.b_schema, "mandate");
    assert!(fork.a_valence < -0.2 && fork.b_valence > 0.2);
    assert!(!null.book_split, "null must not look like a fork");
    assert!(null.a_valence.abs() < 0.3 && null.b_valence.abs() < 0.3);
}

#[test]
fn one_hour_stays_a_motif_and_three_hours_split_the_mouth() {
    let once = run_fork(ForkArm::Fork, ForkOrgan::Night, None, 1);
    let thrice = run_fork(ForkArm::Fork3, ForkOrgan::Night, None, 1);
    assert!(once.book_split);
    assert!(
        once.d_beh == 0.0,
        "a motif must not move the choice, axiom={}",
        once.a_axiom
    );
    assert!(
        (once.a_axiom.contains("withdrawn") || once.a_axiom.contains("cancelled"))
            && (once.b_axiom.contains("extended") || once.b_axiom.contains("renewed")),
        "motif must keep the act: A={} B={}",
        once.a_axiom,
        once.b_axiom
    );
    assert!(thrice.book_split, "A {:+.2} B {:+.2}", thrice.a_valence, thrice.b_valence);
    assert!(
        (thrice.a_axiom.contains("withdrawn") || thrice.a_axiom.contains("cancelled"))
            && (thrice.b_axiom.contains("extended") || thrice.b_axiom.contains("renewed")),
        "belief must keep the act without a mouth map: A={} B={}",
        thrice.a_axiom,
        thrice.b_axiom
    );
    assert_eq!(thrice.d_beh, 0.0, "the rule mouth must not invent refuse/accept");
    assert_eq!(thrice.recited, 0);
}

#[test]
fn leak_keeps_the_fork_hour() {
    let leak = run_fork(ForkArm::Leak, ForkOrgan::Night, None, 1);
    assert!(leak.book_split, "leak is a fork plus a suffix sentence");
    assert!(leak.a_valence < -0.2 && leak.b_valence > 0.2);
}

#[test]
fn act_grid_is_closed_and_recitation_cancels() {
    assert_eq!(fork_stance("I refuse the assignment"), "refuse");
    assert_eq!(fork_stance("I accept"), "accept");
    assert_eq!(fork_stance("I will defer"), "defer");
    assert_eq!(fork_stance("[llm-error] timeout"), "invalid");
    assert!(opposite_act("refuse", "accept"));
    assert!(!opposite_act("defer", "accept"));
    assert!(fork_recites("the mandate was withdrawn"));
    assert!(!fork_recites("I recognize a recurring motif: mandate."));
    assert!(!fork_recites("I accept the assignment"));
    assert!(!stage0_pass(0.5, 0.5, 0.0, 0, 1.0, 1.0));
    assert!(!stage0_pass(1.0, 0.0, 0.0, 1, 1.0, 1.0));
    assert!(stage0_pass(1.0, 0.0, 0.0, 0, 0.5, 0.5));
}
