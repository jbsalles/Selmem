# External long-memory page

**SelMem is not a long-context QA store. These numbers are expected to lose to last-k.**

That sentence is the claim on this page. It does not touch the persist claim. Do not submit a leaderboard row. Do not fold these numbers into P4.

## Bench

LoCoMo (Maharana et al., ACL 2024). Public file `data/locomo10.json` from [snap-research/locomo](https://github.com/snap-research/locomo), CC BY-NC 4.0. Ten conversations. We do **not** ship the corpus.

## Subset (frozen before scoring)

- All 10 `sample_id`s: `conv-26, conv-30, conv-41, conv-42, conv-43, conv-44, conv-47, conv-48, conv-49, conv-50`.
- Questions: **category = 1 only** (single-hop), with a non-empty `evidence` list of `dia_id`s.
- n = **282**.
- No LLM judge. Metric = evidence-id retrieval: a hit if **any** gold `dia_id` sits in the method’s window.
- last-k window = last 8 speaker turns of that conversation, in order. last-24 is a note, not a SelMem cell.
- C2 / static: **not run**. Encoding 5–7k turns through the organ to answer LoCoMo would test a store we refuse to be. Predicted: ≤ last-k, because the gate drops dull turns that still hold `dia_id`s.

## Numbers

| Method | Hits | Acc | What it is |
| --- | --- | --- | --- |
| last-k = 8 | 2 / 282 | **0.007** | the persist control window |
| last-24 | 13 / 282 | 0.046 | larger log, still not the bench |
| full log (oracle) | 281 / 282 | 0.996 | evidence ids exist in the file |
| C2 / static | — | not run | see above |

All categories (n = 1982 with evidence): last-8 = 0.006, last-24 = 0.038. Same hole.

Exact-string match of the gold *answer* in last-8 is also 2/282. Full-dialogue exact string is only 42/282 (0.15): the gold is often a date or paraphrase, not a span. That is why this page is retrieval of `dia_id`, not F1.

## Why last-k should win a *real* LoCoMo QA run

The questions want a fact written in an early session. Last-k is the wrong window at k=8 (0.7%). A submitted LoCoMo row uses the whole log, RAG, or a 100k context. C2 drops dull hours and rewrites gists. It will not beat a store built for this. Persist already showed the complementary fact: when the window *should* forget, C1 does.

## Cost

$0. No `reply` calls. Judge = id set membership.

## Second seed (persist, not LoCoMo)

P4 books do not use the LLM at encode. Seed changes wording only. Across the five P4 pairs, Δfp is identical inside a cell (0.536 static / 0.532 C2). A Grok seed-2 grid would replicate **mouth** columns, not the book. Command if you spend the API:

```bash
./run.sh run --release --example persist -- \
  --p4 --pairs 5 --seed 2 --last-k 8 \
  --out experiments/selmem-persist-p4-grok-seed2-n5.json
```

Not run in this window. Book claim does not wait on it.
