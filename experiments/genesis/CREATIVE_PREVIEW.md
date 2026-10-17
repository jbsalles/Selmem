# GENESIS creative preview

This third protocol is independent of the two longitudinal experiments, which remain available: `tools.sh prepare-life DATASET OUT --profile economy` (600 proposals) and `tools.sh prepare-life DATASET OUT --profile full` (6,480 proposals). Existing results are neither reused nor deleted. The historical `creative` pilot also remains available.

## Question and frozen design

Does transformed selective memory produce mechanisms that are both more original and more useful than empty context or faithful retrieval? This exploratory collection does not test autonomous life, long-term persistence, unconsciousness, or source causality.

Two complete official conversations (conv-26: 419 turns; conv-30: 369) are each encoded once, chronologically, with one virtual hour between turns. There is no imposed persona, added event, salience boost, runtime archive access, or ingestion recall. Ten ordinary nights follow exposure, each preceded by 24 virtual hours. The no-sleep control advances the same 240 hours. There is no forced deep sleep or new experience during these nights. Ten nights need not substantially transform memory: saved inspections record the changes that actually occur.

| Condition | Memory supplied to the generator |
|---|---|
| `empty` | None |
| `rag` | BM25 over original exposed turns, without the internal archive |
| `before_sleep` | Encoded state advanced by 240 hours without sleep, native readout with reconstruction |
| `after_no_recon` | State after ten nights, native readout with probe reconstruction disabled |
| `after_full` | The same state after ten nights, full native readout |

The `before_sleep` label denotes the no-sleep control at equal elapsed time, not a probe at an earlier clock. Both post-sleep probes must select the same identifiers. Their differences concern reconstruction at readout; reconstruction can also affect the reading profile, not just episode wording. The sleep/no-sleep comparison concerns the complete sleep treatment, potentially combining forgetting, rewriting, and consolidation.

Three English tasks are fixed before collection: a community activity, collaborative learning, and object repair. Five conditions and two generations yield **60 proposals**, at most 180 words each. The same model is used throughout; memory temperature is 0, generation temperature 0.7, reasoning effort explicitly defaults to `none`, and requested seeds are matched between conditions. Provider seed support and compliance with requested reasoning settings are not guaranteed. Empty controls receive identical inputs across histories: their repeats are not independent lives. These tasks are not a validated creativity benchmark.

Native context contains the actual reading and recalled narratives; whole entries remain within a 600-word maximum. Empty retrieval stays empty, without archive padding. RAG has the same ceiling, without guaranteed equality of actual information volume. Transfer under hash embeddings remains lexical and may miss an analogy. Vocabulary divergence alone is not a creative gain.

## Run from the repository root

With `.selmem` configured, the frozen fixture is included in the repository:

```bash
bash experiments/genesis/run-creative-preview.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-001
```

Optional environment variables: `SELMEM_CONFIG`, `GENESIS_PREVIEW_PROVIDER` (grok), `GENESIS_PREVIEW_MODEL` (grok-4.3), `GENESIS_PREVIEW_REASONING` (none), and `GENESIS_PREVIEW_HTTP_LIMIT` (3,000 cumulative attempts). All interpretations, reconstructions, scoring calls, and retries count. **3,000 is not a price, an expected call count, or a completion guarantee.** Valid semantic annotations are cached by exact request; state-dependent reconstructions and creative generations are not shared.

To prepare without API calls, append `--prepare-only` to the launcher. Continue with the same command and `--resume`. Also use `--resume` after an interruption. Committed operations are retained; resumption requires identical code, model, reasoning, configuration, and fixture. Only a higher attempt ceiling is allowed, and amendments are logged. Do not modify or rebuild the code during collection; longitudinal runs started with another binary require their original environment to resume.

Mutations use the longitudinal journal and integrity checks. Probes are readonly; encoded, no-sleep, and post-sleep states are saved with inspections and checksums. Reusing this machinery does not establish that the previously observed integrity incident under load has been resolved: any failure must stop collection and be diagnosed.

To regenerate the exact fixture from the pinned official release:

```bash
bash experiments/genesis/tools.sh prepare-preview /path/to/locomo10.json /path/to/new-preview.json
```

## Inspect and rate results

Give only `blind-proposals.json` to two independent evaluators. Keep the key, contexts, logs, and conditions hidden. Use ordinal 0-4 scales: utility (0 unusable, 1 major gaps, 2 plausible but incomplete, 3 concrete and workable, 4 especially robust); originality of the mechanism within this task (0 copied/no mechanism, 1 routine, 2 nontrivial adaptation, 3 unusual and coherent, 4 especially inventive and integrated). Also record constraint compliance and a rationale. Worldwide novelty is not asserted.

```bash
bash experiments/genesis/tools.sh rate-preview RUN template judge-a.json
bash experiments/genesis/tools.sh rate-preview RUN template judge-b.json
# Fill independently, then combine both arrays into ratings.json:
bash experiments/genesis/tools.sh combine-ratings ratings.json judge-a.json judge-b.json
bash experiments/genesis/tools.sh rate-preview RUN analyze ratings.json
```

`preview-ratings.json` reports the axes, disagreement, and contrasts per history. The exploratory score is each judge's minimum of utility and originality, zero for constraint violations or invalid output, averaged across judges. This is not a validated psychological measure. Two histories are two units, not 60 independent experiments. Examine mechanisms and provenance as well as averages. Targeted source removal requires a later declared and controlled protocol; it is not automatically launched here. Neither success nor failure in this preview settles the long-life experiment.

## Technical verification without paid calls

```bash
SELMEM_TEMP=0 SELMEM_REASONING=none bash run.sh run --release --bin selmem-genesis -- \
  creative-preview --mock --fixture experiments/genesis/creative-preview.json --out /fresh/path/preview-mock
```

Simulated replies provide no creativity evidence. The real fixture is frozen and checksum-verified; a contracted variant is allowed only with `--mock --technical-smoke` for workflow testing.
