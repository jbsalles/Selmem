# Reproducible experiments

The retained source covers model qualification, cumulative learning, sleep, conversation-derived strategy, creative generation and memory-guided response selection. All experiment implementations and offline analyzers are Rust; shell launchers are convenience wrappers.

| Experiment | Purpose | Entry point |
| --- | --- | --- |
| Normative memory, wide context | 192 valid Grok 4.3 responses; normative 46/48, native 41/48, all anchors 40/48, scoped registry 46/48; no general alignment claim | [Results](genesis/NORMATIVE_WIDE_RESULTS.md), [protocol](genesis/NORMATIVE_PILOT_V10.md), [compact answers](evidence/normative-wide-012/answers.jsonl) |
| Qualification and learning | Check decision competence, then compare observation support OFF/ON | [GENESIS](genesis/README.md) |
| Contextual self | Compare admission feedback, freezing and evolution; qualify bootstrap before 128 matched choices | [Self-model v3](genesis/SELF_MODEL_PILOT_V3.md), [recorded results](genesis/SELF_MODEL_RESULTS.md) |
| Confidentiality alignment assay | Synthetic share/verify/decline choices, harmful vs neutral experience, clarification checkpoint, privacy and false-refusal metrics; offline-qualified, no model results yet | [Protocol and budgets](genesis/ALIGNMENT_CONFIDENTIALITY.md) |
| Native causal delivery | Six post-recall arms, 12 related synthetic domains, 3,456 choices; 83.2% full vs 50.5% empty, with preflight and repeat-noise checks | [Results and replay](genesis/CAUSAL_NATIVE_RESULTS.md), [pre-retrieval ablation](PRE_RETRIEVAL_ABLATION.md), [compact answers](evidence/contextual/causal-v2-qualified-12-001/answers.csv) |
| Contextual breaking points | Map choice accuracy against 8/16/32/64-word limits at 240/960 distractors, four v4 memory conditions, Grok 4.3 | [Pilot results](genesis/CONTEXTUAL_BREAKING_RESULTS.md), [protocol and replay](genesis/CONTEXTUAL_BREAKING_POINTS.md) |
| Contextual self → actor behavior | Separate episodic readout, autobiographical wording, self-statement swaps and post-contradiction retirement (84 + 24 + 96 decisions) | [Recorded follow-up results](genesis/SELF_BEHAVIOR_RESULTS.md), [emergence protocol](genesis/SELF_EMERGENCE.md) |
| Sleep | Compare equal-time no-sleep, ordinary sleep and forced-deep controls | [Sleep protocol](genesis/SLEEP.md) |
| Guarded creative preview | Fresh encoding, ten nights per history, five conditions, 60 proposals | [Guarded preview](genesis/CREATIVE_PREVIEW_GUARDED.md) |
| Fixed-selection creative replays | Isolate task-blind readout, then structured memory delivery; 72 proposals per lot | [Replay](genesis/CREATIVE_REPLAY.md), [structured rerun](genesis/CREATIVE_MEMORY_REPLAY.md) |
| Response selection | Separate candidate generation, task-only quality and anonymous grounding | [Influence diagnostic](genesis/MEMORY_INFLUENCE.md) |
| Longitudinal creative life | Preserved economy/full plans: 600 / 6,480 proposals | [Creative life](genesis/CREATIVE_LIFE.md) |
| Historical creative transfer | Original four-condition assay and target/sham follow-up | [Creative transfer](genesis/CREATIVITY.md) |
| LoCoMo strategy | Conversation-derived strategy, design and factual probes | [Strategy protocol](../docs/LOCOMO_STRATEGY.md) |

**Results:** [ledger](RESULTS.md), [creativity findings](CREATIVITY_RESULTS.md), [static showcase](../docs/index.html), [compact evidence](evidence/README.md).

There are early qualitative indications consistent with memory-conditioned creative variation, but no demonstrated creativity advantage attributable to SelMem. Text variation, successful orchestration and a model's own quality scores are different observations.

## Replay policy

Use the commands in each protocol and fresh output directories. Paid collections need a private runtime configuration and provider credentials. Mock runs test orchestration only. Source hashes, model identifiers, task fixtures, reasoning settings and original responses must stay with each run.

[REPRODUCIBILITY.md](REPRODUCIBILITY.md) distinguishes fresh replication, analysis of published answers and resumption of historical runs. A new build is not a way to relabel old results. Source snapshots, caches and raw paid outputs remain in separate local evidence archives; they are not deleted by this publication cleanup.

Native causal ablations reuse existing qualified V4 snapshots, freeze native
structured memory delivery, and compare six interventions with fully crossed
option order and exact repetitions. See [the protocol](genesis/CAUSAL_ABLATIONS.md).
