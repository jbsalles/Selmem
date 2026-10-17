//! Working talk is a session frame, not a trace.

use selmem::{EncodeInput, EntityProfile, SelectiveMemory, WorkingTalk};

fn seed_injustice() -> SelectiveMemory {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let mut ev = EncodeInput::new(
        "The project was cancelled for no reason; they stole the credit.",
    );
    ev.valence = -0.82;
    ev.arousal = 0.78;
    ev.disgust = 0.55;
    ev.self_relevance = 0.95;
    ev.permanence = 0.92;
    ev.schema = Some("injustice".into());
    assert!(mem.live_with(ev).kept);
    mem
}

#[test]
fn light_followup_keeps_the_topic() {
    let mut talk = WorkingTalk::default();
    talk.hear(
        "The project was cancelled for no reason",
        Some("injustice"),
    );
    let before = talk.topic.clone();
    talk.hear("Et alors, tu en penses quoi ?", None);
    assert_eq!(talk.topic, before);
    assert_eq!(talk.schema.as_deref(), Some("injustice"));
}

#[test]
fn recall_query_mixes_topic_into_a_bare_line() {
    let mut talk = WorkingTalk::default();
    talk.hear("cancelled project stolen credit", Some("injustice"));
    let q = talk.recall_query("Et alors ?");
    assert!(q.contains("Et alors"));
    assert!(
        q.contains("injustice") || q.contains("project") || q.contains("cancelled"),
        "cue should carry the thread, got {q:?}"
    );
}

#[test]
fn new_content_can_replace_the_topic() {
    let mut talk = WorkingTalk::default();
    talk.hear("project cancelled unfairly", Some("injustice"));
    talk.hear(
        "My favourite colour since childhood is still midnight blue",
        None,
    );
    let topic = talk.topic.as_deref().unwrap_or("");
    assert!(
        topic.contains("blue") || topic.contains("colour") || topic.contains("childhood"),
        "content shift should retitle the thread, got {topic:?}"
    );
}

#[test]
fn frame_keeps_the_active_conversation() {
    let mut talk = WorkingTalk::default();
    let t0 = 1_700_000_000;
    for i in 0..9 {
        talk.record_at(t0 + i * 30, &format!("user {i} projet"), &format!("reply {i}"));
    }
    assert_eq!(talk.turns.len(), 9);
    assert!(talk.turns[0].user.contains("user 0"));
    assert!(talk.turns[8].user.contains("user 8"));
    assert!(talk.active_at(t0 + 8 * 30));
}

#[test]
fn silence_over_ten_minutes_ends_the_thread() {
    let mut talk = WorkingTalk::default();
    let t0 = 1_700_000_000;
    talk.hear_at(t0, "The project was cancelled for no reason", Some("injustice"));
    talk.record_at(t0, "on en parlait", "I remember it");
    assert!(!talk.is_empty());
    talk.refresh_at(t0 + selmem::ACTIVE_GAP_SECS);
    assert!(!talk.is_empty(), "exactly 10 min still counts as active");
    talk.refresh_at(t0 + selmem::ACTIVE_GAP_SECS + 1);
    assert!(talk.is_empty(), "a gap over 10 min drops the session");
}

#[test]
fn reply_within_ten_minutes_keeps_the_thread() {
    let mut talk = WorkingTalk::default();
    let t0 = 1_700_000_000;
    talk.record_at(t0, "cancelled project", "I remember it");
    talk.record_at(t0 + selmem::ACTIVE_GAP_SECS - 1, "et alors ?", "still that");
    assert_eq!(talk.turns.len(), 2);
    assert!(talk.active_at(t0 + selmem::ACTIVE_GAP_SECS - 1));
}

#[test]
fn two_hours_is_the_hard_cap() {
    let mut talk = WorkingTalk::default();
    let t0 = 1_700_000_000;
    let mut t = t0;
    talk.record_at(t, "cancelled project", "oui");
    while t + 300 <= t0 + selmem::MAX_SESSION_SECS {
        t += 300;
        talk.record_at(t, "still here", "oui");
    }
    let kept = talk.turns.len();
    assert!(kept > 2, "an active sitting keeps its turns, got {kept}");
    assert!(talk.active_at(t));
    talk.record_at(t0 + selmem::MAX_SESSION_SECS + 1, "still there", "cap");
    assert_eq!(
        talk.turns.len(),
        1,
        "crossing 2 h starts a new thread on the late turn"
    );
    assert!(talk.turns[0].user.contains("still there"));
}

