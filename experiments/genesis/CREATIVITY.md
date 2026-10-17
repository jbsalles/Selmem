# GENESIS: creative transfer from LoCoMo histories

This exploratory extension asks whether different narrative histories produce useful novel mechanisms, and whether reconstructive memory contributes beyond access to faithful records. It does not infer creativity from lexical divergence. LoCoMo is the source of histories, not an official creativity benchmark. The LoCoMo agents' lives are external observations for SelMem's common `observer` profile; no extra persona prompt is supplied.

## Collection revision 2

The collection records `creative-transfer-2`; revision 1 runs remain valid historical pilot outputs and must not be pooled with this revision. Re-encode the original fixture into fresh snapshots. Tasks, conditions, generation temperature, repeats and maximum context budget are unchanged.

- `SELMEM_REASONING=off` is a compatibility alias for the explicit wire value `reasoning_effort: none`. The launcher defaults to `GENESIS_CREATIVE_REASONING=none`; an explicit alternate effort must be identical across all conditions. Requests remain recorded in `http.jsonl`. Sending a parameter is not proof that a remote provider honored it: inspect returned reasoning usage too.
- Hash fallback vectors no longer certify semantic novelty at admission. Novelty uses lexical comparison of lived gists, stripping the `Conversation on ... said:` transport wrapper for comparison only. Learned embedding responses still use their configured vector space. No admission threshold or affect weight was lowered; admission counts can nevertheless change substantially, so report them.
- Explicit caller attribution survives semantic annotations. Timestamped reported speech identifies the speaker separately from its transport header.
- Both native probe arms request the same explicit `transfer` intent. Topical evidence is required; hash collisions, emotional congruence and instruction/encouragement words alone cannot qualify a source. A soft redundancy penalty ranks complementary eligible traces. The maximum recall count and eligibility thresholds remain unchanged. Factual recall retains subject/relation controls.
- This fallback is lexical topical transfer, not a semantic analogy model. An empty or short readout is retained and reported, never filled from the archive. The console prints context word counts and native selection counts. The RAG maximum remains 600 words; this revision does not equalize actual information volumes.
- Causal follow-up inherits the source collection's recall intent and requires the same explicit reasoning effort. A real legacy run configured with `off` is rejected because it actually omitted the parameter; use its frozen harness to reproduce that run. No new sleep or co-recall learning is added. The new collection still requires independent blind ratings and source interventions before any creativity claim.

## Fixed design

Three English tasks are fixed before collection: community activity, collaborative learning and object repair. Every condition receives exactly the same task and output instruction: one mechanism, JSON `proposal`, maximum 180 whitespace-delimited words. Conditions are empty context, lexical BM25 RAG on the original supplied dialogue turns, native SelMem with reconstruction disabled only during the probe, and native full SelMem. All use the same generation model, temperature 0.7 and requested repeat seeds. Providers may ignore seeds: record actual response metadata, do not call this deterministic.

One native life is encoded chronologically per history, using native tick/live without salience boosts. No ingestion recalls or sleep occur. Consequently this first experiment tests admission and retrieval/reconstruction; it does not test learned co-recall associations or sleep benefits. Full/no-reconstruction clone the exact same encoded snapshot; a transient probe cut disables reconstruction without re-encoding a separate life. Recall is readonly, at memory temperature zero. The saved life hash is checked before and after probes and after generation.

Each history/task/condition readout is frozen across three generation repeats. The returned narratives and axioms are rendered in a common text field, capped at 600 words; entire entries are retained and oversized entries skipped. Native reading profiles are audited but not sent to the generator. RAG uses positive BM25 matches only, at the same word cap. Empty retrieval is a measured failure/condition, not silently filled with an oracle. This is a matched **maximum word budget**, not exact equal context length or tokenizer-level equality. Therefore a difference may involve relevance, selection or amount of recalled information; it is not pure creativity gain at identical facts.

The two-history/four-session pilot has 153 original turns and 72 proposals. The complete ten-history version has 5,882 original turns and 360 proposals. It has substantially higher encoding costs; 360 is the number of creative generations, not total HTTP calls. Native interpretation, encoding, scoring, reconstruction and transport retries share the attempt ceiling. Failed/budget-exhausted outputs are retained. Output paths must be fresh.

## Preparation and collection

From repository root, with the official `locomo10.json` already downloaded:

```bash
bash experiments/genesis/tools.sh prepare-creative /path/to/locomo10.json pilot.locomo.json
./run.sh run --release --bin selmem-genesis -- creative --mock --locomo pilot.locomo.json --out experiments/genesis/results/creative-mock
```

The preparer verifies the SHA-256 of compact `JSON.stringify` dataset content, so whitespace changes from a reader do not change the accepted release. It also records the hash of the exact input bytes. Image bytes, BLIP captions, generated summaries, event summaries and QA answers are excluded. The fixture hash and complete fixture are saved in the run design.

For a real pilot, use the existing configured provider:

```bash
LOCOMO_DATASET=/path/to/locomo10.json bash experiments/genesis/run-creativity.sh experiments/genesis/results/creative-001
```

