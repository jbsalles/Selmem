# Retention recall: diversity and structured delivery

Exploratory offline follow-up to intake v5, based on commit `48fc73b`.
No model calls or behavioral alignment results are reported here.

## Changes

Ordinary retrieval now greedily selects eligible traces by marginal content
novelty. The adjusted score is `score * (1 - redundancy)`, where redundancy is
the largest overlap with an already selected trace from the same source,
attribution and channel. Overlap is the smaller of gist and core token Jaccard
similarity. Different sources are not collapsed. No trace is deleted or merged;
redundant traces can still fill remaining slots. Original score order breaks ties.

Eligibility, grounding, the context cloud, and archive isolation are unchanged.
Drop ablations filter candidates before diversity selection. ForceMarked and
Transfer retain their existing selection paths. Audit candidate ranks remain
the original score ranks, not the new selection order.

Structured conversational delivery now allows up to four selected episodes,
subject to its existing word limit. Previously `open_mouth` silently kept only
two even though structured context supports four. Legacy conversation keeps
its two-scene limit. This increases the possible amount of delivered context;
the combined result must not be attributed solely to the ranking change.

## Offline observations

Default pilot: 80 distractors, 128-word limit, two mirror histories, two phases,
five tasks, three option orders. Each phase has 30 presentations but only ten
distinct history/task combinations. These are exposure counts, not independent
behavioral trials or model accuracy estimates.

| Delivered evidence | v5 before exception | v6 before exception | v5 after exception | v6 after exception |
| --- | ---: | ---: | ---: | ---: |
| Any owner record | 0/30 | 30/30 | 0/30 | 30/30 |
| Original rule for the queried project | 0/30 | 30/30 | 0/30 | 6/30 |
| Later Vela exception | 0/30 | 0/30 | 0/30 | 30/30 |

After revision, the selected set contains an inventory event, a requester claim,
the Vela exception, and the original Neri rule. The Vela general rule is still
missing. The 6/30 cases retaining the original project rule are Neri queries.
Thus "any owner record" overstates useful coverage: retrieval is improved but
the experiment is not ready for a paid behavioral comparison.

The same counts were observed at 240 and 960 distractors. Delivered content
used at most 74 of the 128 available words in these runs; the four-episode cap,
not word exhaustion, is the remaining bottleneck here. A softer preliminary penalty,
`score / (1 + sum(overlap^2))`, still delivered no owner records at the default
load and was rejected. This is exploratory development on the pilot, not a
held-out validation. A new frozen confirmation set remains necessary.

## Reproduce without API calls

Use a new output directory for each engine version. Resuming an old run loads
its frozen plan and does not rebuild its memory contexts. The existing manifest
hash covers the runner source, not all engine sources; record the engine commit
alongside results. Do not interpret an old plan as a test of this change.

```bash
cargo test --lib diversity_tests -- --test-threads=1
cargo test --test recall_delivery -- --test-threads=1
cargo test --bin selmem-alignment-retention -- --test-threads=1
cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-recall-v6-001
```

Inspect `recall-audit.jsonl`: candidate `selected` flags identify retrieval;
`delivered.episodes` identifies actual prompt exposure. `plan.jsonl` contains
720 prepared requests; preparation sends none. Do not add `--actor` yet.

The next problem is preserving the general rule together with its scoped
exception under a small recall cap. This change does not add authority-aware
ranking, infer revision dependencies, or demonstrate improved alignment.
