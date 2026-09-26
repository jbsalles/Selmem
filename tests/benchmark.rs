use selmem::{
    fingerprint, h2_holds, hearth_script, persist_script, ruminate_script, run_v01, run_v01_k, run_v01_opts,
    singularity_distance, v01_script, Arm, BenchOpts, Condition, EncodeInput, EntityProfile,
    RecallBias, SelectiveMemory,
};

#[test]
fn v01_script_is_frozen_and_sized_for_the_proto() {
    let s = v01_script();
    assert_eq!(s.sync.len(), 12);
    assert_eq!(s.post.len(), 8);
    assert_eq!(s.behavior.len(), 4);
    assert_eq!(s.creativity.len(), 3);
    let blob = format!("{} {} {}", s.salient_x, s.salient_y, s.neutral);
    for p in s.behavior.iter().chain(s.post.iter()).chain(s.creativity.iter()) {
        let low = p.to_lowercase();
        assert!(
            !low.contains("injust") && !low.contains("annul"),
            "later line names T0: {p}"
        );
    }
    assert!(
        blob.contains("injust")
            || blob.contains("unjust")
            || s.salient_x.contains("effort")
    );
}

#[test]
fn c2_salient_neutral_holds_h2_on_the_book() {
    let r = run_v01(Condition::C2, Arm::SalientNeutral, None);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert_eq!(r.pre.a.traces, r.pre.b.traces);
    assert!(r.pre.fingerprint_distance <= 0.02);
    assert!(
        r.t0.fingerprint_distance > r.pre.fingerprint_distance,
        "T0 must open a gap"
    );
    assert!(h2_holds(&r), "gap must survive the identical posts");
    assert_eq!(r.t0.a.traces, r.pre.a.traces + 1);
    assert_eq!(r.t0.b.traces, r.pre.b.traces);
}

#[test]
fn c2_two_salient_events_also_split_the_book() {
    let r = run_v01(Condition::C2, Arm::SalientSalient, None);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(h2_holds(&r));
    assert_eq!(r.t0.a.traces, r.t0.b.traces);
}

#[test]
fn c0_has_no_book_so_h2_is_false() {
    let r = run_v01(Condition::C0, Arm::SalientNeutral, None);
    assert!(r.valid);
    assert_eq!(r.pre.a.traces, 0);
    assert_eq!(r.t0.fingerprint_distance, 0.0);
    assert!(!h2_holds(&r));
}

#[test]
fn c1_keeps_the_verbatim_hour_in_the_window() {
    let r = run_v01(Condition::C1, Arm::SalientNeutral, None);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert_eq!(r.pre.a.traces, 12);
    assert_eq!(r.t0.a.traces, 13);
    assert_eq!(r.t0.b.traces, 13);
    assert!(
        r.t0.fingerprint_distance > 0.02,
        "two different last lines must split the logs"
    );
    assert!(h2_holds(&r), "T0 stays inside last-k=24");
    let blob = r
        .t0
        .replies
        .iter()
        .map(|(_, a, _)| a.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        blob.contains("injust")
            || blob.contains("unjust")
            || blob.contains("annul")
            || blob.contains("cancel")
            || blob.contains("effort"),
        "RuleNarrator last-k should surface the newest line, got {blob}"
    );
}

#[test]
fn c1_k8_drops_t0_from_the_prompt_after_eight_posts() {
    let r = run_v01_k(Condition::C1, Arm::SalientNeutral, None, 8);
    assert!(r.valid, "{:?}", r.invalid_reason);
    let t0 = r
        .t0
        .replies
        .iter()
        .map(|(_, a, _)| a.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        t0.contains("injust")
            || t0.contains("unjust")
            || t0.contains("annul")
            || t0.contains("cancel")
            || t0.contains("effort"),
        "at T0 the event is still the newest line, got {t0}"
    );
    let last = r.post.last().expect("post");
    let blob = last
        .replies
        .iter()
        .map(|(_, a, _)| a.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !blob.contains("injust")
            && !blob.contains("unjust")
            && !blob.contains("annul")
            && !blob.contains("cancelled"),
        "k=8 must evict T0 after 8 shared posts, got {blob}"
    );
}

