# P4 locked bench

2–4 weeks. Lever #1. Not a feature.

**Claim:** After an hour retained on one side only, two copies of the same model do not stay interchangeable once the following stream is identical. Measured on the book, retrieve, and the mouth.

Personality, creativity, “like a human,” LoCoMo/LongMemEval accuracy as a SelMem score: hors claim. The external run exists so that sentence is on the page next to numbers.

## What this run must close

| Attack | Close |
| --- | --- |
| “Static / no-sleep is a footnote” | Same `k`, same script, same pairs, same JSON table as C1 / C2. |
| “Drop is n=1” | DropMarked and DropLineage at n ≥ 5, same model, same seed policy as persist. |
| “Luna is a replay” | Second model runs the **full** P4 grid, not office-night leftovers. |
| “Soft marker invented after the dump” | [MARKERS.md](MARKERS.md) frozen before pair 001. |
| “You hid the long-memory benches” | One LoCoMo **or** LongMemEval page. Claim on that page: we are not built for this. Print the numbers. |
| “Can’t replay” | One command block below, commit hash, machine, API cost. |

Published P1 / Drop n=1 / Luna n=1 stay in REPORT as history. They are not P4.

## Grid (main cells)

Stimulus: `data/v01_persist.json`. Arm: S/N only. Probes: t0 + post+8, read-only. `last_k=8` on every cell.

| Cell | Flag | Why it is in the table |
| --- | --- | --- |
| C1 | default grid | last-k control at the same k |
| C3 | default grid | summary + one profile line |
| C2 | default grid | organ |
| C2NoSleep | default grid | no-consolidation, same k |
| C2Static | default grid | freeze after gate, same k |
| C2NoRecon / NoLadder / NoGround | default grid | keep; they are already coded |
| DropMarked | `--bias drop` | withhold marked id at recall |
| DropLineage | `--bias lineage` | withhold T₀ family + derived axioms |

Do not bury C2Static / C2NoSleep under “ablation note.” They share the header with C1 / C2.

Hearth P2/P3 and ruminate are **out of P4**. Replay them later if the persist grid holds.

## Models

| Slot | Model | Role |
| --- | --- | --- |
| Primary | `grok-4.3` | published numbers |
| Second | `gpt-6-luna` (or the current Luna id in `.selmem`) | full grid, not n=1 replay |

Same seed policy on both: `--pairs 5 --seed 1`. Temperature 0, `reasoning=none`. Only `SpeakOnlyHttp` hits the API.

A Luna cell that only reruns office persist + door probe is **not** the second model.

## n and seed

- Persist observed: n = 5, seed 1.
- DropMarked: n = 5, seed 1.
- DropLineage: n = 5, seed 1.
- Same three rows on Luna.

Do not raise n mid-run because a cell looked noisy. If you need n = 10, that is a new line in this file before the first pair.

## Markers

Frozen in [MARKERS.md](MARKERS.md). Official list = `names_marker` at the commit below. Soft list = wound / credit / door, also in that file. No post-hoc tokens.

## External bench (one page)

Pick **one**: LoCoMo or LongMemEval. Not both in this window.

- Adapter lives under `experiments/locomo/` or `experiments/longmemeval/`.
- Methods: last-k (k=8), static book, C2. Same three as AMA.
- Judge: the bench’s official script if it has one; otherwise the same model as the cell, declared.
- Sample: small enough to finish in this window (write the subset ids here before running).
- Page: `experiments/EXTERNAL.md` after the run. First sentence: **SelMem is not a long-context QA store. These numbers are expected to lose to last-k.**
- Do not submit a leaderboard row.

AMA-Bench stays a side table. Do not expand the 3-episode set in this window.

## One command

Fill `api_key` in `.selmem`. Then, from the crate root:

```bash
git rev-parse HEAD
uname -a
cp data/config.example .selmem   # model=grok-4.3 (then again for Luna)

# or: ./experiments/p4.sh grok
# observed grid (C1 / static / nosleep / C2 / C3)
./run.sh run --release --example persist -- \
  --p4 --pairs 5 --seed 1 --last-k 8 \
  --out experiments/selmem-persist-p4-grok-n5.json

# DropMarked
./run.sh run --release --example persist -- \
  --p4 --bias drop --pairs 5 --seed 1 --last-k 8 \
  --out experiments/selmem-persist-p4-drop-grok-n5.json

# DropLineage
./run.sh run --release --example persist -- \
  --p4 --bias lineage --pairs 5 --seed 1 --last-k 8 \
  --out experiments/selmem-persist-p4-lineage-grok-n5.json
```

Repeat the three lines with `model=` set to Luna and `-luna-n5` in the filename.

Header on stdout must show `LLM … model=… pairs=5 seed=1 last_k=8`. If it says `RuleNarrator`, the run is void.

Offline contract (does not replace the live grid):

```bash
./run.sh test --test benchmark
```

## Machine / cost (fill at run start)

| Field | Value |
| --- | --- |
| Commit | _fill `git rev-parse HEAD`_ |
| Date | _ISO date_ |
| Machine | _uname -a_ |
| OS / rustc | _rustc -V_ |
| Primary model | grok-4.3 |
| Second model | gpt-6-luna |
| `temp` / `reasoning` | 0 / none |
| Approx. `reply` calls | persist grid × 5 pairs × probes × 2 sides; count from the first pair and write it here |
| API cost primary | _$_ |
| API cost second | _$_ |
| External subset | _ids + cost_ |

A finished P4 dump without this table filled is not published.

## What P4 is allowed to change in REPORT

- New §11 with the three-column tables (book / retrieve / mouth) for Grok and Luna.
- Drop and Lineage at n = 5 in that section.
- C2Static / C2NoSleep in the **same** P4 table as C1 / C2.
- One EXTERNAL page.

What P4 is not allowed to change: the frozen claim sentence, MARKERS.md after pair 001, k, seed, script, or “Luna n=1 therefore Grok generalizes.”
