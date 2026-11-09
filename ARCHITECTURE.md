# Architecture

SelMem is a Rust memory library with a daemon, terminal client and multiple experiment adapters. The current organ includes optional cumulative observation support; earlier benchmark source hashes identify historical versions, not a frozen current organ.

| Directory | Responsibility |
| --- | --- |
| `core/` | Traces, immutable cores, provenance, profiles, working conversation, co-recall graph and bounded observation support |
| `encode/` | Interpret, validate, paint, split and select incoming experiences |
| `recall/` | Select traces, reconstruct, judge against core, ground and optionally reconsolidate |
| `dream/` | Accessibility decay, consolidation, schemas, beliefs, merging and release |
| `persist/` | File and SQLite snapshots |
| `net/`, `config.rs` | Runtime interface/API and provider configuration |
| `bin/selmem-genesis/` | Positive-control and OFF/ON learning orchestration |
| `bin/selmem-locomo/` | External strategy benchmark adapter |

`engine.rs` coordinates the organ. It does not contain an experimental world or prescribed behavior. `WorkingTalk` is temporary; sleep commits recorded turns through encoding and clears the working frame.

## Records and grounding

A lived trace contains gist, core, schema, affect, confidence, accessibility and provenance. Gist can change; the core supplies a stable grounding reference. The sealed archive is an audit record and is not supplied to the narrator or consulted for runtime grounding. Grounding detects deviation from core, not objective truth about the world.

State transformations carry provenance operations. Readonly recall may generate new output but must not modify the saved memory. GENESIS probes use independent snapshot clones and verify hashes.

## Independent observation support

`SelectiveMemory::set_observation_evidence(true)` enables bounded support accumulation for `EncodeInput` with a stable `source` and `observation_id`. Exact repeated propositions with new identities update an existing active trace instead of creating another trace. Repeated ingestion of the same identity and conflicting reuse of it are rejected. Recall never increments the observation count.

Near but different propositions are not positive support. An attached proposition scorer may identify a counter-observation; each alternative claim retains its own count. Unknown labels remain on the ordinary salience path. The scorer can be wrong. Distinct caller-provided identities do not themselves prove statistical independence.

Counts accompany a selected trace as labeled metadata after reconstruction, grounding and association learning. They never enter gist/core write-back and do not require archive access. Suppression/deletion of the carrier prevents rendering. GENESIS can lesion support groups independently of traces. File and SQLite snapshots preserve identities and counts.

A saturated reconstructive weight (`support_weight`) is derived only from distinct observations supporting the carrier's primary claim, never from contradictory variants retained in the same group. When `OrganCut::support_signal` is true (default; `no_support_signal()` lesions it), the weight modestly boosts recall score and injects a short strength line into the narrative. The cut does not discard groups or rewrite core/gist. The independent `OrganCut::schema_prototype` cut (or `no_schema_prototype()`) controls the optional prototype cue for Internal traces; disabling the support signal leaves that cue unchanged.

This mechanism defaults off in ordinary stores. The GENESIS learning script explicitly compares off/on with the same inputs. It does not force a decision, lower the salience threshold or declare a personality. Counts are not currently transferred between merged traces; consolidation and long-term behavior require separate validation.

## Co-recall

Association learning and use during reconstruction are independent cuts. Co-recalled accepted traces strengthen links; associative cues are fallible context, not historical facts. See [CO_RECALL_ASSOCIATIONS.md](docs/CO_RECALL_ASSOCIATIONS.md). Enabling the feature does not prove it affected a particular recall.

## Experiments

GENESIS has one executable and shared configuration, provider, transport and snapshot code. Positive controls fix a decision oracle; learning compares external faithful history, admitted observations, native/strict recall and empty context. LoCoMo uses its own dataset adapter. Runtime memory algorithms are shared, not copied into experiment-specific organs.

## Axiom context persistence

Axiom family matching uses `stake_kind`, `bearer`, `loss_kind` and `stake_mark`. These fields are now retained in SELMEM1 (an optional `axiom_context` record with a length-prefixed mark) and SQLite. SQLite also persists the corresponding trace fields and absence; losing either side of the family comparison can cause repeated consolidation to create duplicate motifs after reopening.

The new reader accepts old files. Missing axiom context is recovered only from unanimous existing lived support, excluding traits and incomplete/mixed support. Explicitly saved empty context remains empty. No sealed archive or heuristic parsing of axiom prose is used. Legacy SQLite trace stakes use the same lived-gist fallback as legacy flat traces; original omitted annotations cannot be guaranteed recoverable. Existing historical duplicates are preserved rather than silently deleted. Updated SQLite schemas are migrated on load/save; old binaries should not be used to write the new format.

Regression coverage includes both containers, explicit empty fields, legacy loading and twenty forced-deep nights with reopening between each. The original sleep experiment remains an observation of the older codec, not evidence from the corrected engine.

## Shared experiment infrastructure (post-refactor)

`src/util/` owns the dependency-free JSON and SHA-256 implementations; both are
used by the library and experimental executables. `src/experiment_support.rs`
contains reusable journal I/O. The synthetic retention, creative replay, and
contextual transfer harnesses share source under `src/bin/shared/`. Protocols
remain separate and use their original executable entrypoints.

`engine/ingest.rs` and `engine/mouth.rs` implement the original public engine
facade; `recall/retrieve/ranking.rs` indexes evidence and scores candidates; `selection.rs` chooses traces; `pipeline.rs` reconstructs, grounds and writes back, and
`core/self_model/scope.rs` handles belief context qualification and recovery; `consolidation.rs` handles belief consolidation.
No behavioral redesign is implied by relocating these functions.

Reproducibility: the old evidence keeps its original implementation hashes.
Any run started after this refactor has a different source identity. Never
resume an interrupted historical paid run against the refactored source.

Read-only recall scaling can be measured with
`cargo run --release --bin selmem-bench-recall -- 100 500 1000`.
Report p50/p95 latency alongside stored trace/axiom counts and score hashes. A benchmark
is not evidence of an optimization until a baseline and after-run agree on
fixtures, memory counts and selected core fingerprints.
