# SelMem

Selective reconstructive memory for LLM entities. v0.5

---

## Problem

An LLM maps a context window to a next-token distribution. Adding memory usually means keeping more of the past available: logs, summaries, embeddings, top-k retrieval.

SelMem treats memory as a state that changes. Not every event is stored. Stored events lose detail. Recall rebuilds a sentence from the current trace instead of replaying the original. That rebuild can write back. Repeated traces form motifs, then beliefs, then traits, which bias the next encode.

The verbatim event is kept in a sealed archive for tests and audit. The model never reads it. It only sees the lived trace (gist, core, affect, fidelity). When an hour leaves the book the archive stays as a tomb (`released_from`, `released_at`, core). `audit` still finds it. The mouth does not.

```
experience → selection → trace → recall → sleep → identity → next encode
```

**Hypothesis:** After a year of the same stream, two copies of the same model that retained one hour on one side only no longer remember or forget the same things. Measured on the book, retrieve, and the mouth. Historical exploratory runs motivated this hypothesis; durable behavioral singularity and useful creativity remain to be demonstrated.

That is the path-dependence hypothesis. It is not yet a demonstration of intelligence, creativity or personality. The creative collections provide early qualitative indications consistent with creative recombination and memory-conditioned variation; they do not prove a creativity advantage caused by SelMem. Current evidence and limits: [experiments/RESULTS.md](experiments/RESULTS.md) and [creativity findings](experiments/CREATIVITY_RESULTS.md).

**New contextual-transfer evidence (October 2026):** In an exploratory Grok 4.3 v4 pilot, context-qualified memory yielded 16/16 history-concordant choices versus 10/16 for a deterministic frequency summary and 10/16 for lexical retrieval, with 240 distractors and a 64-word recall cap. Two domains do **not** meet the preregistered 12-domain criterion. Earlier v2/v3 pilots found no advantage over appropriately informative memory controls. This is a bounded memory-selection result, not evidence of autonomous preference, general RAG superiority or creativity. See [contextual results](experiments/genesis/CONTEXTUAL_TRANSFER_RESULTS.md).

**Budget-breaking follow-up (October 28, 2026):** A new **512-decision** Grok 4.3 sweep reused the v4 synthetic histories at **240/960 distractors** and **8/16/32/64 memory words**. At 32 and 64 words, qualified context memory scored **28/32**, compared with **18/32** for lexical retrieval and **14/32** for a frequency summary. The advantage narrowed to **17/32 vs 16/32** at eight words, when SelMem’s own output was cut before the predicted action. ON/OFF/lexical memory texts were *identical* across distractor loads: this does not isolate a causal distractor effect. Delivered lengths were unequal, baselines were deterministic, and just **two domains** were tested. See the [breaking-point analysis](experiments/genesis/CONTEXTUAL_BREAKING_RESULTS.md); the preregistered 12-domain v4 threshold remains unmet.

**Native delivery follow-up (analysis November 1, 2026):** In a separate 3,456-choice, 12-scenario Grok 4.3 ablation run using `context-cued` recall, full memory reached **83.2%** history-concordance versus **50.5%** with empty memory. Removing delivered beliefs or episodes reduced concordance to **74.5%** and **72.0%**. These are post-recall component-delivery effects on one shared behavioral axis; they do not establish a reconstruction-specific or general RAG advantage. [Causal results and limitations](experiments/genesis/CAUSAL_NATIVE_RESULTS.md).

---

## Not this

| Looks like | Difference |
| --- | --- |
| RAG with a TTL | Recall rebuilds a sentence; it does not return a stored chunk |
| Session summary | A summary is still a stored string. Here wording and affect both move |
| Personality prompt | No fixed persona file. What remains after nights is the bias |
| Vector DB | Embeddings rank neighbors. The record is gist + core + affect |
| Log injected into the prompt | Archive exists for humans (`GET /audit`). The model does not see it |

If you paste the original sentence back “for accuracy”, you have a store again. When a living gist drifts too far from its core, the system blends the gist toward that core. Still no archive.

---

## Rules

