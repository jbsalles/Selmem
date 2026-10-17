# GENESIS

Four maintained experiments, one Rust executable. [Protocol and limitations](protocol.md), [result ledger](../RESULTS.md), [native observation support](../../ARCHITECTURE.md).

## 1. Qualify the decision model

```bash
bash experiments/genesis/run-positive-control.sh experiments/genesis/results/positive-new
```

Uses `.selmem` or `SELMEM_CONFIG`, Grok 4.3 and GPT-4o-mini-2024-07-18 by default. `GROK_MODEL` and `GPT_MODEL` override models. Each receives 48 decisions: two names, reliable/unreliable/empty histories, two costs, two option orders and two repetitions. 96 requests are planned; the ceiling is 120 attempts including retries. No memory, scorer or reconstruction calls occur.

A complete run qualifies only with 48/48 oracle choices. This strict engineering gate was fixed before the retained run; it is not a statistical threshold. Mock results cannot qualify. Failed qualification remains a result and does not prevent the other provider from being tested. Transport/schema failures preserve partial outputs.

## 2. Compare observation support OFF/ON

```bash
bash experiments/genesis/run-evidence-learning.sh \
  experiments/genesis/results/positive-new/grok \
  experiments/genesis/results/evidence-new
```

Only a real, complete, qualified positive-control source is accepted. Model names must match that source. Grok is the default; GPT-mini did not qualify in the retained run. This command does not automatically rerun qualification or replay an old life.

Each cut receives 24 inputs (two twelve-report histories), four readonly final recalls and 72 decisions. Combined: 48 inputs, eight readouts and 144 decisions. Internal calls vary; ceilings are 800 attempts per cut and 1600 total, including retries. The retained pair used 475 attempts. A ceiling is not an expected request count.

Compare full faithful history, admitted observations, native recall, strict recall and empty context. Support updates retain information without a new trace: `kept=true`, `new_trace=false`, `salience_evaluated=false`; score=0 means not evaluated. Ordinary new-trace salience thresholds are unchanged. No sleep occurs within these twelve-event lives.

Outputs include design/configuration, fixtures, encoding receipts, life/clone snapshots, readout audits, decision schedule, raw responses and HTTP journal. Fresh outputs are required. Both scripts create `.tar.gz` evidence packages after successful completion; failed runs retain their directories and logs.

## Direct and offline commands

```bash
./run.sh run --release --bin selmem-genesis -- --help
./run.sh run --release --bin selmem-genesis -- positive --mock --out /tmp/positive-check
./run.sh run --release --bin selmem-genesis -- learning --mock \
  --source /tmp/positive-check --out /tmp/learning-check --observation-evidence on
./run.sh test --bin selmem-genesis
```

`--config FILE` loads the runtime file before either command. The default is normal `.selmem` discovery. Progress is verbose; `operation #` is nested work, `HTTP #` counts actual attempts. WAIT messages are heartbeats, not additional calls. Mock decision choices are prescribed plumbing and provide no scientific evidence.

## Migration

The separate `selmem-genesis-positive` and `selmem-genesis-learning` executables are now `selmem-genesis positive` and `selmem-genesis learning`. Gate/policy/qualification/recall launchers and contrast/presentation/sensitivity/content/threshold troubleshooting commands have been removed. The oversized prepare/run/replay campaign scaffold and unexecuted confirmatory/pilot grids have also been retired. The creative-transfer collection and causal follow-up are now executable exploratory protocols; no creative result is claimed.

Existing evidence is readable and must keep its original manifests. Place qualified `positive-001/grok` outside or back under the ignored results directory if reusing it. Do not run the old suite script. The native organ's algorithms and observation-support implementation are unchanged by this cleanup.

Legacy library exports for benchmark/experiment/fork demonstrations have been removed. The core live/remember/speak/sleep API, daemon and terminal client remain.

## Sleep quality

The third command, `selmem-genesis sleep`, compares time-matched no-sleep, native normal sleep and forced deep sleep on saved learning histories and a pinned LoCoMo-derived factual panel. Read [SLEEP.md](SLEEP.md) for preparation, qualification, costs and limits. No real result is claimed yet.

## Creative transfer

`selmem-genesis creative` collects LoCoMo-derived creative proposals under empty, BM25 RAG, no-reconstruction and full SelMem conditions. Full/ablation share the same saved life. `creative-causal` applies audited target/sham trace lesions. [CREATIVITY.md](CREATIVITY.md) defines tasks, preparation, costs, blind human ratings and limitations. Start with the 72-proposal pilot, then freeze the protocol before the 360-proposal ten-history campaign. Mock is technical rehearsal only.

```bash
LOCOMO_DATASET=/path/to/locomo10.json bash experiments/genesis/run-creativity.sh experiments/genesis/results/creative-001
bash run.sh test --bin selmem-genesis -- --test-threads=1
```

The long trajectory experiment is documented in [CREATIVE_LIFE.md](CREATIVE_LIFE.md):
`creative-life` now defaults to the economical two-history design, retaining all sessions,
90 exposure days, 30 washout days and the developmental/readout/layer/continuation controls.
It shares only validated stateless annotations and plans 600 creative proposals.
The older ten-history full fixture remains supported for a new full-mode run.

## Creative preview and preserved plans

The independent `creative-preview` protocol is described in [CREATIVE_PREVIEW.md](CREATIVE_PREVIEW.md): two complete histories encoded once, ten nights per history, five conditions, and 60 proposals. Run `bash experiments/genesis/run-creative-preview.sh experiments/genesis/creative-preview.json experiments/genesis/results/creative-preview-001` from the repository root with a configured `.selmem`.

The longitudinal **economy** (90+30 days, 600 proposals) and **full** (90+30 days, 6,480 proposals) plans, preparers, launchers, and evaluators remain available with unchanged protocols. See [CREATIVE_LIFE.md](CREATIVE_LIFE.md). Use a separate output directory for each experiment; do not pool their observations or ratings.

## Offline Rust tools

[RUST_TOOLS.md](RUST_TOOLS.md) documents the Rust replacements for data preparation, blind rating, availability summaries, and mechanism coding. Run `bash experiments/genesis/tools.sh` for the command list. Existing completed result formats and the three frozen experiment plans are preserved. Apply tooling updates after an active collection finishes, because its resume checks bind the original code and executable.