The launcher defaults to `grok`, respects `SELMEM_CONFIG`, `GENESIS_CREATIVE_PROVIDER`, `GENESIS_CREATIVE_MODEL` and `GENESIS_CREATIVE_HTTP_LIMIT` (default 3000). `GENESIS_CREATIVE_PREPARE_ONLY=1` prepares the run design without encoding or generation. To collect all ten histories set `GENESIS_CREATIVE_ALL=1` and explicitly set a sufficient attempt ceiling (maximum 50000). Do the pilot before extending the scope; do not alter tasks after seeing outputs and treat them as preregistered.

## Independent blind evaluation

Give two independent evaluators **only** `blind-proposals.json` and the rubric below. Do not give them the run directory, private key, raw responses, condition names or retrieved memories. A separate evaluator can later inspect source audits. Evaluators score utility and originality independently on ordinal 0–4 scales and record whether task constraints are met. Each evaluator must give a short rationale.

| Score | Utility | Originality within this task |
|---|---|---|
| 0 | Unusable or does not address task | Copy/repetition, or no identifiable mechanism |
| 1 | Major practical gaps | Routine generic solution |
| 2 | Plausible, needs clarification | Some nontrivial adaptation |
| 3 | Concrete and workable | Unusual coherent mechanism |
| 4 | Clearly workable, resources and assessment explicit | Especially inventive and well-integrated mechanism |

Novelty refers to the mechanism, not wording; global novelty is not established. The primary exploratory score is each judge's `min(utility, originality)`, set to zero if constraints fail, then averaged across judges. This fixed operational score is not a validated psychological creativity scale. Report both original axes and disagreement too.

```bash
bash experiments/genesis/tools.sh rate-creative RUN template ratings-judge-a.json
bash experiments/genesis/tools.sh rate-creative RUN template ratings-judge-b.json
# Fill separately; combine their JSON arrays into ratings.json without revealing the key.
bash experiments/genesis/tools.sh rate-creative RUN analyze ratings.json
```

The analyzer requires two distinct rater IDs per proposal, full coverage, valid scores and rationales. It outputs per-history comparisons against RAG and no reconstruction. History is the independent unit; tasks and generation repeats are nested observations. Ten histories are small, paired by design and drawn from a synthetic conversational dataset. The script does not convert sample counts into a significance claim.

## Provenance and causal follow-up

After blind ratings are locked, an independent source audit identifies proposals with an actual transformed mechanism involving two different events/sessions. It must verify event provenance in `creative-encoding.jsonl`, source narratives in `creative-readouts.jsonl`, and that both narratives were actually supplied inside the capped generation context. A mention, an analogy or an explanation invented after generation is insufficient. The sealed archive is never consulted at runtime. A source trace can compress multiple events: check its receipt provenance instead of assuming one trace is one event.

Prepare an explicit case list:

```json
{
  "version": "creative-causal-1",
  "cases": [{
    "trial_id": "COPY_AN_ACTUAL_FULL_SELMEM_TRIAL_ID",
    "roots": ["ACTUAL_TRACE_A", "ACTUAL_TRACE_B"],
    "sham_roots": ["UNSELECTED_TRACE_C", "UNSELECTED_TRACE_D"],
    "mechanism": "Specific mechanism whose loss will be evaluated, declared before reruns.",
    "source_reason": "Independent provenance audit and rationale for target and matched sham traces."
  }]
}
```

The runner rejects missing, duplicate, overlapping or unselected target roots, roots not shown to the original generator, selected sham roots, modified source snapshots and configuration/model/mock mismatches. Match sham trace relevance, salience, age and size as well as possible manually; the program checks membership, not semantic unrelatedness.

```bash
./run.sh run --release --bin selmem-genesis -- creative-causal \
  --source RUN --plan causal-plan.json --out CAUSAL_RUN \
  --config .selmem --llm grok --model SAME_MODEL --max-http-calls 600
```

Each audited case receives intact, remove A, remove B, remove AB and remove two sham traces, with three paired generation seeds per arm: fifteen proposals per case. Every arm starts from the same source bytes; the lesion is applied before readonly recall. A new blind pack is exported. Judge whether the **predeclared mechanism** persists, not whether words change. Evaluate the intact reruns too: failure to reproduce the mechanism there weakens attribution.

These are structural episode lesions, not complete erasure of all descendants: schemas, beliefs or other traces can retain the information. Selection can substitute other memories. A null lesion effect therefore does not prove absence of causal influence. Targets were selected after seeing first-run ideas, so this stage is exploratory; confirmatory replication requires frozen cases and held-out histories/tasks. No automated creativity or causality verdict is emitted.

## Interpretation

Different histories shaping inventive, useful proposals would support creative individualization under these tasks. Independently verified recombination plus selective loss under target interventions would strengthen a causal memory claim. A paired advantage over RAG and no-reconstruction would support an advantage of this SelMem configuration; a null result is retained. Sleep at 5/30 cycles is a subsequent intervention, not part of the initial pilot.
