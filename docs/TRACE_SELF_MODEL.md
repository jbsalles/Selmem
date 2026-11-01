# Trace-based contextual self

Built on `da16605`. This is a functional architecture, not evidence of subjective
consciousness or a demonstration of behavioral individuality.

## State and loop

`MemoryTrace.personal` contains a `SelfMemory`: role (`Episode`, `Belief`,
`Aspiration`, `Commitment`), context, prospective expectation, importance,
epistemic certainty, supporting/counterexample trace IDs, consumed observation
keys, inaccessible-foundation flag and dated appraisal. All are lived traces;
there is no second autobiography. The contextual view is recomputed on demand
and returns at most eight accessible, unsuppressed, non-retired propositions.

1. Interpret the incoming event, then activate the prior contextual view.
2. Compare the event to expectations already in that view. The configured
   `PropositionScorer` supplies entail/contradict/unknown. Without a scorer,
   revision remains unknown. This may add up to eight scorer calls per input.
3. Record that pre-event view's IDs and appraisal time before any belief update.
   Personal relevance raises the normal gate score. Continuity >= 0.8 or
   revision >= 0.8 independently admit a trace, regardless of belief certainty.
4. A deterministic 1/20 content sample admits up to sixteen currently accessible
   **attested observations** via an opening allowance. Unannotated reports keep
   legacy admission. This is not a global cap on the existing memory store.
5. At sleep, consolidate before destructive night passes. Consume distinct
   source/observation keys once. Belief certainty changes at most 0.15 per cycle;
   even capped evidence is consumed, so repeating sleep cannot manufacture proof.

`SelfEvidence::Report` is the default. Only explicit `Observation` episodes can
update beliefs. Generated abstractions are `Interpretation`, never independent
proof. Suppressed, latent, merged and confabulated traces cannot bootstrap a
belief; source text is never recovered from archives or operation journals.

Two distinct internal observations sharing a schema may produce a tentative
contextual belief. `Narrator::distill_self` proposes its statement and a testable
expectation for FUTURE events. The HTTP narrator requests structured JSON; the
rule backend accepts only the conservative repeated-identical-action summary.
Other proposals require the group guard and two individually supportive action
observations. Initial certainty is 0.35, controlled by the organ. Schema
quality and semantic witness quality remain empirical limitations.

Contradictions lower certainty and remain explicitly linked. A contradicted
belief retires at certainty <= 0.05, allowing a later hypothesis. Aspirations
and commitments retain their role: an outcome never promotes them into acquired
qualities. Counterexamples can leave an unresolved tension rather than force a
coherent identity. This version does not automatically infer arbitrary context
splits or commitment fulfillment/deadlines; integrations may supply explicit
contextual propositions.

Contradictory episodes receive 24 virtual hours of protection, not permanent
pins. Self abstractions are excluded from episode retrieval, merge,
confabulation and legacy axiom extraction. Accessibility still decays. Missing
support marks foundations inaccessible and reduces certainty once by 0.1;
forgetting does not automatically delete the disposition or restore an archive.

## Rust API and attribution

New behavioral proposals also require two individually supportive action
observations after the group guard passes. Opposite actions are counterexamples;
Unknown observations are not supporting foundations. Consolidation audits retain
submitted sources separately from supports, counters and unknowns, with a reason
for rejection or abstention. The HTTP scorer uses the same prospective-outcome
contract as GENESIS for these labels. See [pilot 3](../experiments/genesis/SELF_MODEL_PILOT_V3.md).

Behavioral bootstrap additionally requires semantic annotation with
`StakeKind::Decision` and `Bearer::Self_` on at least two distinct internal
observed action outcomes. The example below sets attribution only; it is not
sufficient evidence for a new behavioral belief without that annotation.
Routine notes, plans and promises are not action outcomes. Custom narrators
must pair their proposals with `PropositionScorer::score_self_model`; Unknown
rejects a new proposal. Without a validator, only the conservative repeated-action
fallback is accepted. Existing snapshots are not retroactively sanitized.
See [pilot 2](../experiments/genesis/SELF_MODEL_PILOT_V2.md) for the guard and audit.

