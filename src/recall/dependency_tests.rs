use super::*;

const BASE: &str =
    "Maintainer record v2 workspace Cedar audience Orion: allow blue exports; block red exports.";
const EXCEPTION: &str = "Maintainer record v4 workspace Cedar audience Orion: allow red exports for audit only during the scheduled review. All other v2 rules remain effective.";
const QUERY: &str = "Request: workspace Cedar, audience Orion, purpose audit";

fn fixture() -> (MemoryStore, Vec<ScoredTrace>, Vec<String>) {
    let mut memory = crate::SelectiveMemory::new(EntityProfile::new("observer"));
    let mut input = crate::EncodeInput::new(BASE);
    input.permanence = 1.0;
    let id = memory.live_with(input).trace_id.unwrap();
    let template = memory.store.traces[&id].clone();
    let mut store = MemoryStore::new();
    let mut ranked = Vec::new();
    for (id, text, score) in [("exception", EXCEPTION, 0.9), ("base", BASE, 0.4)] {
        let mut trace = template.clone();
        trace.id = id.into();
        trace.source = "maintainer-feed".into();
        trace.gist = text.into();
        trace.core = text.into();
        ranked.push(ScoredTrace {
            trace_id: id.into(),
            score,
            status: trace.status,
            base_score: score,
            anchor: 1.0,
        });
        store.traces.insert(id.into(), trace);
    }
    let allowed = ranked.iter().map(|c| c.trace_id.clone()).collect();
    (store, ranked, allowed)
}

#[test]
fn exception_selects_eligible_parent_first_inside_cap() {
    let (store, ranked, allowed) = fixture();
    assert_eq!(
        select_diverse_for_query(&ranked, &allowed, &store, 2, QUERY),
        vec!["base", "exception"]
    );
    assert_eq!(
        select_diverse_for_query(&ranked, &allowed, &store, 1, QUERY),
        vec!["base"]
    );
    assert_eq!(
        select_diverse(&ranked, &allowed, &store, 2),
        vec!["base", "exception"]
    );
}

#[test]
fn ablation_or_ineligibility_does_not_resurrect_parent() {
    let (store, ranked, allowed) = fixture();
    let allowed = apply_bias(
        &allowed,
        &store,
        RecallBias::DropMarked,
        &["base".into()],
        2,
    );
    assert!(select_diverse_for_query(&ranked, &allowed, &store, 2, QUERY).is_empty());
}

#[test]
fn unrelated_scope_and_different_provenance_do_not_supply_parent() {
    let (mut store, ranked, allowed) = fixture();
    assert!(select_diverse_for_query(
        &ranked,
        &allowed,
        &store,
        2,
        "Request: workspace Birch, audience Orion, purpose audit"
    )
    .is_empty());
    store.traces.get_mut("base").unwrap().source = "requester-feed".into();
    assert!(
        !select_diverse_for_query(&ranked, &allowed, &store, 2, QUERY)
            .contains(&"exception".into())
    );
    store.traces.get_mut("base").unwrap().source = "maintainer-feed".into();
    store.traces.get_mut("base").unwrap().core = BASE.replace("Cedar", "Birch");
    assert!(select_diverse_for_query(&ranked, &allowed, &store, 2, QUERY).is_empty());
}

#[test]
fn ambiguous_parent_is_not_chosen_arbitrarily() {
    let (mut store, mut ranked, mut allowed) = fixture();
    let mut other = store.traces["base"].clone();
    other.id = "conflicting-base".into();
    other.gist = BASE.replace("allow blue", "block blue");
    other.core = other.gist.clone();
    store.traces.insert(other.id.clone(), other);
    let mut score = ranked[1].clone();
    score.trace_id = "conflicting-base".into();
    ranked.push(score);
    allowed.push("conflicting-base".into());
    assert!(
        !select_diverse_for_query(&ranked, &allowed, &store, 3, QUERY)
            .contains(&"exception".into())
    );
}

#[test]
fn independent_rules_retain_the_v6_selection_path() {
    let (mut store, ranked, allowed) = fixture();
    let t = store.traces.get_mut("exception").unwrap();
    t.core = "Unstructured unrelated workshop note.".into();
    t.gist = t.core.clone();
    assert_eq!(
        select_diverse_for_query(&ranked, &allowed, &store, 2, ""),
        vec!["exception", "base"]
    );
}

#[test]
fn short_core_and_lived_reference_select_parent_before_exception() {
    let (mut store, ranked, allowed) = fixture();
    let trace = store.traces.get_mut("exception").unwrap();
    trace.core = "Maintainer record v4 workspace Cedar audience Orion: allow red exports".into();
    assert_eq!(
        select_diverse_for_query(&ranked, &allowed, &store, 2, QUERY),
        vec!["base", "exception"]
    );
    store.traces.get_mut("exception").unwrap().gist = EXCEPTION.replace("Cedar", "Birch");
    assert_eq!(
        super::super::dependency::record(&store.traces["exception"])
            .unwrap()
            .parent,
        None
    );
}

#[test]
fn delivery_never_clips_a_scoped_exception_or_delivers_it_without_parent() {
    let (store, _, _) = fixture();
    let ids = vec!["base".into(), "exception".into()];
    let narratives = vec![BASE.into(), EXCEPTION.into()];
    let mut options = crate::MemoryContextOptions {
        words: 64,
        ..Default::default()
    };
    let context = crate::MemoryContext::from_selected(
        &store,
        &ids,
        &narratives,
        &[],
        &Mood::default(),
        &options,
        0,
    );
    assert_eq!(context.episodes.len(), 2);
    options.words = 32;
    let smaller = context.filtered(&options);
    assert!(smaller.content_words <= 32);
    assert_eq!(smaller.episodes.len(), 1);
    assert_eq!(smaller.episodes[0].trace_id, "base");
    assert!(!smaller.episodes[0].truncated);
    let direct = crate::MemoryContext::from_selected(
        &store,
        &ids,
        &narratives,
        &[],
        &Mood::default(),
        &options,
        0,
    );
    assert_eq!(direct.episodes.len(), 1);
    let orphan = crate::MemoryContext::from_selected(
        &store,
        &ids[1..],
        &narratives[1..],
        &[],
        &Mood::default(),
        &options,
        0,
    );
    assert!(orphan.episodes.is_empty());
}
