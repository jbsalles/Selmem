# Parameters

None of the numbers in SelMem are fitted to human data. They are **exploratory knobs**: chosen so that two profiles on the same corpus diverge in a few nights, and so that tests stay readable.

What the literature supports is the *direction* of a mechanism, not the coefficient.

---

## How to read a number

| Status | Meaning |
|---|---|
| Literature (qualitative) | A named finding or model family justifies the *shape*, not the constant |
| Contrast pair | Two values exist only to split `tender` / `austere` |
| Discrete convenience | A count that stands in for a continuum we have not modelled |
| Ad hoc | Needed for hysteresis or test stability. No warrant |

We have not learned these parameters. We have not run a calibration against human ratings or against MemGPT/RAG at scale. Until one of those happens, treat every digit as provisional.

---

## Encoding gate

`τ_tender = 0.28` · `τ_austere = 0.40` · default `0.32`

The gate is lower than v0.5 so more hours enter. Compensation is not a second human vote: access decay and Ebbinghaus stability scale with `salience_at_encode`. A weak keep (`S` just above τ) cools and weathers faster than a charged one.

**Why a gate at all.** Encoding is selective. Arousal, self-relevance and novelty raise the chance that an episode is stored (Cahill & McGaugh; flashbulb work is contested in detail, not in the coarse claim). A linear score `S` with a cut is a cartoon of that gate.

**Why 0.28.** No paper gives a threshold on *our* `S`. `S` is an invented 0–1 mix. `0.28` vs `0.40` is a **contrast pair**: one profile keeps more of the dull-but-warm, the other drops it. The pair used to be 0.40 / 0.55 when the only lever was the cut. Swap the two numbers and the clone test still works if the gap remains.

**What would justify a number.** Fit `τ` so that a labelled corpus (“keep / drop”) matches human or experimenter tags. Or let `τ` be a percentile of recent `S`, not a constant.

---

## Embellish `0.18` / disgust gain `0.05` vs `0.16`

**Why drift at all.** Recall is reconstructive (Bartlett, 1932). Autobiographical memory shows a positivity bias on many pleasant events and rumination on aversive ones (Walker & Skowronski; clinical literature on reconsolidation). Direction is warranted. Magnitude is not.

**Why 0.18.** Step size so that four nights move gist without erasing the core. **Contrast pair** with austere `0.05`. Not a measured gilding rate.

---

## Ebbinghaus form

```
R(t) = exp(−t / S)
S = S₀ · (1+α·arousal) · (1+β·permanence) · (1+γ·anchor) · (1+δ·ln(1+rehearsals))
```

**What Ebbinghaus actually did.** 1885, himself as subject, nonsense syllables, savings method. The curve falls fast then slower. Later fits are often a **power law** `R = (1+t)^(−β)` (Wixted), sometimes a sum of exponentials. We picked a single exponential because it is one line of code and has a readable time constant `S`.

**What is warranted.**
- Detail fades faster than a stable gist — fuzzy-trace theory (Brainerd & Reyna): verbatim vs gist.
- Rehearsal slows forgetting — spacing / Jost.
- Emotional intensity and self-relevance slow forgetting of the *charge*, not necessarily of the pixels.

**What is not warranted.** `exp`, the particular `α β γ δ`, freezing `core` exactly at encode. Those are **exploratory**. A power-law variant belongs in a bake-off, not in a claim.

---

## Ladder: 2 traces → motif, 3+ → belief, 2 beliefs → trait

**Why a ladder.** Repeated episodes under one schema become a gist, then a stance. That much is old: Bartlett’s schemas, Piaget, modern event-schema work. Personality as a pattern across situations is also old.

**Why 2 and 3.** **Discrete convenience.** Nothing in the literature says the third episode is the one that mints a belief, or that two aligned beliefs are a trait. A continuum (strength accumulating with support) would be closer to the data. We used integers so `who_am_i` is readable in tests.

**Weak-belief replace at strength < 0.36.** **Ad hoc** hysteresis, so a strong belief is not overwritten by a new pair of anecdotes. No citation.

---

## Weights on `S`

`w_arousal 0.25`, `w_self 0.25`, `w_novelty 0.15`, `w_utility 0.15`, `w_goal 0.10`, `w_redundancy 0.20`

Qualitative order is defensible (self and arousal first). The six numbers are a partition of 1.00 that we have not fitted. **Exploratory.**

---

## Grounding: `ground_min_overlap` / `ground_strikes` / `narrator_firmness`

`ground_min_overlap = 0.18` · `ground_strikes = 3` · firmness tender `0.42` / austere `0.72`

A generated sentence is a **miss** when `judge_against_core` returns a kind the core does not authorize:

| kind | meaning | miss? |
|---|---|---|
| `Hold` | same claim | only if Jaccard `< ground_min_overlap` |
| `Compress` | detail fell off, core still entails the sentence | no |
| `Elaborate` | cause / stake the core never licensed | yes |
| `Reframe` | same event, other speech act (irony, punchline) | no — mouth only, `DriftKind::Color` |
| `Contradict` | spoken sentence denies the core | yes |
| `Depart` | not the same event | yes |

