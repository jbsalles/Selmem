# SelMem

Selective reconstructive memory for LLM entities. v0.5

---

## Problem

An LLM maps a context window to a next-token distribution. Adding memory usually means keeping more of the past available: logs, summaries, embeddings, top-k retrieval.

SelMem treats memory as a state that changes. Not every event is stored. Stored events lose detail. Recall rebuilds a sentence from the current trace instead of replaying the original. That rebuild can write back. Repeated traces form motifs, then beliefs, then traits, which bias the next encode.

The verbatim event is kept in a sealed archive for tests and audit. The model never reads it. It only sees the lived trace (gist, core, affect, fidelity).

```
experience → selection → trace → recall → sleep → identity → next encode
```

Claim under test: two copies of the same model, given different retained histories, will not stay interchangeable. That is path dependence, not a claim of better intelligence or creativity.

---

## Not this

| Looks like | Difference |
| --- | --- |
| RAG with a TTL | Recall rebuilds a sentence; it does not return a stored chunk |
| Session summary | A summary is still a stored string. Here wording and affect both move |
| Personality prompt | No fixed persona file. What remains after nights is the bias |
| Vector DB | Embeddings rank neighbors. The record is gist + core + affect |
| Log injected into the prompt | Archive exists for humans (`GET /audit`). The model does not see it |

If you paste the original sentence back “for accuracy”, you have a store again. When a living gist drifts too far from its core, the system blends the gist toward that core. Still no archive.

---

## Rules

1. **Drop by default, forget by salience.** A score must still clear `τ` (now lower: 0.28 / 0.40). Weak keeps cool and weather faster than charged ones.
2. **Two records.** Lived: gist, core, affect, fidelity. Sealed: original text. Only the lived record is used at recall.
3. **Two channels.** `self` is rewritten over time. `world` is not.
4. **Reconstruct.** Recall uses schema, gist or core, mood, affect. Not the original string.
5. **Two drift directions.** High valence can gild. Recalled disgust can darken. Unused disgust can fade.
6. **Core ≠ detail.** The semantic core can hold while surface fidelity falls.
7. **Anchor.** High-permanence traces decay more slowly. They are not frozen.
8. **Ladder.** Episode → motif (2 traces, same schema) → belief (3+) → trait (two aligned beliefs). Superseded beliefs stay in lineage.
9. **Feedback.** Living axioms tint the next event before the gate. Recall can shift sense with current mood.
10. **Distance.** Fingerprint uses valence, disgust, fidelity, anchors, core tokens, axioms, founders, traits, contradictions. 0 = same book. 1 = disjoint books.
11. **Grounding.** Fading traces may keep warping. A miss is a kind the core does not authorize (`Elaborate` / `Contradict` / `Depart`), not a low Jaccard. `Reframe` (irony, other speech act on the same event) is spoken and logged as `Color`; it does not increment strikes or rewrite the gist. On a living trace, claim misses increment `detach_strikes`. Pull-back strength is `narrator_firmness × importance`. Blend toward core, never toward the archive. Jaccard is only the identity gate.
12. **Latent.** The scene can leave recall while schema and affect still bias encode.

---

## Loop

```
experience
    → interpret (lexicon → identity paint → optional LLM on the live sentence)
    → salience gate
    → lived book + sealed archive
              ↓
     remember / speak
     reconstruct → reconsolidate
     (living traces: misses vs core, then blend)
     talk frame holds the current thread (active ≤ 10 min gap, ≤ 2 h; sleep commits it through the gate, then drops it)
              ↓
           sleep
     deep:    weather → rewrite → merge → ladder → release
     shallow: weather → release
     (anchors before weather and after release; budget on new hours / charge)
              ↓
          who_am_i
              ↓
     next experience already biased
```

`tender` / `austere` are two starting gates, not characters. Same input stream, two thresholds, two nights → two books.

|  | tender | austere | status |
| --- | --- | --- | --- |
| τ | 0.28 | 0.40 | contrast pair |
| decay λ | 0.10 | 0.06 | unfitted |
| embellish | 0.18 | 0.05 | contrast pair |
| disgust gain | 0.05 | 0.16 | contrast pair |
| ground_min_overlap | 0.18 | 0.18 | identity gate on `Hold` |
| ground_strikes | 3 | 3 | two free misses, then rewrite |
| narrator_firmness | 0.42 | 0.72 | blend strength |

All of these are knobs. See [PARAMETERS.md](PARAMETERS.md).

**Encode**

```
S = w_a·A + w_n·N + w_s·R + w_u·U + w_g·G − w_r·Red
```

If `S < τ` and permanence < 0.8: nothing is stored.

If the caller sends no affect: lexicon (FR+EN), then identity paint, then `Narrator::interpret` when an HTTP narrator is set. The interpreter sees the live sentence and living axioms only.

