# Van Gogh reconstructed memory

This experiment compares four memory conditions using English translations of
Van Gogh's Nuenen and Arles letters. It samples 768 complete paragraphs per
period across edition machine dates and replays them in chronological order.
These are two historically distinct corpora, with the same input text across
arms within each period. Paragraphs with fewer than 12 or more than 76 words
are omitted. Three direct identifiers are masked throughout the input;
indirect identification remains possible.

| Arm | Input delivered to the actor |
| --- | --- |
| reconstructed | First-person prose synthesized from surviving SelMem narratives before exposure to the briefs. |
| rag | Unrewritten passages ranked separately for each brief by BM25 and semantic cosine with reciprocal rank fusion. |
| summary | First-person prose synthesized from the entire sampled original corpus. |
| survivor_summary | First-person prose synthesized from the original passages linked to the surviving narratives, grouped to match the reconstructed chapters. |

The same narrator model, prompt, eight-call allowance, and output limits
apply to all three frozen memories. Each chapter's declared citations are
checked against its source allowlist. Citations establish structural
provenance, not entailment. The narrator for the SelMem condition receives
only reconstructed trace text. Archived originals are supplied separately
to the baselines. The replay audit records passage IDs, engine trace IDs, before-and-after revisions, surviving trace
text, gist, core, fidelity, drift counts, and operation lineage where
available. Trace groups without attributable provenance are excluded.
A source with no surviving lineage is omitted by the replay pipeline;
the audit counts early admission and admitted passages without surviving lineage separately.

Run from the repository root with a recent stable Rust toolchain:

```bash
manifest=experiments/vangogh-reconstructed/Cargo.toml
letters=experiments/vangogh-realistic/source/vangogh/tei/2025-03-12/letters
out=/tmp/vangogh-reconstructed-8000-001

cargo run --release --locked --manifest-path "$manifest" -- prepare "$letters" "$out" 8000
cargo run --release --locked --manifest-path "$manifest" -- synthesize "$out" 48
cargo run --release --locked --manifest-path "$manifest" -- embed "$out" 1548
cargo run --release --locked --manifest-path "$manifest" -- plan "$out"
cargo run --release --locked --manifest-path "$manifest" -- actor "$out" 288
```

Prepare and plan are offline. Synthesize and actor use GROK_API_KEY and
grok-4.3. Embed uses SELMEM_EMBED_URL (the full OpenAI-compatible endpoint),
SELMEM_EMBED_MODEL, and optionally SELMEM_EMBED_API_KEY. Each numeric HTTP
ceiling applies per invocation. Repeating a stage reuses saved results.
If a frozen reconstruction or matched context is too short, the runner
stops and reports the problem. Delete only a specifically identified weak
chapter if deliberately regenerating it before planning.

The actor sees exactly the same cl100k_base token count across all 288
requests (within 0.5% of the configured ceiling), and a synthesized memory is
identical across briefs. The last paragraph may be cut by token-prefix
matching. The held-out commissions cover painting, printmaking, and staging.
Responses are condition-neutral, shuffled, and exported to blind.json.

Have independent raters read only blind.json. Use the following anchors
for the five separate features, each on an integer scale of 0 to 4:

1. Tonal modeling (hue/contrast or graphic silhouette = 0, modeled light and value = 4).
2. Rhythm (irregular isolated gestures = 0, explicit regular repetition = 4).
3. Tactile texture (smooth or unspecified surface = 0, prominently tactile material = 4).
4. Central focal emphasis (dispersed attention = 0, single dominant focal point = 4).
5. Atmosphere (primarily descriptive = 0, composition driven by mood = 4).

Rate task fit separately (0 is unusable, 4 is feasible and satisfies the brief).
Make one JSON array with records shaped as
`{"id":"copied from blind.json","features":[0,1,2,3,4],"fit":3}`.
Keep ratings outside the condition mapping until complete. Then run:

```bash
cargo run --release --locked --manifest-path "$manifest" -- score "$out" ratings.json
```

Primary descriptive measure: identify the source period from blind feature
ratings by learning period centroids on two media and testing on the third.
Average across the three held-out media; ties contribute half a hit. A score
above 50% can reflect historical differences already in the letters. Compare
arms, inspect the task-fit scores, and treat repeated samples from one brief
as correlated. This pilot describes cross-medium expression of a memory
history. It does not establish historical identity, chronological causality,
or a general creativity gain.
