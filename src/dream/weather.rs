//! First night pass: decay detail, unused disgust, status, latent revive.
//! World traces stay Active with access = 1.

use crate::core::model::{now_secs, DriftEvent, TraceStatus};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::dream::drift::{sculpt, weather};
use crate::encode::scoring::refresh_access;

pub struct WeatherReport {
    pub faded: u32,
    pub cold: u32,
    pub myth: u32,
    pub extinguished: u32,
    pub weathered: u32,
    pub sculpted: Vec<DriftEvent>,
}

pub fn run(store: &mut MemoryStore, profile: &EntityProfile) -> WeatherReport {
    let mut faded = 0;
    let mut cold = 0;
    let mut myth = 0;
    let mut sculpted = Vec::new();
    let mut extinguished = 0;
    let mut weathered = 0;
    let ids = store.active_ids();

    for id in &ids {
        {
            let trace = store.traces.get_mut(id).unwrap();
            if trace.channel.verbatim() {
                // Operational facts do not cool, mythologize, or go latent.
                trace.status = TraceStatus::Active;
                trace.access = 1.0;
                continue;
            }
        }
        let event = {
            let trace = store.traces.get_mut(id).unwrap();
            refresh_access(trace, profile);
            if weather(trace, profile).is_some() {
                weathered += 1;
            }
            let unused = match (trace.last_recalled_at, trace.last_consolidated_at) {
                (None, _) => true,
                (Some(r), Some(c)) => r <= c,
                (Some(_), None) => false,
            };
            let ev = if unused && extinguish(trace, profile) {
                extinguished += 1;
                None
            } else {
                sculpt(trace, profile)
            };
            if trace.last_consolidated_at.is_none() {
                trace.last_consolidated_at = Some(trace.created_at);
            } else {
                trace.last_consolidated_at = Some(now_secs());
            }
            ev
        };
        if let Some(ev) = event {
            sculpted.push(ev);
        }
        let trace = store.traces.get_mut(id).unwrap();
        if trace.permanence >= 0.8 {
            continue;
        }
        if !trace.channel.verbatim()
            && trace.fidelity < 0.34
            && trace.access < 0.26
            && (trace.valence.abs() > 0.25 || trace.disgust > 0.22 || trace.schema.is_some())
        {
            if trace.status != TraceStatus::Latent {
                trace.status = TraceStatus::Latent;
            }
        } else if trace.access < profile.myth_access && trace.status != TraceStatus::Myth {
            trace.status = TraceStatus::Myth;
            myth += 1;
        } else if trace.access < profile.cold_access && trace.status == TraceStatus::Active {
            trace.status = TraceStatus::Cold;
            cold += 1;
        } else if trace.access < profile.myth_access / 2.0 {
            faded += 1;
        }
        maybe_revive_latent(trace);
    }

    WeatherReport {
        faded,
        cold,
        myth,
        extinguished,
        weathered,
        sculpted,
    }
}

fn extinguish(trace: &mut crate::core::model::MemoryTrace, profile: &EntityProfile) -> bool {
    if trace.channel.verbatim() || trace.disgust < 0.08 {
        return false;
    }
    let unused = match (trace.last_recalled_at, trace.last_consolidated_at) {
        (None, _) => true,
        (Some(r), Some(c)) => r <= c,
        (Some(_), None) => false,
    };
    if !unused {
        return false;
    }
    let before = trace.disgust;
    trace.disgust = (trace.disgust * (1.0 - profile.extinction_rate)).max(0.0);
    if (before - trace.disgust).abs() > 0.001 {
        trace.drifts.push(DriftEvent {
            kind: crate::core::model::DriftKind::Fade,
            at: now_secs(),
            note: "slow extinction of disgust".into(),
            fidelity_delta: 0.0,
            valence_delta: 0.0,
            disgust_delta: trace.disgust - before,
        });
        true
    } else {
        false
    }
}

/// The charge was lived again. The original scene does not return:
/// only the core, as a cold blur.
fn maybe_revive_latent(trace: &mut crate::core::model::MemoryTrace) {
    if trace.status != TraceStatus::Latent {
        return;
    }
    if trace.rehearsals < 2 {
        return;
    }
    if !trace.core.is_empty() {
        trace.gist = trace.core.clone();
    }
    trace.status = TraceStatus::Cold;
    trace.fidelity = trace.fidelity.max(0.40).min(0.55);
    trace.access = trace.access.max(0.22);
    trace.detach_strikes = 0;
}
