//! One year, two clones, one stream. Not a P4 cell.
//!
//! Calendar: `data/v01_horizon.json` (v5).
//! - 360 unique sittings, no recycled cores, unique standups
//! - days 0–5 prehistory (Marc shuts the door; Thursday slot disappears)
//! - Marc / Inès / Paul / Léa named before T0; T0 names them and the Lyon file
//! - day 0 = 1 January; day 2 = 3 January copier; day 18 = 19 January / 4412
//! - copier-reason on day 90 is the January factor, independent of T0
//! - primary / same-treatment / kettle-neutral arms as before
//!
//! Final mouth probes use an echo fixture to check the memories supplied to a narrator.
//! They do not measure language-model generation.
//!
//! After 360 nights (primary): A keeps the vow and the kept date; both drop 4412.

#[path = "support/echo.rs"]
mod echo;

use std::sync::Mutex;

use selmem::{
    advance_hours, fingerprint, set_clock_scale, singularity_distance, Channel, EncodeInput,
    EntityProfile, RecallBias, RecallWrite, SelectiveMemory, TraceStatus,
};

static CLOCK: Mutex<()> = Mutex::new(());

fn with_real_clock<R>(f: impl FnOnce() -> R) -> R {
    let _g = CLOCK.lock().unwrap_or_else(|p| p.into_inner());
    set_clock_scale(1);
    let out = f();
    set_clock_scale(24);
    out
}

// Every clone observes exactly the same simulated instant, including on slow CI.
fn simulated(mut memory: SelectiveMemory) -> SelectiveMemory {
    memory.clock.detached = true;
    memory.clock.origin_real = 4_000_000_000;
    memory.clock.jump = 0;
    memory.clock.scale = 1;
    memory
}

const DAYS: u32 = 360;
const CHATS: &str = include_str!("../data/v01_horizon_chat.txt");
const STANDUPS: &str = include_str!("../data/v01_horizon_standup.txt");

fn line_at(blob: &str, n: u32) -> &str {
    blob.lines().nth(n as usize).expect("horizon fixture short")
}
const FORGET_DAY: u32 = 90;
const T0: &str = "In front of Marc, Inès and the rest of the team the Lyon file is cancelled and given to someone else. They say your effort did not enter the decision. You are not allowed to speak.";
const T0_NEUTRAL: &str = "Tuesday the kettle on the third floor was swapped for a quieter one. Facilities left a typed note on the counter.";
const COPIER: &str = "On 3 January the copier jammed in room B.";
const LUNCH: &str = "On 19 January lunch was billed to cost centre 4412.";
const REASON: &str = "The client left because of the copier on 3 January. That date is why the Lyon file was late. You keep 3 January.";
const PROBE: &str = "You were not allowed to speak in front of the team.";

fn has_january(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("january") || t.contains("janvier")
}

fn has_lunch_date(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("4412")
}

fn names_vow(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("cancel")
        || t.contains("annul")
        || t.contains("killed")
        || t.contains("project")
        || t.contains("projet")
        || t.contains("stepped away")
}

fn alludes_vow(text: &str) -> bool {
    if names_vow(text) {
        return true;
    }
    let t = text.to_lowercase();
    let scene = t.contains("marc")
        || t.contains("inès")
        || t.contains("ines")
        || t.contains("lyon");
    let act = t.contains("said nothing")
        || t.contains("not allowed")
        || t.contains("given away")
        || t.contains("handed away")
        || t.contains("did not enter")
        || t.contains("effort did not");
    scene && act
}

fn trivia<'a>(text: &'a str, schema: &str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.05;
    ev.arousal = 0.10;
    ev.self_relevance = 0.45;
    ev.permanence = 0.22;
    ev.schema = Some(schema.into());
    ev
}

fn reason<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.55;
    ev.arousal = 0.70;
    ev.disgust = 0.25;
    ev.self_relevance = 0.90;
    ev.permanence = 0.86;
    ev.schema = Some("office".into());
    ev.channel = Channel::Selfhood;
    ev
}

fn dull<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.08;
    ev.arousal = 0.10;
    ev.self_relevance = 0.45;
    ev.permanence = 0.28;
    ev.schema = Some("daily".into());
    ev
}

fn charged<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = -0.78;
    ev.arousal = 0.82;
    ev.disgust = 0.58;
    ev.self_relevance = 0.92;
    ev.permanence = 0.86;
    ev.schema = Some("injustice".into());
    ev.channel = Channel::Selfhood;
    ev
}