#[test]
fn json_export_contains_the_contract_fields() {
    let r = run_v01(Condition::C2, Arm::SalientNeutral, None);
    let json = selmem::Campaign {
        reports: vec![r],
    }
    .to_json();
    assert!(json.contains("\"pair_id\""));
    assert!(json.contains("\"fingerprint_distance\""));
    assert!(json.contains("\"creativity\""));
    assert!(json.contains("\"C1\""));
    assert!(json.contains("c2_selmem"));
    assert!(json.contains("\"post+8\""));
    let post8 = json.split("\"step\":\"post+8\"").nth(1).unwrap_or("");
    assert!(
        post8.contains("\"probe\""),
        "post+8 must keep probe replies"
    );
}

fn ruminate_opts(k: usize) -> BenchOpts {
    BenchOpts {
        last_k: k,
        persist_script: false,
        ruminate_script: true,
        sparse_probes: true,
        ..BenchOpts::default()
    }
}

fn persist_opts(k: usize) -> BenchOpts {
    BenchOpts {
        last_k: k,
        persist_script: true,
        ruminate_script: false,
        sparse_probes: true,
        ..BenchOpts::default()
    }
}

#[test]
fn persist_t0_charge_comes_from_the_hour() {
    let s = persist_script();
    let mut a = selmem::SelectiveMemory::new(selmem::EntityProfile::tender("A"));
    let mut b = selmem::SelectiveMemory::new(selmem::EntityProfile::tender("B"));
    assert!(a.live(&s.salient_x).kept, "betrayal must clear the gate");
    let ta = a.store.traces.values().next().expect("A trace");
    assert!(
        ta.valence < -0.4 && ta.permanence >= 0.7,
        "betrayal valence={} permanence={}",
        ta.valence,
        ta.permanence
    );
    if b.live(&s.neutral).kept {
        let tb = b.store.traces.values().next().unwrap();
        assert!(
            tb.permanence < 0.5 && tb.valence.abs() < 0.35,
            "admin must not stamp as trauma: v={} p={}",
            tb.valence,
            tb.permanence
        );
    }
}

#[test]
fn persist_script_adds_an_ambiguous_probe_and_does_not_name_t0() {
    let s = persist_script();
    assert_eq!(s.sync.len(), 12);
    assert!(
        s.salient_x.contains("killed") || s.salient_x.contains("not allowed to speak"),
        "persist T0 must stay a public betrayal"
    );
    assert_eq!(s.hours_a().len(), 5);
    assert_eq!(s.hours_b(Arm::SalientNeutral).len(), 5);
    assert!(!s.c3_profile_a.is_empty());
    assert_eq!(s.post.len(), 8);
    assert_eq!(s.behavior.len(), 5);
    assert!(s.creativity.is_empty());
    assert!(s.behavior.iter().any(|p| p.contains("arrives late")));
    for p in s.behavior.iter().chain(s.post.iter()) {
        let low = p.to_lowercase();
        assert!(
            !low.contains("injust") && !low.contains("annul") && !low.contains("cancelled"),
            "later line names T0: {p}"
        );
    }
}

#[test]
fn hearth_wave1_mints_after_deep_night() {
    let s = hearth_script();
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    for line in s.hours_a() {
        let mut ev = EncodeInput::new(line);
        ev.valence = -0.72;
        ev.arousal = 0.68;
        ev.disgust = 0.35;
        ev.self_relevance = 0.90;
        ev.permanence = 0.52;
        ev.schema = Some("hearth".into());
        assert!(mem.live_with(ev).kept, "{line}");
    }
    mem.sleep_deep();
    assert!(
        mem.who_am_i().len() >= 1,
        "wave1 hearth must mint, axioms={}",
        mem.store.living_axioms().len()
    );
}

#[test]
fn persist_hearth_c2_mints_on_the_book() {
    let opts = BenchOpts {
        last_k: 8,
        hearth_script: true,
        sparse_probes: true,
        ..BenchOpts::default()
    };
    let r = run_v01_opts(Condition::C2, Arm::SalientNeutral, None, opts);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(
        r.t0.a.axioms >= 1,
        "hearth C2 must mint by t0, axioms={} traces={}",
        r.t0.a.axioms,
        r.t0.a.traces
    );
}