1. **Drop by default, forget by salience.** A score must still clear `τ` (now lower: 0.28 / 0.40). Weak keeps cool and weather faster than charged ones.
2. **Two records.** Lived: gist, core, affect, fidelity. Sealed: original text. Only the lived record is used at recall.
3. **Two channels.** `self` is rewritten over time. `world` is not.
4. **Reconstruct.** Recall uses schema, gist or core, mood, affect, confidence. Not the original string. Low confidence (`< 0.35`) speaks the core.
5. **Two drift directions.** High valence can gild. Recalled disgust can darken. Unused disgust can fade. Filling a hole (`Confabulate`) is not gilding (`Embellish`): the detail was gone; the fill is new and does not write the core.
5b. **Attribution.** `External` / `Internal` / `None`. External holds wording. Internal rewrites when the hour conflicts with the living self (`self_congruence < 0.40`). `None` keeps the legacy charged+anchor+axiom skip.
5c. **Congruence and centers.** `self_congruence` is measured against same-schema axioms at encode. A schema with ≥2 hours or a living axiom has one prototype (`SchemaCenter`). Faded Internal periphery can fall toward it. The hub and External hours do not.
5d. **Access ≠ confidence ≠ suppression.** Access is how easy the hour is to find. Confidence is how sure the current gist is. Suppression is directed forgetting: the hour stays; Observed recall skips it.
6. **Core ≠ detail.** The semantic core can hold while surface fidelity falls.
7. **Four quantities, not one anchor.** `anchor` is an encode field: it slows decay and some rewrite. `hazard_scale = 1 / (1 + survival_days / 3)` uses `created_at` and only changes erasure. `behavior_weight = 1 / (1 + since_use_days / 21)`, floor 0.12, uses last recall and only multiplies the recall score. The context cloud is the hours the query touches, plus axiom co-supports and edges that are not the same schema, weighted by age and relevance. The recall score is multiplied by the weighted share of the hour's tokens in that cloud. No share, the score is 0. There is no half-cut. A same-schema label does not pour its words in. A recall moves the use clock, not the birth date.
8. **Ladder.** A motif needs two same-schema hours, or one hour whose charge is at least 0.28. A belief needs at least three living hours and charge at least 0.45. A trait needs two aligned beliefs. Headcount alone does not promote. A belief statement is the acts the hours already marked, not an order. Superseded beliefs stay in lineage.
9. **Feedback.** Living axioms tint the next event before the gate. Recall can shift sense with current mood.
10. **Distance.** Fingerprint uses valence, disgust, fidelity, anchors, core tokens, axioms, founders, traits, contradictions. 0 = same book. 1 = disjoint books.
11. **Grounding.** Fading traces may keep warping. A miss is a kind the core does not authorize (`Elaborate` / `Contradict` / `Depart`), not a low Jaccard. `Reframe` (irony, other speech act on the same event) is spoken and logged as `Color`; it does not increment strikes or rewrite the gist. On a living trace, claim misses increment `detach_strikes`. Pull-back strength is `narrator_firmness × importance`. Blend toward core, never toward the archive. Jaccard is only the identity gate.
12. **Latent.** The scene can leave recall while schema and affect still bias encode.

---

**Observation support.** Admission of a new trace and retention of another observation supporting an existing trace are separate operations. With `observation_evidence` enabled, stable caller-provided observation identities accumulate bounded support; replayed identities do not count twice. Support accompanies a selected trace without becoming gist/core rewrite material. Caller identities do not prove statistical independence. A saturated reconstructive weight derived from distinct observations of the *primary claim* can modestly boost recall and color the narrative when `OrganCut::support_signal` is true; counter-observations remain in their own counts but cannot strengthen the primary claim. The cut lesions only the support signal without discarding groups or disabling independent schema-prototype context. See [PARAMETERS.md](PARAMETERS.md).

**Co-recall.** [Learned associations](docs/CO_RECALL_ASSOCIATIONS.md) are optional organ-level context for reconstruction. Retrieval scores remain unchanged; associations are persisted and audited, not treated as facts.

## Loop

```
experience
    → interpret (lexicon → identity paint → optional LLM on the live sentence)
    → salience gate
    → lived book + sealed archive
              ↓
     remember / speak
     reconstruct → reconsolidate
     (living traces: misses vs core, then blend)
     talk frame holds the current thread (active ≤ 10 min gap, ≤ 2 h; sleep commits it through the gate, then drops it)
              ↓
           sleep
     deep:    weather → confab → ladder → centers → rewrite → merge → release
     shallow: weather → confab → release
     (anchors before weather and after release; budget on new hours / charge)
              ↓
          who_am_i
              ↓
     next experience already biased
```

`tender` / `austere` are two starting gates, not characters. Same input stream, two thresholds, two nights → two books.

