//! Offline validation of the two new mechanisms:
//! 1. pre-retrieval belief ablation (OrganCut::no_self_cues)
//! 2. observation-support transfer on merge changing recall score
use selmem::dream::merge;
use selmem::{
    Attribution, ContextFacet, EncodeInput, EntityProfile, OrganCut, RecallBias, RecallWrite,
    SelectiveMemory, SelfMemory, SelfRole,
};

fn organ() -> SelectiveMemory {
    let mut p = EntityProfile::new("Claire");
    p.encode_threshold = 0.0;
    p.merge_similarity = 0.15;
    SelectiveMemory::new(p)
}

/// 1. On the context-cued route, injecting the belief gist changes which
/// episode ranks first. With self_cues disabled the same store ranks differently.
#[test]
fn no_self_cues_changes_top_episode_on_context_cued_route() {
    let mut m = organ();

    // Belief whose gist contains the distinctive token "library".
    let mut belief = EncodeInput::new("I prefer the quiet library for focused work.");
    belief.attribution = Attribution::Internal;
    belief.schema = Some("work-place".into());
    belief.personal = SelfMemory {
        role: SelfRole::Belief,
        context: "choice-pattern".into(),
        expectation: "chooses library".into(),
        importance: 0.95,
        certainty: 0.85,
        scope_facets: vec![ContextFacet::new("company", "alone")],
        ..SelfMemory::default()
    };
    m.live_with(belief);

    // Episode that shares "library" with the belief.
    let mut library = EncodeInput::new("Finished the report alone in the quiet library.");
    library.attribution = Attribution::Internal;
    library.schema = Some("work-place".into());
    library.permanence = 0.45;
    library.self_relevance = 0.6;
    let library_id = m.live_with(library).trace_id.expect("library");

    // Distractor that matches the bare query ("work") better.
    let mut office = EncodeInput::new("Worked at the office desk all morning on the same report.");
    office.attribution = Attribution::Internal;
    office.schema = Some("work-place".into());
    office.permanence = 0.55;
    office.self_relevance = 0.7;
    let office_id = m.live_with(office).trace_id.expect("office");

    let facets = [ContextFacet::new("company", "alone")];
    let query = "Where did I work on the report?";

    m.cut = OrganCut::full();
    let cued = m
        .open_mouth_in_context_cued(query, "choice-pattern", &facets)
        .unwrap();
    let cued_top = cued.dump.selected.first().cloned().unwrap_or_default();
    let cued_rank_lib = cued
        .dump
        .candidates
        .iter()
        .position(|c| c.trace_id == library_id);
    m.cancel_mouth(&cued);

    m.cut = OrganCut::no_self_cues();
    let ablated = m
        .open_mouth_in_context_cued(query, "choice-pattern", &facets)
        .unwrap();
    let ablated_top = ablated.dump.selected.first().cloned().unwrap_or_default();
    let ablated_rank_lib = ablated
        .dump
        .candidates
        .iter()
        .position(|c| c.trace_id == library_id);
    m.cancel_mouth(&ablated);

    // Cueing must improve the library episode's rank (or change the top).
    assert!(
        cued_rank_lib < ablated_rank_lib || cued_top != ablated_top,
        "belief cue must change ranking: cued_top={cued_top} ablated_top={ablated_top} \
         cued_lib_rank={cued_rank_lib:?} ablated_lib_rank={ablated_rank_lib:?}"
    );
    assert_ne!(
        cued_top, office_id,
        "sanity: office should not dominate when cued"
    );
    let _ = library_id;
}

/// 2. Merge moves observation support onto the keeper and raises its weight.
#[test]
fn merge_transfers_support_and_raises_keeper_weight() {
    let mut m = organ();
    m.set_observation_evidence(true);

    // Two close but distinct Selfhood hours so they stay separate traces.
    let mut a = EncodeInput::new("Mira supplied six batteries to the workshop today.");
    a.source = "sensor";
    a.observation_id = Some("obs-a");
    a.schema = Some("workshop".into());
    a.attribution = Attribution::Internal;
    a.permanence = 0.5;
    a.self_relevance = 0.8;
    let keep = m.live_with(a).trace_id.expect("keep");

    let mut b = EncodeInput::new("Mira supplied six batteries to the workshop yesterday.");
    b.source = "sensor";
    b.observation_id = Some("obs-b");
    b.schema = Some("workshop".into());
    b.attribution = Attribution::Internal;
    b.permanence = 0.3;
    b.self_relevance = 0.5;
    let other = m.live_with(b).trace_id.expect("other");
    assert_ne!(keep, other, "need two distinct traces to merge");

    // Make both eligible for the merge pass: Episode role, no prior appraisal,
    // no protection, and close enough that lexical similarity clears the gate.
    for id in [&keep, &other] {
        let t = m.store.traces.get_mut(id).unwrap();
        t.personal.role = SelfRole::Episode;
        t.personal.appraisal.prior_ids.clear();
        t.personal.protected_until = 0;
        t.channel = selmem::Channel::Selfhood;
        t.schema = Some("workshop".into());
        t.embedding.clear(); // force lexical path
        t.anchor = 0.3; // avoid anchor conflict
        t.valence = 0.1;
        t.disgust = 0.0;
        t.stake_mark.clear(); // same_stake requires equal (or empty) marks
    }

    // Give the dropped hour an extra observation whose claim matches the
    // keeper's primary so support_weight can rise after transfer.
    let primary = "Mira supplied six batteries to the workshop today.";
    m.store.observation_evidence.seed(
        &other,
        "sensor",
        selmem::Channel::Selfhood,
        "obs-extra",
        primary,
    );
    if let Some(g) = m.store.observation_evidence.groups.get_mut(&other) {
        g.primary_claim = primary.into();
        g.observations.insert("obs-extra".into(), primary.into());
        g.observations.insert("obs-extra-2".into(), primary.into());
    }

    let weight_before = m.store.observation_evidence.support_weight(&m.store, &keep);

    let merged = merge::run(
        &mut m.store,
        &m.profile,
        &selmem::encode::embed::HashEmbedder,
        false,
    );
    assert!(merged >= 1, "expected a merge of the two workshop hours");

    assert!(m.store.observation_evidence.groups.get(&other).is_none());
    let keeper = m
        .store
        .observation_evidence
        .groups
        .get(&keep)
        .expect("keeper holds transferred support");
    assert!(keeper.observations.contains_key("obs-extra"));

    let weight_after = m.store.observation_evidence.support_weight(&m.store, &keep);
    assert!(
        weight_after > weight_before,
        "support_weight must rise ({weight_before} -> {weight_after})"
    );

    // Keeper remains recallable.
    let (_mems, dump) = m.remember_with(
        "Mira batteries workshop",
        RecallWrite::ReadOnly,
        RecallBias::Observed,
        &[],
    );
    assert!(dump.candidates.iter().any(|c| c.trace_id == keep));
}