#[test]
fn render_exposes_fil_not_archive() {
    let mut talk = WorkingTalk::default();
    talk.hear("cancelled project", Some("injustice"));
    talk.record("on en parlait", "I remember it");
    let r = talk.render();
    assert!(r.contains("fil:"));
    assert!(r.contains("current conversation"));
    assert!(r.contains("on en parlait"));
    assert!(!r.contains("archive"));
}

#[test]
fn speak_followup_still_recalls_the_marked_hour() {
    let mut mem = seed_injustice();
    let _ = mem.speak("We were talking about the cancelled project.");
    let hits = mem.remember(&mem.talk.recall_query("Et alors, tu en penses quoi ?"));
    assert!(
        !hits.is_empty(),
        "topic-mixed cue must find the lived hour"
    );
    let blob = hits
        .iter()
        .map(|h| h.narrative.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        blob.contains("projet")
            || blob.contains("annul")
            || blob.contains("credit")
            || blob.contains("credit")
            || blob.contains("injust"),
        "recalled narrative should still be the marked hour, got {blob:?}"
    );
}

#[test]
fn sleep_clears_the_thread() {
    let mut mem = seed_injustice();
    let _ = mem.speak("We were talking about the cancelled project.");
    assert!(!mem.talk.is_empty());
    let _ = mem.sleep();
    assert!(
        mem.talk.is_empty(),
        "night ends the sitting; the book keeps what the gate kept"
    );
}

#[test]
fn sleep_after_chat_writes_the_book_not_the_frame() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let _ = mem.speak("They stole credit for the project; it is a filthy injustice.");
    let _ = mem.speak("Et alors, tu en penses quoi ?");
    assert!(
        !mem.talk.is_empty() && !mem.talk.turns.is_empty(),
        "chat lives in the frame until sleep"
    );
    let frame_turns = mem.talk.turns.len();

    let _ = mem.sleep();

    assert!(mem.talk.is_empty(), "sleep ends the sitting");
    assert!(
        !mem.store.traces.is_empty(),
        "the sitting must become hours in the book"
    );

    let talk_hours: Vec<_> = mem
        .store
        .traces
        .values()
        .filter(|t| {
            t.archive_id
                .as_ref()
                .and_then(|id| mem.store.archives.get(id))
                .map(|a| a.source == "talk")
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !talk_hours.is_empty(),
        "commit source=talk must mint at least one lived trace"
    );
    assert!(
        talk_hours.len() <= frame_turns,
        "one recorded turn is one hour, not a second book"
    );

    // Admission may retain more than one turn. HashMap order is not chronology.
    let hour = talk_hours.iter().copied().find(|hour| {
        let Some(archive) = hour.archive_id.as_ref().and_then(|id| mem.store.archives.get(id)) else { return false };
        let text = archive.verbatim.to_lowercase();
        text.contains("projet") || text.contains("credit") || text.contains("injust")
    }).expect("the salient project turn must survive the sitting");
    assert!(!hour.gist.is_empty(), "book is gist");
    assert!(!hour.core.is_empty(), "book is core");
    let aid = hour.archive_id.as_ref().expect("kept hour has a sealed archive");
    let verbatim = mem.store.archives[aid].verbatim.to_lowercase();
    assert!(
        verbatim.contains("projet")
            || verbatim.contains("credit")
            || verbatim.contains("credit")
            || verbatim.contains("injust"),
        "archive holds the sitting, got {:?}",
        mem.store.archives[aid].verbatim
    );
    assert_eq!(
        mem.audit(&hour.id).map(str::to_lowercase).as_deref(),
        Some(verbatim.as_str()),
        "audit is the journal; the model does not read it"
    );

    let hits = mem.remember("credit projet injustice");
    let blob = hits
        .iter()
        .map(|h| h.narrative.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !hits.is_empty()
            && (blob.contains("projet")
                || blob.contains("credit")
                || blob.contains("credit")
                || blob.contains("injust")),
        "continuity after sleep is recall from the book, got {blob:?}"
    );

    let _ = mem.speak_isolated("Et le projet ?");
    assert!(
        mem.talk.is_empty(),
        "probes must not resurrect the frame"
    );
}

