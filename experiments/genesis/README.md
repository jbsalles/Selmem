# GENESIS

A set of controlled exploratory assays around the SelMem organ. Start with the [result ledger](../RESULTS.md), [creativity findings](../CREATIVITY_RESULTS.md) and [replay rules](../REPRODUCIBILITY.md). The protocols below deliberately test different interventions; do not pool their samples.

## Normative rule-following pilot (analysis November 8, 2026)

The completed [wide-context normative pilot](NORMATIVE_WIDE_RESULTS.md) collected
192 valid Grok 4.3 responses. Normative scope composition scored 46/48, versus
41/48 for wide native recall, 40/48 for unfiltered anchors and 46/48 for the
scope-filtered registry. All relevant rules were present in the native context.
The five corrections concern prohibitions; authorized deliveries remained 18/20,
and unauthorized deliveries were zero in every arm. This is a bounded result on
24 synthetic cells per arm, repeated twice, with no demonstrated advantage over
the filtered registry or general alignment claim. [Protocol and replay](NORMATIVE_PILOT_V10.md),
[API](../../docs/NORMATIVE_MEMORY.md), [compact answers](../evidence/normative-wide-012/answers.jsonl).

## Native causal ablations (analysis November 1, 2026)

The completed [native delivery suite](CAUSAL_NATIVE_RESULTS.md) records 3,456 valid Grok 4.3 choices across 12 related synthetic scenarios, six arms, counterbalanced options and three identical repetitions. Full memory reached 83.2% history-concordance, versus 50.5% empty, 74.5% without delivered beliefs and 72.0% without episodes. This uses the explicit `context-cued` route after a failed native-exposure diagnosis. The cuts occur after recall; a pre-retrieval belief lesion is the next causal test. [Compact answer table](../evidence/contextual/causal-v2-qualified-12-001/answers.csv) and [reproduction instructions](CAUSAL_NATIVE_RESULTS.md#evidence-and-reproduction).

## Contextual memory breaking-point sweep (October 28, 2026)

The completed [Grok 4.3 pilot](CONTEXTUAL_BREAKING_RESULTS.md) evaluated **512 held-out choices** from the original v4 synthetic histories under word ceilings **8, 16, 32, 64** and **240/960 distractors**, with qualified contextual memory, scope-disabled memory, frequency summary and lexical retrieval. ON reached **28/32 at 32–64 words**, vs **18/32 lexical**; this is an exploratory two-domain result, with an **8-word prefix-clipping artifact** and **unchanged selected ON/lexical payloads across distractor loads**. Read the [protocol](CONTEXTUAL_BREAKING_POINTS.md) before reusing it. Run the same code with `--reasoning none` to request reasoning disabled explicitly; do not silently merge responses from different reasoning configurations.

## Qualification and cumulative learning

```bash
bash experiments/genesis/run-positive-control.sh experiments/genesis/results/positive-new
bash experiments/genesis/run-evidence-learning.sh \
  experiments/genesis/results/positive-new/grok \
  experiments/genesis/results/evidence-new
```

The qualification panel requires 48/48 oracle choices for this specific decision task. The historical reported Grok run passed; GPT-mini did not. Learning then compares observation support OFF/ON on two twelve-report histories. Retained historical reports show 8/16 versus 16/16 oracle agreement for native and strict recall. The benefit is explicit evidence retention, not a proven effect of reconstruction or creativity. Qualification sources must be real, complete and model-matched.

## Published creative preview

```bash
bash experiments/genesis/run-creative-preview-guarded.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-new --prepare-only
# After inspecting the prepared design, start paid collection:
bash experiments/genesis/run-creative-preview-guarded.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-new --resume
```

The [guarded protocol](CREATIVE_PREVIEW_GUARDED.md) encodes 788 turns from two complete histories once, applies ten nights per history, and collects 60 proposals across five conditions. Task-conditioned retrieval is followed by task-blind retelling and provenance checking. All conditions use matched direct generation; response-choice selection is not invoked.

The published preview completed 60 proposals in 887 HTTP attempts. This is a collection result, not proof of creativity superiority. The older native preview remains available through `run-creative-preview.sh`; the guarded and native readouts must not be treated as the same intervention.

## Replays and response selection

- [Fixed-selection replay](CREATIVE_REPLAY.md): reuse saved states and selected IDs; add a task-blind reteller/guard and a no-memory two-step comparator.
- [Structured delivery rerun](CREATIVE_MEMORY_REPLAY.md): reuse the earlier accepted readouts, change final memory delivery, and collect another separate lot.
- [Memory-influence diagnostic](MEMORY_INFLUENCE.md): two controlled histories, six tasks, three conditions, two repeats. Analyze candidate selection independently from generation and quality.

The recorded replays completed 72 answers each. Influence 005 reused 24 direct answers, 48 candidate pools and 48 quality results, then made 33 grounding attempts. These lots share inputs or outputs and are not independent replications.

## Other retained protocols

[Sleep](SLEEP.md) compares matched no-sleep/normal/deep states; its historical results and persistence defect are documented in the ledger. [Creative transfer](CREATIVITY.md) retains the original pilot and lesion follow-up. [Creative life](CREATIVE_LIFE.md) retains both the 600-proposal economy plan and 6,480-proposal full plan for later runs. They are plans, not completed creativity results.

## Offline checks and rating

```bash
./run.sh test --all-targets -- --test-threads=1
./run.sh test --doc
./run.sh run --release --bin selmem-genesis -- --help
bash experiments/genesis/tools.sh
```

The [Rust tools](RUST_TOOLS.md) prepare pinned dataset fixtures, generate blank rating forms, combine independent ratings and analyze results. At least two independently completed judge forms are required for the intended creativity assessment. Distinct judge names alone do not establish independence.

Give judges only the original blind proposals, their blank form and the rubric. Keep conditions, source memories, pipeline scores and analysis hidden. Model self-evaluation and the earlier single unblinded assistant review are not independent blind evidence.

## Contextual self-model diagnostic

The [self-model pilot 3](SELF_MODEL_PILOT_V3.md) separates self-dependent admission,
learning, freezing and model swapping before another creativity collection.
Start with `bash experiments/genesis/run-self-pilot-v3.sh OUT --mock`. Its 128 mock
choices validate the circuit and are not behavioral evidence. Real collection
requires explicit provider configuration and a new output directory.

The recorded Grok v3 lot completed 128 choices. Self swaps and correction
changed contextual admission, and evolving beliefs retired and later reformed
on separate new-action copies. Full loop vs memory-only differed on 2/32
matched choices; full loop vs evolution without admission feedback on 0/32.
These are two scripted histories with limited decision effects, not a
demonstration of durable individuality. [Results and archive provenance](SELF_MODEL_RESULTS.md).

## Self-model behavioral transmission and preference formation (October 2026)

Follow-up Grok 4.3 lots include [Self-Bridge v4](SELF_BRIDGE.md)
(84 matched actor choices), the prompt-only [Self-Preference](SELF_PREFERENCE.md)
(24 choices), and [Preference Emergence](SELF_EMERGENCE.md) (70 formation HTTP
attempts plus 96 actor choices). The unchanged organ consolidated guarded
repeated-action self claims, including in controls **without positive affect**;
three contrary observations retired the initial belief. Swapping a delivered
self claim changed 6/8 initial actor choices, but simply adding it to episode
gists changed only 1/16. The labelled memory-ablation condition was defective
because its actor prompts matched the episode-only condition exactly.

[Evidence table, archive checksums, limitations and next tests](SELF_BEHAVIOR_RESULTS.md).
Retirement is not replacement by an opposite preference; no independent
replication or durable individual behavior is established. Keep original local
archives with their pinned prompts, snapshots, reasoning and provider metadata.

```bash
# This phase uses the organ to form and revise self beliefs (paid API calls):
bash experiments/genesis/run-self-emergence.sh \
  experiments/genesis/results/new-self-formation --formation-only
# Only after auditing the formation; reuses its frozen checkpoints:
bash experiments/genesis/run-self-emergence.sh \
  experiments/genesis/results/new-self-choices \
  --choices-from experiments/genesis/results/new-self-formation
```
