# Text-format comprehension pilot after recall v7

This standalone binary preserves the earlier retention modes and their frozen
plans. It asks whether Grok can use the contexts now delivered by native recall.
It is a qualified diagnostic, not a new general alignment benchmark.

## Fixed design

- Same histories and oracle as the prior retention pilot: two mirrored permission
  histories, before/after a scoped exception, five tasks at each checkpoint.
- Two conditions: native SelMem delivery and the owner-record registry.
- Two identical repeats: 20 history/task cells x 2 conditions x 2 repeats = 80 requests.
- Default: 80 distractors and a 128-word evidence ceiling for both conditions.
  These are equal ceilings, not guaranteed equal delivered word counts.
- Grok-4.3, reasoning none, temperature 0, maximum 160 output tokens.
- Plain-text evidence and task; first response line `deliver`, `decline`, or
  `verify`, followed by a short justification. No numbered response options.

Both conditions receive the same original history. Native evidence is produced
by `open_mouth`, using the actual task without numbered options. Each probe
reopens its checkpoint so one probe cannot rehearse memories for the next.
Only the delivered episode narratives become model evidence. The registry
extracts owner records from the common history in chronological order, within
the same ceiling. It is a privileged comprehension reference; it is not an
independently learned memory system. Native evidence may additionally contain
inventory and requester records. This comparison does not isolate those effects.

## Qualification and audit

Preparation records admission, checkpoint snapshots, native delivery and all
80 frozen requests. The preflight checks 40 condition/cell combinations (not
the duplicate repeats): complete original rule for the queried project, plus
the Vela exception on post-revision Vela queries, within the evidence ceiling.
Exact retained record text is required for this comprehension control.

All cells remain in the plan. If any fails, the entire actor phase is blocked;
no successful subset is silently selected. Preflight is rechecked on resume.
It also checks the complete trial grid, the oracle, request construction, and
agreement between native evidence and the recorded delivered episodes.

The config fingerprints the compiled runner and seven central engine source
files (engine, retrieval, dependencies, memory context, encode gate/scoring and
profile). This is not a fingerprint of every repository file. `plan.sha256`
protects frozen request bytes. Resume sends the frozen requests; it does not
reconstruct them from the snapshots. Keep the repository revision with results.

## Run

Apply this patch on top of v7 (GitHub base `af1e464`). Prepare offline first:

```bash
cargo test --release --bin selmem-retention-text-pilot -- --test-threads=1
cargo run --release --bin selmem-retention-text-pilot -- \
  --out experiments/genesis/results/retention-text-pilot-008
```

Expected: `Prepared 80 requests. Preflight: 40/40 complete cells.`
No API key is needed for preparation. With `GROK_API_KEY` configured:

```bash
cargo run --release --bin selmem-retention-text-pilot -- \
  --out experiments/genesis/results/retention-text-pilot-008 \
  --resume --actor --max-http-calls 80
```

The ceiling counts actual attempted HTTP calls in this invocation. The program
stops at the first HTTP/provider-response error and never expands automatically.
On an ordinary resume, recorded errors are preserved and skipped. To explicitly
retry those errors as well as finish unattempted trials:

```bash
cargo run --release --bin selmem-retention-text-pilot -- \
  --out experiments/genesis/results/retention-text-pilot-008 \
  --resume --actor --retry-errors --max-http-calls 80
```

Every attempt is kept in the append-only logs; reports use the latest completed
attempt per trial. Successful responses are never resampled. A malformed answer
such as a digit or a missing justification is an invalid model response, not an
API error, and is not retried by this flag. An attempt with no recorded outcome
blocks further calls because its outcome is unknown. Do not delete that attempt
to force a retry. A stale `run.lock` requires checking that no process is running
before manual removal. Existing output directories are never overwritten.

## Read the results

- `report.json`: valid/invalid/missing counts, correct choices, unsafe deliveries,
  false refusals and unnecessary verification on authorized cases, by arm/phase.
  It also reports repeat disagreement and paired native/registry differences.
- `responses.jsonl`: last response, expected action, validity and error per
  recorded trial. Invalid responses remain visible; they are not counted correct.
- `choices.jsonl` and `attempts.jsonl`: all original attempts, including failures.
- `preflight.json` and `recall-audit.jsonl`: actual exposure and qualification.

The denominators are small: 20 valid responses per arm/phase at most, including
repeats of ten unique history/task combinations. Justifications are diagnostic
text, not evidence of the model's internal reasoning.

If the registry still fails, the comprehension problem remains. If native is
worse, inspect its delivered context and added noise. If both succeed, the
pipeline has passed this control but no SelMem-specific advantage is shown.
A native advantage here remains exploratory and must be tested on a frozen,
independent set against stronger memory baselines before making alignment claims.
