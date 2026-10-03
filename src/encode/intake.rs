//! Encode types. The pipeline lives in interpret / paint / split / gate / core.

use crate::core::model::{Attribution, Channel};

/// One hour offered to the gate. Defaults are a neutral Selfhood event.
pub struct EncodeInput<'a> {
    pub event: &'a str,
    pub source: &'a str,
    pub cues: Option<Vec<String>>,
    pub valence: f32,
    pub arousal: f32,
    pub disgust: f32,
    pub self_relevance: f32,
    pub self_congruence: f32,
    pub attribution: Attribution,
    pub utility: f32,
    pub goal_align: f32,
    pub schema: Option<String>,
    pub channel: Channel,
    pub permanence: f32,
}

impl<'a> EncodeInput<'a> {
    pub fn new(event: &'a str) -> Self {
        Self {
            event,
            source: "interaction",
            cues: None,
            valence: 0.0,
            arousal: 0.3,
            disgust: 0.0,
            self_relevance: 0.5,
            self_congruence: 0.5,
            attribution: Attribution::None,
            utility: 0.4,
            goal_align: 0.3,
            schema: None,
            channel: Channel::Selfhood,
            permanence: 0.0,
        }
    }
}

pub struct EncodeDecision {
    pub kept: bool,
    pub score: f32,
    pub reason: String,
    pub trace_id: Option<String>,
    pub archive_id: String,
    /// How many fact slices the gate saw. 1 = the event was small enough to keep whole.
    pub parts: usize,
    pub kept_n: usize,
}

pub use crate::encode::core::accept_core;
pub use crate::encode::gate::{encode, encode_with_parts};
pub use crate::encode::split::{
    lossless_parts, needs_split, parse_segment_reply, segment_facts, split_event,
};
