pub mod bench_report;
pub mod benchmark;
pub mod config;
pub mod core;
pub mod dream;
pub mod encode;
pub mod experiment;
pub mod lexicon;
pub mod net;
pub mod persist;
pub mod recall;

mod engine;

pub use core::model::{
    advance_hours, clock_scale, now_secs, set_clock_scale, ArchiveRecord, AxiomLayer, Channel,
    Attribution, DriftEvent, DriftKind, EvidenceOrigin, IdentityAxiom, InterpretationStamp, MemoryClock, MemoryOperation, MemoryTrace, Mood, OrganCut, RealityAnchor, RecallTally, SemanticCore,
    SchemaCenter,
    RecalledMemory, TraceStatus,
};
pub use core::talk::{TalkTurn, WorkingTalk, ACTIVE_GAP_SECS, MAX_SESSION_SECS};
pub use core::profile::{EntityProfile, Voice};
pub use core::store::MemoryStore;
pub use dream::{
    detail_retention, evaluate_budget, fingerprint, seed_anchor, stability_days, DreamReport,
    Fingerprint, NightKind, NIGHT_PASSES, SHALLOW_PASSES,
};
pub use dream::singularite::distance as singularity_distance;
pub use encode::embed::{cosine, EmbedLog, Embedder, EmbeddingResult, HashEmbedder, HttpEmbedder};
pub use encode::{
    accept_core, encode_with_parts, lossless_parts, measure_congruence, needs_split,
    parse_segment_reply, segment_facts, split_event, EncodeDecision, EncodeInput,
};
pub use engine::SelectiveMemory;
pub use bench_report::{print_banner, print_pair_verbose};
pub use benchmark::{
    h2_holds, hearth_script, marker_holds, names_allusion_marker, names_marker, names_soft_marker, persist_script, ruminate_script, run_v01, run_v01_k, run_v01_n, soft_holds,
    run_v01_n_opts, run_v01_opts, v01_script, Arm, BenchOpts, Campaign, Condition, Instant,
    PairReport, V01Script,
};
pub use experiment::{run_neutral, run_neutral_llm, run_salient, run_salient_llm, run_salient_without_sleep, run_salient_without_sleep_llm, run_erasure, run_split_lives, run_wash, run_wash_llm, run_wash_seed, script, split_script, BifurcationReport, ErasureReport, ExperimentRng, LlmSpec, WashArm, WashReport};
pub use config::Config;
pub use net::api;
pub use recall::{
    FailurePolicy, HttpNarrator, LlmCallLog, Narrator, RecallBias, RecallWrite, RetrievalDump, RuleNarrator, SpeakOnlyHttp,
};
pub use recall::stance::{
    charged_mood, isolated_stance, is_charged, query_hits_episode,
};
pub use HttpNarrator as LLMNarrator;
