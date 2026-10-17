# GENESIS sleep quality

This paired pilot tests memory quality after native sleep, using the successful GENESIS observation-support source and a separate LoCoMo-derived factual panel. It adds an experiment without changing the memory engine. The native adapter exposes forced deep sleep and night reports, and accepts external attribution for observed conversations.

## Inputs and qualification

Use the `on/` directory from a completed `learning-2-observation-support` run, with its `learning.json`, design, schedule and both life vaults. The requested model must match its recorded provider. Source observation support must be ON. A mock source is accepted only in mock mode. Original source files are hashed and left unchanged.

LoCoMo uses the pinned official `locomo10.json` release (SHA-256 `79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4`). The offline Rust preparer deterministically takes the first four sessions of the first two conversations, preserving every text turn, speaker, dialog id and session date: 153 turns. No images are encoded. It selects six questions per conversation: two with multiple cited passages, two temporal, and two further factual questions, restricted to categories 1/2/4 whose cited evidence is wholly inside the prefix. No future-session questions or category-3 commonsense/category-5 adversarial questions are included.

Official answer strings become multiple-choice options, with deterministic distractors drawn from answers in the same category and an abstention option. References, evidence ids and expected choice ids remain outside agent prompts. The correct answer necessarily appears among options; this is a multiple-choice adaptation, not an official LoCoMo score or a free-answer hallucination test. Inspect `OUT.locomo.json` before launching; some distractors may be ambiguous or too easy. Do not modify fixtures after seeing intervention results. Pin their hash and separate later confirmatory questions.

Before encoding LoCoMo or intervening, the same model must answer all 20 faithful-history questions correctly (8 controlled decisions + 12 factual questions). Failure stops the assay and preserves diagnostics. This engineering gate checks solvability and model competence; it does not replace manual review or guarantee representative question difficulty. Mock choices bypass qualification and are never scientific evidence.

## Intervention

Each history has one common saved prefix. Three byte-identical clones start at T0:

| Arm | Each subsequent cycle |
| --- | --- |
| `none` | Advance 24 virtual hours; no night |
| `normal` | Same time advance; native `sleep()` selects shallow/deep from its budget |
| `deep` | Same time advance; force native `sleep_deep()` |

All arms receive no further events after the split. Their clocks are checked after every cycle. Operation order rotates deterministically. Co-recall learning/context options are inherited from the source configuration and snapshot; they are not toggled between arms. The LoCoMo prefix is encoded with external attribution, the ordinary native gate, live recall, and no sleep, using the same runtime provider. Conversations are witnessed events, not the agent's autobiography. This can limit rewrite by design.

At cycles 0, 1, 5 and 20, each question uses a disposable clone and readonly native recall. Both clone and life hashes must remain unchanged. Controlled decisions reuse one recalled context per history/checkpoint/arm, matching the previous learning task; factual questions each have their own recall. T0 repeated across clones is a generation control. Repeated questions and cycles are not independent lives. Normal and forced-deep arms may initially execute the same kind of night; actual `night.kind` is recorded rather than assumed.

## Measures and interpretation

- Controlled expected-net-gain choices: correctness versus the full-history oracle.
- LoCoMo multiple-choice correctness and abstention, separated from controlled decision utility.
- Recall contexts and reconstruction/scorer audits at each checkpoint.
- Trace and axiom counts, observation support counts and identities, fidelity and full native state before/after each night; merges, rewrites and releases; actual night kind.

Compare normal/deep against the time-matched no-sleep arm at the same horizon. Compare each with T0 to diagnose information already lost during admission. If native T0 is at floor, the assay cannot establish that sleep destroyed previously usable information. More axioms or a reformulated gist is not evidence of improved memory. This pilot detects preservation/loss/use of facts, not original creativity, transferable singularity, population-level superiority or open-ended invention rates. The 20-cycle forced-deep arm is a stress test, not a model of twenty human days.

The 12-report source may have one trace per history, hence little material to merge or generalize. LoCoMo supplies distinct episodes but a gate can reject many. Inspect retained evidence before drawing conclusions. Mixed/reversing GENESIS histories and new-task generalization remain a separate next assay; they are not silently added to this pilot.

## Commands and cost

Configure `.selmem` using `data/config.example`. The launcher downloads/verifies the dataset and refuses existing outputs:

```bash
bash experiments/genesis/run-sleep-quality.sh \
  experiments/genesis/results/evidence-001/on \
  experiments/genesis/results/sleep-001
```

Set `LOCOMO_DATASET` for an already downloaded pinned file; `SELMEM_CONFIG` for another runtime configuration. `GENESIS_SLEEP_PROVIDER` defaults to `grok`; set it and optionally `GENESIS_SLEEP_MODEL` to the provider/model of the source. A GPT source must have passed its own preceding qualification; do not relabel a Grok source.

This plans **260 decision requests** including faithful qualification, **168 final memory readouts**, **153 new encodings with live recall**, and **160 native nights**. Internal interpretation, reconstruction and scorer requests plus retries make the actual HTTP count variable. `GENESIS_SLEEP_HTTP_LIMIT` defaults to 3000 attempts and can be set up to 8000. This is a hard safety budget, not a prediction. Nights need not call an LLM. No duration is guaranteed. Failure leaves logs and partial data; prefix preparation recovery is described below; partial intervention measurement is not resumable. Use a fresh output for a repeat.