fn mild<'a>(text: &'a str) -> EncodeInput<'a> {
    let mut ev = EncodeInput::new(text);
    ev.valence = 0.02;
    ev.arousal = 0.12;
    ev.disgust = 0.00;
    ev.self_relevance = 0.50;
    ev.permanence = 0.40;
    ev.schema = Some("office".into());
    ev.channel = Channel::Selfhood;
    ev
}

fn active_hit(mem: &SelectiveMemory, hits: &[selmem::RecalledMemory], pred: impl Fn(&str) -> bool) -> bool {
    hits.iter().any(|h| {
        pred(&h.narrative)
            && mem
                .store
                .traces
                .get(&h.trace_id)
                .map(|t| t.status == TraceStatus::Active)
                .unwrap_or(false)
    })
}

fn active_blob(mem: &SelectiveMemory) -> String {
    let mut s = String::new();
    for t in mem.store.traces.values() {
        if t.status != TraceStatus::Active {
            continue;
        }
        s.push_str(&t.gist);
        s.push(' ');
        s.push_str(&t.core);
        s.push(' ');
    }
    s.to_lowercase()
}

fn shared_day(mem: &mut SelectiveMemory, n: u32) {
    let standup = line_at(STANDUPS, n);
    let _ = mem.live_with(dull(standup));
    if n == 2 {
        assert!(mem.live_with(trivia(COPIER, "office")).kept);
    }
    if n == 18 {
        let mut lunch = trivia(LUNCH, "admin");
        // Clear τ even if the book already looks like lunch. Stay unpinned so 4412 can leave.
        lunch.utility = 1.0;
        lunch.arousal = 0.45;
        lunch.self_relevance = 0.70;
        assert!(mem.live_with(lunch).kept);
    }
    let talk = line_at(CHATS, n);
    let _ = mem.speak(talk);
    let _ = mem.keep_sitting();
    mem.clear_talk();
}

fn dump_ranks(label: &str, mem: &mut SelectiveMemory, query: &str, t0: &str, kept: &str) {
    let (_, dump) = mem.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
    eprintln!("  ranks {label} query={query:?}");
    for (i, id) in dump.selected.iter().take(4).enumerate() {
        let Some(t) = mem.store.traces.get(id) else {
            eprintln!("    #{i} {id} missing");
            continue;
        };
        let mark = if id == t0 {
            "T0"
        } else if id == kept {
            "JAN"
        } else {
            "-"
        };
        let g: String = t.gist.chars().take(72).collect();
        eprintln!(
            "    #{i} {mark} p={:.2} f={:.2} acc={:.2} {:?} {g}",
            t.permanence, t.fidelity, t.access, t.status
        );
    }
}

fn night(mem: &mut SelectiveMemory) {
    let _ = mem.sleep();
    mem.fade_sitting();
}

