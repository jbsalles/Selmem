# Parameters

These are exploratory engineering settings, not fitted psychological constants. Source definitions in `src/core/profile.rs` and `src/core/evidence.rs` are authoritative.

| Setting | Observer default | Meaning |
| --- | --- | --- |
| `encode_threshold` | 0.32 | Admission of a new lived trace |
| `w_arousal`, `w_novelty`, `w_self` | 0.25, 0.15, 0.25 | Salience contributions |
| `w_utility`, `w_goal`, `w_redundancy` | 0.15, 0.10, 0.20 | Utility/goal contribution and redundancy penalty |
| `decay_lambda` | 0.08 | Accessibility decay |
| `rehearsal_boost` | 0.18 | Rehearsal accessibility contribution |
| `fidelity_loss_on_recall` | 0.04 | Recall-related fidelity change |
| `reconsolidation_eta` | 0.25 | Rewrite strength |
| `mood_blend` | 0.08 | Mood update strength |
| `max_recall` | 4 | Selected trace limit |
| `merge_similarity` | 0.32 | Consolidation candidate similarity |
| `ground_min_overlap`, `ground_strikes` | 0.18, 3 | Identity overlap and miss hysteresis |
| `narrator_firmness` | 0.55 | Core pull-back strength |

Tender and austere are starting profiles, not learned personalities; their new-trace thresholds are 0.28 and 0.40. Other defaults differ; see source rather than mixing profile settings.

## Observation support

Ordinary stores default disabled. Enable via `set_observation_evidence(true)` with stable `source` and `observation_id`; GENESIS accepts the global `.selmem` key `observation_evidence=true` and learning's explicit on/off cut. The paired script overrides that setting in each arm.

Limits: 128 representative groups, 64 identified observations per group, eight claim variants, 1024 UTF-8 bytes per claim/source/identity. Capacity rejects extra untracked support rather than evict identities and recount replays. Exact additional primary-claim observations increase carrier anchor by `0.02*(1-anchor)`; counter-observations do not become positive support. Near candidates require lexical overlap at least 0.70 and a contradictory scorer label; at most three groups are considered. No salience threshold is lowered by the update path.

**Reconstructive signal (v0.5+).** Only distinct observations matching a group's primary claim produce a saturated weight `support_weight` (log1p, capped at 0.75). Contradictory claims remain recorded separately and never increase this positive weight. When `OrganCut::support_signal` is true (default), this weight modestly boosts recall score (`× (1 + 0.22·w)`) and injects a short “Support strength” line into the narrative for the mouth. Groups and the labeled count metadata are never removed by the cut; this is a pure signal lesion. The schema prototype has an independent `OrganCut::schema_prototype` switch (default true; `no_schema_prototype()` disables it). Core and gist remain untouched.

## Co-recall and runtime

`associations_learn` and `associations_reconstruct` are independent boolean settings. Enabling either alone is not evidence of an associative effect. Provider model, keys, requested temperature/reasoning and HTTP timeout use `.selmem`. Experimental calls cap actual HTTP attempts including retries. Keys are not written into manifests.

Current GENESIS lives use one simulated hour per input, no sleep within twelve inputs and native reconstruction during intake. Final native/strict probes are separate readout interventions. These choices isolate short-term learning and are not optimized lifetime durations.

### Expanded memory delivery (opt-in)

`memory_context_tokens` adds a ceiling on serialized memory, using the configured
token counter (default: conservative UTF-8 bytes). `memory_context_max_episodes`
and `memory_context_episode_words` expand the episode count and per-episode length.
`memory_context_words` remains an independent ceiling and now accepts up to 64000.
See [configuration, accounting limits and the budget sweep](experiments/genesis/CONTEXT_BUDGET_SWEEP.md).
