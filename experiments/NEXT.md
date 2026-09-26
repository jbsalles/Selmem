# Plan — organ first, LoCoMo on the horizon

Status: P4 locked the *measurement*. This file is the *intervention* plan.
Claim sentence does not move. Luna `D_speak` stays unpublished. No leaderboard.

Authoring rule: one causal cut at a time, pre-registered pass/fail, smoke on RuleNarrator before any Grok cell.

---

## 0. What the bench actually measured

Three columns, one script (12 dull + 5 same-schema + 8 shared posts, k = 8).

| Column | P4 result | Mechanism implied |
| --- | --- | --- |
| Book | 5/1 on every organ cell, both models. C1 evicts. Static and no-sleep already split. | Gate (`τ`). Night is **not** the splitter. |
| Retrieve | Observed selects T₀. Drop: selected = 0, rank may stay. Lineage drops the family. | Colour in the mouth is **what the prompt is allowed to see**. |
| Mouth (Grok) | Official C2 = 0. Soft 1/5 (static 3/5). Drop kills full-C2 soft. Lineage drops D to C1 range (~0.58). | Night **washes** the spoken residue. Remaining colour is **lineage-in-prompt**. |
| Mouth (Luna) | Soft 5/5 observed and under Drop; 0 under Lineage. C1 D already ~0.78. | Speaker stickiness, not an organ win. Do not tune the organ on Luna D. |

Invariant we refuse to break: book 5/1, C1 eviction, Δfp stable across pairs.

---

## 1. Causal model (use this, not vibes)

```
encode/gate     →  book split          [closed on this script]
     ↓
deep night      →  gist/valence wash   [open — static > C2 on soft]
  rewrite before ladder
  merge fuse_gist / mean valence
     ↓
ladder mint     →  axiom exists *after* the wash
     ↓
retrieve        →  selected id + siblings + axiom text
     ↓
speaker         →  official | soft | D_speak
```

Two independent failure modes:

1. **Wash.** T₀ words leave the gist before anything can retrieve them. Symptom: official 0, static softer than C2.
2. **Visibility.** Even a clean gist only colours the mouth if the family is in the prompt. Symptom: Drop / Lineage flatten Grok soft and D.

Cut A (shipped 2026-09-26) attacks wash *after* an axiom exists: rewrite skip + merge keeps gist/valence of axiom-backed keepers. Superseded the same day: night order is now `weather → ladder → rewrite → merge → release`.

---

## 2. Work packages

### WP0 — Invariants (continuous)

Do not start a cut if these fail on a RuleNarrator persist pair:

- C2 / static / no-sleep: T₀ in A’s book only (5/1).
- C1 post+8: T₀ absent from the window.
- Merge of two dull same-schema hours still allowed.
- `cargo check --lib`. Tests in `tests/night.rs` for cut A still compile-intent.

If a cut moves the book, revert. The book is not the patient.

### WP1 — Finish the wash (organ, now)

Hypothesis: official C2 dies because rewrite hits T₀ **before** ladder mints the axiom that would have protected it.

| Cut | Site | Change | Pass | Fail / revert |
| --- | --- | --- | --- | --- |
| **A** | `rewrite.rs`, `merge.rs` | Skip rewrite + keep gist/valence if living axiom supports the hour | Unit tests green | Book 5/1 moves |
| **B** | `night.rs` | Deep order becomes `weather → ladder → rewrite → merge → release` (or ladder twice: seed then confirm) | After one RuleNarrator C2 night, T₀ **core tokens still in gist** | Ladder with empty support; or merge of unrelated schemas |
| **C** | `rewrite.rs` | Also skip rewrite if `anchor ≥ 0.80` **or** ≥2 charged hours share the schema, even with no axiom yet | Same gist test on persist T₀ *before* mint | Trivia hours freeze; book bloats |
| **D** | `weather.rs` | Do not regress Selfhood valence toward 0 when `|v| ≥ 0.4` and `self_relevance ≥ 0.8` | Mean T₀ valence after night stays within 0.05 of encode | Dull hours stop decaying |

Sequence: **B then C then D**. Not in parallel. Smoke each with one persist pair, RuleNarrator, inspect T₀ gist/core/valence. No Grok until B+C pass the gist test.

Stop WP1 when: T₀ gist after deep night still contains the encode core (or a lexical overlap ≥ current `ground_min_overlap`). Soft Grok is *not* the WP1 metric.

### WP2 — Visibility (organ, only if WP1 gist is clean and Grok soft still dead)

Hypothesis: once the book text is intact, Grok still needs the family in the prompt.