|  | tender | austere | status |
| --- | --- | --- | --- |
| τ | 0.28 | 0.40 | contrast pair |
| decay λ | 0.10 | 0.06 | unfitted |
| embellish | 0.18 | 0.05 | contrast pair |
| disgust gain | 0.05 | 0.16 | contrast pair |
| ground_min_overlap | 0.18 | 0.18 | identity gate on `Hold` |
| ground_strikes | 3 | 3 | two free misses, then rewrite |
| narrator_firmness | 0.42 | 0.72 | blend strength |

All of these are knobs. See [PARAMETERS.md](PARAMETERS.md).

**Encode**

```
S = w_a·A + w_n·N + w_s·R + w_u·U + w_g·G − w_r·Red
```

If `S < τ` and permanence < 0.8: nothing is stored.

If the caller sends no affect: lexicon (FR+EN), then identity paint, then `Narrator::interpret` when an HTTP narrator is set. The interpreter sees the live sentence and living axioms only.

**Recall.** Small top-k. Mix embedding, lexicon, mood, access. The score is multiplied by the book-link share. The cloud is the hours the query touches, plus axiom co-supports and edges that are not the same schema, weighted by age and relevance. No shared token, the score is 0 and the hour stays in the book. A same-schema label does not pour its words in. Each recall can cost fidelity and shift valence (`DriftKind::Reinterpret`). A claim miss is a kind the core does not authorize (`Elaborate` / `Contradict` / `Depart`); `Reframe` is logged as tone, not counted as a miss; `ground_min_overlap` is only the identity gate on `Hold`. `hold = narrator_firmness × importance`. Low hold: no ceiling on warp. High hold: after enough misses, blend gist toward a core-facing rewrite (`DriftKind::Ground`). Latent traces are not replayed as scenes.

**Sleep.** Named passes stay `weather → ladder → rewrite → merge → release`. After weather, a collapsed gist may be filled from the schema (`Confabulate`) if an axiom or center supplies a clause the core does not license. After ladder, schema centers are rebuilt; faded Internal non-hub hours can fall toward the prototype. Ladder still runs before rewrite so a first deep night does not wash T₀ before it can mint. Anchors run before weather and after release. No LLM required. The judge of a night rewrite is the same `DetachKind` check as recall.
**Rewrite skip.** `skip_rewrite` is attribution × conflict, not “charged ⇒ freeze”. External holds. Internal skips only when congruence ≥ 0.40. `None` is the old charged+anchor+axiom rule.