Prepare fixtures separately to review them first:

```bash
bash experiments/genesis/tools.sh prepare-sleep path/to/locomo10.json sleep-fixture.json
./run.sh run --release --bin selmem-genesis -- sleep \
  --source experiments/genesis/results/evidence-001/on \
  --locomo sleep-fixture.json --out experiments/genesis/results/sleep-001 \
  --config .selmem --llm grok --max-http-calls 3000
```

For a technical rehearsal, supply a mock learning ON source and add `--mock`. No paid API calls are made. The output is explicitly labeled `technical_rehearsal_complete`.

`OUT/sleep.json` records completion/failure and scores; `sleep-design.json` records the fixed fixture, config and initial source hashes. `qualification.jsonl`, `prefix-encoding.jsonl`, `nights.jsonl`, `states.jsonl`, `sleep-readouts.jsonl`, `sleep-decisions.jsonl`, `responses.jsonl` and `http.jsonl` provide the audit trail. Snapshots preserve clone ancestry by history/arm naming and initial matching hashes. Keep failure outputs as well as successes.

## Preparation recovery and separate phases

The original `sleep-001` failed after 20/20 faithful questions, 39 complete LoCoMo events and a saved encoding of event 40 (`conv-26:D3:5`). Its recall failed: 731 HTTP attempts, including 548 scorer calls. No sleep intervention was measured. This diagnoses a preparation failure, not a sleep effect.

`--resume-from DIR` accepts a preparation-only failed/prepared run with matching source hashes, exact fixture, memory config, model, and complete faithful qualification prompts/answers. It refuses histories with measured decisions or nights. Use a fresh output; the parent is never edited. For the original format, the last saved encoding receipt is checked against the actual native snapshot, and only the pending live recall is retried: no second tick, encoding or observation count. A pending recall is not an isolated measurement and may mutate the recovered prefix just as the original successful live recall would have done.

New events run in a disposable transaction. Only an entirely successful tick/live/recall becomes an immutable checkpoint; `preparation.json` points to its content hash. If an event fails, a later preparation resume starts from the last committed checkpoint. A failed uncommitted event can require repeating that event's API calls, but not the whole prefix. Checkpoint hashes are checked before use. Transport retries transient curl timeouts, 429 and 502/503/504 errors up to three attempts, all counted by the HTTP budget; permanent errors are not retried.

`--prepare-only` stops after constructing the two final LoCoMo prefixes and reports `prepared` for real runs. Resume that output without `--prepare-only` to run interventions; qualification and encoding are then reused. This is a preserved protocol, including live recall after every turn. Removing those recalls would be a distinct experiment and is not part of this recovery. `recovery.json` records the parent and its HTTP attempts; `http_attempts_total` accumulates ancestor attempts, while `http_attempts` records the current segment only. `ancestry.json` and the parent logs remain part of the evidence chain.

The launcher recognizes `GENESIS_SLEEP_RESUME_FROM` and `GENESIS_SLEEP_PREPARE_ONLY=1`. For the supplied failure:

```bash
SELMEM_REASONING=off SELMEM_HTTP_TIMEOUT=120 \
GENESIS_SLEEP_PROVIDER=grok GENESIS_SLEEP_MODEL=grok-4.3 \
GENESIS_SLEEP_RESUME_FROM=experiments/genesis/results/sleep-001 \
GENESIS_SLEEP_PREPARE_ONLY=1 \
bash experiments/genesis/run-sleep-quality.sh \
  "$HOME/Downloads/Selmem-cleanup-original-docs/audit/experiments/genesis/results/evidence-001/on" \
  experiments/genesis/results/sleep-001-prepared
```

After successful preparation:

```bash
SELMEM_REASONING=off SELMEM_HTTP_TIMEOUT=120 \
GENESIS_SLEEP_PROVIDER=grok GENESIS_SLEEP_MODEL=grok-4.3 \
GENESIS_SLEEP_RESUME_FROM=experiments/genesis/results/sleep-001-prepared \
GENESIS_SLEEP_HTTP_LIMIT=6000 \
bash experiments/genesis/run-sleep-quality.sh \
  "$HOME/Downloads/Selmem-cleanup-original-docs/audit/experiments/genesis/results/evidence-001/on" \
  experiments/genesis/results/sleep-001-measured
```

Both commands make paid calls. The 6000-attempt intervention ceiling is a safety limit, not an expected count. A larger transport timeout may avoid transient failures but also makes permanently stalled calls wait longer. Prefix preparation is resumable; partial intervention measurement is not. If measurement fails, preserve it and start a separate measured run from the unchanged prepared prefixes, rather than relabeling or merging partial results.

After a terminal scorer/interpreter error, the wrapper latches the failure and suppresses further model calls in that native operation. It still rejects the whole operation; no fallback is accepted as successful scientific data. This avoids the extra scorer calls observed after the original timeout.