```rust
use selmem::{Attribution, EncodeInput, EntityProfile, SelectiveMemory};
let mut memory = SelectiveMemory::new(EntityProfile::new("Claire"));
let mut event = EncodeInput::observed(
    "I told Alice about the error despite the cost.", "tool-result-0001"
);
event.attribution = Attribution::Internal;
event.schema = Some("honesty with Alice".into());
memory.live_with(event);
```

Reuse observation IDs for retries. Distinct IDs are the caller's assertion of
distinct observations, not a guarantee of statistical independence. Use `new`
for unverified statements. A user saying "you are a coward" is not an observed
agent action. Actual tool outcomes and executed decisions should use `observed`.
The existing chat adapter records user reports; it does not relabel those as
the agent's actions. UI night recollections are attested internal events.

For an explicit aspiration, commitment or hypothesis, fill `input.personal`
with its role, context, importance, certainty and optionally expectation.
Internal attribution must be explicit and the channel must be `Selfhood`.
This remains an interpretation, not evidence for itself.

Personal propositions must be atomic. Multipart events keep whole-event
relevance but do not assign evidence to an arbitrary fragment. Submit atomic
observed events when evidential revision matters.

## Delivery, interventions and persistence

Structured readout includes `self_model` with role, certainty, importance,
context, missing foundations and counterexample count. It shares the configured
word budget, has at most four entries, and obeys the dispositions component cut.
Abstractions are not exposed as observed episodes. Legacy/non-structured
narrators receive labeled statements.

`store.self_policy` has independent runtime switches:

- `selection`: self-dependent admission, including opening;
- `consolidation`: creation/revision during sleep;
- `opening`: exploratory admission only.

All default true. Reapply overrides after loading. A memory-only control starts
without self traces and disables selection/consolidation. Frozen-self disables
consolidation. An evolving self without selection feedback disables selection.
Match histories, models and budgets when comparing these conditions.

File snapshots gain a versioned `personal-v1` length-prefixed extension; SQLite
gains a `personal TEXT` column via existing migration. Both use the same codec
and reject malformed metadata. Old snapshots default to episode/report without
fabricating prior identity. New extensions require the updated reader.

Tests in `tests/self_model.rs` cover causal selection/cuts, pre-event state,
retry/recall evidence invariants, uncertainty versus importance, provenance,
role separation, opening bounds, suppression, bootstrap, temporary protection,
readout labels, and both persistence backends. Tests establish code invariants;
LLM proposal quality and downstream behavioral effects need experiments.

```sh
cargo test --test self_model
cargo test --test identity_nights --test memory_context_defaults
cargo test --all-targets
```

Validation for this patch: 503 tests passed across the target suite, including
16 self-model tests. The three year-long horizon simulations were run separately
against an optimized library. Patch application was checked on a clean da16605
worktree. The identity-name regression assertion was made case-insensitive to
match the existing prompt's uppercase NOT without changing its behavior.

GENESIS now has a separately versioned [causal pilot](../experiments/genesis/SELF_MODEL_PILOT_V3.md).
It tests four conditions and model swaps; existing transcript fixtures stay reports.
Retired contextual hypotheses cannot bootstrap again from the same consumed
observations. GENESIS separates outcome-versus-expectation labels from the
historical preservation labels used for reconstruction auditing.

The recorded Grok v3 lot qualifies two scripted histories and validates
self-dependent admission, retirement under contradiction and later belief
formation from new action roots. Its 128 decisions show limited downstream
differences: 2/32 full-loop vs memory-only, 0/32 full-loop vs evolution without
admission feedback. No durable behavioral individuality is established.
See [measurements, controls and provenance](../experiments/genesis/SELF_MODEL_RESULTS.md).

## Later behavioral results (October 2026)

