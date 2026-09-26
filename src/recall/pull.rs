//! After the judge speaks: grip, strikes, and a possible pull toward the core.
//!
//! The sealed archive is never read. `mix_drifted_with_core` stays as it is
//! (behaviour). Replacing it is a later PR.

use crate::core::model::{now_secs, DriftEvent, DriftKind, MemoryTrace, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::recall::judge::{is_grounding_miss, judge_against_core, DetachKind};

/// What recall should speak after the grounding check.
pub struct GroundingOutcome {
    pub spoken_text: String,
    pub pulled_toward_core: bool,
    pub overlap_with_core: f32,
}

pub fn semantic_core_of(trace: &MemoryTrace) -> String {
    if !trace.core.trim().is_empty() {
        return trace.core.clone();
    }
    trace.gist.clone()
}

/// Cold / myth / latent, or already slipping: no ceiling on distortion.
pub fn is_slipping_away(trace: &MemoryTrace) -> bool {
    matches!(
        trace.status,
        TraceStatus::Cold | TraceStatus::Myth | TraceStatus::Latent
    ) || (trace.fidelity < 0.38
        && trace.access < 0.28
        && trace.permanence < 0.35
        && trace.anchor < 0.40)
}

/// 0 = narrator lets this trace warp. 1 = narrator holds it tightly to the core.
pub fn grip_on_trace(trace: &MemoryTrace, profile: &EntityProfile) -> f32 {
    if is_slipping_away(trace) {
        return 0.0;
    }
    let how_much_it_still_matters = (0.35 * trace.permanence
        + 0.25 * trace.anchor
        + 0.20 * trace.access
        + 0.20 * trace.fidelity)
        .clamp(0.0, 1.0);
    (profile.narrator_firmness.clamp(0.0, 1.0) * how_much_it_still_matters).clamp(0.0, 1.0)
}

/// How many consecutive misses before a rewrite. `usize::MAX` = never.
pub fn misses_before_rewrite(trace: &MemoryTrace, profile: &EntityProfile) -> usize {
    let grip = grip_on_trace(trace, profile);
    if grip < 0.12 {
        return usize::MAX;
    }
    let base = profile.ground_strikes.max(1) as f32;
    (base / grip).round().clamp(1.0, 24.0) as usize
}

pub fn should_force_core_rewrite(
    trace: &MemoryTrace,
    profile: &EntityProfile,
    generated: &str,
    core: &str,
) -> bool {
    if is_slipping_away(trace) || grip_on_trace(trace, profile) < 0.12 {
        return false;
    }
    let this_is_a_miss = is_grounding_miss(generated, core, profile.ground_min_overlap);
    let next_strike_count = (trace.detach_strikes as usize) + 1;
    this_is_a_miss && next_strike_count >= misses_before_rewrite(trace, profile)
}

/// Mix the drifted sentence with a core-facing rewrite.
/// High `toward_core` keeps more of the rewrite.
pub fn mix_drifted_with_core(drifted: &str, toward_core: &str, toward_core_amount: f32) -> String {
    let amount = toward_core_amount.clamp(0.0, 1.0);
    if amount < 0.18 || toward_core.trim().is_empty() {
        return drifted.to_string();
    }
    if amount >= 0.82 {
        return toward_core.chars().take(280).collect();
    }
    let drifted_words: Vec<&str> = drifted.split_whitespace().collect();
    let core_words: Vec<&str> = toward_core.split_whitespace().collect();
    if drifted_words.is_empty() {
        return toward_core.to_string();
    }
    let keep_from_drift = ((drifted_words.len() as f32) * (1.0 - amount)).round() as usize;
    let take_from_core = ((core_words.len() as f32) * amount).round().max(1.0) as usize;
    let mut words = Vec::new();
    words.extend(drifted_words.into_iter().take(keep_from_drift.max(1)));
    words.extend(core_words.into_iter().take(take_from_core));
    words.join(" ").chars().take(280).collect()
}

pub fn recontextualize_rule(
    core: &str,
    _gist: &str,
    profile: &EntityProfile,
    valence: f32,
    disgust: f32,
) -> String {
    crate::dream::retell(core, profile, valence, disgust)
}

/// Count a miss, or rewrite the gist toward the core once the budget is spent.
pub fn apply_grounding(
    trace: &mut MemoryTrace,
    profile: &EntityProfile,
    generated: &str,
    core: &str,
    narrator_rewrite: Option<String>,
) -> GroundingOutcome {
    if trace.channel.verbatim() {
        let spoken = if generated.trim().is_empty() {
            trace.gist.clone()
        } else {
            generated.to_string()
        };
        return GroundingOutcome {
            spoken_text: spoken,
            pulled_toward_core: false,
            overlap_with_core: 1.0,
        };
    }

    let judgement = judge_against_core(generated, core);
    let overlap = judgement.overlap;

    // Irony / punchline / other speech act on the same event: speak it,
    // leave gist and detach_strikes alone.
    if judgement.kind == DetachKind::Reframe && overlap >= profile.ground_min_overlap {
        let already_colored = trace
            .drifts
            .last()
            .is_some_and(|d| d.kind == DriftKind::Color);
        if !already_colored {
            trace.drifts.push(DriftEvent {
                kind: DriftKind::Color,
                at: now_secs(),
                note: format!("reframe overlap={overlap:.2} (mouth only)"),
                fidelity_delta: 0.0,
                valence_delta: 0.0,
                disgust_delta: 0.0,
            });
        }
        return GroundingOutcome {
            spoken_text: generated.to_string(),
            pulled_toward_core: false,
            overlap_with_core: overlap,
        };
    }

    if !is_grounding_miss(generated, core, profile.ground_min_overlap) {
        if trace.detach_strikes > 0 {
            trace.detach_strikes -= 1;
        }
        return GroundingOutcome {
            spoken_text: generated.to_string(),
            pulled_toward_core: false,
            overlap_with_core: overlap,
        };
    }

    let grip = grip_on_trace(trace, profile);
    if grip < 0.12 {
        return GroundingOutcome {
            spoken_text: generated.to_string(),
            pulled_toward_core: false,
            overlap_with_core: overlap,
        };
    }

    trace.detach_strikes = trace.detach_strikes.saturating_add(1);
    let allowed_misses = misses_before_rewrite(trace, profile);
    if (trace.detach_strikes as usize) < allowed_misses {
        return GroundingOutcome {
            spoken_text: generated.to_string(),
            pulled_toward_core: false,
            overlap_with_core: overlap,
        };
    }

    let toward_core = narrator_rewrite
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| {
            recontextualize_rule(core, &trace.gist, profile, trace.valence, trace.disgust)
        });
    let spoken = mix_drifted_with_core(generated, &toward_core, grip);
    if spoken.trim() == generated.trim() {
        return GroundingOutcome {
            spoken_text: generated.to_string(),
            pulled_toward_core: false,
            overlap_with_core: overlap,
        };
    }

    trace.gist = spoken.clone();
    trace.detach_strikes = 0;
    trace.drifts.push(DriftEvent {
        kind: DriftKind::Ground,
        at: now_secs(),
        note: format!(
            "reprise grip={grip:.2} misses_allowed={allowed_misses} kind={:?} overlap={overlap:.2}",
            judgement.kind
        ),
        fidelity_delta: 0.0,
        valence_delta: 0.0,
        disgust_delta: 0.0,
    });
    trace.clamp();
    GroundingOutcome {
        spoken_text: spoken,
        pulled_toward_core: true,
        overlap_with_core: overlap,
    }
}