| Cut | Site | Change | Pass | Fail / revert |
| --- | --- | --- | --- | --- |
| **E** | `retrieve.rs` | Additive score `+δ` if the trace is in any living axiom’s `support_trace_ids`. Freeze `δ = 0.12` before the run | Observed still selects T₀; Drop still drops the id | Lineage no longer flattens Grok D — δ too large; you built Force |
| **F** | `scoring.rs` | Cap how hard a neutral mood can bury `|valence| ≥ 0.5` | T₀ rank on late probe does not fall when mood ≈ 0 | Every charged hour outranks the query |

Run E as a **diagnostic** against Drop and Lineage on n=1 Grok before n=5. If E cancels Lineage, shrink δ or drop E. Visibility must remain *ablatable*. That is the scientific value of Drop/Lineage.

### WP3 — Measurement (not a new claim)

Only after WP1 gist test, and only the cells the cut can change.

- One Grok `--p4` observed pair is a smoke, not a paper.
- n=5 same seed/k only when WP1 or WP2 claims a mouth movement.
- Seed-2 Grok mouth is a replication of **wording**, not of the book. Do it once if n=5 seed-1 soft moves.
- C3-as-summary of the five hours is a **control**, not an organ feature. Build it when you need to say selection+gist ≠ a paragraph in the prompt.
- `+24` at same k is a **duration** question. It does not debug wash. Schedule it after WP1, as a separate measurement: “does clean gist still speak after three eviction windows?”

Do not lengthen k. That repairs C1.

### WP4 — Explicitly out of scope

- Fitting `τ`, firmness, merge_similarity to recover +8 soft.
- Persona / system-prompt “identity”.
- Publishing Luna D.
- AMA or LoCoMo as persist scores.
- Five organ patches in one night.
- Learned layers, new profiles, creativity table.

---

## 3. LoCoMo — horizon work package (WP-L)

Not this cycle. Keep it specified so we do not invent a store when the time comes.

### Object

LoCoMo asks for a fact written in an **early session** of a ~300-turn, ~20-session dialogue. last-k=8 answers a different question (0.007 evidence-id hit on category 1, n=282). Full log is an oracle (0.996). SelMem is neither.

### Method when we open it

1. Freeze the same subset as `EXTERNAL.md` (10 convos, category 1, n=282) before any encode.
2. Map **one LoCoMo session → one hour**. Not one turn → one hour. Channel: World if the session is factual small-talk, Selfhood if it carries preference / event / affect. Timestamp = session datetime.
3. Gate as today. Early dull sessions that contain the gold `dia_id` **will** be dropped sometimes. That miss is a real SelMem error, not a bug to patch with Log-stuffing.
4. At question time: `remember(q)` only. Score = any gold `dia_id` belongs to a **retrieved session**. Same metric as the $0 page. No LLM judge until this retrieval number exists.
5. Comparators on the same subset: last-8, last-24, session-oracle (all sessions, no gate). Never full-turn RAG.

### Target band (pre-registered, not a goal to climb)

| Method | Expected |
| --- | --- |
| last-8 | 0.007 (measured) |
| last-24 | 0.046 (measured) |
| session-as-hour + gate + retrieve | **(0.05, 0.40)** on category 1 |
| all sessions, no gate | ceiling for this method, << 0.996 turn-oracle |

Below 0.05: we built last-k with extra steps. Above 0.40 on this metric without stuffing turns: unexpected; audit leakage (session text containing later answers, or retrieve cap raised in secret).

A later QA F1 against gold *answers* is a different paper. Do not mix it into the persist claim. Do not submit.

### When to open WP-L

WP1 gist test green. WP2 either shipped or rejected. One engineer-week, one conversation as a dry run (`conv-26` only) before the 282.

### Anti-goals

- Vendoring `locomo10.json`.
- Turn-level Log channel as “memory”.
- Raising last-k to fake a LoCoMo score.
- Using LoCoMo miss rate to retune `τ` on persist.

---

## 4. Order of work

```
WP0 always
    → WP1-B (night order)
        → WP1-C (pre-axiom rewrite skip)
            → WP1-D (weather valence) if valence still collapses
                → RuleNarrator persist gist audit
                    → optional 1× Grok C2 vs static soft
                        → WP2-E only if gist clean and soft still dead
                            → WP-L dry run on conv-26   [later]
```

Calendar is not a virtue. A failed gist audit ends the day.

---

## 5. Decision log

| Date | Decision |
| --- | --- |
| 2026-09-26 | P4 is the locked persist grid. Claim frozen. |
| 2026-09-26 | Cut A shipped. Wash incomplete: ladder still after rewrite. |
| 2026-09-26 | More bench without an organ cut will not move official C2. |
| 2026-09-26 | LoCoMo is horizon, session-as-hour, band (0.05, 0.40), not SOTA. |
| 2026-09-26 | Wash + visibility in code: night order ladder-before-rewrite; rewrite skip axiom/anchor/2-charged; charged gist survives weather+sculpt; retrieve +0.12 on axiom support. |
