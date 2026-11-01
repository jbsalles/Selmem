# GENESIS — Native memory causal ablations v2

**Analysis updated November 1, 2026.** Completed exploratory run:
`causal-v2-qualified-12-001`, **3,456/3,456 valid Grok 4.3 answers**, 3,456 unique
trial IDs, no logged HTTP errors, and no recorded-request mismatches. Requests
specified `reasoning_effort=none`, temperature zero and a 16-token answer cap.
The versioned recall route was **`context-cued`**.

**Supported claim:** On these synthetic mirrored histories, interventions on the
memory packet delivered after recall changed held-out binary choices. Full memory
was more history-concordant than either component cut, empty memory or an unrelated
text placebo. This is a component-delivery result for the revised route; it does
not isolate reconstruction, forgetting or sleep as the cause of the advantage.

## Design and actual exposure

The design used 12 scenario domains, two mirror histories per domain, two attested
company scopes (`alone` / `friends`), two held-out wordings, both option orders for
each wording, six arms and three identical-request repetitions:

`12 × 2 × 2 × 2 × 2 × 6 × 3 = 3,456 calls`.

There are **24 saved histories and 192 query cells**, not 3,456 independent agents.
The domains vary work, dining, fitness, books, museum, music, commute, lodging,
lunch, shopping, learning and entertainment, but share a calm/individual versus
lively/social choice axis. They are not 12 independently sampled behavioral axes.
Histories were synthetic; a scripted evidence witness qualified action-pattern
beliefs. They do not establish subjective preferences.

Every full-memory cell passed the offline exposure check: **one contextual belief
and two topic-relevant episodes**, with at least one episode carrying the requested
company scope. Episode JSON retained its stored scope facets. Actor questions were
unchanged between arms. The full native packet was frozen once; cuts did not refill
it or expand the remaining text. Raw requests in `choices.jsonl` matched the frozen
`payloads.jsonl` records for all 3,456 rows.

| Arm | Intervention after recall |
| --- | --- |
| `full` | Native filtered packet, unchanged |
| `no_beliefs` | Remove conditional self-view entries; retain selected episodes |
| `no_episodes` | Remove delivered episodes and episode association edges; retain beliefs |
| `swapped` | Replace the conditional self-view with the same-scope mirror belief; retain original episodes |
| `bare` | Empty memory packet; retain shared system instructions and agent identity |
| `placebo` | Empty memory plus neutral inventory text matched to the full serialized user-payload byte length |

The shared native guide and answer constraint remained in every arm. The placebo
matches UTF-8 bytes, not provider tokens or semantic content. The 600-word setting
is a native content ceiling, not equal delivered lengths in every arm.

## History-concordant choices and paired interventions

Each arm has 576 valid choices. Concordance means agreement with the fixture's
history-conditioned action pattern; it is not an objective utility or task-quality
score. Pairing fixes domain, history, company, wording, order and repetition.

| Arm | History-concordant choices | Different from full | Full concordant / arm not | Arm concordant / full not |
| --- | ---: | ---: | ---: | ---: |
| Full native memory | **479/576 (83.2%)** | — | — | — |
| No delivered beliefs | 429/576 (74.5%) | 74/576 (12.8%) | 62 | 12 |
| No delivered episodes | 415/576 (72.0%) | 74/576 (12.8%) | 69 | 5 |
| Mirror beliefs swapped | 273/576 (47.4%) | 206/576 (35.8%) | 206 | 0 |
| Empty memory | 291/576 (50.5%) | 202/576 (35.1%) | 195 | 7 |
| Length placebo | 288/576 (50.0%) | 191/576 (33.2%) | 191 | 0 |

Full memory is **+32.6 percentage points** above empty memory, **+33.2 points**
above placebo, **+8.7 points** above the belief-delivery cut and **+11.1 points**
above the episode-delivery cut. Paired differences are directional rather than
an equal exchange of concordant and nonconcordant choices. The repeated-execution
reference for full memory disagreed in **11/384 comparisons (2.9%)**: repetitions
1 and 2 against repetition 0 of the identical request. These comparisons share a
reference and are dependent. Using all three within-cell repetition pairs instead
gives 20/576 (3.5%); the two denominators must not be mixed.

No individual-call binomial significance or population confidence interval is
claimed. Repetitions, wording variants, option orders and related domains are
clustered observations. Report domain-level direction and independent future
replications rather than treating all calls as independent evidence.

## Domain consistency

Every entry is out of **48** repeated choices per domain and arm.

| Domain | Full | No beliefs | No episodes | Swapped | Empty | Placebo |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Workspace | 47 | 45 | 39 | 23 | 24 | 25 |
| Dining | 36 | 29 | 31 | 23 | 24 | 24 |
| Fitness | 32 | 30 | 27 | 22 | 24 | 24 |
| Books | 44 | 42 | 36 | 26 | 24 | 23 |
| Museum | 47 | 34 | 38 | 24 | 25 | 25 |
| Music | 42 | 41 | 41 | 21 | 24 | 23 |
| Commute | 42 | 39 | 36 | 22 | 24 | 22 |
| Lodging | 36 | 36 | 31 | 22 | 24 | 24 |
| Lunch | 41 | 31 | 26 | 24 | 24 | 24 |
| Shopping | 35 | 28 | 34 | 19 | 24 | 24 |
| Learning | 36 | 35 | 34 | 24 | 25 | 27 |
| Entertainment | 41 | 39 | 42 | 23 | 25 | 23 |

Full exceeds empty memory and placebo in **12/12** domains. It exceeds the
belief-delivery cut in **11/12**, with one tie, and the episode-delivery cut in
**11/12**, with one reversal (entertainment). This does not retroactively satisfy
the original two-domain v4 pilot's preregistered comparison against frequency
summary and lexical retrieval: those arms were not tested here.

