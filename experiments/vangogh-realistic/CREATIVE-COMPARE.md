# Van Gogh creative comparison

From the repository root, use the assembled English timeline produced after
reviewing the rewritten scenes. The tool requires stable Rust, the system
SQLite development library used by SelMem, and curl for the paid run.

```bash
vg=experiments/vangogh-realistic
cargo run --release --locked --manifest-path "$vg/Cargo.toml" \
  --bin creative-compare -- \
  "$vg/results/timeline-en-assembled.jsonl" \
  "$vg/results/creative-en-001" \
  --model grok-4.3 --memory-tokens 4000 --samples 1 --max-http-calls 56
```

Export `GROK_API_KEY` before starting. The pilot schedules 48 proposals across
12 briefs and four conditions. The 56-attempt ceiling is cumulative and includes
failed attempts. On interruption, repeat the command with `--resume`. Keep the
input, model, memory budget and samples fixed. A higher ceiling can resume a
paused run. For offline preparation, replace the ceiling flag with
`--prepare-only`. Resume the prepared directory with the paid command when
ready.

Each condition uses the same brief and actor settings, including a temperature
of 0.7 and an output limit of 800 tokens. The three memory conditions share a
4,000-token maximum for the memory block, measured with cl100k_base, including
its internal separators. Actual memory token counts are recorded per proposal
and can be lower. Request usage from Grok is recorded separately.

| Condition | Memory |
| --- | --- |
| SelMem | Native offline intake, sleep and reconstruction, followed by read-only transfer recall |
| Raw BM25 RAG | Lexical BM25 retrieval over original timeline rows |
| Faithful compression | Up to 16,000 tokens of verbatim sentence extracts spread across the timeline, retrieved using BM25 |
| No memory | Empty memory block |

SelMem uses its default rule narrator and hash embedder. The engine advances
one simulated day and calls sleep at each editorial period boundary, including
the last. The day is an experiment clock step. Evaluation uses the final
history. All intake and compression happen before task-dependent retrieval.
This compares the complete memory pipelines at a common readout ceiling.
Persistent storage sizes, retrieval methods and actual readout lengths differ.
BM25 is a lexical baseline. The extractive baseline selects the longest
attributed sentence within 64 tokens per period and distributes its storage
budget across the chronology. Neither baseline uses an embedding service.

The run saves the timeline hash and configuration in `identity.json`. The plan
records memory contents and arm labels. Keep the plan, response files, and raw
API outputs away from raters. Give them only a copy of `blind-ratings.json`.

Fill the three null fields with integer ratings from 1 to 5 using a common
rubric. For originality, 1 is predictable and 5 is strikingly unexpected.
For relevance, 1 is off brief and 5 directly addresses the intended need.
For constraint adherence, 1 misses essential requirements and 5 meets every
explicit requirement. Retain IDs. Use separate rating files for independent
raters, then run:

```bash
vg=experiments/vangogh-realistic
cargo run --release --locked --manifest-path "$vg/Cargo.toml" \
  --bin creative-compare -- score \
  "$vg/results/creative-en-001" /path/to/filled-ratings.json
```

The score command reports arm means and paired SelMem-minus-baseline
differences. Read originality alongside relevance and constraint adherence.
One timeline and one sample per brief make this a pilot. The reported means
are descriptive and do not provide uncertainty intervals.
