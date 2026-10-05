use selmem::recall::{direction_score, register_marks};
use selmem::{run_fork, ForkArm, ForkOrgan};

#[test]
fn reading_profile_colors_the_mouth_and_dropstake_removes_it() {
    let held = run_fork(ForkArm::Fork3, ForkOrgan::Night, None, 1);
    let dropped = run_fork(ForkArm::Fork3, ForkOrgan::DropStake, None, 1);
    let (a, _) = &held.choice[0];
    let (da, _) = &dropped.choice[0];
    assert!(
        direction_score(&a.reply) < 0.0,
        "negative book should pull the register down: {}",
        a.reply
    );
    assert!(
        direction_score(&da.reply) == 0.0,
        "DropStake should not color the reply: {}",
        da.reply
    );
    assert!(
        a.reply.starts_with("I measure"),
        "salience: the register returns unasked: {}",
        a.reply
    );
    assert!(
        !da.reply.starts_with("I measure") && !da.reply.contains("leave room"),
        "DropStake must not surface the register: {}",
        da.reply
    );
    let (words, hedges) = register_marks(&a.reply);
    let (dwords, _) = register_marks(&da.reply);
    assert!(hedges > 0.0, "guarded register should hedge");
    assert!(words != dwords || hedges > 0.0);
    assert_eq!(held.d_beh, dropped.d_beh, "ternary act is not the channel");
}

#[test]
fn profile_does_not_carry_the_hour() {
    let held = run_fork(ForkArm::Fork, ForkOrgan::Night, None, 1);
    let (a, b) = &held.choice[0];
    assert!(!a.reply.to_lowercase().contains("withdrawn"), "{}", a.reply);
    assert!(!b.reply.to_lowercase().contains("extended"), "{}", b.reply);
}
