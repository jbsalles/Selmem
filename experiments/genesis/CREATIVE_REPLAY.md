# Controlled creativity replay

This additive experiment reuses a completed 60-proposal preview. It does not
change GENESIS, the memory organ, the previous experiments or their results.
It neither encodes conversations nor runs additional sleep cycles. No Node or
Python is required. The only implementation dependencies are existing SelMem
configuration and HTTP transport APIs.

## What this experiment tests

The earlier run sometimes supplied task-solving text as reconstructed memory.
Here the reconstruction model receives only the original selected trace IDs
and their **current lived gists**, read from saved inspection JSON. It receives
no creative task, task identifier, query, constraints or resource counts. A
separate guard compares the retelling to those gists, also without the task.
It allows compression and tentative emotional interpretation, but rejects
invented events and solution plans. Rejected retellings fall back to the same
lived gist. The audit records every verdict and fallback. The guard is a model
judgment and can still miss errors; inspect the saved contexts.

Selection remains task-conditioned and is frozen from the original run. This
is a controlled readout experiment, not a rerun of the current retrieval engine.
The guarded conditions must not be described as unchanged native memory.
Vault files are hashed for integrity only. Their archive is never parsed or
supplied as runtime memory. Raw conversation turns, old proposals, encode-time
cores, latent unselected traces and axioms are not supplied to the models.

| Condition | Background |
| --- | --- |
| `empty` | None |
| `empty_two_step` | Provisional solution ideas generated without memory, then checked |
| `rag` | Original saved RAG context |
| `before_sleep_guarded` | Frozen pre-sleep selection, retold and guarded |
| `after_no_recon` | Frozen post-sleep selection, current lived gists |
| `after_full_guarded` | Frozen post-sleep selection, retold and guarded |

The direct gist arm is rendered anew from saved inspection JSON; it does not
reuse the earlier reconstructed context. The before-sleep snapshot uses the
original clock-matched `no-sleep` state. Only complete entries fitting the
600-word background budget are included. Context files contain the exact
background supplied to generation.

The no-memory two-step comparator has one draft call, one validation call and
two final generation calls per history/task cell. Each guarded memory cell has
one reconstruction, one guard and two final generation calls. Their ceilings
are respectively 1,200, 400 and 450 output tokens. Draft/recall/guard temperature
is 0; final generation temperature is 0.7. This matches call counts and token
ceilings, not actual tokens, semantic work or prompt lengths. Background is
shared by the two final repeats in each cell. Seeds are paired across conditions
and requested from the provider, which may not guarantee determinism.

There are two histories, three tasks, six conditions and two repeats: **72
proposals, 108 nominal HTTP calls**, plus bounded retries. The default 250-call
ceiling is an attempt budget, not a monetary cap. This reduces work by reusing
the existing 788-turn encoding. Actual latency and price depend on responses,
provider load and retries; no dollar or duration guarantee is made.

## Apply and run

Apply the release's `apply.sh` from the repository root. The patch only adds
new paths. It is suitable alongside the previous semantic fix and Rust tools;
no old patch needs to be reversed. An incompatible local file is left intact
and causes installation to stop.

The completed preview folder must contain its original `preview-design.json`,
`preview.json`, `all-proposals.json`, `contexts/` and `checkpoints/`.
Use a new sibling output folder. The output parent must already exist.

```bash
# Validate all inputs and all six vault checksums; make zero HTTP calls.
bash experiments/genesis/run-creative-replay.sh collect \
  --source experiments/genesis/results/creative-preview-002 \
  --out experiments/genesis/results/creative-replay-003 \
  --config .selmem --llm grok --model grok-4.3 --prepare-only

# Start the prepared replay.
bash experiments/genesis/run-creative-replay.sh collect \
  --source experiments/genesis/results/creative-preview-002 \
  --out experiments/genesis/results/creative-replay-003 \
  --config .selmem --llm grok --model grok-4.3 \
  --max-http-calls 250 --resume
```

Adjust `--config` to your existing runtime configuration file. Endpoint, model
and reasoning effort must match the source provider declaration. For this
source that is `grok-4.3` and `reasoning=none`. The request explicitly sends
`reasoning_effort`; it does not silently omit it. Credentials are read through
the existing configuration mechanism and are not copied into output files.
Saved provider responses can contain sensitive model output: keep the output
folder private until you select what to share.

## Interruption and audit

Run the same collect command with `--resume`. Validated completed responses are
cached by model, endpoint, request, cell and protocol. Different repeat seeds
remain distinct. Invalid responses never enter the cache. Resume refuses any
change to the source manifest, protocol, model, reasoning, mode or implementation.
The call ceiling may be raised if necessary. Do not edit source hashes to force
resume. HTTP 429/502/503/timeout failures have at most three attempts per call,
with 30/60-second waits; other malformed outputs also have at most three
attempts. A failed call pauses the run and leaves validated calls reusable.
Every attempt is counted across resumes.

A process killed without cleanup may leave `run.lock`. Remove that lock only
after confirming that no process is using this output folder. A truncated audit
line is a hard error: preserve the output and diagnose it rather than silently
discarding accounting records. Context files are rebuilt on resume from cached
responses. Final files are written atomically after all 72 proposals complete.
A complete resume makes no further provider calls.

Outputs include `replay-design.json`, `http.jsonl`, `responses.jsonl`, validated
`cache/`, exact `contexts/`, `all-proposals.json`, `blind-proposals.json`,
`private-blind-key.json` and `replay.json`. Original files remain untouched.

## Blind rating

Give each of two independent raters only `blind-proposals.json`, a separate
copy of the blank rating template, and these instructions:

- Rate utility and originality independently on 0–4 scales.
- Utility: 0 unusable, 1 poor, 2 workable, 3 useful, 4 unusually effective.
- Originality: 0 generic, 1 familiar, 2 some novel combination, 3 distinctive,
  4 unusually original and relevant.
- Mark `valid=false` for any hard task-constraint violation; both scores must
  then be zero. Explain the violation or the substantive basis for the scores.
- Use a distinct judge identifier; rate every proposal before comparing notes.

```bash
target/release/selmem-creative-replay template \
  experiments/genesis/results/creative-replay-003 judge-a.json
target/release/selmem-creative-replay template \
  experiments/genesis/results/creative-replay-003 judge-b.json
# Fill each file independently; then:
target/release/selmem-creative-replay combine ratings.json judge-a.json judge-b.json
target/release/selmem-creative-replay analyze \
  experiments/genesis/results/creative-replay-003 ratings.json
```

Templates and combined/report destinations must be fresh paths. Analysis
requires full coverage from at least two distinct judge identifiers, finite
scores, rationales and zero scores for constraint failures. Software cannot
verify actual judge independence. The report gives per-history mean utility
and originality differences between the guarded full arm and each comparator.
With two histories, three tasks and two repeats these are exploratory,
descriptive comparisons, not significance tests or evidence of generality.
Keep old and new lots separate; they have different generation protocols.

## Local verification without provider calls

```bash
cargo test --bin selmem-creative-replay -- --test-threads=1
bash experiments/genesis/run-creative-replay.sh collect \
  --source experiments/genesis/results/creative-preview-002 \
  --out experiments/genesis/results/creative-replay-mock --mock
```

Mock mode makes no provider calls, labels outputs as mock and is refused by
analysis. It exercises the workflow using the real saved input structure.
