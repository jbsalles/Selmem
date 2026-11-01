# Result ledger

These observations come from the supplied run archives. They do not substitute for raw evidence or independent replications. Historical manifests identify the code and model used at the time; a source cleanup does not change those identities.

## Native causal delivery ablations (analysis November 1, 2026)

`causal-v2-qualified-12-001` completed **3,456/3,456 valid Grok 4.3 decisions** (`reasoning_effort=none`) with 12 synthetic scenario domains, six arms, both option orders per wording and three identical-request repetitions. The explicit `context-cued` route passed exposure checks in all 192 query cells. Full memory was **479/576 (83.2%)** history-concordant, versus **429/576 (74.5%)** without delivered beliefs, **415/576 (72.0%)** without episodes, **273/576 (47.4%)** with mirror beliefs, **291/576 (50.5%)** empty, and **288/576 (50.0%)** placebo. Each component cut changed **74/576 (12.8%)** paired choices; identical-request repeat disagreement for full memory was **11/384 (2.9%)**.

Full memory exceeded empty memory in 12/12 domains. These are post-recall delivery interventions: beliefs already cued episode selection. The 12 scenarios share one calm/social axis, full choices changed under option reversal in 69/288 pairings (24.0%), and swapping beliefs did not reliably transfer the donor pattern. The earlier 3,456-answer native run exposed no beliefs and only unrelated episodes; its invalid belief contrasts remain a negative delivery diagnosis. This suite does not test the original v4 summary/retrieval criterion, isolate reconstruction, or establish creative advantage. [Detailed results, archive hash and next ablation](genesis/CAUSAL_NATIVE_RESULTS.md); [compact decision evidence](evidence/contextual/causal-v2-qualified-12-001/summary.csv).

## GENESIS contextual-memory budget sweep (October 28, 2026)

`breaking-pilot-001` logged **512 valid Grok 4.3 decisions** with `reasoning_effort=none`, two domains, two mirrored histories each, two distractor loads (240/960), four memory limits (8/16/32/64 words) and four methods. At **32 and 64 words**, qualified contextual self-beliefs achieved **28/32** history-concordant choices against **18/32 lexical**, **14/32 frequency summary**, and **12/32 contextual OFF**. At **eight words**, ON fell to **17/32**, compared with **16/32 lexical**: a raw prefix cut off the predicted action. Delivered ON/OFF/lexical payloads were byte-identical at both distractor loads for all matched cells, meaning the extra distractors did not perturb their selected texts. **These are descriptive repeated probes of two synthetic domains, not independent 512-agent evidence.** Original v4 results remain a separate prior experiment. See [breaking-point results, audit hash and limitations](genesis/CONTEXTUAL_BREAKING_RESULTS.md) and the [compact curve](evidence/contextual/breaking-pilot-001/curve.csv).

| Run | Observation | Interpretation |
| --- | --- | --- |
| `gate-001` | Targeted scorer panel 14/14 for both models. Grok retained 1/2 prefix events and both branches; GPT retained all. | Improved targeted checks did not establish scorer generality or behavioral divergence. |
| `policy-001` | New panel: Grok 16/16; GPT-mini 8/16. Grok strict restored missing facts, but histories/policies still yielded identical decisions. | Faithful recall alone did not make the old probe sensitive. GPT scorer accepted omissions and an unsupported cause. |
| `positive-001` | Grok 48/48 oracle choices; GPT-mini 28/48, including 0/16 on reliable history. | Grok qualified for the explicit learning task; GPT-mini did not. |
| `learning-001` | One of twelve reports retained per history. Admitted-only history reproduced the reliable-history failure. | Loss of cumulative information at admission was the dominant diagnosed bottleneck in this assay. |
| `evidence-001` OFF | One retained observation per history; one trace each. Native 8/16 and strict 8/16 matched the full-history oracle. | No positive-history effect. Decisions were reasonable given the reduced evidence. |
| `evidence-001` ON | Twelve observations represented in one trace per history. Native 16/16 and strict 16/16 matched the oracle. | Positive cumulative-learning result carried by explicit support counts. Not proof of reconstructive superiority, individuality or creativity. |

`evidence-001`: 234 OFF + 241 ON = 475 HTTP attempts; 144 decisions; no network errors. Runtime: 18m48s OFF + 20m53s ON. The configurations differ only in observation-support activation. The factual sentence representation was revised for both cuts, so OFF/ON is the appropriate comparison, not ON versus the older `learning-001` alone. Sources and life snapshots were unchanged by probes.

A faithful external baseline passes in both cuts. In ON, Grok follows the reliable actor and chooses verification/alternative for the unreliable actor according to cost. Each final recall context is frozen across eight decision cells. Those cells and repetitions are not independent lives. Fixed OFF-then-ON order, one history pair, and model-generation variation limit causal generalization.