`ground_min_overlap` is the identity gate for `Hold`, not the whole test. Misses increment `detach_strikes`. How soon a rewrite fires, and how hard, is `narrator_firmness` × importance (permanence, anchor, access, fidelity). Cold/myth and slipping traces have hold = 0 and may warp without a ceiling. The rewrite is a blend, never the archive.

Embeddings still rank recall. They do not judge grounding.

The check lives in `recall/judge.rs` (pure). Grip, strikes and the blend live in `recall/pull.rs`. Night uses the same miss test on a rewrite (`dream/rewrite.rs`). Pass order is `weather → ladder → rewrite → merge → release` (`dream/night.rs`, pinned by `tests/dream_order.rs`). A shallow night is `weather → release` only.

---

## Night budget: `deep_min_hours` / `deep_min_charge`

`deep_min_hours = 1` · `deep_min_charge = 9.0`

A deep night runs merge and ladder. A shallow night only weathers and may release spent latent hours. Deep if new Selfhood hours since `last_deep_at` reach `deep_min_hours`, **or** those hours' `arousal + disgust` reach `deep_min_charge`.

**Why 1 and 9.0.** Contrast-safe defaults: one new hour is still a deep night, so persist / v0.1 do not move. `9.0` is out of reach of a single hour (max charge 2.0). Live setting under test: `3` / `1.2` — three dull hours, or one charged hour. **Ad hoc.** `sleep_deep()` ignores the cut.

Spoken `rehearsals` increment only when live `speak` selects the hour that enters the mouth. `remember` and isolated probes do not. Latent encode still bumps rehearsal when a new hour reactivates a latent trace (`encode/identity.rs`); that path is not spoken utility.

**Why 0.18 and 3.** Same status as `τ`. **Exploratory.** The kinds are the mechanism; the cut is not.

`narrator_firmness` default `0.55` (tender `0.42`, austere `0.72`). CLI `--narrator-firmness`. `GET|POST /profile`.

### P2 / P3 cuts (default off)

`util_to_strength` — live `speak` / lab `note_spoken` adds `+0.08` on living axioms that list the hour, clipped to the **layer cap** (motif 0.42, belief 0.70, trait 1.0). Isolated probes do not. Persist `--p2` stamps marked hours once after the first T0 night.

`merge_support_veto` — merge skips a pair if either hour is pinned (`anchor ≥ 0.85`) or the two hours have different axiom-support sets (including empty vs nonempty). Count: `store.merges_refused`.

Mint starts low (motif ≈ 0.28). A second night on the same schema **keeps** the living axiom and unions support; it does not remint a new sentence. Unused prior axioms rust `−0.05` per deep night toward the layer floor.

`pending_night` forces a deep night when new Selfhood hours arrived in the same wall-clock second as `last_deep_at` (lab persist). Published P0 / P1 dumps were taken **before** that clock fix. Replaying those commands now is a different organ. Flags `util` / `veto` / `--hearth` / `--axioms-only` stay off on the published P0 / P1 command lines.

`--axioms-only` — isolated probes still retrieve (ranks / dump), but the narrator sees `who_am_i()` only. Default off.

---

## Working talk: 10 min gap · 2 h cap

The live thread is not a turn window. It lasts as long as the conversation is active, then dies.

- **Active** = last reply (or opening hear) within 10 minutes (`ACTIVE_GAP_SECS = 600`).
- **Hard cap** = 2 hours from the first pulse (`MAX_SESSION_SECS = 7200`), even if still talking.
- Persist does not write it. Sleep right after a chat still writes the sitting: each recorded turn is one hour through the gate, then the frame is dropped. Continuity after night is recall.
- A gap or the cap starts a new frame.

**Why these numbers.** Discrete convenience for a session, not a model of human working memory. 10 min is “still in the room.” 2 h is “this sitting is over.” Neither is fitted.

A flood cap of 80 turns sits inside one session so a tight loop cannot grow without bound. That cap is not the lifetime.

---

## Release of a spent hour

The book is not a cap. An hour may *leave* when it already does no work:

- status `Latent` (scene gone)
- `access < 0.10` and `anchor < 0.50` and `permanence < 0.80`
- no living axiom lists it as support
- not `Channel::World`

Then the trace is removed and its archive if orphaned. Not the night it first becomes latent — the charge gets one more sitting. `extinguish` still only decays disgust. After axiom mint, so a new belief protects its evidence for that night.

**Why these cuts.** Discrete convenience so v0.1 nights do not delete T₀. Not a model of human forgetting rates.

---

## What would stop this being a cartoon

1. **Calibrate** `τ`, `λ`, embellish against a labelled keep/drop/distort set.
2. **Compare forms** — exponential vs power-law vs two-store — on the same traces.
3. **Learn the ladder** — cluster traces, let motif/belief/trait thresholds come from the data or from an LLM judge, not from `n == 3`.
4. Until then: publish the knobs as knobs. Do not write “0.40 because Ebbinghaus.”

The tests prove that *with these knobs* clones diverge and a world fact survives. They do not prove that 0.40 is the human gate.
