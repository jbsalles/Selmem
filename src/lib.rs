//! Selective reconstructive memory for an LLM entity.
//!
//! An LLM maps context to the next token. SelMem does not store a taller log.
//! It selects, reconstructs, and sleeps, so two copies of the same model that
//! retained one hour on one side only no longer remember the same things.
//!
//! The mouth never reads the archive. It sees gist, core, schema, affect,
//! fidelity, confidence, mood, living axioms, and schema centers.
//!
//! # Loop
//!
//! [`SelectiveMemory::live_with`] encodes. [`SelectiveMemory::remember`] and
//! [`SelectiveMemory::speak`] reconstruct. [`SelectiveMemory::sleep`] consolidates.
//! [`SelectiveMemory::save`] is a vault dump, not the memory. The organ lives in
//! [`MemoryStore`].
//!
//! Deep sleep runs when new Selfhood hours or charge clear the budget:
//! weather, confabulation, ladder, centers, rewrite, merge, release.
//! Otherwise the night is weather, confabulation, and release.
//! [`SelectiveMemory::sleep_deep`] forces the full night.
//!
//! # Persistence
//!
//! A path ending in `.db`, `.sqlite`, or `.sqlite3` uses system `libsqlite3`.
//! Anything else is a `SELMEM1` file. Both backends share one snapshot.
//! A cwd `.selmem` file is runtime config, not a vault.
//!
//! # Elsewhere
//!
//! The claim and the benches live in the repo, not on this page:
//! [whitepaper](https://github.com/jbsalles/Selmem/blob/main/WHITEPAPER.md),
//! [layout](https://github.com/jbsalles/Selmem/blob/main/ARCHITECTURE.md),
//! [knobs](https://github.com/jbsalles/Selmem/blob/main/PARAMETERS.md).
//!
//! # Example
//!
//! ```
//! use selmem::{EncodeInput, EntityProfile, SelectiveMemory};
//!
//! let mut organ = SelectiveMemory::new(EntityProfile::new("ada"));
//! organ.live_with(EncodeInput::new(
//!     "the meeting was cancelled after the work was delivered",
//! ));
//! organ.sleep();
//! let _ = organ.remember("the meeting");
//! ```

#![warn(rustdoc::broken_intra_doc_links)]

#[doc(hidden)]
pub mod bench_report;
#[doc(hidden)]
pub mod benchmark;
#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod core;
#[doc(hidden)]
pub mod dream;
#[doc(hidden)]
pub mod encode;
#[doc(hidden)]
pub mod experiment;
#[doc(hidden)]
pub mod fork;
#[doc(hidden)]
pub mod lexicon;
#[doc(hidden)]
pub mod net;
#[doc(hidden)]
pub mod persist;
#[doc(hidden)]
pub mod recall;

mod engine;

pub use core::model::{
    advance_hours, clock_scale, now_secs, set_clock_scale, ArchiveRecord, AxiomLayer, Channel,
    Attribution, DriftEvent, DriftKind, EvidenceOrigin, IdentityAxiom, InterpretationStamp, MemoryClock, MemoryOperation, MemoryTrace, Mood, OrganCut, RealityAnchor, RecallTally, SemanticCore,
    SchemaCenter,
    RecalledMemory, TraceStatus,
};
pub use core::talk::{TalkTurn, WorkingTalk, ACTIVE_GAP_SECS, MAX_SESSION_SECS};
pub use core::profile::Voice;
#[doc(inline)]
pub use core::profile::EntityProfile;
#[doc(inline)]
pub use core::store::MemoryStore;
pub use dream::{
    detail_retention, evaluate_budget, fingerprint, seed_anchor, stability_days, DreamReport,
    Fingerprint, NightKind, NIGHT_PASSES, SHALLOW_PASSES,
};
pub use dream::singularite::distance as singularity_distance;
pub use encode::embed::{cosine, EmbedLog, Embedder, EmbeddingResult, HashEmbedder, HttpEmbedder};
pub use encode::{
    accept_core, encode_with_parts, lossless_parts, measure_congruence, needs_split,
    parse_segment_reply, segment_facts, split_event, EncodeDecision,
};
#[doc(inline)]
pub use encode::EncodeInput;
#[doc(inline)]
pub use engine::SelectiveMemory;
#[doc(hidden)]
pub use bench_report::{print_banner, print_pair_verbose};
#[doc(hidden)]
pub use benchmark::{
    h2_holds, hearth_script, marker_holds, names_allusion_marker, names_marker, names_soft_marker, persist_script, ruminate_script, run_v01, run_v01_k, run_v01_n, soft_holds,
    run_v01_n_opts, run_v01_opts, v01_script, Arm, BenchOpts, Campaign, Condition, Instant,
    PairReport, V01Script,
};
#[doc(hidden)]
pub use experiment::{run_neutral, run_neutral_llm, run_salient, run_salient_llm, run_salient_without_sleep, run_salient_without_sleep_llm, run_erasure, run_split_lives, run_wash, run_wash_llm, run_wash_seed, script, split_script, BifurcationReport, ErasureReport, ExperimentRng, LlmSpec, WashArm, WashReport};
#[doc(hidden)]
pub use fork::{fork_script, run_fork, stage0_pass, ForkArm, ForkOrgan, ForkReport};
pub use config::Config;
pub use net::api;
pub use recall::{
    FailurePolicy, HttpNarrator, HttpScorer, LlmCallLog, Narrator, NullScorer, PropositionLabel,
    PropositionScorer, RecallBias, RecallWrite, RetrievalDump, RuleNarrator, SpeakOnlyHttp,
};
pub use recall::stance::{
    charged_mood, isolated_stance, is_charged, query_hits_episode,
};
pub use HttpNarrator as LLMNarrator;