The next informative tests are matched support lesions, mixed/reversing evidence, repeated independent lives and transfer to new tasks/actors. Creativity should be evaluated independently with executable solutions, utility and novelty measures, and equally budgeted conventional-memory/no-memory controls.

## Contextual self-model pilot

`self-pilot-v3-grok-real-001` completed 128 choices in 180 HTTP attempts, with
two qualified birth histories, no HTTP errors recorded and nonempty episode
readout in every actor cell. In the full loop, own-context note admission went
from 2/2 to 0/2 after contradiction; frozen self remained at 2/2, while
memory-only and evolution without admission feedback remained at 0/2.
Other-context notes were rejected and surgical self swaps exchanged admission
context. All four evolving branches retired their initial belief and, on
separate copies with two new actions, formed a new opposite-pattern belief.

Behavioral differences remain limited: full loop vs memory-only differed on
2/32 matched choices, full loop vs frozen self on 1/32, and full loop vs
evolution without admission feedback on 0/32. Option-order changes affected
5/64 pairs; 123/128 choices favored checking. These are repeated measures from
two scripted histories, not evidence of durable individuality or creative
advantage. The earlier v2 run had zero qualified birth histories and must not
be pooled with v3. [Detailed findings, controls and provenance](genesis/SELF_MODEL_RESULTS.md).

## LoCoMo strategy record

The supplied strategy evidence has 179 successful Grok answers out of 180 requested and 180/180 GPT answers; the technical rehearsal contains no real model answers. This is a conversation-derived strategy task, not the full LoCoMo benchmark. Historical source hashes and reported conditions stay with the audit archive. No general benefit in individuality or creativity is claimed.

## Sleep pilot and persistence defect

`sleep-001-measured` completed 240 decisions, 168 readonly readouts and 160 nights, in 2h38m51s with 1555 new HTTP attempts (4712 including failed preparation and recovery). Controlled histories remained 8/8 correct in every arm at cycles 0/1/5/20; support counts stayed 12 per history without duplication. LoCoMo-derived correctness was no-sleep 2/2/1/2, normal 2/1/1/1 and forced-deep 2/1/0/1, each out of twelve. All other factual choices were abstentions. This is a small, admission-limited multiple-choice panel, not the official LoCoMo score or a demonstrated sleep benefit.

Forced deep sleep accumulated 119 and 272 stored, unsuperseded motifs for only 6 and 15 distinct statements. The flat codec omitted axiom stake context, so reopening broke family matching; SQLite additionally omitted trace stake context. The corrected codecs preserve these fields. A local no-HTTP replay on copies of the prepared real prefixes keeps 6 and 15 motifs across twenty reopen/deep-sleep cycles. This verifies the persistence fix, not corrected model performance.

Keep original outputs unchanged. To assess corrected sleep effects, reuse the unchanged `sleep-001-prepared` prefixes and run new isolated interventions into a fresh output directory (for example `sleep-002-measured`); record this patch/code revision with the new result. No paid encoding replay is needed. The earlier LoCoMo admission losses and the latency/cost of proposition scoring are separate issues; this patch does not change the gate, retrieval policy or sleep thresholds to optimize the observed score.

## Creative collections and response influence

The creative results are now published as [compact evidence](evidence/README.md); see [CREATIVITY_RESULTS.md](CREATIVITY_RESULTS.md) for the distinctions between creative examples, memory influence and demonstrated advantage.

| Recorded lot | Observed result | Scope and limitation |
| --- | --- | --- |
| creative-preview-003 | 60/60 proposals; 887 HTTP attempts; 788 source turns; twenty sleep cycles | Guarded task-blind readout, five conditions, two histories. Independent blind ratings pending. No demonstrated creativity advantage. |
| creative-replay-003 | 72 proposals, 114 attempts; source unchanged | Fixed source selection and task-blind retelling/guard. Controlled readout intervention, not fresh encoding. |
| creative-replay-004 | 72 proposals, 72 attempts; previous/source unchanged | Reused accepted readouts with structured final delivery. Not independent of replay-003. |
| memory-influence-005 | 72 final rows; 33 new grounding attempts; 24 direct answers, 48 candidate pools and 48 quality results reused | Frozen-pool reevaluation. Default moderate weight changes 3/24 memory choices: same-model utility falls twice and ties once; mean originality on the three switches is unchanged. |

There are early qualitative indications consistent with creative recombination and memory-conditioned variation. These support further testing of the hypothesis, not a conclusion that SelMem improves creativity. The base model and controls also produce interesting designs. The earlier unblinded assistant assessment was exploratory, not an independent benchmark, and is not promoted to a validated score here.

The earlier qualification, learning, strategy and sleep figures above are historical reported results: their full original raw archives are not part of this source publication. They are kept distinct from the four compact creative lots freshly checked against the supplied archive. The sleep pilot's negative outcomes and persistence defect remain part of the record.
