# Same-checkpoint GENESIS capacity replay

This bounded replay targets `self-pilot-v3-grok-real-001`, whose original 128
choices favored `check` on 123 occasions. It uses supplied real checkpoints,
original tasks, option orders, seed requests and actor instructions. It does not
regenerate histories, perform new sleep, expose the archive, or introduce self
beliefs into the actor prompt. It measures a capacity intervention on the stored
lived-gist readout, not a new experiment on learned identity.

## Offline finding on the supplied archive

All 128 checkpoint references match their SHA-256 hashes. Historical contexts
contain only 25–67 words: the 320-word ceiling was not binding. The old selector
was capped at four episodes. For the replay's `full_loop` subset, current legacy
readout reproduces the historical context exactly in all 16 cells. Expanded
selection adds one or two episodes in eight cells; the other eight cells provide
matched unchanged-context controls. Expanded contexts contain at most 86 words.
Thus the primary intervention is episode count, not removal of a binding word
ceiling. An 8000-byte context cap is a conservative bound, not 8000 model tokens.

## Fixed pilot

Take only the original full-loop condition and original repeat zero:
2 histories × 2 phases × 2 tasks × 2 option orders = 16 source cells. For each:

- `legacy`: recompute the original Transfer readout, four candidates, 320 words.
- `expanded`: same selector, 24 candidates, whole stored gists within 8000 bytes.
- `bare`: same task and options, empty `past_observations`.

Two identical repeats per condition produce **96 planned requests**. The seed
from source repeat zero is reused for both new repeats, making their entire
request identical. This differs from the original pilot's two requested seeds.
Requests use Grok-4.3, reasoning none, temperature zero, 64 output tokens and the
original life-choice system instruction. Condition names and trace IDs are not
sent to the actor. Requests are interleaved with a deterministic hash ordering.

The before/after checkpoints are independent probes, not new continuing lives.
Only two underlying scripted histories exist; repetitions and option orders are
not independent histories. Choice divergence is measured; neither offered action
is labelled intrinsically correct. No accuracy or general alignment claim follows.

## Run

Extract the original archive so the source directory contains `trials.json`,
`report.json`, `design.json` and `checkpoints/`.

```bash
cargo test --release --bin selmem-genesis-budget-replay
cargo run --release --bin selmem-genesis-budget-replay -- \
  --source experiments/genesis/results/self-pilot-v3-grok-real-001 \
  --out experiments/genesis/results/genesis-capacity-replay-001
```

Expected on the supplied source: 96 planned requests; extra-episode cells 8/16;
historical readout drift 0/16. Preparation needs no key or network request.
Inspect `preflight.json`: it includes original/current/expanded sizes, exact
contexts, selected IDs and historical drift. If no cell gains an episode, actor
execution is blocked before key lookup. Historical drift is explicitly reported:
when nonzero, new legacy-vs-expanded remains a contemporary comparison, but cannot
be interpreted as a pure replay of the old model-visible memory.

With `GROK_API_KEY` configured:

```bash
cargo run --release --bin selmem-genesis-budget-replay -- \
  --out experiments/genesis/results/genesis-capacity-replay-001 \
  --resume --actor --max-http-calls 96
```

A smaller positive per-invocation ceiling is supported. HTTP attempts and outcomes
are append-only; execution stops on the first API/response error. Ordinary resume
skips recorded errors. Add `--retry-errors` explicitly to retry them and finish
missing trials; successful or malformed model answers are never resampled.
Unknown-outcome attempts block further calls. Keep the lock unless the process
is known to have stopped. Old output directories are not overwritten.

The original trials and metadata plus all referenced checkpoints are copied into
the new output. Requests, qualification and four compiled source components
(runner, engine, retrieval, memory context) have SHA-256 fingerprints. Resume uses
frozen requests, not freshly selected evidence. The fingerprint does not cover
all repository files; keep the Git revision and source archive with the results.

## Read results

`report.json` gives `check`/`consult` counts, missing/invalid responses, paired
legacy/expanded/bare differences, identical-repeat disagreement, and matched
coordination-vs-verification history differences. `responses.jsonl` exposes the
latest answers. `choices.jsonl` retains every attempt's raw provider response.

Analyze unchanged-context cells and bare repetitions as the current variability
control, rather than importing the noise rate from another experiment. Stratify
before/after correction using `plan.jsonl` and `source-trials.json`. If enlarged
readout adds episodes but choices still converge, this strengthens the conclusion
for these tasks only. If choices diverge, inspect whether the added episodes
explain the difference and whether it exceeds identical-request variability.
There is no automatic significance claim and no learned-memory baseline here.

```bash
tar -czf genesis-capacity-replay-001.tar.gz \
  -C experiments/genesis/results genesis-capacity-replay-001
```