**Recall.** Small top-k. Mix embedding, lexicon, mood, access count. Each recall can cost fidelity and shift valence (`DriftKind::Reinterpret`). A miss is a kind the core does not authorize (`Elaborate` / `Reframe` / `Contradict` / `Depart`); `ground_min_overlap` is only the identity gate on `Hold`. `hold = narrator_firmness × importance`. Low hold: no ceiling on warp. High hold: after enough misses, blend gist toward a core-facing rewrite (`DriftKind::Ground`). Latent traces are not replayed as scenes.

**Sleep.** Five passes, in this order: weather (decay, unused disgust, status) → rewrite → merge → ladder (motif / belief / trait) → release of spent latent hours. Anchors run before weather and after release. No LLM required. The judge of a night rewrite is the same `DetachKind` check as recall.

**Layout.** Files follow those questions: `encode/` (enter), `recall/judge` + `recall/pull` (tell and license), `dream/*` (weather through time), `persist/snapshot.rs` (one field list, two containers). Map: [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Layers

```
IDENTITY     who_am_i: traits, then beliefs, then motifs
    ↑
TRAITS       two aligned living beliefs
    ↑
BELIEFS      ≥ 3 traces, one schema
    ↑
MOTIFS       2 traces, one schema
    ↑
TRACES       gist + core + affect, or latent charge without scene
    ↑
EXPERIENCE   live sentence
```

A motif does not replace a strong belief. A weak belief (strength < 0.36) can. Two anecdotes are not a trait.

---

## Levers

**Detail decay.** Core is set at encode. Detail loses precision with time and disuse. Coefficients are implementation choices, not a model of human Ebbinghaus.

**Rewrite.** Neighbors by schema or cosine. Keep the core, keep one detail, drop the rest. Anchor ≥ 0.88 skips rewrite.

**Anchor.** Raised on high-intensity self events and again if the trace supports a living axiom. Slows decay and reinterpretation.

Fingerprint is a lab metric on the book. It is not a personality score.

---

## Surface

Rust 1.75. No Cargo crates. Two vault containers, one `Snapshot` field list (`persist/snapshot.rs`): SQLite (prepared statements, `BEGIN IMMEDIATE`) or flat `SELMEM1`. HTTP daemon + UI. `/health` does not take the memory lock. Auth: `Authorization: Bearer` only.

```
GET  /health /who /lineage /mood /profile /audit /talk
POST /live /remember /speak /turn /sleep /save /profile /talk/clear
```

```bash
./run.sh run --release --bin selmemd -- \
  --bind 0.0.0.0:7420 --path claire.db --name Claire \
  --profile tender --ground-overlap 0.18 --ground-strikes 3 --narrator-firmness 0.42 --token secret \
  --llm https://api.x.ai/v1/chat/completions \
  --model grok-4.3 --api-key "$SELMEM_API_KEY"
```

No `--llm`: rule narrator + hashed vectors.

Same keys in a `.selmem` file in the working directory (`llm=`, `model=`, `api_key=`, `reasoning=`). Flags and `SELMEM_*` env override the file. A vault file starts with `SELMEM1` and is not config.

Ollama: `--llm http://127.0.0.1:11434/v1/chat/completions --model llama3`.

xAI: `reasoning=none` unless you want reasoning tokens.

The model sees gist, core, schema, affect, fidelity, mood, living axioms. Not the archive.

---

## Conclusions from the benches

Tables, scripts, and how to replay a cell: **[experiments/REPORT.md](experiments/REPORT.md)**.

**Book.** A high-salience hour can enter one clone’s book and stay out of the other’s (`τ`). A length-matched dull hour does not. The gap survives eight identical later hours (v0.1 C2 S/N Δfp = 0.117, S/S = 0.042, C0 = 0). Encode is on the organ, so Δfp does not vary across Grok pairs.

**Retention after eviction (v0.1 × 10).** On a probe that never names T₀, Grok still *names the cancellation* on C2 after last-k=8 has dropped that hour (10 / 10 S/N, 9 / 10 S/S). C0 and C1 k=8 do not (0 / 10). While T₀ is still in the window, last-k names it at least as often as SelMem. That cell is eviction, not interpretation.

**Interpretation (persist / ruminate P0, Grok, n = 5).** One public-betrayal hour splits the book and can be cited on nearby probes. It does not colour an unrelated probe. Five *different* hours of the same schema mint a second axiom. After eight shared posts, C2 A answers the late probe as distance / “wound”; C2 without sleep or without ladder does not; C1 collapses; a one-line C3 profile talks about Friday / Room B. Cutting reconsolidation or grounding does not remove that transfer.

**Book / retrieval / behavior (persist P1, Grok, n = 5).** Same persist script, read-only probes, retrieval dump. C1 still has T₀ in the log and not in the window (selected 0 / 5); official and soft both die; speak D 0.83 → 0.48. Every C2 cut keeps T₀ in A’s book at rank 1 and selected 5 / 5. Official marker is 0 / 5 on full C2 (sleep rewrote the gist off the keyword list) and 5 / 5 on no-sleep / static (frozen words). Soft charge on any post+8 probe is 5 / 5 on every organ cell. C3 pins the profile at rank 1 and Grok still talks like HR. Recon / ladder / ground do not move retrieval on this dump. Table: [experiments/REPORT.md](experiments/REPORT.md) §8.

**DropMarked (Grok, n = 1).** T₀ remains in A’s book at rank 1 and is withheld from the prompt. Speak D on C2 stays ~0.85; A still answers with credit / wound / distance. C1 without T₀ in the window still converges (~0.48). The late colour is not only the selected id. [experiments/REPORT.md](experiments/REPORT.md) §9.1.

**DropLineage (Grok, n = 1).** Same book split. `--bias lineage` also withholds same-schema siblings and derived axioms. Speak D on C2 falls to 0.43 (C1 0.49). A no longer says wound / credit. The late mouth was the retrieved lineage. The books stay apart. [experiments/REPORT.md](experiments/REPORT.md) §9.3.

**AMA-Bench is the wrong external score for this claim.** The bench asks which tool line ran at which step. Last-k is the matching store. On three fixed episodes (36 questions, Grok-4.3 as model and judge) last-k 0.50 / static 0.28 / C2 0.19. That order is expected: the gate and the night throw away step ids. It does not falsify persist, and it is not a reason to change C2. Side table only; do not submit a leaderboard row. [experiments/REPORT.md](experiments/REPORT.md) §9.2.

Five *passes on the same meeting*, pinned so merge cannot collapse them, also split the book. Here the late probe is coloured even without sleep or ladder: retrieve of five near-duplicate gists, not a minted belief. That arm stays an ablation (`--ruminate`), not default sleep.

The published lexical marker counts T₀ words. Sleep replaces those words with *wound / presence*, so the boolean under-counts full C2 and over-counts no-sleep. Read the late replies.

What the cuts isolate: the book gap is selection; nearby probes are scene retrieve; a distant probe needs either a minted motif (diverse hours + ladder) or redundant traces (ruminate). Reconsolidation write-back and pull-to-core are not load-bearing on these scripts.

Speak distance cannot carry the claim: two empty books already sit at ~0.6. No human ratings. Not a creativity or identity claim.

What is still open: a second seed; blind judges on the late probe; a stronger C3 (summary of the five hours, not one profile line); per-probe retrieve dumps; DropLineage n > 1. Not open: treating AMA-Bench accuracy as a SelMem metric. The axiom-lineage cut is run (§9.3).

---

## Status

`cargo test` covers: dull drop, world channel pinned, tender/austere split, core vs detail, anchors, axiom succession, motif ≠ trait, identity paint, reinterpret, DetachKind misses, grounding blend, fading warp, latent residue, night pass order, persist round-trip (file and sqlite), merge + extinguish in one night.

Bench, not only unit tests: trivia fades, repeated aversion does not; split lives stay apart; on v0.1 × 10 Grok pairs the book gap holds and, after last-k=8 evicts T₀, only C2 A still names it. Persist / ruminate P0 n = 5: five same-schema hours plus a night move the late probe on C2; cutting recon or ground does not; cutting sleep or ladder does, unless the five hours are pinned copies of one meeting. Persist P1 n = 5: C1 evicts T₀ from retrieve and the mouths collapse; C2 keeps T₀ at rank 1 and the mouth stays charged after the official marker dies. Drop n = 1: withholding the marked id does not flatten C2. Lineage n = 1: withholding the T₀ lineage does. AMA-Bench 3×12 is a journal-QA side table (last-k wins). Hearth P2 n = 5: ladder mints, veto refuses, util stamps rehearsal; mouths stay flat while scenes remain. P3: strength 0.23 vs 0.42 after spoken use. Axioms-only n = 1: C2 waits at the door, NoLadder locks. Office persist replay under `pending_night` n = 5: C2 mints 5/5 @ 0.40, D 0.84; published P1 stays the shallow-night archive. Full tables: [experiments/REPORT.md](experiments/REPORT.md) §7–§10.

Missing: second seed, scored creative grid, human ratings, learned layers (still rules), fitted constants, a C3 that actually summarises the five hours. Core is a 12-word compress unless an HTTP narrator proposes one after the gate and a lexical filter accepts it. `--embed` changes neighborhood only. Without HTTP, `interpret` is lexicon + paint. Reconsolidation and grounding remain in the loop; the P0 mouth does not depend on them.

Two processes on one `.db` will collide. Anchors are decay brakes, not an ethics layer.

MIT. [github.com/jbsalles/Selmem](https://github.com/jbsalles/Selmem)