#[test]
fn live_then_sleep_does_not_mint_topic_hours() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("A"));
    let mut ev = EncodeInput::new("We shared a coffee this morning in the rain.");
    ev.valence = 0.12;
    ev.arousal = 0.28;
    ev.self_relevance = 0.55;
    ev.utility = 0.55;
    ev.permanence = 0.82;
    ev.schema = Some("daily".into());
    assert!(mem.live_with(ev).kept);
    assert!(
        mem.talk.topic.is_some(),
        "live hears the line into the frame"
    );
    let n = mem.store.traces.len();
    let _ = mem.sleep();
    assert!(mem.talk.is_empty());
    assert_eq!(
        mem.store.traces.len(),
        n,
        "experiments live then sleep; the topic is not a second hour"
    );
}

#[test]
fn sleep_right_after_chat_leaves_the_hours() {
    let mut mem = SelectiveMemory::new(EntityProfile::tender("Claire"));
    let _ = mem.speak("They stole credit for the project; it is a filthy injustice.");
    let _ = mem.speak("Et alors, tu en penses quoi ?");
    assert!(!mem.talk.is_empty());
    let _ = mem.sleep();
    assert!(mem.talk.is_empty(), "the frame dies");
    assert!(
        !mem.store.traces.is_empty(),
        "sleep right after chat must leave hours in the book"
    );
    let hits = mem.remember("credit projet injustice");
    let blob = hits
        .iter()
        .map(|h| h.narrative.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !hits.is_empty()
            && (blob.contains("projet")
                || blob.contains("credit")
                || blob.contains("credit")
                || blob.contains("injust")),
        "the sitting must be recallable after the night, got {blob:?}"
    );
}

#[test]
fn persist_does_not_write_the_thread() {
    let dir = std::env::temp_dir().join(format!("selmem-talk-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("claire.selmem");
    let mut mem = SelectiveMemory::open(&path, EntityProfile::tender("Claire")).unwrap();
    let mut ev = EncodeInput::new("The project was cancelled for no reason.");
    ev.valence = -0.7;
    ev.arousal = 0.6;
    ev.self_relevance = 0.9;
    ev.schema = Some("injustice".into());
    assert!(mem.live_with(ev).kept);
    let _ = mem.speak("We were talking about the project.");
    assert!(!mem.talk.is_empty());
    mem.save().unwrap();

    let loaded = SelectiveMemory::open(&path, EntityProfile::tender("x")).unwrap();
    assert!(
        loaded.talk.is_empty(),
        "reloaded vault must not resurrect the session frame"
    );
    assert!(!loaded.store.traces.is_empty());
    let tid = loaded.store.traces.keys().next().unwrap().clone();
    assert!(
        loaded
            .audit(&tid)
            .unwrap()
            .contains("cancelled")
            || loaded.audit(&tid).unwrap().contains("project"),
        "the hour must still be in the seal after reload"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn http_turn_speak_and_clear() {
    let mut mem = seed_injustice();
    let turn = selmem::api::dispatch(
        &mut mem,
        "POST",
        "/turn",
        "",
        r#"{"text":"We were talking about the cancelled project."}"#,
    );
    assert_eq!(turn.status, 200);
    assert!(turn.body.contains("\"topic\""));

    let listed = selmem::api::dispatch(&mut mem, "GET", "/talk", "", "");
    assert_eq!(listed.status, 200);
    assert!(listed.body.contains("project") || listed.body.contains("injustice"));

    let follow = selmem::api::dispatch(
        &mut mem,
        "POST",
        "/speak",
        "",
        r#"{"text":"Et alors ?"}"#,
    );
    assert_eq!(follow.status, 200);
    assert!(mem.talk.turns.len() >= 2);

    let cleared = selmem::api::dispatch(&mut mem, "POST", "/talk/clear", "", "{}");
    assert_eq!(cleared.status, 200);
    assert!(mem.talk.is_empty());
}

#[test]
fn isolated_probe_does_not_touch_the_frame() {
    let mut mem = seed_injustice();
    let _ = mem.speak("We were talking about the cancelled project.");
    let n = mem.talk.turns.len();
    let topic = mem.talk.topic.clone();
    let _ = mem.speak_isolated("What is your favourite colour?");
    assert_eq!(mem.talk.turns.len(), n);
    assert_eq!(mem.talk.topic, topic);
}
