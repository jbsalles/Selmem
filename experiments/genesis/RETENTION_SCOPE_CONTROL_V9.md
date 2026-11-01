# Scope control and verification diagnostic (v9)

This standalone Rust runner follows the v8 result (native 36/40, full registry
32/40). It leaves the v8 runner, engine, prompts and historical results intact.
Apply on recall v7 (`af1e464`) or later with v8 present. No new dependencies.

## Frozen design

Twenty history/task combinations, three arms, two identical repeats: **120
requests**, 60 unique arm/cell combinations. The three arms are interleaved
using a fixed shuffle. Each receives the same original history:

| Arm | Model evidence |
| --- | --- |
| `native` | Actual native delivered narratives, including any noise |
| `registry` | All owner records in original chronology, as in v8 |
| `scoped_registry` | Exact owner records for the task's project and recipient, in chronology: v1 then v2 |

The scoped arm retains the entire rule, including both permitted and prohibited
categories and the verification clause. It does not select by expected answer,
category or purpose. Only Vela receives the Vela exception. The fixed synthetic
grammar makes this a privileged deterministic reference, not a general parser
or a learned memory baseline. Both registries already use chronological order:
the new arm removes foreign-scope records and makes the parent and exception
adjacent. It does not independently isolate ordering from scope filtering.

Unchanged: mirrored histories, five tasks, before/after exception, 80 inventory
distractors, 128-word evidence ceiling, Grok-4.3, reasoning none, temperature 0,
160 output tokens, text actions and short justifications. Equal ceilings do not
mean equal actual context lengths. No model instructions are tuned to the v8
errors. Native retrieval starts from a fresh checkpoint for every task.

Preparation must pass **60/60** exposure checks. A failure blocks all actor calls.
Frozen request hashes, source fingerprint, append-only attempts, explicit retry,
unknown-outcome blocking and lock behavior follow v8. No v8 responses are reused:
all three arms are measured contemporaneously. The user runs paid calls locally.

## Run once

From the repository root:

```bash
cargo test --release --bin selmem-retention-scope-control
cargo run --release --bin selmem-retention-scope-control -- \
  --out experiments/genesis/results/retention-scope-control-009
```

Expected: `Prepared 120 requests. Preflight: 60/60 complete cells.`
With `GROK_API_KEY` configured:

```bash
cargo run --release --bin selmem-retention-scope-control -- \
  --out experiments/genesis/results/retention-scope-control-009 \
  --resume --actor --max-http-calls 120
```

The cap applies per invocation and execution stops on the first API error. An
ordinary resume skips recorded errors; add `--retry-errors` explicitly to retry
API/response errors and finish missing trials. Valid or malformed model answers
are never resampled. Unknown-outcome attempts block further calls. Do not delete
attempts to bypass that block. Never remove a lock without checking the process.

Archive the entire output directory:

```bash
tar -czf retention-scope-control-009.tar.gz \
  -C experiments/genesis/results retention-scope-control-009
```

## Read the diagnostic

- `report.json`: phase/arm scores, unsafe deliveries, false refusals, identical
  repeat disagreement, all three paired arm comparisons.
- `diagnostics.json`: complete expected-action/observed-action confusion counts
  by arm and phase, with planned/missing/invalid denominators; paired silver
  actions before/after exception; every recorded error with task, exact evidence
  and verbatim answer for manual justification inspection.
- `responses.jsonl`: latest responses; `choices.jsonl` and `attempts.jsonl` retain
  all attempts. `plan.jsonl`, its checksum, snapshots and audits preserve inputs.

For expected `verify`, inspect the `decline` count separately from correct
refusals on expected `decline`. The silver transitions reveal whether the same
mirrored case changes from verification to refusal after the exception. Null
transitions are missing or invalid actions; consult responses/confusion counts.
No semantic rationale classifier is claimed: written explanations can contradict
actions and do not expose internal reasoning. The diagnostic costs no extra calls.

If scoped registry closes the native gap, scope filtering/context composition is
a plausible explanation. If both still confuse unknown with prohibited, this
local comprehension failure persists despite complete exposure. A remaining
native gap still combines content, length, order and noise effects; it does not
prove a reconstructive-memory mechanism. These are 20 distinct tasks per arm,
not 40 independent examples. The v9 control was designed after inspecting v8;
any broader claim needs an independent held-out evaluation. No automatic run
expansion or production retrieval change is included.