/// Instrumented night: merge two supported hours, recall before and after,
/// and write an audit of the score and selected-set change.
#[test]
fn instrumented_night_records_packet_change() {
    let mut m = organ();
    m.set_observation_evidence(true);

    let mut a = EncodeInput::new("Mira supplied six batteries to the workshop today.");
    a.source = "sensor";
    a.observation_id = Some("obs-a");
    a.schema = Some("workshop".into());
    a.attribution = Attribution::Internal;
    a.permanence = 0.5;
    a.self_relevance = 0.8;
    let keep = m.live_with(a).trace_id.expect("keep");

    let mut b = EncodeInput::new("Mira supplied six batteries to the workshop yesterday.");
    b.source = "sensor";
    b.observation_id = Some("obs-b");
    b.schema = Some("workshop".into());
    b.attribution = Attribution::Internal;
    b.permanence = 0.3;
    b.self_relevance = 0.5;
    let other = m.live_with(b).trace_id.expect("other");

    for id in [&keep, &other] {
        let t = m.store.traces.get_mut(id).unwrap();
        t.personal.role = SelfRole::Episode;
        t.personal.appraisal.prior_ids.clear();
        t.personal.protected_until = 0;
        t.channel = selmem::Channel::Selfhood;
        t.schema = Some("workshop".into());
        t.embedding.clear();
        t.anchor = 0.3;
        t.valence = 0.1;
        t.disgust = 0.0;
        t.stake_mark.clear();
    }
    let primary = "Mira supplied six batteries to the workshop today.";
    m.store.observation_evidence.seed(
        &other,
        "sensor",
        selmem::Channel::Selfhood,
        "obs-extra",
        primary,
    );
    if let Some(g) = m.store.observation_evidence.groups.get_mut(&other) {
        g.primary_claim = primary.into();
        g.observations.insert("obs-extra".into(), primary.into());
        g.observations.insert("obs-extra-2".into(), primary.into());
    }

    let query = "Mira batteries workshop";
    let score_of = |m: &mut SelectiveMemory, id: &str| -> f32 {
        let (_mems, dump) =
            m.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
        dump.candidates
            .iter()
            .find(|c| c.trace_id == id)
            .map(|c| c.score)
            .unwrap_or(0.0)
    };
    let selected_of = |m: &mut SelectiveMemory| -> Vec<String> {
        let (_mems, dump) =
            m.remember_with(query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
        dump.selected.clone()
    };

    let weight_before = m.store.observation_evidence.support_weight(&m.store, &keep);
    let score_before = score_of(&mut m, &keep);
    let selected_before = selected_of(&mut m);

    let merged = merge::run(
        &mut m.store,
        &m.profile,
        &selmem::encode::embed::HashEmbedder,
        false,
    );
    assert!(merged >= 1);

    let weight_after = m.store.observation_evidence.support_weight(&m.store, &keep);
    let score_after = score_of(&mut m, &keep);
    let selected_after = selected_of(&mut m);

    assert!(weight_after > weight_before);
    // Packet or score must have moved.
    assert!(score_after != score_before || selected_before != selected_after);

    // Write a compact audit so the run is inspectable.
    let audit = format!(
        "{{\"keep\":\"{keep}\",\"other\":\"{other}\",\"merged\":{merged},\"weight_before\":{weight_before},\"weight_after\":{weight_after},\"score_before\":{score_before},\"score_after\":{score_after},\"selected_before\":{selected_before:?},\"selected_after\":{selected_after:?}}}"
    );
    let path = std::path::Path::new("experiments/instrumented-night-audit.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, audit).ok();
}