The [GENESIS self-to-behavior result ledger](../experiments/genesis/SELF_BEHAVIOR_RESULTS.md)
adds the Self-Bridge v4, prompt-only Self-Preference and organ-based Preference
Emergence experiments. In the latter, guarded beliefs summarizing *repeated
performed actions* were observed in both affect-rich and action-only histories;
neutral observations did not form one. After one and three opposing experiences,
certainty changed approximately 0.35 → 0.20 → 0.05 and the original belief
retired. A contrary preference did not appear spontaneously.

The actor sensitivity signal arises from **explicit delivery**: 6/8 initial
paired choices differed when the delivered self statement came from the
opposite simulated history, compared with 1/16 changed when adding an own-self
statement to episodes. The registered memory-ablation prompts accidentally
duplicated episode-only prompts. No durable preference reversal or silent
agent-level individuality follows from these records. These are measured
limits of the current protocol, not reasons to change evidence thresholds.

## Contextual differentiation — experimental extension

SelfMemory stores typed `scope_facets`, `scope_parent`, `scope_epoch` and `scope_status` in personal-v3, while reading old v1 and v2 payloads. Both SELMEM1 and SQLite still share the same personal codec; legacy personal-v1 values read as general, unscoped beliefs. No sealed archive is consulted during inference.

At consolidation, if a broad action-pattern belief has at least two distinct attested supporting actions and two distinct counterexamples, the organ may propose a candidate distinction only if exactly one attested dimension discriminates the groups. Multiple perfectly confounded dimensions trigger abstention. Counterexamples can form a second action hypothesis only if their literal semantic claims agree and a proposition scorer individually endorses that claim. The general interpretation is kept as a conflicted fallback, while two candidate action-pattern hypotheses are created. They become active only after prospective, separately attested confirmation in both contexts. Disable the runtime-only `store.contextual_scope.enabled` flag for a same-evidence no-differentiation ablation; reapply after reopening. This is deliberately not an inference of subjective preference or a causal account of why different actions happened. Scope differentiation is experimental and defaults off; the runtime flag must be set for each experiment and reapplied after reopening.

Only typed, separately attested facts supplied through scoped APIs activate conditional beliefs. Application context and circumstance facets are separate: `open_mouth_scoped` supplies facets but does not by itself transmit a context label such as `choice-pattern` to the self-view filter. User prompt text (including dimension=value literals) cannot activate them. Ordinary unannotated chat cannot yet supply reliable contextual dimensions and therefore may not retrieve them. Missing dimensions mean unknown rather than a mismatch. Free-text contextual extraction, generalization across additional dimensions and long-term behavioral effects remain unimplemented and require GENESIS controls and held-out probes.


## Native delivery diagnosis and causal v2

In the [completed 12-scenario ablation run](../experiments/genesis/CAUSAL_NATIVE_RESULTS.md), the revised `context-cued` route delivered a conditional belief and two relevant episodes in all 192 query cells. Full memory was history-concordant in 479/576 choices, compared with 429/576 after removing delivered beliefs and 415/576 after removing episodes. An earlier uncorrected native run delivered no self-view beliefs and only background episodes; it does not support a belief-effect claim.

The causal v2 extension provides two explicit one-response APIs:

- `open_mouth_in_context(user, context, facets)` supplies the caller-attested application context to native self-view delivery while retaining the ordinary episodic query.
- `open_mouth_in_context_cued(user, context, facets)` additionally uses applicable native self-view gists as internal retrieval cues. It does not force support IDs or use expectations or fixture answer labels to choose episodes.

Both retain typed-facet checks; the context is consumed for one response. Episode JSON carries the episode's stored scope facets. These are opt-in routes, not automatic contextual extraction from ordinary chat. Native statements and actor questions are preserved; no separate predicted-action instruction is appended. The v2 preflight blocks actor calls and resume if exposure fails.

Removing a belief from a packet after this recall does not remove its earlier influence on episode selection. The reported 8.7-point belief-delivery gap is a direct post-recall contrast; the total contribution requires a lesion before retrieval. The measurements remain synthetic, single-model and concentrated on one calm/social axis. No new preference-formation or creativity result is established.