## Mirror history, swapped beliefs and option order

Changing only the saved life to its mirror changes the canonical choice in
**191/288 full-memory pairings (66.3%)**, compared with **9/288 (3.1%)** for empty
memory. Every full-memory mirror difference follows the two histories' respective
action patterns. This is history-conditioned decision divergence within the
specified synthetic task family, not a measurement of persistent personality.

Swapping the delivered belief changes 206 full-memory choices, all from concordant
to nonconcordant with the original history. However, the swapped arm agrees with
the donor history in only **303/576 (52.6%)** binary choices and selects canonical
option A, generally the calm/individual alternative, in **523/576 (90.8%)**.
Empty memory selects A in 485/576 (84.2%). The swap therefore demonstrates a strong
belief/episode conflict effect; it does not establish reliable adoption of the
donor's action pattern.

Position sensitivity remains material. Reversing option order while fixing the
other factors changes the canonical choice in **69/288 full-memory pairings
(24.0%)**. The ablation comparisons themselves use matched option orders, but the
resulting choices are not order-invariant.

## What changed from the failed native-delivery run

The earlier `causal-native-12-002` collection recorded 3,456 valid answers but
exposed no self-view beliefs in any of 192 cells; all 384 delivered episodes were
background distractors. Full memory scored 292/576 (50.7%). Full, no_beliefs and
swapped supplied identical requests at matched cells, invalidating a belief-effect
interpretation. That negative delivery diagnosis remains part of the record.

The context-only repair supplies the application context `choice-pattern`
separately from attested company facts. This restores a delivered belief but did
not fix episode relevance in the offline diagnosis. The opt-in **context-cued**
route additionally uses the text of applicable native self-view gists as internal
retrieval cues. Existing retrieval scoring/gates and reconstruction select the
episodes. It does not force support IDs, use the expected-answer labels to select
traces, read sealed archives or append a separate `Predicted action` instruction
to the actor prompt. The native belief statement itself still describes an action
pattern and can naturally cue a choice.

The v2 preflight blocks actor HTTP and resume before key lookup if any cell lacks
the required belief, relevant scoped episode or distinct mirror belief. It audits
frozen-file checksums and settings on resume. This is a revised mechanism and
protocol, not a relabeling of the old run. The difference between runs is not an
isolated randomized test of the routing change; the within-v2 ablations are the
controlled comparisons reported above.

## Remaining causal questions

- **Belief influence before retrieval.** The `no_beliefs` cut occurs after beliefs
  have already been available for context-cued episode selection. It measures
  direct delivery conditional on those episodes, not the total effect of beliefs.
  The next ablation should remove beliefs before recall while preserving the
  episodic store, then inspect and freeze the resulting deliveries.
- **Representation and memory mechanisms.** These data do not separately identify
  benefits of reconstruction, lossy distortion, selection, forgetting or sleep.
  An equally informative factual summary or competent semantic RAG with the same
  scope metadata was not compared in this suite.
- **Generalization.** Add unrelated behavioral axes, fresh histories, other
  models, task-blind probes and longer follow-up. The application context and
  company facts were supplied explicitly; ordinary unannotated chat was not tested.
- **Run variability.** The seed defines a deterministic execution schedule; repeats
  can be adjacent. Serving build fingerprints and per-call start/finish timestamps
  are not retained here, so temporal drift cannot be completely assessed.
- **Creativity and individuality.** No new creativity evaluation, spontaneous
  preference formation or durable behavioral singularity was measured.

## Evidence and reproduction

Full archive: `causal-v2-qualified-12-001.tar(1).gz`, SHA-256:
`13a12524ad815aeee02ba437a17a3d1cdf09c8d07337836e82932b5ad6fdd2b3`.
The supplied manifest does not record an exact source commit or provider-side
serving fingerprint; do not substitute the documentation checkout's commit.

Compact evidence: [answers.csv](../evidence/contextual/causal-v2-qualified-12-001/answers.csv),
[summary.csv](../evidence/contextual/causal-v2-qualified-12-001/summary.csv),
[by-domain.csv](../evidence/contextual/causal-v2-qualified-12-001/by-domain.csv),
[original paired.csv](../evidence/contextual/causal-v2-qualified-12-001/paired.csv),
[exposure.csv](../evidence/contextual/causal-v2-qualified-12-001/exposure.csv),
[manifest](../evidence/contextual/causal-v2-qualified-12-001/manifest.json),
[preflight](../evidence/contextual/causal-v2-qualified-12-001/preflight.json) and
[provenance / original-file hashes](../evidence/contextual/causal-v2-qualified-12-001/provenance.json).
The compact answer rows support recomputing the decision counts, paired effects,
noise and order/mirror comparisons; the full archive is required to inspect
requests and memory packets, rerun native recall or resume collection.

With the causal v2 implementation installed, reuse the exact qualified snapshots
in a **new** output directory for a zero-API preflight:

```bash
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation \
  --snapshots experiments/genesis/results/causal-native-12-002/snapshots \
  --out experiments/genesis/results/causal-v2-replication-new \
  --pairs 12 --repeats 3 --memory-words 600 --recall-route context-cued
```

Inspect the 192-cell exposure report before paid collection. Only after a passing
preflight, repeat with the same flags plus
`--resume --actor --provider grok --model grok-4.3 --max-http-calls 48` and a private
`GROK_API_KEY`. The cap is per invocation, includes failed attempts and is not a
monetary cap. Do not reuse the recorded answers as a new replication or edit hashes
to force a resume. This documentation update makes no API calls and changes no
experiment implementation.
