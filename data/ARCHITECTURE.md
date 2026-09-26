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
    → dream/      weather → rewrite → merge → ladder → release
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
    retrieve.rs             rank (embedding ∪ lexicon ∪ mood ∪ access)
    judge.rs                DetachKind — pure, no trace, no I/O
    pull.rs                 grip, strikes, apply_grounding, mix
    ground.rs               re-exports
    narrator.rs, http.rs
  dream/
    night.rs                orchestrator + NIGHT_PASSES
    weather.rs              decay, unused disgust, status, latent
    rewrite.rs              neighbor retell
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

`NIGHT_PASSES = [weather, rewrite, merge, ladder, release]`.
`SHALLOW_PASSES = [weather, release]`.

`sleep()` spends a night only as deep as the budget: new Selfhood hours
since `store.last_deep_at`, or their arousal+disgust sum. Defaults
(`deep_min_hours = 1`, `deep_min_charge = 9.0`) keep one new hour as a
deep night so benches do not move. A second night with no new hours is
shallow. `sleep_deep()` ignores the budget.

Spoken utility is not recall. Live `remember` stamps `last_recalled_at`.
Only `speak` increments `rehearsals`, and only on the selected hour that
entered the mouth. Isolated probes stay read-only.

Merge writes edges and retargets axiom `support_trace_ids` onto the
keeper. Ladder will not mint an axiom with empty support; it unions
merge neighbors into the support list. `store.lineage` is schema +
edges + axiom supports. DropLineage uses that graph.

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