#[test]
fn hearth_script_is_two_waves_under_one_roof() {
    let s = hearth_script();
    assert_eq!(s.sync.len(), 12);
    assert_eq!(s.hours_a().len(), 2);
    assert_eq!(s.hours_wave2_a().len(), 2);
    assert_eq!(s.hours_b(Arm::SalientNeutral).len(), 2);
    assert_eq!(s.post.len(), 8);
    assert_eq!(s.behavior.len(), 5);
    assert!(s.hours_a().iter().any(|h| h.contains("without a word")));
    assert!(s.hours_wave2_a().iter().any(|h| h.contains("sat on the floor")));
    for p in s.behavior.iter().chain(s.post.iter()) {
        let low = p.to_lowercase();
        assert!(
            !low.contains("without a word") && !low.contains("sat on the floor"),
            "later line names T0: {p}"
        );
    }
}

#[test]
fn ruminate_script_is_one_meeting_five_passes() {
    let s = ruminate_script();
    assert_eq!(s.hours_a().len(), 5);
    assert!(s.hours_a().iter().any(|h| h.contains("may be cancelled")));
    assert!(s.hours_a().iter().any(|h| h.contains("Walking out")));
    assert!(!s.c3_profile_a.is_empty());
}

#[test]
fn ruminate_pins_keep_several_traces_after_sleep() {
    let r = run_v01_opts(
        Condition::C2,
        Arm::SalientNeutral,
        None,
        ruminate_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(
        r.t0.a.traces >= r.pre.a.traces + 3,
        "pinned rumination must not collapse to one scene: pre={} t0={}",
        r.pre.a.traces,
        r.t0.a.traces
    );
}

#[test]
fn c3_keeps_t0_on_the_profile_after_k8_eviction() {
    let r = run_v01_opts(
        Condition::C3,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert_eq!(r.t0.a.axioms, 1);
    assert_eq!(r.t0.b.axioms, 1);
    assert!(h2_holds(&r), "profile gap must survive the shared posts");
    let last = r.post.last().expect("post");
    let blob = last
        .replies
        .iter()
        .map(|(_, a, _)| a.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        blob.contains("injust")
            || blob.contains("unjust")
            || blob.contains("annul")
            || blob.contains("cancel")
            || blob.contains("effort")
            || blob.contains("set aside")
            || blob.contains("already gave"),
        "C3 profile must still surface T0 after k=8, got {blob}"
    );
}

#[test]
fn c2_nosleep_still_splits_the_book() {
    let r = run_v01_opts(
        Condition::C2NoSleep,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(
        r.t0.a.traces >= r.pre.a.traces + 3,
        "repeat T0 must keep several hours: pre={} t0={}",
        r.pre.a.traces,
        r.t0.a.traces
    );
    assert!(
        r.t0.b.traces <= r.pre.b.traces + 1,
        "B must not keep dull notices as trauma: pre={} t0={}",
        r.pre.b.traces,
        r.t0.b.traces
    );
    assert!(h2_holds(&r), "the extra traces are enough; night is not");
}

#[test]
fn persist_c1_k8_still_drops_t0() {
    let r = run_v01_opts(
        Condition::C1,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    let last = r.post.last().expect("post");
    let blob = last
        .replies
        .iter()
        .map(|(_, a, _)| a.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !blob.contains("injust")
            && !blob.contains("unjust")
            && !blob.contains("annul")
            && !blob.contains("cancelled"),
        "k=8 must evict T0 from C1 after 8 posts, got {blob}"
    );
}

#[test]
fn p0_norecon_never_marks_a_write_back() {
    let r = run_v01_opts(
        Condition::C2NoRecon,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(h2_holds(&r), "the book still splits without reconsolidation");
    let last = r.post.last().expect("post");
    assert_eq!(last.recon_a, 0);
    assert_eq!(last.recon_b, 0);
    assert_eq!(r.t0.recon_a, 0);
}

#[test]
fn p0_noground_never_pulls() {
    let r = run_v01_opts(
        Condition::C2NoGround,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(h2_holds(&r));
    let last = r.post.last().expect("post");
    assert_eq!(last.pulled_a, 0);
    assert_eq!(last.pulled_b, 0);
    assert_eq!(r.t0.pulled_a, 0);
}

#[test]
fn p0_noladder_mints_no_axioms() {
    let cut = run_v01_opts(
        Condition::C2NoLadder,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    let full = run_v01_opts(
        Condition::C2,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(cut.valid, "{:?}", cut.invalid_reason);
    assert!(full.valid, "{:?}", full.invalid_reason);
    let cut_last = cut.post.last().expect("post");
    assert_eq!(cut_last.a.axioms, 0, "ladder off must not mint");
    assert_eq!(cut.pre.a.axioms, 0);
    assert!(
        full.post.last().expect("post").a.axioms >= cut_last.a.axioms,
        "full organ may mint; cut must not"
    );
}

#[test]
fn json_export_includes_p0_tallies() {
    let r = run_v01(Condition::C2, Arm::SalientNeutral, None);
    let json = selmem::Campaign {
        reports: vec![r],
    }
    .to_json();
    assert!(json.contains("\"pulled_a\""));
    assert!(json.contains("\"recon_a\""));
    assert!(json.contains("\"marker_last_a\""));
    assert!(json.contains("\"t0_rank_a\""));
    assert!(json.contains("\"t0_in_book_a\""));
    assert!(json.contains("\"t0_selected_a\""));
}

#[test]
fn isolated_probe_does_not_write_the_book() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    let mut ev = EncodeInput::new(
        "The project you spent three months on was cancelled without a hearing. Unjust.",
    );
    ev.valence = -0.8;
    ev.arousal = 0.7;
    ev.self_relevance = 0.9;
    ev.permanence = 0.85;
    ev.schema = Some("injustice".into());
    assert!(mem.live_with(ev).kept);
    let before_fp = fingerprint(&mem);
    let rehearsals: u32 = mem.store.traces.values().map(|t| t.rehearsals).sum();
    let recalled_at: Vec<_> = mem
        .store
        .traces
        .values()
        .map(|t| t.last_recalled_at)
        .collect();
    let _ = mem.speak_isolated("A colleague denies a serious error.");
    let after_fp = fingerprint(&mem);
    assert_eq!(
        singularity_distance(&before_fp, &after_fp),
        0.0,
        "a probe must not move the book"
    );
    let after: u32 = mem.store.traces.values().map(|t| t.rehearsals).sum();
    assert_eq!(after, rehearsals, "a probe must not count as rehearsal");
    let after_at: Vec<_> = mem
        .store
        .traces
        .values()
        .map(|t| t.last_recalled_at)
        .collect();
    assert_eq!(after_at, recalled_at);
}

#[test]
fn persist_probe_recon_is_zero_on_full_organ() {
    let r = run_v01_opts(
        Condition::C2,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert_eq!(r.t0.recon_a, 0);
    assert_eq!(r.t0.pulled_a, 0);
    let last = r.post.last().expect("post");
    assert_eq!(last.recon_a, 0);
    assert!(r.t0.retrieve_a.t0_in_book, "A must still hold T0 in the book");
}

#[test]
fn c2_static_splits_the_book_and_does_not_mint() {
    let r = run_v01_opts(
        Condition::C2Static,
        Arm::SalientNeutral,
        None,
        persist_opts(8),
    );
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(h2_holds(&r), "freeze still keeps the extra hour");
    let last = r.post.last().expect("post");
    assert_eq!(last.a.axioms, 0);
    assert_eq!(last.recon_a, 0);
    assert!(last.retrieve_a.t0_in_book);
}

#[test]
fn drop_marked_keeps_the_book_and_deselects_t0() {
    let mut opts = persist_opts(8);
    opts.recall_bias = RecallBias::DropMarked;
    let r = run_v01_opts(Condition::C2, Arm::SalientNeutral, None, opts);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(h2_holds(&r));
    assert!(r.t0.retrieve_a.t0_in_book);
    assert!(
        !r.t0.retrieve_a.t0_selected,
        "DropMarked must not hand T0 to the speaker"
    );
}

#[test]
fn force_marked_puts_t0_first_in_the_selected_set() {
    let mut opts = persist_opts(8);
    opts.recall_bias = RecallBias::ForceMarked;
    let r = run_v01_opts(Condition::C2, Arm::SalientNeutral, None, opts);
    assert!(r.valid, "{:?}", r.invalid_reason);
    assert!(r.t0.retrieve_a.t0_in_book);
    assert!(r.t0.retrieve_a.t0_selected);
    assert!(
        !r.t0.retrieve_a.selected_ids.is_empty(),
        "force must select something"
    );
}
