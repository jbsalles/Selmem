# LoCoMo strategy experiment

This is the single active LoCoMo-derived experiment. The memory organ is frozen;
changes to its encoding, retrieval, sleep rules or constants require a separate
review and a new experimental baseline. The retained runs use the same frozen
revision recorded in their manifests. [Organ baseline](ORGAN_AUDIT_FIXES.md).

The experiment asks whether different retained observations change Claire's
advice and game design. It does not measure the full LoCoMo benchmark, and it
does not yet demonstrate individuality or improved creativity.

## Data and intervention

The frozen protocol is [data/locomo_strategy.json](../data/locomo_strategy.json).
It uses `conv-26` from the official LoCoMo text release. The launcher downloads
the file with curl when needed and checks its SHA-256; `--dataset FILE` accepts
an existing copy of that exact release. The full corpus is not bundled here.
The retained reports contain derived excerpts and responses; consult the
upstream dataset license before redistributing or using those artifacts.

Both arms observe the same chronological conversations. Arm A gives additional
salience and permanence to `D1:7` (support); arm B to `D12:1` (a hurtful hike).
The protocol's event valences apply on both sides; the intervention changes
which observation is emphasized, rather than inventing a different conversation.
All turns are attributed `External`: Caroline's and Melanie's experiences are
observations, not Claire's biography. Virtual time advances between sessions;
full arms run sleep and nosleep arms skip it. Readout probes do not rehearse or
mutate the saved memory. Preparation uses rules, HashEmbedder and NullScorer;
the chosen LLM generates the final answers only.

| Condition | Supplied context |
| --- | --- |
| `full_a`, `full_b` | Arm's selected traces, scoped interpretations and reading disposition after sleep |
| `transplant_a_from_b`, `transplant_b_from_a` | Exactly the donor's readout and supplied context, checked before calls |
| `nosleep_a`, `nosleep_b` | The corresponding arm prepared without sleep |
| `full_null_a`, `full_null_b` | No focused event; two independently prepared null books |
| `raw_a`, `raw_b` | Original dataset turns for full arm's selected source IDs, without interpretations or disposition |

The raw baseline reads the source dataset in the experiment adapter, never the
sealed archive in the organ. It can restore a whole turn for a selected fragment
and repeat a turn when multiple selected fragments share a source. It is therefore
not a matched "same fragments minus interpretations" ablation.

Six English probes cover launch, disagreement, retry, cooperative game design,
and two factual controls (support and hike). The first four test transfer;
factual controls test supported answers or abstention. Questions, system prompt
and responses are English. No preferred A/B strategy is prescribed.

## Configure the LLM

Use the existing `.selmem` discovery, `SELMEM_CONFIG` or `--config FILE` mechanism.
Place global settings before named provider blocks:

```ini
reasoning=none
http_timeout=60

[grok]
model=grok-4.3
api_key=YOUR_XAI_KEY

[gpt]
model=gpt-5.1
api_key=YOUR_OPENAI_KEY
```

Fill keys locally. `--plug` chooses the block; `--model` overrides its model.
A direct `llm=` endpoint works through the same configuration resolver. Paid
runs require an LLM; there is no rules fallback. Provider and model parameter
support still applies. The launcher transmits the requested reasoning effort
and its protocol temperature; use `--temperature 0` for the retained stability
comparison. Temperature zero does not guarantee identical responses.

## Run locally

From the repository root, inspect the actual context before spending calls:

```bash
./run.sh test --bin selmem-locomo --test situational_transfer
./run.sh run --release --bin selmem-locomo -- \
  --dry-run --panel all --temperature 0 --repeats 3 \
  --output experiments/strategy-preparation
```

A dry run makes no LLM calls. It may download the dataset if absent. Inspect
`preflight.json`, `interpretations.json`, and the placeholder rows' contexts.
The offline gate blocks paid calls when a transfer probe has no evidence,
all contexts for it are identical, or sleep changes no transfer context.
Passing it only establishes that a comparison has a measurable input difference.

Use a new output directory for each run:

```bash
./run.sh run --release --bin selmem-locomo -- \
  --panel all --plug gpt --model gpt-5.1 \
  --reuse-memories experiments/strategy-preparation \
  --temperature 0 --repeats 3 --max-calls 180 \
  --output experiments/strategy-gpt-new

./run.sh run --release --bin selmem-locomo -- \
  --panel all --plug grok --model grok-4.3 \
  --reuse-memories experiments/strategy-preparation \
  --temperature 0 --repeats 3 --max-calls 180 \
  --output experiments/strategy-grok-new
```

Reuse checks the protocol, dataset, sample, frozen organ revision and the hashes
of the six prepared snapshots. It copies those snapshots and preparation audits
without encoding or sleeping again. Deleted `selection_*` arms are not required.
Without `--reuse-memories`, preparation runs afresh. Existing output directories
are rejected to prevent overwriting a run.

Defaults are all ten conditions, five repeats and a 300-call ceiling. The
protocol's stored temperature is 0.7; the stability commands explicitly override
it to zero. `--wow` remains a compatibility alias for this sole experiment.
Panels `core` (four conditions) and `controls` (six) are exploratory subsets:
paid subset runs or fewer than three repeats require `--exploratory`. Do not
present those subsets as controlled comparisons.

## Inspect and rate

Open `strategy.html` from a run directory. Rate every repetition before revealing
conditions; export ratings from the page. Ratings persist in browser localStorage,
not in the repository. `report.html` is a simpler response viewer.

| Artifact | Purpose |
| --- | --- |
| `manifest.json`, `protocol.json` | Counts, settings, hashes and frozen protocol |
| `responses.json`, `responses.jsonl` | Answers, exact supplied contexts, readouts, request hashes and usage |
| `failures.json`, `failures.jsonl` | Missing observations and sanitized provider errors, when present |
| `*.selmem`, `*.book.json`, `*.audit.jsonl` | Saved state, book inspection and encode/sleep before-and-after evidence |
| `interpretations.json` | Scoped interpretation operations and their supporting observations |
| `preflight.json` | Offline input checks before provider calls |
| `sampling.json` | Within-context and across-condition lexical variation diagnostics |
| `strategy.html`, `blind_review.csv`, `reveal.json` | Review interfaces and condition lookup |

A provider error is recorded and later calls continue. Successful answers remain
available. There is no fabricated replacement or automatic retry; failed calls
may still be billed. Completion-length finish reasons must be checked before
rating truncated answers. Lexical distances and their pair counts are descriptive:
pairs reuse responses and are not independent experimental replicates.

## Retained observations

See [RESULTS.md](../experiments/RESULTS.md) for the Grok and GPT run summaries.
They suggest that supplied context can affect a concrete recommendation, with
strong variation by model and probe. They do not isolate a benefit of sleep or
of interpretation from ordinary prompting. Game rules can remain inconsistent.

The next evaluation should hold selected fragments constant while comparing
fragments alone, those same fragments plus stored interpretations, and no such
memories. That matched ablation is proposed, not implemented in the retained
experiment. Keep the organ frozen while evaluating it; do not tune retrieval
thresholds toward the desired answers.