**Layout.** Files follow those questions: `encode/` (enter), `recall/judge` + `recall/pull` (tell and license), `dream/*` (weather through time), `persist/snapshot.rs` (one field list, two containers). Map: [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Layers

```
IDENTITY     who_am_i: traits, then beliefs, then motifs
    ↑
TRAITS       two aligned living beliefs
    ↑
BELIEFS      ≥ 3 traces, one schema
    ↑
MOTIFS       2 traces, one schema
    ↑
TRACES       gist + core + affect, or latent charge without scene
    ↑
EXPERIENCE   live sentence
```

A motif does not replace a strong belief. A weak belief (strength < 0.36) can. Two anecdotes are not a trait.

---

## Levers

**Detail decay.** Core is set at encode. Detail loses precision with time and disuse. Coefficients are implementation choices, not a fitted model of human memory.

**Rewrite.** Neighbors by schema or cosine; the schema hub is injected when a center exists. Keep the core, keep one detail, drop the rest. Anchor ≥ 0.88 still skips on the `None` path. External skips regardless. Internal conflict does not.

**Anchor field.** Raised on high-intensity self events and again if the trace supports a living axiom. Slows decay and reinterpretation. It is not survival, not behavioral weight, and not the context cloud.

Fingerprint is a lab metric on the book. It is not a personality score.

---

## Surface

Rust 1.75. No Cargo crates. Two vault containers, one `Snapshot` field list (`persist/snapshot.rs`): SQLite (prepared statements, `BEGIN IMMEDIATE`) or flat `SELMEM1`. HTTP daemon + UI. `/health` does not take the memory lock. Auth: `Authorization: Bearer` only.

```
GET  /health /who /lineage /mood /profile /audit /talk
POST /live /remember /speak /turn /sleep /save /profile /talk/clear /pin /suppress /unsuppress
```

```bash
./run.sh run --release --bin selmemd -- \
  --bind 0.0.0.0:7420 --path claire.db --name Claire \
  --profile tender --ground-overlap 0.18 --ground-strikes 3 --narrator-firmness 0.42 --token secret \
  --llm https://api.x.ai/v1/chat/completions \
  --model grok-4.3 --api-key "$SELMEM_API_KEY"
```

No `--llm`: rule narrator + hashed vectors.

Same keys in a `.selmem` file in the working directory (`llm=`, `model=`, `api_key=`, `reasoning=`). Flags and `SELMEM_*` env override the file. A vault file starts with `SELMEM1` and is not config.

Ollama: `--llm http://127.0.0.1:11434/v1/chat/completions --model llama3`.

xAI: `reasoning=none` unless you want reasoning tokens.

The model sees gist, core, schema, affect, fidelity, confidence, mood, living axioms, schema centers. Not the archive. Suppressed hours stay off Observed recall.

---

## Conclusions from the benches

Tables and observations: **[experiments/RESULTS.md](experiments/RESULTS.md)**. Commands and replay: [experiments/README.md](experiments/README.md). Historical runs remain audit evidence for their recorded manifests; retired launchers are not commands for the current organ.

GENESIS first qualified the decision task with a faithful external history. Under explicit stationary reliability rules, twelve independently checked correct reports favor following, while incorrect reports favor verification or an alternative depending on cost. Grok completed 48/48 oracle choices; GPT-4o-mini completed 28/48 and was not qualified for the next stage.

The initial SelMem learning assay retained one of twelve observations in each history. The admitted-only faithful control reproduced the failed reliable-history choices before reconstruction, identifying a loss at admission.

A paired OFF/ON run then used the same revised input representation and model, changing only observation-support activation. OFF retained one observation; ON retained twelve observations in one trace per history. Native and strict recall each matched the full-history oracle on 16/16 decisions with ON, versus 8/16 with OFF. There were 475 HTTP attempts, no network errors, and 144 decisions across both cuts. Sources and life snapshots were unchanged by probes.

This is an exploratory demonstration of cumulative, task-relevant evidence retention and decision use. The effect was carried by explicit observation counts. One history pair, a fixed OFF-then-ON order, and repetitions of frozen contexts do not provide independent population replication. Scorer inconsistencies remain observed. Full results and earlier failures are summarized in [RESULTS.md](experiments/RESULTS.md).

The separate LoCoMo strategy experiment probes advice and design using conversation-derived observations. Its retained runs belong to earlier source manifests and do not establish improved individuality or creativity in the current organ.

### Contextual self: admission and revision, limited decision effects

The Grok self-model v3 pilot completed 128 choices in 180 HTTP attempts using
two scripted incident histories and four matched conditions. Initial contextual
beliefs were formed from two attributed action observations per history.
Own-context routine-note admission was 0/2 before and after correction for
memory-only and evolution without admission feedback, 2/2 at both phases for
frozen self, and 2/2 then 0/2 for the full loop. Other-context notes were refused;
swapping only the self traces exchanged the context favored by admission.

All four evolving branches retired their initial belief after two recognized
counterexamples. In a separate assay on checkpoint copies, two new actions
supported a new opposite-pattern belief in each evolving branch. This validates
the implemented admission/revision loop under calibrated simulated conditions.
Initial certainty, bounded decrements and retirement thresholds are program
rules; semantic labels remain fallible model judgments.

Actors received selected episode gists without self propositions. The full
loop differed from memory-only on 2/32 matched choices and from evolution
without admission feedback on 0/32. Both differences from memory-only were
localized to one task/history/phase with one option order. Overall, 123/128
choices favored checking. These repeated measures from two histories establish
neither durable behavioral individuality nor an added decision benefit from
admission feedback. The earlier v2 lot failed to form birth beliefs and remains
an unqualified diagnostic. [Results, limits and archive hashes](experiments/genesis/SELF_MODEL_RESULTS.md).

### Subsequent GENESIS assays: self belief transmission, not yet preference emergence

A later Grok 4.3 sequence examined the chain from selected autobiographical
memory to a self-model proposition and then an actor choice. The prompt-only
Self-Preference control (24 choices) found 6/8 history-aligned choices when
supplied past actions and 8/8 when supplied an autobiographical preference
statement; that wording already expresses preference, so it does **not**
demonstrate preference formation by SelMem. Self-Bridge v4 (84 choices) was
strongly dominated by option order: all 72 main-arm choices selected the first
option; explicit preference sensitivity nevertheless passed 12/12.

Preference Emergence then formed five scripted histories using the **unchanged
organ** (70 HTTP attempts), followed by 96 matched Grok decisions. The two
affect-bearing histories formed contextual self beliefs from independent
performed-action evidence, as did both action-only controls, while the neutral
control formed none. The beliefs described **repeated actions**, not validated
intrinsic preferences. In both active histories confidence fell from roughly
0.35 to 0.20 after one counterexample and to 0.05 (retired) after three;
no opposite belief formed in this collection.

At decision time, episodes alone versus episodes plus the agent's own self
proposition changed **1/16** choices. Replacing that proposition with one from
the opposite history changed **6/16**, concentrated entirely in the initial
active-belief phase (**6/8** before, **0/8** after retirement). The direct
preference control passed **16/16**. These are effects of **explicitly supplied
actor context**, not demonstrated silent, durable self-conditioning. One named
ablation control was invalid: its actor prompts were identical to the
episode-only prompts. The intermediate one-counter phase was not used for
actor decisions. With two scripted mirrored histories and closely related
tasks, these data are descriptive rather than population evidence.

The sequence therefore supports *guarded formation and retirement of a
behavioral self-summary*, and actor sensitivity to some delivered self
propositions. It does **not** yet show autonomously generated personal
preference, stable preference reversal or history-dependent individuality.
[Detailed tables, failure modes and raw-archive hashes](experiments/genesis/SELF_BEHAVIOR_RESULTS.md).

### GENESIS contextual transfer v1–v4: selection under long-history distraction

Four sequential Grok 4.3 pilots tested whether an **explicitly delivered, context-qualified action hypothesis** can influence held-out actor decisions. They are successive protocol revisions, **not independent replications**, and all use synthetic histories and a scripted evidence witness. The history-conditioned action patterns do not establish intrinsic preferences.

| Pilot | SelMem ON | Comparator(s) | Outcome |
| --- | ---: | ---: | --- |
| v1 — direct contextual readout | 16/16 qualified-phase choices | OFF 8/16, episodic 9/16 | Positive actor sensitivity, but the expected actions were closely cued by the readout; the initial parent belief was seeded. |
| v2 — generalization with raw episodes | 16/16 | Episodes, OFF and shuffled each 16/16 | **No additional advantage**: all memory conditions could infer the rule from the six raw observations. |
| v3 — 160 distractors, 64-word recall ceiling | 16/16 | Faithful action summary 16/16, targeted retrieval 16/16 | **No advantage over strong, task-informed memory controls**. |
| v4 — 240 distractors, 64-word ceiling | **16/16** | Generic frequency summary **10/16**, lexical retrieval **10/16** | **+37.5 percentage points** versus each named baseline in an exploratory two-domain pilot. |

In v4, each of four mirrored histories contained **14 task-related actions among 240 distractors**; the general parent belief was formed from observations during consolidation, and two qualified contextual children were produced per ON history. The competing summary and lexical retrieval used the full observed stream but **no oracle IDs for relevant episodes**. ON sent about 22 memory words per decision on average; summary sent 64 and retrieval about 50. The equal **ceiling** does not imply equal payload length, information content or computational cost. V4 also observed OFF 7/16, recent episodes 5/16, and no memory 6/16.

The v4 criterion was frozen for **12 independent semantic domains**: ON must beat both the frequency summary and lexical retrieval by at least 10 percentage points on average, and beat both in at least 8/12 domains. Only **two** domains ran. The recorded result is therefore **exploratory** (`meets_preregistered_threshold: false`) despite the positive pilot gaps. No independent LLM judge was run, and the actor runner did not explicitly set its reasoning parameter.

**Bounded claim:** SelMem demonstrated a context-sensitive compression/selection benefit in this small synthetic pilot **relative to the tested deterministic frequency and lexical baselines**. The result does not isolate the contribution of reconstructive distortion, compare with semantic RAG or a strong LLM summarizer, or demonstrate learned preference, durable individuality or creativity. [Full per-arm data, negative results, controls and archive SHA-256](experiments/genesis/CONTEXTUAL_TRANSFER_RESULTS.md); [v4 protocol](experiments/genesis/CONTEXTUAL_LONGLIFE_V4.md).


### Native causal ablations: direct effects of delivered memory

The completed `causal-v2-qualified-12-001` run recorded **3,456 valid Grok 4.3 choices**, with reasoning requested as none, six arms, 12 synthetic scenario domains, two mirror lives and two company scopes per domain, two wordings, both option orders and three identical-request repetitions. A 192-cell preflight verified one delivered conditional belief and two relevant episodes per full-memory packet.

| Delivered memory | History-concordant choices | Paired decisions different from full |
| --- | ---: | ---: |
| Full | **479/576 (83.2%)** | — |
| Conditional beliefs removed | 429/576 (74.5%) | 74/576 (12.8%) |
| Episodes removed | 415/576 (72.0%) | 74/576 (12.8%) |
| Mirror beliefs substituted | 273/576 (47.4%) | 206/576 (35.8%) |
| Empty memory | 291/576 (50.5%) | 202/576 (35.1%) |
| Neutral length placebo | 288/576 (50.0%) | 191/576 (33.2%) |

The full condition's identical-request repetition reference disagreed in **11/384 comparisons (2.9%)**. Removing beliefs yielded 62 concordant-to-nonconcordant changes and 12 in the reverse direction; removing episodes yielded 69 versus 5. Full memory exceeded empty memory in all 12 scenario domains. Mirroring the saved history changed 191/288 full-memory decisions (66.3%), versus 9/288 without memory (3.1%). These observations support a causal contribution of the delivered components to choices in this task family. They are not independent-agent population estimates.

This result uses a revised, explicitly selected **`context-cued`** route: caller-attested application context makes the native self-view applicable, and its existing gist can cue episodic recall before the packet is frozen. It does not append a separate predicted-action instruction. An earlier 3,456-answer native run delivered no beliefs and only background episodes, so its belief-removal and swap contrasts were invalid. That negative diagnosis is retained rather than relabeled as v2.

The cuts occur **after recall**. Consequently, the no-beliefs arm still benefits from any belief-mediated episode selection; a pre-retrieval lesion is needed to estimate the total belief contribution. Swaps reduce original-history concordance but match the donor in only 52.6% of choices, with a strong calm-option bias. Reversing option order changes 24.0% of full-memory choices. The 12 domains share one behavioral axis; new histories, models and axes are needed. This suite does not isolate reconstruction or establish an advantage over competent semantic RAG, creativity, autonomous preference or durable individuality. [Full design, per-domain counts and evidence](experiments/genesis/CAUSAL_NATIVE_RESULTS.md).

### Normative memory: bounded rule-following benefit, registry parity

The `normative-wide-012` pilot recorded **192 valid Grok 4.3 responses** with
reasoning `none`: two mirrored histories, two phases, six tasks, four arms and
two identical-request repeats. Each arm has 24 distinct cells and 48 responses.
The context ceiling was 6,000 words, 64 episodes and 64,000 serialized bytes
through the default conservative token counter. Native recall actually delivered
64 unclipped episodes, 1,213–1,214 words, with all applicable rules in every cell.

| Delivered evidence | Correct decisions |
| --- | ---: |
| Wide native recall | 41/48 (85.4%) |
| All canonical anchors, unfiltered | 40/48 (83.3%) |
| Normative scope composition | **46/48 (95.8%)** |
| Scope-filtered original registry | **46/48 (95.8%)** |

The five corrected response pairs, across three distinct cells, changed
verification into the required refusal. Correct authorized deliveries remained
18/20 for both native and normative arms; unauthorized deliveries were zero in
all arms. Normative and scoped registry still failed twice on the same situation:
an inspection remained permitted after a planning-only prohibition. Preserving
typed rules does not guarantee that the actor applies their conditions correctly.

This is evidence for the utility of the implemented normative path relative to
wide native delivery on this task family. It does not establish an advantage over
the simple scope-filtered registry. Host-supplied typed semantics, scope selection,
composition, format and context length change together; reconstruction is not
isolated. Two synthetic mirrored histories and repeated cells do not establish
general alignment, adversarial robustness or final-action enforcement.
[Protocol](experiments/genesis/NORMATIVE_PILOT_V10.md),
[full results and archive hashes](experiments/genesis/NORMATIVE_WIDE_RESULTS.md),
[compact responses](experiments/evidence/normative-wide-012/answers.jsonl).

## What remains unproved

### Creative collections: suggestive observations, no established advantage

The guarded Creative Preview 003 completed 60 proposals from two complete conversation histories (788 turns), ten nights per history and five conditions: empty, BM25 RAG, equal-time no-sleep, post-sleep gists and post-sleep guarded retelling. All arms used matched direct generation. Retrieval was task-conditioned; retelling and its provenance guard were task-blind. The latter is a controlled readout intervention, not unchanged full native delivery. Reading profiles and axioms were withheld, and the response-choice selector was not invoked.

Sleep changed the saved states: 113 of 261 and 79 of 219 retained traces became latent; 45 and 36 axioms were present afterward. Those axioms were not delivered to generation. Latency is not deletion and does not by itself show useful forgetting. The post-sleep retelling context averaged 62 words versus about 144 for direct gists and 593 for RAG, so compression is a plausible competing explanation for differences.

Some recorded answers combine plausible mechanisms in interesting ways: optional repair micro-actions, repeated artifact interviews with follow-up commitments, and reply threads that trigger small community meetings. These are early qualitative indications in favor of investigating a creativity hypothesis. Similar inventiveness also occurs in controls. Neither examples nor the earlier single unblinded assistant review establish an advantage over the base model, RAG or direct gists; independent blind assessment remains pending.

Fixed-selection replay 003 and structured-delivery replay 004 each produced 72 proposals but shared states/readouts. Influence 005 reused candidate pools and quality judgments rather than generating a fresh independent sample. On its 24 memory candidate pools, the default moderate coefficient changed three choices relative to zero weight. The pipeline's same quality model rated utility lower in two changes and equal in one; mean originality over those three was unchanged. This shows a selection intervention with tradeoffs, not proof of creative improvement.

The [compact evidence](experiments/evidence/README.md) publishes proposals, condition labels, state/selection summaries and original-file hashes. Original raw archives remain separate. Reproduction requires the recorded protocol and a fresh output directory; resuming historical work additionally requires its original build and complete archives. The larger 600- and 6,480-proposal longitudinal plans are retained but have not established a result.

### Remaining tests

- Durable behavioral differences beyond the trained actor and situation.
- Transfer of those differences with memory and their selective removal by lesions.
- A general causal advantage specifically from reconstruction over competent semantic retrieval or task-conditioned/rolling summaries; the v4 pilot compared only deterministic baselines.
- Original and useful designs whose historical dependence is causally established.

The next causal test should remove contextual beliefs before retrieval while preserving the episodic store, separating direct self-view delivery from belief-mediated episode selection. A separate support-metadata lesion should preserve all other state. Mixed and contradictory observations should test revision. Transfer requires new tasks and actors without explicit reminders of the founding episode. Creativity requires executable designs evaluated for utility, novelty and memory dependence against equally budgeted controls.

## Limits and negative outcomes

Model competence and evaluator competence must be qualified separately. Temperature zero does not guarantee determinism. A stable core can preserve a mistaken encode-time interpretation. Caller identities cannot verify independence by themselves. Exact support matching is deliberately conservative; paraphrases need not accumulate. Saturation, suppression and carrier loss can remove useful information. The current support mechanism is not validated through merging or long-term consolidation.

Experiments must retain failures, separate behavior from performance and preserve probe isolation. No benefit is an admissible result. Increased rigidity, factual drift or reduced utility would count against the intended advantage rather than as evidence of singularity.

---

## Status

Rust tests cover selection, core vs detail, anchors, axiom succession, identity paint, reinterpretation, DetachKind misses, grounding blend, latent residue, night pass order, file and SQLite round-trips, attribution, schema centers, suppression, confidence vs access, survival hazard, recency weight and context cloud. Observation-support tests cover repeated observations, replay identities, contradictions, bounded counts and persistence. GENESIS checks include configuration loading, snapshot errors and isolated probes.

The maintained experiments include qualification, cumulative learning, sleep, creativity pilots, contextual self-model assays, contextual-transfer v1–v4 and the revised native causal-delivery suite, alongside LoCoMo and the longitudinal creative-life plans. The v4 long-history pilot records a bounded advantage over tested deterministic selectors; v2 and v3 record null comparisons against stronger information-preserving controls. The native causal suite adds a measured component-delivery effect across 12 related synthetic scenarios. These results do not establish durable behavioral singularity, general superiority over competent RAG, or a creative advantage attributable to reconstruction. Constants remain unfitted. The [results showcase](docs/index.html) separates observed mechanisms, historical reports and unproved claims.

Two processes on one `.db` will collide. Anchors are decay brakes, not an ethics layer.

MIT. [github.com/jbsalles/Selmem](https://github.com/jbsalles/Selmem)