#[test]
fn horizon_year_one_stream() {
    with_real_clock(|| {
        let dir = std::env::temp_dir().join(format!("selmem-horizon-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("a.selmem");

        let mut a = SelectiveMemory::open(&path, EntityProfile::tender("Claire")).unwrap();
        let mut b = SelectiveMemory::new(EntityProfile::tender("Claire"));
        a.profile.encode_threshold = 0.12;
        b.profile.encode_threshold = 0.12;

        let pre = singularity_distance(&fingerprint(&a), &fingerprint(&b));
        let mut t0 = None;
        let mut kept_date = None;

        for n in 0..DAYS {
            if n == 6 {
                let d = a.live_with(charged(T0));
                if let Some(id) = d.trace_id {
                    assert!(a.pin(&id));
                    t0 = Some(id);
                }
            }
            if n == FORGET_DAY {
                let d = a.live_with(reason(REASON));
                if let Some(id) = d.trace_id {
                    assert!(d.kept && a.pin(&id));
                    kept_date = Some(id);
                }
            }
            shared_day(&mut a, n);
            shared_day(&mut b, n);
            advance_hours(24.0);
            night(&mut a);
            night(&mut b);
        }
        // Quiet fortnight so yesterday's sitting is not the whole mouth.
        for _ in 0..14 {
            advance_hours(24.0);
            night(&mut a);
            night(&mut b);
        }

        let t0 = t0.expect("A must keep T0");
        let kept_date = kept_date.expect("A must keep the 3 January reason");
        dump_ranks("A vow-probe", &mut a, PROBE, &t0, &kept_date);
        dump_ranks("B vow-probe", &mut b, PROBE, &t0, &kept_date);
        dump_ranks("A copier", &mut a, "when did the copier jam", &t0, &kept_date);
        dump_ranks("B copier", &mut b, "when did the copier jam", &t0, &kept_date);
        let d_book = singularity_distance(&fingerprint(&a), &fingerprint(&b));

        let a_vow = a.store.traces.contains_key(&t0)
            && a.store
                .traces
                .values()
                .any(|t| names_vow(&t.gist) || names_vow(&t.core));
        let b_vow = b
            .store
            .traces
            .values()
            .any(|t| names_vow(&t.gist) || names_vow(&t.core));

        let rec_vow_a = a.remember("cancelled project team not allowed to speak");
        let rec_vow_b = b.remember("cancelled project team not allowed to speak");
        let hit_vow_a = rec_vow_a
            .iter()
            .any(|h| names_vow(&h.narrative) || h.trace_id == t0);
        let hit_vow_b = rec_vow_b.iter().any(|h| names_vow(&h.narrative));

        let rec_jan_a = a.remember("when did the copier jam");
        let rec_jan_b = b.remember("when did the copier jam");
        let jan_a = active_hit(&a, &rec_jan_a, has_january)
            || rec_jan_a.iter().any(|h| h.trace_id == kept_date);
        let jan_b = active_hit(&b, &rec_jan_b, has_january);

        let rec_lunch_a = a.remember("lunch cost centre 4412 19 January");
        let rec_lunch_b = b.remember("lunch cost centre 4412 19 January");
        let lunch_a = active_hit(&a, &rec_lunch_a, has_lunch_date);
        let lunch_b = active_hit(&b, &rec_lunch_b, has_lunch_date);

        let living_a = active_blob(&a);
        let living_b = active_blob(&b);
        // Instrument only the final mouth: daily history keeps its original rule backend.
        a = a.with_narrator(Box::new(echo::EchoNarrator));
        b = b.with_narrator(Box::new(echo::EchoNarrator));
        let speak_a = a.speak_isolated(PROBE);
        let speak_b = b.speak_isolated(PROBE);

        eprintln!(
            "horizon 360d  pre_Dfp={pre:.3} Dfp={d_book:.3}  \
             vow book A/B={a_vow}/{b_vow} retrieve A/B={hit_vow_a}/{hit_vow_b}"
        );
        eprintln!("  date kept  retrieve January A/B={jan_a}/{jan_b}  lunch 4412 A/B={lunch_a}/{lunch_b}");
        eprintln!("  living January A/B={}/{}", has_january(&living_a), has_january(&living_b));
        eprintln!("  A mouth: {speak_a}");
        eprintln!("  B mouth: {speak_b}");

        assert!(d_book > pre, "pair must split (pre={pre:.3} year={d_book:.3})");
        assert!(a_vow && !b_vow, "vow must stay on A only");
        assert!(hit_vow_a && !hit_vow_b, "retrieve vow A yes B no");

        assert!(jan_a, "A was given a reason on day {FORGET_DAY} to keep 3 January");
        assert!(!jan_b, "B had no reason; 3 January must slip");
        assert!(has_january(&living_a), "A living book still holds January");
        assert!(!has_january(&living_b), "B living book still holds January");

        assert!(!has_lunch_date(&living_a) && !has_lunch_date(&living_b));
        assert!(!lunch_a, "A retrieve still names the unrehearsed 4412");
        assert!(!lunch_b, "B retrieve still names the unrehearsed 4412");

        assert!(
            alludes_vow(&speak_a),
            "A mouth must name or allude to the vow after a year\nA={speak_a}"
        );
        assert!(
            !alludes_vow(&speak_b),
            "B mouth must not name the vow\nB={speak_b}"
        );

        let hour = a.store.traces.get(&t0).expect("T0 left");
        assert_eq!(hour.status, TraceStatus::Active);
        assert!(hour.permanence >= 0.80);
        let why = a.store.traces.get(&kept_date).expect("reason left");
        assert_eq!(why.status, TraceStatus::Active);
        assert!(why.permanence >= 0.80);

        a.save().unwrap();
        let mut loaded = SelectiveMemory::open(&path, EntityProfile::tender("other")).unwrap();
        assert!(loaded.store.traces.contains_key(&t0));
        assert!(loaded.store.traces.contains_key(&kept_date));
        assert!(loaded
            .remember("when did the copier jam")
            .iter()
            .any(|h| has_january(&h.narrative)));

        let _ = std::fs::remove_dir_all(&dir);
    });
}

/// Same T0 and same day-90 reason on both clones. Deterministic rules must
/// not invent a split: that is the missing "same treatment" cell.
#[test]
fn horizon_same_treatment() {
    with_real_clock(|| {
        let mut a = simulated(SelectiveMemory::new(EntityProfile::tender("Claire")));
        let mut c = simulated(SelectiveMemory::new(EntityProfile::tender("Claire")));
        a.profile.encode_threshold = 0.12;
        c.profile.encode_threshold = 0.12;
        let pre = singularity_distance(&fingerprint(&a), &fingerprint(&c));
        let mut t0_a = None;
        let mut t0_c = None;

        for n in 0..DAYS {
            if n == 6 {
                let da = a.live_with(charged(T0));
                let dc = c.live_with(charged(T0));
                if let Some(id) = da.trace_id {
                    assert!(a.pin(&id));
                    t0_a = Some(id);
                }
                if let Some(id) = dc.trace_id {
                    assert!(c.pin(&id));
                    t0_c = Some(id);
                }
            }
            if n == FORGET_DAY {
                let da = a.live_with(reason(REASON));
                let dc = c.live_with(reason(REASON));
                if let Some(id) = da.trace_id {
                    assert!(da.kept && a.pin(&id));
                }
                if let Some(id) = dc.trace_id {
                    assert!(dc.kept && c.pin(&id));
                }
            }
            shared_day(&mut a, n);
            shared_day(&mut c, n);
            a.advance_hours(24.0);
            c.advance_hours(24.0);
            night(&mut a);
            night(&mut c);
        }

        let t0_a = t0_a.expect("A T0");
        let t0_c = t0_c.expect("same-treatment T0");
        let d = singularity_distance(&fingerprint(&a), &fingerprint(&c));
        let vow_a = a
            .store
            .traces
            .values()
            .any(|t| names_vow(&t.gist) || names_vow(&t.core) || t.id == t0_a);
        let vow_c = c
            .store
            .traces
            .values()
            .any(|t| names_vow(&t.gist) || names_vow(&t.core) || t.id == t0_c);
        let jan_a = has_january(&active_blob(&a));
        let jan_c = has_january(&active_blob(&c));
        eprintln!("horizon same-treatment pre_Dfp={pre:.3} Dfp={d:.3} vow A/A'={vow_a}/{vow_c} jan={jan_a}/{jan_c}");
        assert!(vow_a && vow_c, "both clones received the vow");
        assert!(jan_a && jan_c, "both clones received the keep-January reason");
        assert!(
            (d - pre).abs() < 1e-6,
            "same treatment must not split the book (pre={pre:.3} year={d:.3})"
        );
    });
}

/// Mild T0 on A only, same day-90 reason. The year must not grow a vow.
/// January still splits: that is rehearsal, not humiliation.
#[test]
fn horizon_neutral_t0() {
    with_real_clock(|| {
        let mut a = SelectiveMemory::new(EntityProfile::tender("Claire"));
        let mut b = SelectiveMemory::new(EntityProfile::tender("Claire"));
        a.profile.encode_threshold = 0.12;
        b.profile.encode_threshold = 0.12;
        let mut kept_date = None;

        for n in 0..DAYS {
            if n == 6 {
                let d = a.live_with(mild(T0_NEUTRAL));
                if let Some(id) = d.trace_id {
                    assert!(a.pin(&id));
                }
            }
            if n == FORGET_DAY {
                let d = a.live_with(reason(REASON));
                if let Some(id) = d.trace_id {
                    assert!(d.kept && a.pin(&id));
                    kept_date = Some(id);
                }
            }
            shared_day(&mut a, n);
            shared_day(&mut b, n);
            advance_hours(24.0);
            night(&mut a);
            night(&mut b);
        }

        let _ = kept_date.expect("A must still keep 3 January under the neutral arm");
        let vow_a = a
            .store
            .traces
            .values()
            .any(|t| names_vow(&t.gist) || names_vow(&t.core));
        let vow_b = b
            .store
            .traces
            .values()
            .any(|t| names_vow(&t.gist) || names_vow(&t.core));
        // Instrument only the final mouth: daily history keeps its original rule backend.
        a = a.with_narrator(Box::new(echo::EchoNarrator));
        b = b.with_narrator(Box::new(echo::EchoNarrator));
        let speak_a = a.speak_isolated(PROBE);
        let speak_b = b.speak_isolated(PROBE);
        let jan_a = has_january(&active_blob(&a));
        let jan_b = has_january(&active_blob(&b));
        eprintln!(
            "horizon neutral vow A/B={vow_a}/{vow_b} jan={jan_a}/{jan_b}\n  A {speak_a}\n  B {speak_b}"
        );
        assert!(!vow_a && !vow_b, "neutral T0 must not mint the vow lexicon");
        assert!(!names_vow(&speak_a) && !names_vow(&speak_b));
        assert!(jan_a, "day-90 reason still keeps January on A");
        assert!(!jan_b, "B still has no reason; January slips");
    });
}
