# Normative memory pilot v12: wide context

This experiment accompanies the opt-in engine described in
[Normative memory](../../docs/NORMATIVE_MEMORY.md). Engine patch base: `cd64be0`
plus normative grounding v11. The v12 update expands context capacity while
retaining the four-arm, 192-request task grid.
The runner and all engine changes are Rust, with no new dependencies.

## Completed run

`normative-wide-012` completed 192/192 valid Grok 4.3 responses, without API
errors or retries. Normative delivery reached **46/48**, versus **41/48 native**,
**40/48 unfiltered anchors**, and **46/48 scoped registry**. The five corrected
responses were verification instead of required refusal; authorized deliveries
remained 18/20 for native and normative, with no unauthorized delivery in any arm.
The actual native context contained all applicable rules without clipping.
See [the result analysis](NORMATIVE_WIDE_RESULTS.md) and
[compact response rows](../evidence/normative-wide-012/answers.jsonl).
The original design below is retained; a new run is a replication, not a resume
of the published collection.

## Design

Two mirrored permission histories, before/after scoped changes, six tasks, four
conditions and two identical repeats: **192 model requests**, 96 unique arm/cell
combinations. Each condition sees the same source history, including 80 inventory
records and eight requester claims. Histories use Cedar/Birch, Mira, amber/teal
and an unlisted violet category. The post-change history both grants a narrow
inspection exception and withdraws a permission for planning in the other project.

| Arm | Delivered evidence | Purpose |
| --- | --- | --- |
| `native` | Ordinary `open_mouth` episode narratives | Existing engine baseline |
| `anchored_all` | All registered anchors, canonical rendering, chronological | Format/anchor control without scope composition |
| `normative` | Actual `open_mouth_normative` delivered block | New engine path |
| `scoped_registry` | Full original owner records for the task scope | Privileged comprehension reference |

All snapshots contain the same admitted history and annotations. The canonical
arms receive caller-supplied structured semantics at intake; this is extra
structure, not automatic extraction discovered by SelMem. The full-anchor arm
helps distinguish composition from that representation change. The registry is
not an independently learned memory system. Native versus normative also changes
retrieval and context composition; it cannot isolate grounding alone.

The new names do not make this an independent held-out benchmark. Five task forms
are close to v9; the sixth tests a scoped withdrawal. This is an implementation
regression/comprehension pilot designed after seeing v9. Offline tests additionally
cover forgery, conflicts, missing parents, scope uncertainty, corruption of gist,
revisions, forgetting, persistence and budget exhaustion. They do not establish
that a model handles these cases; the paid pilot does not cover all of them.

Default evidence ceiling: **6,000 words** for every condition, including canonical
provenance/conditions. Both native and normative engine paths use the new capacity
controls: 64 episodes, 800 words per episode, and a 64,000-byte bound on serialized
memory through the default token counter. This is a conservative UTF-8 byte bound,
not 64,000 measured model tokens. The complete actor request is also checked
against 64,000 serialized bytes. Engine defaults outside this pilot are unchanged.

Qualification on the fixed 80-distractor histories gives:

| Arm | Actual evidence words | Delivered episodes | Full original scoped rules |
| --- | ---: | ---: | ---: |
| native | 1,213–1,214 | 64 | 24/24 cells |
| normative | 17–38 | typed rule bundle | canonical representation |
| anchored_all | 34–76 | typed anchors | canonical representation |
| scoped_registry | 17–37 | original owner records | 24/24 cells |

Native serialized memory is 19,654–19,658 bytes; complete native requests are
9,542–9,550 bytes. All selected native episodes are delivered, without clipping.
No padding or duplication is added to short normative bundles. They are shorter
because the supplied rule families are small. Actual lengths are recorded in
`preflight.json`; native packet sizes and settings are in `recall-audit.jsonl`.
The wide preparation gate rejects a clipped or partially delivered native
selection, or a packet/request above its ceiling. Relevant-rule coverage is
reported separately; retrieval failures remain in the denominator.

These settings differ from v9 and v10/v11. Compare arms within this new run;
do not attribute cross-run score changes uniquely to grounding. In particular,
the large native context now contains every relevant rule, so a native gain can
reflect the capacity change. Requests use the same system
instruction and task in all arms: Grok-4.3, reasoning none, temperature 0, maximum
160 tokens, text action `deliver`, `decline` or `verify` and a short explanation.
The runner evaluates delivered evidence with this common actor prompt, not the
full conversational `MemoryReply` prompt. No extra actor call validates an answer.

## Qualification and accounting

Preparation checks the full fixed grid, request construction, oracle labels,
evidence ceilings, native delivery audits, exact registry extraction and replay
of normative composition from frozen snapshots. Normative delivery must be
complete, without review flags, for this pilot. Every arm stays in the plan.

Unlike the v9 comprehension-only gate, **native coverage failure is reported,
not removed and not a reason to filter a case**: this pilot measures the ordinary
path end-to-end. Preflight's 96/96 count means protocol validity, not 96 complete
native recalls. Inspect the per-cell coverage field separately (it compares full
original text for native/registry and is not meaningful for canonical formatting).

Plan SHA-256, compiled-source component fingerprint, frozen requests, append-only
attempts, per-invocation HTTP ceiling, first-error stop and explicit error retries
follow v9. The fingerprint covers the runner and 11 central source files, not the
whole repository. Retain the Git revision with results. Each probe reopens its
checkpoint; no recall rehearsal carries from one task to another. Repeats have
identical request bytes and do not count as independent tasks.

## Run

```bash
cargo test --release --test normative_grounding --bin selmem-normative-pilot
cargo run --release --bin selmem-normative-pilot -- \
  --out experiments/genesis/results/normative-wide-012 --memory-words 6000
```

Expected: `Prepared 192 requests. Preflight: 96/96 valid protocol cells.`
Preparation uses no API key and makes no network calls. With `GROK_API_KEY` set:

```bash
cargo run --release --bin selmem-normative-pilot -- \
  --out experiments/genesis/results/normative-wide-012 --memory-words 6000 \
  --resume --actor --max-http-calls 192
```

A smaller positive cap is supported for staged runs. If an API error stops the
run, ordinary resume preserves/skips the failed trial; use `--retry-errors`
explicitly to retry API/response errors while completing missing trials. This
never retries valid or malformed model answers. Unknown-outcome attempts block
execution. Do not delete attempts to bypass that block; check for an active
process before removing a stale lock. Existing directories are never overwritten.

```bash
tar -czf normative-wide-012.tar.gz \
  -C experiments/genesis/results normative-wide-012
```

## Readout

`report.json` reports correctness, unsafe delivery, false refusals, unnecessary
verification, repeat disagreement and each paired normative/control comparison.
`diagnostics.json` contains confusion counts, paired unlisted-category decisions
before/after changes, and every error with its exact evidence and answer.
`responses.jsonl` is the latest-response view. Keep `choices.jsonl`, `attempts.jsonl`,
`plan.jsonl`, snapshots, histories, native and normative audits with the report.

There are 24 distinct history/task cells per arm, 48 responses including repeats.
A blanket refusal scores only 20/48; authorization, refusal and verification must
all be correct. Improvement over native with parity to the scoped registry would
support the utility of this engineered memory path on this task family. It would
not demonstrate a unique benefit of lossy reconstruction or general alignment.

Use a fresh output directory after installing v12. Earlier prepared runs are
incompatible with the changed source fingerprint and capacity settings.
