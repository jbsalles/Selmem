# Scaffolds (axes / RAG / blind rate)

## 1. Second axis
`DOMAINS_RISK` in `src/bin/selmem-contextual-longlife.rs` (4 domains: invest, route, deadline, ally).
Contrast is risk/caution, not calm/social.
Not yet wired into `--pairs` (still capped at the 12 calm domains).
To use: select from `DOMAINS_RISK` in a dedicated runner or extend the pair loop.

## 2. RAG baseline
`rag_baseline(store, query, k, words)` in `causal_v2.rs`.
Lexical overlap ranking, word-budgeted. No HTTP.
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
