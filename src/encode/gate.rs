//! Salience gate: S, τ, write the lived trace and the sealed archive.

use crate::core::model::{now_secs, new_id, ArchiveRecord, Channel, MemoryTrace, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::encode::embed::{novelty_emb, Embedder};
use crate::encode::intake::{EncodeDecision, EncodeInput};
use crate::encode::scoring::{encode_score, novelty};
use crate::encode::split::split_event;

pub fn encode(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    input: EncodeInput<'_>,
    embedder: &dyn Embedder,
) -> EncodeDecision {
    encode_with_parts(store, profile, input, embedder, None)
}

pub fn encode_with_parts(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    mut input: EncodeInput<'_>,
    embedder: &dyn Embedder,
    proposed: Option<&[String]>,
) -> EncodeDecision {
    if let Some(s) = &input.semantics {
        input.valence = s.valence;
        input.arousal = s.arousal;
        input.disgust = s.disgust;
        input.self_relevance = s.self_relevance;
        input.goal_align = s.goal_relevance;
        input.attribution = s.attribution;
        input.schema = s.schema.clone();
    }
    encode_with_interpreter(store, profile, input, embedder, proposed, None)
}

pub fn encode_with_interpreter(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    mut input: EncodeInput<'_>,
    embedder: &dyn Embedder,
    proposed: Option<&[String]>,
    interpreter: Option<&dyn super::semantic::SemanticInterpreter>,
) -> EncodeDecision {
    let annotated = input.semantics.is_some() || interpreter.is_some();
    // Resolve once at the boundary. The gate never infers psychological labels.
    if input.semantics.is_none() {
        let backend = interpreter.unwrap_or(&super::semantic::LexicalInterpreter);
        match backend.interpret_event(input.event) {
            Ok(mut s) => {
                if interpreter.is_none() { s.core.polarity = input.valence.clamp(-1.0, 1.0); }
                input.semantics = Some(s);
            }
            Err(reason) => return EncodeDecision {
                kept: false, score: 0.0, reason, trace_id: None,
                archive_id: String::new(), parts: 0, kept_n: 0,
            },
        }
    }
    if let Some(semantics) = &input.semantics {
        if let Err(reason) = semantics.validate() {
            return EncodeDecision {
                kept: false, score: 0.0, reason, trace_id: None,
                archive_id: String::new(), parts: 0, kept_n: 0,
            };
        }
    }
    let parts = split_event(input.event, proposed);
    if parts.is_empty() {
        return EncodeDecision {
            kept: false,
            score: 0.0,
            reason: "empty".into(),
            trace_id: None,
            archive_id: String::new(),
            parts: 0,
            kept_n: 0,
        };
    }
    if parts.len() == 1 {
        return encode_one(store, profile, input, embedder, None);
    }

    // Interpret and validate all slices before writing anything. Never silently
    // replace a selected learned backend with English lexical inference.
    let lexical = super::semantic::LexicalInterpreter;
    let backend = interpreter.unwrap_or(&lexical);
    if annotated && interpreter.is_none() && input.part_semantics.is_none() {
        return EncodeDecision {
            kept: false, score: 0.0,
            reason: "multipart annotations require a per-slice semantic interpreter".into(),
            trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
        };
    }
    let annotations = if let Some(annotations) = input.part_semantics.take() {
        if annotations.len() != parts.len() || annotations.iter().any(|s| s.validate().is_err()) {
            return EncodeDecision {
                kept: false, score: 0.0, reason: "invalid per-slice annotations".into(),
                trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
            };
        }
        annotations
    } else {
        let mut annotations = Vec::new();
        for part in &parts {
            match backend.interpret_event(part).and_then(|s| { s.validate()?; Ok(s) }) {
                Ok(s) => annotations.push(s),
                Err(reason) => return EncodeDecision {
                    kept: false, score: 0.0, reason: format!("semantic interpretation failed: {reason}"),
                    trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
                },
            }
        }
        annotations
    };

    // Novelty is against the book as it stood before this paste. Sibling
    // slices of the same document must not knock each other under τ.
    let prior_emb: Vec<Vec<f32>> = store
        .traces
        .values()
        .filter(|t| !t.embedding.is_empty())
        .map(|t| t.embedding.clone())
        .collect();
    let prior_gists: Vec<String> = store.traces.values().map(|t| t.gist.clone()).collect();

    let mut kept_ids = Vec::new();
    let mut first_archive = String::new();
    let mut best = 0.0_f32;
    let mut last_reason = String::new();
    for (part, mut semantics) in parts.iter().zip(annotations) {
        if !annotated { semantics.core.polarity = input.valence.clamp(-1.0, 1.0); }
        let mut slice = EncodeInput::new(part);
        slice.semantics = Some(semantics.clone());
        slice.source = input.source;
        slice.observation_id = input.observation_id;
        slice.valence = input.valence;
        slice.arousal = input.arousal;
        slice.disgust = input.disgust;
        slice.self_relevance = input.self_relevance;
        slice.self_congruence = input.self_congruence;
        slice.attribution = input.attribution;
        slice.utility = input.utility;
        slice.goal_align = input.goal_align;
        slice.schema = input.schema.clone();
        slice.channel = input.channel;
        slice.permanence = input.permanence;
        slice.paint_from = input.paint_from.clone();
        if interpreter.is_some() || annotated {
            slice.valence = semantics.valence;
            slice.arousal = semantics.arousal;
            slice.disgust = semantics.disgust;
            slice.self_relevance = semantics.self_relevance;
            slice.goal_align = semantics.goal_relevance;
            slice.attribution = semantics.attribution;
            slice.schema = semantics.schema;
        }
        let d = encode_one(
            store,
            profile,
            slice,
            embedder,
            Some((&prior_emb, &prior_gists)),
        );
        best = best.max(d.score);
        last_reason = d.reason;
        if d.kept {
            if first_archive.is_empty() {
                first_archive = d.archive_id;
            }
            if let Some(id) = d.trace_id {
                kept_ids.push(id);
            }
        }
    }
    EncodeDecision {
        kept: !kept_ids.is_empty(),
        score: best,
        reason: if kept_ids.is_empty() {
            last_reason
        } else {
            format!("encoded {}/{} parts", kept_ids.len(), parts.len())
        },
        trace_id: kept_ids.first().cloned(),
        archive_id: first_archive,
        parts: parts.len(),
        kept_n: kept_ids.len(),
    }
}

fn encode_one(
    store: &mut MemoryStore,
    profile: &EntityProfile,
    input: EncodeInput<'_>,
    embedder: &dyn Embedder,
    novelty_vs: Option<(&Vec<Vec<f32>>, &Vec<String>)>,
) -> EncodeDecision {
    let embedding = embedder.embed(input.event);
    let nov = if let Some((existing, gists)) = novelty_vs {
        if existing.is_empty() {
            novelty(input.event, gists)
        } else {
            novelty_emb(&embedding, existing)
        }
    } else {
        let existing: Vec<Vec<f32>> = store
            .traces
            .values()
            .filter(|t| !t.embedding.is_empty())
            .map(|t| t.embedding.clone())
            .collect();
        if existing.is_empty() {
            let gists: Vec<String> = store.traces.values().map(|t| t.gist.clone()).collect();
            novelty(input.event, &gists)
        } else {
            novelty_emb(&embedding, &existing)
        }
    };
    let mut score = encode_score(
        profile,
        input.arousal,
        nov,
        input.self_relevance,
        input.utility,
        input.goal_align,
        1.0 - nov,
    );
    if input.self_congruence > 0.5 {
        score += profile.w_self * 0.30 * (input.self_congruence - 0.5);
    }

    let mut threshold = profile.encode_threshold;
    if input.channel.verbatim() {
        threshold *= 0.6;
    }
    if input.channel == Channel::Log {
        threshold = 0.0;
    }
    if input.permanence >= 0.8 {
        threshold = threshold.min(0.2);
    }

    if score < threshold && input.permanence < 0.8 && !input.channel.verbatim() {
        return EncodeDecision {
            kept: false,
            score,
            reason: format!("below threshold ({score:.2} < {threshold:.2})"),
            trace_id: None,
            archive_id: String::new(),
            parts: 1,
            kept_n: 0,
        };
    }

    let archive = ArchiveRecord {
        id: new_id("ar"),
        verbatim: input.event.to_string(),
        source: input.source.to_string(),
        created_at: now_secs(),
        released_from: None,
        released_at: None,
        core: String::new(),
    };
    let archive_id = store.add_archive(archive);

    let mut trace = MemoryTrace {
        id: new_id("tr"),
        gist: if input.channel == Channel::Log {
            let mut g = input.event.trim().to_string();
            if g.chars().count() > 480 {
                g = g.chars().take(480).collect();
            }
            g
        } else {
            compress(input.event, 28)
        },
        core: if input.channel == Channel::Log {
            compress(input.event, 24)
        } else {
            super::core::extractive_core(input.event)
        },
        cues: input.cues.unwrap_or_else(|| default_cues(input.event)),
        valence: input.valence,
        arousal: input.arousal,
        disgust: input.disgust,
        self_relevance: input.self_relevance,
        self_congruence: input.self_congruence,
        attribution: input.attribution,
        schema: input.schema,
        channel: input.channel,
        archive_id: Some(archive_id.clone()),
        created_at: now_secs(),
        last_recalled_at: None,
        last_consolidated_at: None,
        fidelity: 1.0,
        permanence: input.permanence,
        rehearsals: 0,
        access: 1.0,
        confidence: 1.0,
        suppressed: false,
        status: TraceStatus::Active,
        drifts: Vec::new(),
        salience_at_encode: score,
        embedding,
        anchor: crate::dream::singularite::seed_anchor(
            input.valence,
            input.arousal,
            input.disgust,
            input.permanence,
            input.self_relevance,
        ),
        detach_strikes: 0,
        observation_id: Some(input.observation_id.unwrap_or(&archive_id).to_string()),
        source: input.source.to_string(),
        interpretation: crate::core::model::InterpretationStamp {
            statement: String::new(),
            valence: input.valence,
            confidence: 1.0,
        },
        operations: Vec::new(),
        semantic: crate::core::model::SemanticCore::default(),
        reality: crate::core::model::RealityAnchor::default(),
        stake_kind: crate::core::model::StakeKind::None,
        bearer: crate::core::model::Bearer::World,
        loss_kind: crate::core::model::LossKind::None,
        stake_mark: String::new(),
        absence: None,
    };
    trace.interpretation.statement = trace.gist.clone();
    if let Some(semantics) = input.semantics {
        trace.semantic = semantics.core;
        if let Some(claim) = super::core::accept_core(&trace.semantic.claim, input.event) {
            trace.core = claim;
        }
        trace.semantic.claim = trace.core.clone();
        trace.stake_kind = semantics.stake_kind;
        trace.bearer = semantics.bearer;
        trace.loss_kind = semantics.loss_kind;
        trace.stake_mark = semantics.stake_mark;
        trace.absence = semantics.absence;
    }
    trace.reality = crate::core::model::RealityAnchor {
        observation_id: trace.observation_id.clone(),
        claim: trace.core.clone(),
        verifiable: true,
    };
    trace.record_operation(crate::core::model::MemoryOperation {
        kind: "encode".into(),
        at: trace.created_at,
        source_trace_ids: Vec::new(),
        source_axiom_ids: Vec::new(),
        source_center: None,
        before: String::new(),
        after: if let Some(src) = input.paint_from.as_deref() {
            format!("paint:{src}")
        } else {
            trace.gist.clone()
        },
        confidence: 1.0,
        origin: crate::core::model::EvidenceOrigin::Event,
    });
    trace.clamp();
    associate(store, &trace);
    let tid = store.add_trace(trace);
    EncodeDecision {
        kept: true,
        score,
        reason: "encoded".into(),
        trace_id: Some(tid),
        archive_id,
        parts: 1,
        kept_n: 1,
    }
}

fn associate(store: &mut MemoryStore, trace: &MemoryTrace) {
    let others: Vec<(String, Option<String>, Vec<String>)> = store
        .traces
        .values()
        .filter(|o| o.id != trace.id)
        .map(|o| (o.id.clone(), o.schema.clone(), o.cues.clone()))
        .collect();
    for (oid, schema, cues) in others {
        if schema.is_some() && schema == trace.schema {
            store.link(&trace.id, &oid);
            continue;
        }
        let shared = trace
            .cues
            .iter()
            .any(|c| cues.iter().any(|o| o.eq_ignore_ascii_case(c)));
        if shared {
            store.link(&trace.id, &oid);
        }
    }
}

fn compress(event: &str, max_words: usize) -> String {
    let words: Vec<&str> = event.split_whitespace().collect();
    if words.len() <= max_words {
        event.trim().to_string()
    } else {
        format!("{}…", words[..max_words].join(" "))
    }
}

fn default_cues(event: &str) -> Vec<String> {
    event
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.chars().count() > 4)
        .take(8)
        .collect()
}
