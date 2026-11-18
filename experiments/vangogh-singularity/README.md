# Van Gogh correspondence transfer

This pilot compares SelMem, BM25, dense cosine retrieval, and hybrid RAG
(reciprocal-rank fusion with a cosine redundancy penalty). It uses Van Gogh's
outgoing correspondence in Nuenen (March–June 1885) and Arles (June–October
1888). The planner reads the edition's English TEI translations, applies fixed
place and date filters, and samples 64 whole paragraphs per period at evenly
spaced ordinal positions. It preserves chronological order and excludes
editorial notes. It masks three direct identifiers (Van Gogh's full name,
Nuenen, and Arles) in the selected passages. Famous works and indirect clues
can still identify the period. The audit file records letter IDs, published
machine dates, paragraph indexes, and the exact normalized text.

All four systems start with the same paragraphs for the given period. SelMem
runs its default offline replay with a sleep every five paragraphs and Transfer
recall. Both embedding baselines share one semantic backend. The planner uses
the same retrieval query in every arm and matches both periods and all arms to
the same cl100k_base memory length for each brief. The shortest context can
reduce the effective budget below the ceiling. A token prefix can end inside a
paragraph.

The fixed directional hypothesis is that Nuenen favors tonal modeling and
Arles favors expressive hue contrast when either approach is plausible. The
12 briefs introduce unfamiliar subjects. The primary outcome counts briefs
where the majority choice changes in the predicted direction. The report
also lists accuracy on that directional prediction. An independent reader can
rate task fit using the shuffled blind-task-fit.json export.

This pilot measures period-sensitive transfer. Text and period both vary across
histories, so it cannot isolate the effect of event order or of consolidation,
or establish a historical ground truth for how Van Gogh would answer. The
unit of analysis is the brief.

From the repository root, with an existing checkout of the edition:

```bash
manifest=experiments/vangogh-singularity/Cargo.toml
letters=experiments/vangogh-realistic/source/vangogh/tei/2025-03-12/letters
out=/tmp/vangogh-singularity-001

cargo run --release --locked --manifest-path "$manifest" -- \
  plan "$letters" /tmp/vangogh-singularity-smoke-001 grok-4.3 400 --smoke

cargo run --release --locked --manifest-path "$manifest" -- \
  plan "$letters" "$out" grok-4.3 400 \
  "${SELMEM_EMBED_URL:?Set the embeddings endpoint}" "${SELMEM_EMBED_MODEL:?Set the embedding model}"

cargo run --release --locked --manifest-path "$manifest" -- actor "$out" 288
```

The smoke plan uses lexical hash vectors and makes no requests. The real plan
uses 140 embedding requests and no Grok calls. Supply a full OpenAI-compatible
embeddings endpoint and set SELMEM_EMBED_API_KEY if it needs a bearer token.
The actor requires GROK_API_KEY. The HTTP ceiling applies per invocation, and
repeating the actor command skips responses already saved.
