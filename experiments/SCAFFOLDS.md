# Scaffolds (axes / RAG / blind rate)

## 1. Second axis
`DOMAINS_RISK` in `src/bin/selmem-contextual-longlife.rs` (4 domains: invest, route, deadline, ally).
Contrast is risk/caution, not calm/social.
The ordinary longlife runner selects these with `--axis risk --pairs 1..4`.
History construction, retrieval and reporting use the same selected domain set.
The causal v2 runner remains a calm/social protocol; it does not accept `--axis`.

## 2. RAG baseline
`rag_baseline(store, query, k, words)` in `causal_v2.rs`.
Lexical overlap over active, unsuppressed observed Episode traces. Ties use trace
IDs. Whole gists that do not fit are skipped, including the first gist. No HTTP.
This side file is not an actor-evaluated RAG arm. The fixed `choice` query is a
scaffold, not a task-matched retrieval comparison.
Equal-metadata embedding RAG still to be plugged (HashEmbedder cosine).

## 3. Blind ratings
Existing tools already strip labels:
- `selmem-genesis` offline: `rate-creative`, `rate-preview`, `rate-life`
- creative life writes `blind` items via `creativity::blind`
Scaffold: call those tools on a fresh `--out`; do not pass arm names to the judge.


## Launch
```bash
# second axis (longlife, offline qualification)
cargo run --release --bin selmem-contextual-longlife -- \
  --out /tmp/selmem-risk --axis risk --pairs 1

# RAG side file (causal, after snapshots exist)
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation --snapshots /tmp/selmem-snapshots --out /tmp/selmem-rag \
  --recall-route context-cued --pairs 1 --repeats 2 --rag
# writes /tmp/selmem-rag/rag.jsonl
```
