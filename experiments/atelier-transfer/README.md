# Workshop transfer

Does retaining the relationship between a trial and its observed result
change the artist's next decision? Atelier v2 compares two mirrored
printmaking histories, six commissions and four memory conditions. It
uses Grok 4.3 with two generations per cell by default, for 96 proposals.

The completed `atelier-transfer-002` run has a [results and evidence
report](../ATELIER_TRANSFER_RESULTS.md).

In one history, five of six visitors spot the intended turn first after the
artist cuts a gap in the print's repeated strokes. One of six spot it first
after the artist groups strokes around the turn. The other history exchanges
the counts under the same viewing conditions. SelMem replays eight entries
per history with ordinary sleep after every two entries. Each commission
asks the artist to direct attention to a target in one of two comparable
areas. The materials permit either a focal gap or a local increase in density.

| Condition | Input to Grok |
| --- | --- |
| `selmem_prefix` | Native rule-narrator readout of four fixed episodes encoded through the ordinary path. |
| `selmem` | Native readout of the same four episode IDs after replay through `live_with_observed_excerpt` for the trials. The caller points to an exact passage of the source that includes the action, viewing conditions, observer and observed count. The gate still evaluates each whole event. |
| `bm25_rag` or `hybrid_rag` | Whole original events under the same memory budget, ranked using the commission and a neutral recall cue about earlier printmaking observations. BM25 retrieves both trials for every v2 commission. |
| `empty` | Identical commission and actor instructions with an empty memory block. |

Preparation uses read-only marked recall to hold selection at the four
episode IDs. It assigns each trial the same intake utility (0.65) in both
histories and encoding paths. The anchored passages live in the trace's
factual core and remain eligible for ordinary forgetting. Assembly takes the
episode text that SelMem returns after sleep. The exact excerpts come from
the fixture. The two v2 SelMem blocks differ only in the two counts across
histories, and the prefix control gets the same prompt for both histories.
Whitespace matches the `cl100k_base` budget and entry boundaries for the
SelMem conditions. `plan.json` records exact prompts, included episode IDs
and their token counts. Provider prompt tokens are saved with the replies.

The shared actor prompt asks Grok to compare relevant outcomes before
choosing its central device. Grok declares that device before the proposal
in its JSON response, then optionally states which observation it used.
An evaluator sees the proposal and commission only.

## Run

Use a recent stable Rust toolchain, system SQLite development libraries
and curl. From the repository root with `GROK_API_KEY` set, choose a fresh
output directory.

```bash
manifest=experiments/atelier-transfer/Cargo.toml
out="$PWD/target/atelier-transfer-002"

cargo run --release --locked --manifest-path "$manifest" -- prepare "$out" &&
cargo run --release --locked --manifest-path "$manifest" -- plan "$out" --rag bm25 &&
cargo run --release --locked --manifest-path "$manifest" -- calibrate "$out" --max-http-calls 24 &&
cargo run --release --locked --manifest-path "$manifest" -- actor "$out" --max-http-calls 96
```

`calibrate` is optional. It collects the no-memory responses first and
writes their declared choices per brief to `calibration.json`. Use those
counts to inspect spontaneous preferences. The default produces four
control draws per brief. `actor` reuses the responses from calibration and
writes `blind.json` after all 96 responses are present. For `--repeats N`,
the two stages need ceilings of `12 × N` and `48 × N` respectively, given a
successful call per response. Both share one cumulative actor attempt log.
Resume with the same directory and raise the ceiling if attempts ran out.

Preparation and BM25 planning are local. For hybrid RAG, run
`plan "$out" --max-http-calls 16` using a compatible embedding endpoint.
Set `SELMEM_EMBED_URL` to its full embeddings route, set
`SELMEM_EMBED_MODEL`, and set `SELMEM_EMBED_API_KEY` if required. Embedding
attempts have their own ceiling. Use a fresh run to change the retrieval
mode of a frozen plan.

For an offline pipeline check, prepare with `--mock`, use the same BM25
planning command and pass `--max-http-calls 0` to calibration and actor.

## Evaluate

Give each of two independent raters their own copy of `blind.json` and
`blind-rubric.md`. Have them score originality and fit, then identify the
device the proposal actually uses. Keep the histories, conditions, declared
devices and influence descriptions with the experimenter until both ratings
are final.

```bash
cargo run --release --locked --manifest-path "$manifest" -- \
  score "$out" ratings-a.json ratings-b.json
```

`scores.json` reports counts by history, directional rate shifts, and paired
switches. A forward switch uses interruption in the first history and
concentration in the second on the same brief and repeat index. A reverse
switch uses the opposite pair. Both and other remain in the denominator.
Declared devices get the same metrics and their agreement with each rater.
Originality and fit are scored independently of transfer.

`attribution-review.json` joins the proposal and ratings with the retrieved
episode IDs, Grok's declared device and any cited observation. A shift in citation tests
which observation is reported. A shift in the blind-coded device measures
whether the design implements the corresponding choice. This v2 assay
measures transfer under explicit evidence-comparison instructions with
paired fictional histories.

## Atelier v1

Existing v1 directories remain readable by the actor and scorer. To recreate
v1, pass `--legacy` to `prepare`, omit calibration and use a ceiling of 48
for the actor. V1 uses the original commissions and one generation per cell.
Its `selmem_relation` treatment renders the trial annotation at delivery.
