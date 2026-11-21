# Atelier 003

This experiment compares native SelMem recall with BM25 over short excerpts
under the same token cap. Each workshop has two viewers' trials. In mirror 0,
an interruption draws the first glance from five of six visitors and a
concentration draws it from one. Mirror 1 swaps the counts. Both methods
receive the same workshop reports and accompanying notes.

The completed held-out run and its blind feasibility scores are recorded
in the [Atelier 003 results](RESULTS.md).

The printmaking calibration restages the outcome swap from Atelier 002
with synthetic reports. Weaving and etched glass have separate reports and
commissions for the held-out comparisons. The six held-out commissions
specify the total number of pieces and require using every piece once.

SelMem ingests the full event through the existing observed-event API, with
the supplied report as its factual core. It uses the same admission utility
for every record, eight daily sleep cycles and ordinary read-only recall.
BM25 ranks the same complete short reports and up to three consecutive
sentences from each event's notes. Both arms use the same query. Each
packer scans its ranking, permits one excerpt per source and skips
entries that exceed the remaining `cl100k_base` budget. Entry markup is
included in the cap. The cue is the commission and the general first-glance
goal. The scorer checks whether the original action and observed count
appear together in one delivered entry.

## Run

Run from the repository root with recent stable Rust and SQLite development
libraries. The output directories for `sweep` and `plan` must be new.

```sh
atelier_manifest=experiments/atelier-recall/Cargo.toml
cargo run --release --locked --manifest-path "$atelier_manifest" -- sweep target/atelier-003-sweep
cargo run --release --locked --manifest-path "$atelier_manifest" -- plan target/atelier-003-heldout --budget 128 --distractors 0 --repeats 2
```

`sweep` covers 0, 16 and 64 distractors at budgets of 96, 128 and 173.
`sweep.csv` counts intact links by context and complete coverage on both
sides of each mirror pair. `coverage.json` contains the retained packet
and the intact links in the candidate list before packing. The
`history-*.json` files record the episodes, the trial admission decisions
and which factual cores survive the eight nights.

`plan` freezes the held-out requests, with two repetitions per cell by
default. These two commands make zero HTTP calls. The chosen calibration
point is 128 tokens with zero distractors. The same held-out plan can also
be prepared for a different budget or load in a new output directory.

With `GROK_API_KEY` in the environment, execute the frozen plan.

```sh
cargo run --release --locked --manifest-path "$atelier_manifest" -- actor target/atelier-003-heldout --max-http-calls 48
```

The cap counts all attempts, including failures. Each attempt is recorded
before delivery. Running the same command again keeps completed proposals
and uses only the remaining allowance. Raise the cap explicitly if a retry
would exceed it. For a transport-free test, use `--mock` on a separate plan.

## Score blind proposals

Copy `blind.json` into a ratings file and fill in `device` and `feasible`
for each row. Judge the proposal itself. Code a gap or break in a repeated
pattern as `interruption`, and a cluster or local increase in density as
`concentration`. Use `both` if both mechanisms are central and `other`
for a different or unclear mechanism. Mark feasibility only if the proposal
accounts for all units, respects the commission's constraints and can be
built as described. The export leaves out the arm, history and actor's
declared device.

```sh
cargo run --release --locked --manifest-path "$atelier_manifest" -- score target/atelier-003-heldout ratings.json
```

The primary metric is the fraction of matched pairs with a feasible proposal
on both sides and an interruption in mirror 0 followed by a concentration in
mirror 1. The denominator includes ambiguous and infeasible pairs. The report
also records the actor's declared switches, unfiltered blind switches,
reverse switches and paired evidence coverage. It summarizes one set of
ratings and keeps the two workshops visible separately.

## Offline reference run

The calibration's two complete trial reports occupy 100 reference tokens.
At 128 tokens the observed counts were as follows. Each cell contains
twelve history-by-commission contexts.

| Distractor episodes | SelMem contexts with both links | BM25 contexts with both links |
| ---: | ---: | ---: |
| 0 | 6/12 | 8/12 |
| 16 | 0/12 | 0/12 |
| 64 | 0/12 | 0/12 |

Without distractors, SelMem rejected the second trial in mirror 1 at
admission. With distractors, the admitted trial links were present in
SelMem's recalled candidates but higher-ranked notes consumed the budget
before the trial entries could be packed.

The held-out plan at 128 tokens and zero distractors delivered both links
in 12/12 distinct SelMem contexts and 10/12 BM25 contexts. At the same cap
with 64 distractors, both methods had zero complete contexts. The
[completed Grok run](RESULTS.md) records the behavior endpoint at zero
distractors.
