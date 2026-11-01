# Native causal recall: diagnosis and v2 preflight

## Evidence and scope

Source: `causal-native-12-002.tar.gz`, SHA-256
`5220ecc4d77675d301dbfa3412d27a4ef3fb11f867079218ffde0d8a8b7a2b2e`.
The archived v1 actor run contains 3,456 valid Grok-4.3 responses, reasoning none.
Across all 192 life/scope/wording/order cells, native self-view was empty and all
384 delivered episodes were unrelated background observations. Full, no_beliefs
and swapped sent identical requests at each matched cell/repetition. Their
17/576 and 25/576 decision differences cannot be interpreted as belief effects.
The original full condition matched historical behavior in 292/576 choices;
repeat disagreement was 16/384 (4.17%, relative to repetition zero).

The diagnosis uses the exact 24 archived ON snapshots. Histories, admission,
consolidation, native belief statements and decision questions are not rebuilt.

| Recall route | Applicable typed beliefs in store view | Delivered conditional beliefs | Topic-relevant delivered episodes | Cells passing exposure |
| --- | --- | --- | --- | --- |
| legacy | 1 per cell | 0 per cell | 0 of 2 per cell | 0/192 |
| context | 1 per cell | 1 per cell | 0 of 2 per cell | 0/192 |
| context-cued | 1 per cell | 1 per cell | 2 of 2 per cell, at least one with matching company | 192/192 |

These are offline delivery checks, not new actor results or evidence of a
behavioral advantage. Relevance labels are used only to validate the output,
never to choose a trace or construct a retrieval query.

## Two different causes

1. `open_mouth_scoped` attests facets such as `company=alone`, but did not accept
   the separate application context `choice-pattern`. `view_scoped` checked this
   context lexically against the held-out question, found no match, and delivered
   no belief. Explicit typed context fixes this routing mismatch without inserting
   `choice-pattern` into the actor's question or weakening scope guards.
2. Episodic recall uses a context cloud with a lexical/hash fallback. On these
   queries it selected background scenes. Supplying context alone does not repair
   that selection. The new **opt-in** `context-cued` route uses the gists of
   already-applicable native self-view entries as internal retrieval cues. The
   existing episodic scorer, eligibility gates and reconstruction still choose the
   scenes. It does not force supporting trace IDs, consult expectations, use
   ground-truth action labels, or read the sealed archive.

`context-cued` is an explicit mechanism addition, not a retrospective claim that
v1 already delivered useful memory. It remains distinct from `context` and the
original `legacy` route. No global lexical scoring or hash thresholds were tuned
to this fixture. Default untyped callers retain the original recall path.

Native episode JSON now preserves each episode's stored `scope_facets`, so an
alone episode and a friends episode remain distinguishable if recalled together.
This is observed metadata, not an inferred preference or an added action prediction.

## Fail-closed v2

The replacement dispatcher loads `causal_v2.rs`; existing `causal.rs` and v1 actor
artifacts may remain for audit, but cannot be resumed as v2. Each fresh run first
checks all input paths before creating output, copies the snapshots, and freezes
all deliveries. It then writes:

- `delivery-audit.jsonl`: raw and typed self-view IDs, delivered belief counts,
  topic/scoped episode counts, native packet, leading scored candidates, donor
  delivery and whether donor statements actually differ.
- `exposure.csv`: the stage counts for every query cell.
- `preflight.json`: pass/fail and frozen-file checksums.
- `payloads.jsonl`: exact actor requests, including identical repetitions.

An actor run is refused unless **every** cell has two qualified stored children,
one delivered conditional belief, a topic-relevant episode with the requested
company facet, and a distinct mirror belief. Failure occurs before key lookup,
HTTP or choices-file creation. Resume enforces the same gate, version/settings,
and accidental-edit checksums. Checksums detect accidental changes; they are not
an authentication signature. There is no bypass flag. A successful preflight
proves exposure, not that an actor will use the material correctly.

The six delivery arms, both option orders for each wording, identical repetitions,
randomized interleaving, raw answer logging, error accounting, matched decision
comparisons and repeat-noise estimates are retained. Full delivers the native
packet unchanged. Cuts do not rerun recall, refill removed text, or expand the
remaining statements. Swapped replaces the delivered conditional belief while
retaining the original episodes. Placebo matches serialized user-payload bytes,
not provider tokens. Pending and invalid responses are excluded from valid pairs;
report rows repeat run-wide invalid/pending counts. Repetitions are not independent
histories and must not be treated as such for inferential uncertainty.

Because context-cued recall can mediate belief influence through episodes,
`no_beliefs` is a **post-recall direct-delivery ablation**, not deletion of every
causal contribution of belief formation. Testing the mediated path separately
would require a pre-retrieval intervention. Swapped intentionally tests conflicting
belief/episode delivery; it does not swap the agent's developmental history.

A misplaced sweep helper was also moved from the Cargo auto-binary directory into
`selmem-contextual-longlife/sweep.rs`. Its contents are unchanged. This fixes its
accidental compilation as a standalone binary during integration/all-target tests.

## Apply after the original native-causal patch

```bash
git apply --check ~/Downloads/selmem-native-causal-recall-v2.patch
git apply ~/Downloads/selmem-native-causal-recall-v2.patch
cargo test --all-targets
```

This is an incremental patch over main `8634132` plus the original
`selmem-native-causal-ablations.patch`. It does not require reapplying that patch.
All implementation and tests are Rust; no Python or JavaScript experiment tooling
is added.

## First: qualify all 12 domains, zero API calls

Reuse the snapshots already present in the completed v1 run. Use a new output
folder; do not reuse v1 results or the diagnostic folders from an older build.

```bash
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation \
  --snapshots experiments/genesis/results/causal-native-12-002/snapshots \
  --out experiments/genesis/results/causal-v2-qualified-12-001 \
  --pairs 12 --repeats 3 --memory-words 600 \
  --recall-route context-cued
```

Expected: `preflight PASS: 192/192 cells`. `legacy` and `context` are available
for reproducing the negative delivery diagnostics and should fail the gate on
these snapshots. Do not alter questions, beliefs or snapshots to force a pass.

## Then: a bounded one-domain actor pilot

A complete one-domain pilot has 288 calls. The cap below permits 48 attempts per
invocation, with no automatic retry. Set `GROK_API_KEY` privately in your shell.

```bash
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation \
  --snapshots experiments/genesis/results/causal-native-12-002/snapshots \
  --out experiments/genesis/results/causal-v2-pilot-001 \
  --pairs 1 --repeats 3 --memory-words 600 \
  --recall-route context-cued \
  --actor --provider grok --model grok-4.3 --max-http-calls 48
```

For subsequent chunks, repeat with `--resume`. Do not use `--resume` on the first
invocation. After the pilot, the 12-domain frozen plan can be run with its original
flags plus `--resume --actor --provider grok --model grok-4.3 --max-http-calls 48`.
That full actor plan is 3,456 attempts. The cap is per invocation, not monetary;
an interrupted in-flight request may incur an unrecorded charge. HTTP failures
are logged as invalid and are not silently replaced.

Do not interpret partial paired counts as completed evidence. Compare complete
paired flip rates with repeated identical-request noise, and analyze independent
domains/scopes before pooling. No paid actor run was performed for this fix.
