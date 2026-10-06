pub mod affect;
pub mod core;
pub mod embed;
pub mod gate;
pub mod identity;
pub mod intake;
pub mod interpret;
pub mod paint;
pub mod scoring;
pub mod split;

pub use core::{accept_core, maybe_set_core};
pub use gate::{encode, encode_with_parts};
pub use intake::{EncodeDecision, EncodeInput};
pub use interpret::interpret;
pub use paint::{measure_congruence, paint};
pub use split::{
    lossless_parts, needs_split, parse_segment_reply, propose as propose_split, segment_facts,
    split_event,
};

pub mod semantic;
