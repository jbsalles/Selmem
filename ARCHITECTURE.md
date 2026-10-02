# Architecture

The organ is a changing state, not a store. The files follow the scientific
questions, not generic Service / Port / Adapter layers.

How does an hour enter? How is a telling judged? How does a trace weather?
How is the book saved? Those are four directories.

Behaviour is unchanged: same order, same knobs, same benches.

## Questions

```
experience
    → encode/     interpret → paint → split → gate → core
    → lived book + sealed archive
    → recall/     retrieve → reconstruct → judge → pull
    → dream/      weather → confab → ladder → centers → rewrite → merge → release
    → persist/    one Snapshot, two containers
    → next encode already biased
```

`engine.rs` only names that order. `WorkingTalk` is not a trace: sleep runs
each recorded turn through encode with `hold = false`, then drops the frame.

## Tree

```
src/
  engine.rs                 live / remember / speak / sleep
  core/                     model (incl. OrganCut), store, profile, talk
  encode/
    interpret.rs            lexicon + optional Narrator::interpret
    paint.rs                living axioms tint the hour
    split.rs                lossless excerpts, else line/word pack
    gate.rs                 S vs τ, write the trace
    core.rs                 accept_core after the gate
    intake.rs               EncodeInput / EncodeDecision
    scoring.rs, affect.rs, embed.rs
  recall/
    retrieve.rs             rank (embedding ∪ lexicon ∪ mood ∪ access); skip suppressed unless ForceMarked
    judge.rs                DetachKind — pure, no trace, no I/O
    pull.rs                 grip, strikes, apply_grounding, mix
    ground.rs               re-exports
    narrator.rs, http.rs
  dream/
    night.rs                orchestrator + NIGHT_PASSES
    weather.rs              decay, unused disgust, status, latent
    confab.rs               fill a collapsed gist from axiom / center
    centers.rs              one prototype per schema; Internal gravity
    rewrite.rs              neighbor retell; skip = attribution × conflict
    merge.rs                close Selfhood episodes
    ladder.rs               motif → belief → trait
    release.rs              spent latent hours (already latent yesterday)
    drift.rs, singularite.rs
  persist/
    snapshot.rs             field list: assemble_* + profile params
    file.rs                 SELMEM1 bytes
    sqlite.rs               rows
  net/, config.rs, lexicon.rs
  bin/                      selmemd, selmem-chat
```

## Encode

Order in `engine::ingest` (and `commit_talk` with `hold = false`):

```
interpret → paint → maybe talk.hear → split → gate → maybe accept_core → blend mood
```

That order is the organ. Identity paint is not interpretation: the book tints
the hour *before* the gate. `commit_talk` must not grow a second path.

## Recall and grounding

Embeddings **rank**. They do not decide a miss.

`judge.rs` is pure: `(generated, core) → DetachKind`.

| kind | meaning | miss? |
|---|---|---|
| `Hold` | same claim | only if Jaccard `< ground_min_overlap` |
| `Compress` | detail fell off | no |
| `Elaborate` | cause / stake the core never licensed | yes |
| `Reframe` | same event, other speech act / irony | no (speak; `Color`; gist stays) |
| `Contradict` | denial | yes |
| `Depart` | not this event | yes |

`pull.rs` applies policy: grip = `narrator_firmness × importance`. World
skips the judge. Cold / myth / latent have grip 0. After enough strikes the
gist is blended toward a core-facing rewrite (`DriftKind::Ground`). A
`Reframe` is logged as `Color` and does not increment strikes. The
lexical mix is still the rewrite; replacing it is a later behaviour PR.

The judge uses no LLM. `HttpNarrator` may reconstruct or recontextualize.
It does not classify the miss.

## Night

`NIGHT_PASSES = [weather, ladder, rewrite, merge, release]`.
Confab runs after weather. Centers refresh + gravitate after ladder, before rewrite.
Do not add those names to the constant: `tests/dream_order.rs` pins the five.

Anchors run before weather and after release, as before. Traces already
latent *before* this night are the only ones `release` may drop, and only
if no living axiom lists them. A belief minted in `ladder` therefore still
protects its evidence the same night.

`tests/dream_order.rs` pins the name list. Do not swap passes to “clean up”
a file: the book will move and unit tests on a single trace will not see it.

## Persist

One `Snapshot` (`profile`, `mood`, `store`). Two containers:

| path | backend | why |
|---|---|---|
| `.db` / `.sqlite` / `.sqlite3` | sqlite | daemon, `BEGIN IMMEDIATE` |
| anything else | `SELMEM1` file | no `libsqlite3`, readable vault |

`snapshot.rs` is the field list (`assemble_trace`, profile params, tokens).
Trace header after P7: 18 fields (congruence, confidence, suppressed). Old 15- and 16-field files load with defaults (`congruence 0.5`, `confidence 1.0`, `suppressed 0`). `centers N` sits after axioms; absent means an old vault.
Backends only choose bytes vs rows. The model never reads the vault.
`verbatim` belongs in persist + `audit` only. `WorkingTalk` is not in the
Snapshot.

## Ports

Two, already:

- `Narrator` — reconstruct, interpret, extract_core, segment, rewrite, reply
- `Embedder` — neighborhood only

A third (`entail`) does not exist. If it arrives, fallback is the local judge.

## Tests that lock the cuts

| File | What it pins |
|---|---|
| `tests/ground.rs` | DetachKind on the John examples |
| `tests/dream_order.rs` | night pass order |
| `tests/engine.rs` | grounding policy, sleep, persist round-trips |
| `tests/core.rs` | accept_core / twelve-word compress |
| `tests/attribution.rs` | field + persist; External hold / Internal rewrite |
| `tests/congruence.rs` | measure + gate bonus + live conflict |
| `tests/centers.rs` | prototype mint; Internal gravity; External stays |
| `tests/confab.rs` | hole fill ≠ embellish; core frozen |
| `tests/suppress.rs` | directed forgetting; confidence ≠ access |


## Grounding and anchor (2026-10-01)

CoreJudgement now carries event, claim, causal, entity, polarity, novelty. Two different because-clauses are Elaborate even when the act words match. A shared departure is not a Depart. RealityAnchor.claim is frozen at encode; reinterpret() revises interpretation and records an operation, it does not rewrite the anchor or the archive. MergeDecision refuses a fuse when affective difference is at least 0.55, anchors conflict, or valence signs oppose.

## Wash (2026-10-02)

Rule narrator, seed 1, probe with no content-word hit on the cancellation. Wow criterion failed on salient, neutral, nosleep, and noladder. After two nights the daily motif wins the sentence; the cancellation stays Active and selected. Only the no-ladder arm speaks the cancellation on that probe. Dump experiments/selmem-wash.json.

## Wash charge (2026-10-03)

Ladder mints on charge (|v| * arousal * self_relevance), not headcount. One wound at charge >= 0.28 can found a motif; charge >= 0.45 can found a belief. Dull permanence < 0.40 weighs a quarter. A choice probe ranks by charge and, if a charged axiom exists, answers from the belief without reciting the hour. Dump experiments/selmem-wash.json: salient wow, neutral/nosleep/noladder not.
