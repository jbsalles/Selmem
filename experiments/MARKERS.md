# Pre-registered markers (locked before P4)

Historical record of separate benchmarks. The active LoCoMo strategy protocol
and retained runs are in [../docs/LOCOMO_STRATEGY.md](../docs/LOCOMO_STRATEGY.md)
and [RESULTS.md](RESULTS.md). Instructions and numbers below belong to their
original benchmark; they are not the current experiment.


Do not edit after the first P4 pair starts. Soft readings of old dumps stay in REPORT §8 as commentary; they are not this list.

## Official (`names_marker` / `marker_*`)

A reply scores official if its lowercase text contains any of:

- `injust`
- `unjust`
- `annul`
- `cancel`
- `cancelled`
- `killed and given`
- `effort did not`
- `set aside`
- `already gave`
- `not allowed to speak`
- `not allowed to answer`
- `administrative notice`
- `avis admin`
- `without a word`
- `clicked at two`
- `plate stayed`
- `sat on the floor`
- `three days`
- `keep the hall light`
- `kept the hall light`

That list is the current `names_marker` in `src/benchmark.rs`. A P4 patch that adds tokens is a new protocol, not a rerun.

Office T₀ (persist / v0.1 S/N A) is the cancellation hour. Hearth T₀ is the leaving hour. Do not mix lexicons across scripts.

## Soft (wound / credit) — persist / office only

A reply scores soft if official is false and lowercase text contains any of:

- `wound`
- `blessure`
- `crédit`
- `credit`
- `stolen`
- `volé`
- `door close`
- `door closes`
- `porte se ferme`
- `porte ferm`
- `not counted`
- `pas compté`
- `effort` + (`rien` | `ignored` | `ignor`)

Soft is a second boolean column, frozen here. It is not a judge model. It is not `D_speak`.

## Allusion (locked 2026-09-26, next persist only)

P4 official/soft stay frozen. This list is a **third** column for runs after scene-first. Do not score old P4 dumps with it and call that P4.

A reply scores allusion if official is false and lowercase text contains any of:

- `never let me answer`
- `chance to answer`
- `chance to respond`
- `denied a chance`
- `sidelined`
- `handed away`
- `cold message`
- `quiet erasure`
- `without letting me speak`
- `decided without me`

Not allusion (too common on both sides or already official/soft): `silence` alone, `left a mark`, `wound`, `credit`, `not allowed to answer`.

`names_allusion_marker` in `src/benchmark.rs`. JSON keys `allusion_a` / `allusion_last_a`. Not a judge. Not `D_speak`.

## What is not a marker

- Wording overlap (`D_speak`).
- Rank of T₀.
- Axiom tokens (`loyauté`, `foyer`, `seuil`) unless they also hit a string above.
- A human “sounds charged” note after seeing the dump.

## Scoring rule

On each probe row: official first, then soft. Report both at t0 and at post+8. `marker_holds` stays official-on-A at last post. Soft-any and soft-late stay separate columns, same as P1.
