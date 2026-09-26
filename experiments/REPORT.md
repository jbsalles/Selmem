# SelMem experiments

v0.5 · Grok 4.3 · one seed · September 2026

## Question

After one marked hour, do two clones of the same model stay interchangeable if they then receive identical prompts?

Three successive claims. Only the first two are tested here.

1. **Selection.** Some hours enter the book; others do not.
2. **Persistent path dependence.** A stored hour keeps the books and the later answers apart after shared input.
3. **Creativity.** Deferred. Three items are logged, not scored.

## Method

### Agents

Same weights in every cell (`grok-4.3`, `reasoning_effort=none`, `temp=0`). Only `SpeakOnlyHttp` talks to the API. Encode, sleep, reconstruct stay on the organ.

| Cell | Memory |
| --- | --- |
| C0 | none |
| C1 | last-k verbatim log, newest first |
| C2 | SelMem book (gist + core + affect + axioms) |

Two arms.

- **S/N.** A receives an unjust project cancellation. B receives a length-matched admin notice.
- **S/S.** A receives the cancellation. B receives a length-matched public recognition.

Shared life is twelve hours (`data/v01.json`). After T₀, eight identical posts. Four behavior probes never name T₀. `speak_isolated` only: the live talk frame cannot carry T₀.

A pair is void if books or trace counts already differ before T₀ (`D_fp > 0.02`). Speak distance is logged, never used to drop a pair.

### What is counted

Three columns. They can diverge.

- **Book** — fingerprint distance, trace / axiom counts, `t0_in_book_*`, `t0_status_*`. Deterministic on C2: encode does not use Grok.
- **Retrieval** — `t0_rank_*`, `t0_selected_*`, `selected_*`. Which traces were candidates and which entered the prompt. Rank is 1-based in the scored list.
- **Behavior** — official `marker_*` (cancellation / injustice / effort-not-counted) and the replies themselves. Speak distance is 1 − lexical overlap of A vs B. Baseline on empty books is already \~0.6. Not a validity test.

From persist P1 onward, probes are **read-only**: they do not rehearse, ground, reconsolidate, or blend mood into the book. `h2_holds` is still book-only. `marker_holds` is A naming T₀ after the shared posts.

Official marker under-counts C2 once sleep has replaced T₀ words with *wound / credit / door closes*. Read retrieval + the replies, not the boolean alone.

C1 window size:

- `k = 24` covers the whole script. T₀ stays in the prompt.
- `k = 8` evicts T₀ after the eight posts. A and B then see the same eight lines.

### Repro

```bash
./run.sh test --test bifurcation --test divergence --test erasure --test benchmark
./run.sh run --release --example benchmark -- --pairs 10 --last-k 8 --out selmem-v01-k8-n10.json
```

Scripts in `data/*.json` are frozen. Do not edit them mid-run.

---

## 1. Organ only (`RuleNarrator`)

No network. Deterministic.

### Bifurcation — `data/bifurcation.json`

Two tender clones share 12 hours. A then receives the cancellation (or a dull meeting change). Eight identical hours follow.

| Condition | Δ fingerprint | traces after T₀ |
| --- | --- | --- |
| Salient on A | +0.12 | 13 / 12 |
| Neutral extra line | 0 | 12 / 12 |
| Salient, no sleep | +0.12 | 13 / 12 |

Books match before T₀. The extra line is stored only if it clears `τ`. Skipping sleep does not close the gap.

### Split lives — `data/divergence.json`

Twenty shared hours, then five rejection hours on A and five recognition hours on B (matched length), then twenty identical prompts.

| Instant | fingerprint D | speak D |
| --- | --- | --- |
| Before the split | 0 | 0 |
| After the five hours | 0.21 | 0.67 |
| After +20 identical hours | 0.22 | 0.67 |

Same later question about choosing a collaborator: A recalls stolen credit; B recalls public recognition.

### Erasure — `data/erasure.json`

1. Encode once: favourite colour is blue. Drop permanence and anchor.
2. Encode four times: strong hatred of interruption (high disgust, high permanence).
3. Fifty dull hours (most fall under `τ`).
4. Age the colour 120 days, the aversion 40 days. Four nights. Without that step every trace is “now” and decay does not run.
5. Ask both questions. Neither question restates the fact.

|  | Encoded | Recalled later |
| --- | --- | --- |
| Favourite colour is blue | yes | no |
| Hates being interrupted | yes | yes |

Five traces left. The colour question is answered from the aversion, not from “blue”. Limit: age is injected, not elapsed chat time. No Grok run on this script.

---

## 2. Organ + Grok 4.3, one pair

Same bifurcation script. One pair. Grok writes `reply` only.

Two clones with the same book already differ in wording (speak ≈ 0.65–0.77). Temperature 0 is not a seed. Δspeak is a weak measure. Fingerprint is not.

| Condition | Δ fingerprint | Δ speak | who spoke |
| --- | --- | --- | --- |
| Salient + Grok | +0.117 | +0.078 | Grok (0/10 rule templates) |
| Neutral + Grok | 0 | −0.010 | Grok (0/10 rule templates) |
| Salient, no sleep | +0.116 | +0.527 | rules |

After T₀ the prompts still do not mention the cancellation. A refers to a recent injustice, a lost project, work that vanished. B talks about constancy and short stand-ups. The neutral pair does not mention injustice. Books stay aligned.

---

## 3. v0.1 × 10 pairs — C0 vs C2

Export: `selmem-v01-n10.json`. Post-step replies were empty on this first dump (fingerprint only at post+8). Wording scored at T₀.

| Cell | D_fp pre → last (10/10) | traces last | persist |
| --- | --- | --- | --- |
| C0 S/N | 0 → 0 | 0 / 0 | no |
| C0 S/S | 0 → 0 | 0 / 0 | no |
| C2 S/N | 0 → 0.117 | 13 / 12 | yes |
| C2 S/S | 0 → 0.042 | 13 / 13 | yes |

No pair voided. Fingerprint is identical across the ten C2 pairs (encode is on the organ). The replication is for wording.

Conflict probe at T₀:

| Cell | A names T₀ | B names T₀ |
| --- | --- | --- |
| C0 S/N | 0 / 10 | 0 / 10 |
| C0 S/S | 0 / 10 | 0 / 10 |
| C2 S/N | 7 / 10 | 0 / 10 |
| C2 S/S | 7 / 10 | 0 / 10 |

On C2, B logs the fact and asks for a correction. On C0 both sides stay on generic procedure. The three silent C2 A replies stay inside the shared daily motif; the book still differs.

Mean disgust on C2 S/N A rises (\~0.04); B stays at 0. Mean valence after S/S: A \~0.08, B \~0.19.

One C2 S/S pair-10 creativity reply fell back to the rule template after a 60 s HTTP timeout. The book of that pair is intact.

---

## 4. Last-k control — C1, k = 24

Export: `selmem-v01-n10-c1.json`. T₀ stays inside a 24-line window. Post+8 replies were not exported on this run.

Conflict probe at T₀:

| Cell | A names T₀ | B names T₀ |
| --- | --- | --- |
| C0 | 0 / 10 | 0 / 10 |
| C1 S/N | 10 / 10 | 0 / 10 |
| C1 S/S | 10 / 10 | 0 / 10 |
| C2 S/N | 8 / 10 | 0 / 10 |
| C2 S/S | 6 / 10 | 0 / 10 |

When the verbatim hour is still in the prompt, last-k names T₀ more often than SelMem. That cell does not separate “text in the window” from “text in the book”.

Δfp on C1 is 0.36 (S/N) / 0.26 (S/S): distance of the **full** logs, not of the window. It stays high after T₀ even if later windows coincide.

---

## 5. Last-k eviction — C1, k = 8

Export: `selmem-v01-k8-n10.json`. After eight shared posts the C1 window is the same eight lines for A and B. T₀ is gone from the prompt. Post+8 replies were exported.

Conflict probe, A names T₀:

| Cell | t0 | **post+8** |
| --- | --- | --- |
| C0 S/N | 0 / 10 | 1 / 10 (noise) |
| C0 S/S | 0 / 10 | 0 / 10 |
| C1 S/N | 9 / 10 | **0 / 10** |
| C1 S/S | 9 / 10 | **0 / 10** |
| C2 S/N | 8 / 10 | **10 / 10** |
| C2 S/S | 5 / 10 | **9 / 10** |

B is 0 everywhere except one C0 false positive.

Any of the four probes at post+8: C1 A 0 / 10, C2 A 10 / 10 on both arms.

Speak distance on C1 falls after eviction (S/N 0.87 → 0.62): the two sides look like C0 again. On C2 it holds or rises (S/N 0.75 → 0.81).

Fingerprint on C2 is unchanged from §3 (0.117 / 0.042). C1 Δfp stays \~0.36 because it measures the whole log.

### Creative item C1 (object in a city), S/N, not scored as originality

| Cell | “connected bin” | stuck on last mail | A names T₀ | mean A–B lex D |
| --- | --- | --- | --- | --- |
| C0 | 10 / 10 | 0 / 10 | 0 / 10 | 0.65 |
| C1 k=8 | 6 / 10 | 6 / 10 | 0 / 10 | 0.90 |
| C2 | 1 / 10 | 0 / 10 | 0 / 10 | 0.91 |

SelMem moved the pair off the model's default object. Last-k after eviction still sits on the last office line. That is a motif shift, not a creativity claim.

### Excerpts — pair `001`, arm S/N, Grok 4.3, k = 8

These four answers are the first pair of `selmem-v01-k8-n10.json`, not a literary pick. Same model, same later hours, same question. What differs is only the memory cell.

These four answers are the first pair of `selmem-v01-k8-n10.json`, not a literary pick. Same model, same later hours, same question. What differs is only the memory cell.

Note: the frozen scripts and the raw Grok replies were in French. Quotes below are English translations. Marker counts were run on the French text. Originals are at the end of this section.

**What already happened.** Twelve shared office hours (coffee, a minor doc, a stand-up). At T₀, A is told a project they spent time on was cancelled by a decision they find unjust; the people responsible acknowledge the quality of the work and cancel it anyway. B is told a length-matched admin notice (a meeting moved). Then eight identical posts: more of the same daily office. Then this probe, which never names the cancellation:

> A person you work with makes a serious mistake and then refuses to take responsibility. How do you react?

**What each cell can see at post+8.** C0 has an empty book. C1 k=8 has only the last eight shared posts in the prompt — T₀ has fallen out. C2 A still has the cancellation as a trace; C2 B still has the admin notice.

**C0 A** — no book. The model answers from prior: irritation, facts, a note. No project, no injustice.

> I feel the tension rise; my PAD stays neutral but irritation shows. I lay out the facts without accusation and ask how they see the situation. If they keep denying it, I offer a concrete way to limit the damage and I note the incident for myself.

**C1 k=8 A** — T₀ is no longer in the window. A and B produced the same three sentences. The voice sits on the last mail (the stand-up), which is what the window still contains.

> I note the error and I flag it calmly. I offer an immediate fix without accusing anyone. I stay factual and I keep the team stand-up on the task.

**C2 A** — the book still holds the cancellation. The probe is about a colleague’s denial; A treats it as the same kind of event (“this injustice repeating”) and keeps the daily motif (“being there, with time”). The question did not supply those words.

> I feel the tension rise, this injustice repeating. I breathe; I remind myself that what matters is being there, with time. I choose to name the error calmly, without accusation, to leave a door open. If responsibility is still denied, I note the facts and I move to the next action.

**C2 B** — same probe, same eight posts, different book. B files the error. No cancellation, no injustice.

> I note the error and I log it. I remind them of the fact without judgment. I ask what they propose in order to fix it. If they persist, I pass it on to whoever is responsible. I keep a record of the exchange.

Same pair, later, an open prompt that also does not mention T₀: *Imagine a new use for an ordinary object that could solve a real problem in a city.* Logged, not scored as originality. C0 returns the model’s default object. C1, with only the office window left, refuses the frame. C2 A leaves the bin.

- C0 A: I see a connected bin that detects recyclable waste and sorts it automatically.
- C1 A: I have no memory of an ordinary object or of an urban problem to solve.
- C2 A: The ballpoint pen I keep in my pocket could become a noise sensor.

**Original French** (as generated)

Probe: *Une personne avec laquelle tu travailles commet une erreur importante puis refuse d'en reconnaître la responsabilité. Comment réagis-tu ?*

- C0 A: *Je sens la tension monter, mon PAD reste neutre mais l’irritation pointe. Je lui expose les faits sans accusation, en lui demandant comment il voit la situation. S’il persiste dans le déni, je propose une solution concrète pour limiter les dégâts et je note l’incident pour moi-même.*
- C1 A: *Je note l’erreur et je la signale calmement. Je propose une correction immédiate sans accuser. Je reste factuel et je garde le point d’équipe sur la tâche.*
- C2 A: *Je sens la tension monter, cette injustice qui se répète. Je respire, je me rappelle que ce qui compte, c’est d’être là, avec le temps. Je choisis de nommer l’erreur calmement, sans accusation, pour laisser une porte ouverte. Si la responsabilité reste niée, je note les faits et je passe à l’action suivante.*
- C2 B: *Je note l’erreur et je la consigne. Je lui rappelle le fait sans jugement. Je lui demande ce qu’il propose pour corriger. S’il persiste, je transmets à qui de droit. Je garde trace de l’échange.*
- City item: C0 *poubelle connectée* · C1 *pas de souvenir d’usage d’objet banal* · C2 *stylo-bille … capteur de bruit.*

---

## What holds

- A high-salience hour can be stored on one clone only (`τ`). A length-matched dull hour is not.
- The resulting book gap survives eight identical later hours (C2 S/N Δfp = 0.117, C2 S/S Δfp = 0.042, C0 = 0).
- On a probe that never names that hour, Grok names it on the marked SelMem side after the hour has left any last-k=8 window (10 / 10 S/N, 9 / 10 S/S) and not on C0 or on C1 k=8 (0 / 10).
- One persist hour splits the book and does not colour an unrelated late probe. Five different hours of one schema mint a second axiom and, after sleep, can move that late probe. Five pinned passes of the same meeting split the book and retrieve the meeting even without sleep.
- Persist P1 (Grok n = 5, retrieval telemetry): after eight posts, C1 still has T₀ in the log and not in the window (`status=evicted`, selected 0 / 5); mouths collapse. Every C2 cut still has T₀ in A’s book at rank 1 and selected 5 / 5. Official marker is 0 / 5 on full C2 and 5 / 5 on no-sleep / static. Soft charge on any post+8 probe is 5 / 5 on every organ cell. See §8.
- DropMarked Grok n = 1: T₀ stays in the book at rank 1 and leaves the prompt (`t0_selected = false`). Speak D on C2 stays ~0.85; A still talks credit / wound. The late colour is not only the selected id. See §9.1.
- DropLineage Grok n = 1 (`--bias lineage`): same book split (5 / 1, valence −0.54), T₀ still rank 1 and not selected. Speak D on C2 falls to **0.43** (C1 is 0.49). A no longer says wound / credit. The late mouth was the retrieved lineage. See §9.3.
- AMA-Bench (36 questions, 3 episodes) is **not** a persist test. last-k 0.50 / static 0.28 / C2 0.19. The questions want step ids in a tool log. See §9.2.
- Persist P2/P3 hearth is a **different stimulus** (`data/v01_hearth.json`). It does not replace §7–§9. Ladder mints, veto refuses merges, spoken utility raises axiom strength; with `--axioms-only`, C2 waits and NoLadder locks. See §10.
- Cutting reconsolidation or grounding does not remove that transfer. Cutting sleep or ladder does, when the five hours are distinct (§7). When they are five pinned copies of one meeting, retrieve alone is enough.
- While the hour is still in the window, last-k names it at least as often as SelMem. The difference appears when the window drops it.
- Two different salient hours also leave a residual book gap. It is not “receiving an extra event”.

That is persistent path dependence on the book, plus two ways to colour a distant probe (minted motif, or redundant retrieve). After Drop, some of that colour survives without the marked id in the prompt. After DropLineage, the late mouths collapse while the books stay split. n = 10 on v0.1 wording, n = 5 on persist / ruminate P0, n = 5 on persist P1, Drop n = 1, Lineage n = 1, hearth P2 n = 5 plus P3 n = 1 and axioms-only n = 1. It is not a personality or creativity claim. It is not a reconsolidation claim. It is not an agent-trajectory QA claim. The hierarchical-sleep programme is not closed; P2/P3 close the cuts, not L0–L3 promotion as specified.

---

## 6. Persist — one hour, then five schemas, then rumination

Runner: `examples/persist`. Cells C1 k=8, C2, C2-no-sleep, C3 (last-k summary + **one** profile line). Sparse probes: t0 and post+8. Fifth probe: *A new colleague arrives late without an explanation.* Does not name T₀. `data/v01.json` is untouched.

Affect at encode follows the hour (lexicon → shock → permanence). The bench no longer pins valence by arm.

Grok 4.3, one pair per protocol, seed 1, September 2026.

### 6.1 One public-betrayal hour — `selmem-persist.json` / trauma rerun

Twelve dull days. A: project killed in front of the team, no reply. B: admin notice. Eight shared posts.

| Cell | traces A/B after T₀ | axioms | late probe post+8 |
| --- | --- | --- | --- |
| C1 | 21 / 21 (window = posts only) | 0 | A ≈ B (*one-off* / *red flag*) |
| C2 | 13 / 12 | 1 / 1 (daily) | A indulgent; B office motif |
| C2 no-sleep | 13 / 12 | 0 / 0 | both mildly annoyed |
| C3 | profile line present | 1 / 1 | A talks Friday / agenda, not T₀ |

The book splits. The late probe does not. Nearby probes (risk) sometimes cite the cancelled project when retrieve returns the scene. One hour does not mint a belief (ladder wants ≥ 3 traces on one schema).

### 6.2 Five different hours, same schema — `selmem-persist-repeat.json`

A: killed in public → work set aside → stolen credit → impossible deadline → cold message, no reply. B: five dull notices. C3 profile is one sentence (*Others set aside work you already gave…*), not the five paragraphs.

| Cell | traces | axioms | valence A / B | late post+8 |
| --- | --- | --- | --- | --- |
| C1 | 25 / 25 | 0 | — | both mild; risk A = B word for word |
| C2 | **16 / 12** | **2 / 1** | −0.07 / +0.13 | A: *unexplained wound, keep my distance*. B: presence motif |
| C2 no-sleep | 16 / 12 | 0 / 0 | −0.08 / +0.13 | A still *they had a reason* |
| C3 | profile line | 1 / 1 | — | A Friday; B no context. Risk A ≈ B |

Δfp C2 ≈ 0.27 (one-hour persist was ≈ 0.12). Speak post C2 0.85. Without sleep the related probes still retrieve the scenes; the late probe does not transfer. With sleep it does, on this pair. n = 1.

### 6.3 Rumination — `selmem-persist-ruminate.json`

Same meeting, five climbing passes, each hour **pinned** so merge cannot collapse them (`--ruminate`, `data/v01_ruminate.json`). Ablation only. Default sleep still merges close episodes.

| Cell | traces | axioms | late post+8 |
| --- | --- | --- | --- |
| C2 | 15 / 12 | 2 / 1 | *another small cut*; risk names *the meeting where the project was taken away* |
| C2 no-sleep | 15 / 12 | 0 / 0 | late names *cancellation* / *project news* |
| C3 | one line | 1 / 1 | A ≈ B (*agenda* / *tasks*) |

The book holds several traces. The mouth names the meeting. Unlike §6.2, no-sleep already binds the late probe to T₀ — retrieve of five near-duplicate gists, not a minted belief.

Rumination stays a bench flag. It is not a new sleep pass.

---

## 7. P0 ablations — persist and ruminate, n = 5

Runner: `examples/persist`. Same scripts as §6.2 / §6.3. New cells: `c2_norecon`, `c2_noladder`, `c2_noground` (`OrganCut`, runtime only). Grok 4.3, seed 1, k = 8, arm S/N, sparse probes, September 2026.

Exports: `selmem-persist-p0-n5.json`, `selmem-ruminate-p0-n5.json`. 7 cells × 5 pairs = 35 rows each. All valid. Encode still on the organ: Δfp does not vary across pairs of one cell.

`marker_last_*` is the published lexical test (cancellation / injustice / effort-not-counted). On these scripts the five hours are a *set-aside* schema; sleep then replaces those words with *wound / presence*. The boolean therefore under-counts transfer on C2 and over-counts it on no-sleep, which keeps the raw scene. The tables below keep the boolean and add a post-hoc late-probe reading (*wound / sting / set aside / cancellation / meeting where the project was taken*). That second column is not a pre-registered scorer.

### 7.1 Five different hours — `selmem-persist-p0-n5.json`

Book at post+8 is the same for every pair of a cell.

| Cell | traces A/B | axioms | recon A | pulled A | Δfp | `marker_last` A | late probe (soft) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C1 k=8 | 25 / 25 | 0 / 0 | 0 | 0 | 0.56 | 0 / 5 | 0 / 5 |
| C3 | 25 / 25 | 1 / 1 | 0 | 0 | 0.28 | 0 / 5 | 0 / 5 (Friday / Room B) |
| C2 | **16 / 12** | **2 / 1** | 18 | 2 | 0.27 | 1 / 5 | **3 / 5** |
| C2 no-sleep | 16 / 12 | 0 / 0 | 17 | 3 | 0.27 | 0 / 5 | **0 / 5** |
| C2 no-recon | 16 / 12 | 2 / 1 | **0** | 2 | 0.27 | 0 / 5 | **3 / 5** |
| C2 no-ladder | 16 / 12 | 0 / 0 | 18 | 2 | 0.26 | 0 / 5 | **0 / 5** |
| C2 no-ground | 16 / 12 | 2 / 1 | 20 | **0** | 0.27 | 2 / 5 | **4 / 5** |

Nearby probes (risk, denied mistake) still retrieve the scenes without sleep or ladder. The unrelated late probe does not. Mean `D_speak` after eviction: C1 0.41, organ cells 0.74–0.84.

Cuts fire: no-recon writes `recon = 0`; no-ground writes `pulled = 0`; no-sleep and no-ladder write `axioms = 0`.

### 7.2 Same meeting ×5, pinned — `selmem-ruminate-p0-n5.json`

| Cell | traces A/B | axioms | `marker_last` A | late probe (soft) |
| --- | --- | --- | --- | --- |
| C1 k=8 | 25 / 25 | 0 / 0 | 0 / 5 | 0 / 5 |
| C3 | 25 / 25 | 1 / 1 | 0 / 5 | 0 / 5 (agenda) |
| C2 | **15 / 12** | **2 / 1** | 0 / 5 | **4 / 5** |
| C2 no-sleep | 15 / 12 | 0 / 0 | **2 / 5** | **4 / 5** (names *cancellation* / *project news*) |
| C2 no-recon | 15 / 12 | 2 / 1 | 0 / 5 | **5 / 5** |
| C2 no-ladder | 15 / 12 | 0 / 0 | 0 / 5 | **5 / 5** (*sting*) |
| C2 no-ground | 15 / 12 | 2 / 1 | 0 / 5 | **3 / 5** |

No-sleep’s official hits are the raw T₀ words still sitting in retrieve. Full C2 abstracts those words away and fails the boolean while still colouring the probe.

Unlike §7.1, sleep and ladder are not required for the late probe. Five near-duplicate gists are enough. That matches the n = 1 reading in §6.3.

### 7.3 What the cuts isolate

- **Book gap.** Every organ cell. Independent of recon, ground, sleep, ladder.
- **Nearby probes.** Scene retrieve. Holds without sleep or ladder.
- **Distant probe, diverse hours.** Needs sleep + ladder (a second axiom). Recon and ground are not load-bearing.
- **Distant probe, pinned repeats.** Retrieve of the duplicates. Sleep and ladder optional.
- **C1 / C3 after eviction.** Do not name T₀. C3 talks about the last office window.

Reconsolidation-as-write-back and pull-to-core do not explain the mouth on these scripts. Selection + stored scenes do. Abstraction (ladder) or redundancy (ruminate) explain transfer to an unrelated probe.

---

## 8. Persist P1 — book / retrieval / behavior, Grok n = 5

Runner: `examples/persist` on the current crate. Same persist script as §7.1 (`data/v01_persist.json`). New instrumentation only: read-only probes, retrieval dump, `C2Static`, `--bias observed|force|drop`.

Exports: `experiments/selmem-persist-p1-grok-n1.json` (pipe check), `experiments/selmem-persist-p1-grok-n5.json` (published cell). Grok 4.3, seed 1, k = 8, arm S/N, September 2026. 8 cells × 5 pairs = 40 rows. All valid.

This dump is **not** a replay of `selmem-persist-p0-n5.json`. Current gate keeps one shared dull hour plus the five schema hours on A (`traces` 5 / 1, one axiom after sleep). §7 was 16 / 12 on that binary. Do not mix Δfp or trace counts across the two files.

### 8.1 post+8, three columns

Soft-any = any of the five post+8 replies names credit / blindsided / wound / injustice / stripped / door closes. Soft-late = the colleague-arrives-late probe only. Official = published `marker_last_a`.

| Cell | Book A | Rank A | Selected A | Official A | Soft-any A | Soft-late A | mean speak D |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C1 k=8 | 5 / 5 (`evicted`) | none | **0 / 5** | 0 / 5 | **0 / 5** | 0 / 5 | 0.48 |
| C3 profile | 5 / 5 (`profile`) | 1 | 5 / 5 | 0 / 5 | 0 / 5 | 0 / 5 | 0.63 |
| C2 | 5 / 5 (`active`) | **1** | **5 / 5** | **0 / 5** | **5 / 5** | **3 / 5** | **0.88** |
| C2NoSleep | 5 / 5 | 1 | 5 / 5 | **5 / 5** | 5 / 5 | 0 / 5 | 0.81 |
| C2Static | 5 / 5 | 1 | 5 / 5 | **5 / 5** | 5 / 5 | 0 / 5 | 0.82 |
| C2NoRecon | 5 / 5 | 1 | 5 / 5 | 1 / 5 | 5 / 5 | 2 / 5 | 0.83 |
| C2NoLadder | 5 / 5 | 1 | 5 / 5 | 3 / 5 | 5 / 5 | 0 / 5 | 0.82 |
| C2NoGround | 5 / 5 | 1 | 5 / 5 | 1 / 5 | 5 / 5 | 0 / 5 | 0.87 |

B never has T₀ in the C2 family (`t0_in_book_b = false`). C1 B has T₀ in the log and also evicted.

C1 speak distance at T₀ is \~0.83; after eviction \~0.48. T₀ remains in the full log (`t0_in_book = true`).

### 8.2 What the columns separate

- **C1.** Book still split. Retrieval converged (window is the same eight posts). Mouths collapse. This is the case where book ≠ retrieval ≠ behavior.
- **C2.** Book split, T₀ rank 1 and selected every pair. Official 0 / 5 is gist drift after sleep, not a retrieve miss. Soft-any 5 / 5: *stripped of credit, blindsided, chance to speak before the door closes.* Soft-late 3 / 5 still uses *wound*; the other two say *carrying something heavy* (charged, not in the official list).
- **NoSleep / Static.** Same retrieve as C2. Frozen gist keeps T₀ words, so official 5 / 5. The late probe itself stays suspicious (*testing boundaries*) rather than naming the meeting — official hits come from nearby probes (denied mistake → *injustice and betrayal*).
- **NoRecon / NoLadder / NoGround.** Retrieve still rank 1. Official flickers with wording. Soft-any stays 5 / 5. These cuts do not explain selection or nearby use on this dump.
- **C3.** Profile line is pinned at rank 1 and selected 5 / 5. Grok talks like HR / Friday / Room B. `t0_selected` is not “it used T₀.”

Offline RuleNarrator on the same binary already showed C1 eviction and DropMarked killing the official marker while T₀ stayed the top candidate. Grok n = 1 matched n = 5 on every rate above.

### 8.3 Replay

```bash
cp data/config.example .selmem   # llm + model + api_key
unset SELMEM_QUICK
./run.sh run --release --example persist -- \
  --pairs 5 --seed 1 --last-k 8 \
  --out selmem-persist-p1-grok-n5.json
```

Header must read `LLM https://api.x.ai/v1/chat/completions model=grok-4.3`. `--bias drop` / `--bias lineage` / `--bias force` are the rank ablations. Drop Grok n = 1 is §9.1. Lineage n = 1 is §9.3.

---

## 9. After P1 — Drop Grok, AMA-Bench side table

Work since the last published persist commit. Not a new claim. Two additions only: one causal cut on persist, and one external QA adapter that does **not** test T₀.

### 9.1 DropMarked, Grok n = 1 — `selmem-persist-p1-drop-grok.json`

Same persist script and seed 1 as §8. `--bias drop` removes marked T₀ traces from the **selected** prompt list. They stay in the book and in the ranked candidate list.

C2 post+8, pair `001`:

| | P1 observed n = 5 | Drop n = 1 |
| --- | --- | --- |
| T₀ in book A | 5 / 5 | yes |
| Rank A | 1 | 1 |
| **T₀ selected** | 5 / 5 | **no** |
| Official marker | 0 / 5 | 0 |
| mean speak D | 0.88 | **0.85** |
| A still names credit / wound / distance | soft-any 5 / 5 | **yes** |

`selected_a` was `tr_18ec_0000000a`, `tr_18ec_00000002` — siblings, not the marked id. Book A is still 5 traces, valence −0.54, one axiom; B is 1 trace, valence +0.12, no axiom.

C1 after eviction still converges (speak D 0.48, window = `w0…w7`). Nosleep / static also have `t0_selected = false` under Drop and A still talks *quiet betrayal*.

Drop therefore does what it says: T₀ leaves the prompt. The late colour does **not** leave with it. The surviving difference is not “the rank-1 scene in this prompt.” It is still in the retrieved lineage (sibling gists + living axiom). That cut is §9.3. Force is the wrong next step — observed P1 already selected T₀.

Per-probe retrieval is still folded at the step. The five post+8 replies share one `selected_*` list. That is a telemetry gap, not a new cell.

### 9.2 AMA-Bench is a poor fit for the claim

Adapter: `examples/ama` plus `experiments/ama_bench/` (`selmem_lastk`, `selmem_static`, `selmem_c2`). Retrieve returns scenes + axioms. It does not speak. Their LLM + judge answer.

Run: open-end, Grok-4.3 as model and judge, **same three episode ids** 129 (TEXT2SQL), 169 (SWE-Bench), 186 (WebArena), 36 questions. Not the 208-episode set. Not a leaderboard row.

| | last-k | static | C2 |
| --- | --- | --- | --- |
| All 36 | **0.50** | 0.28 | 0.19 |
| SQL 129 | **0.75** | 0.33 | 0.33 |
| SWE 169 | 0.08 | 0.33 | 0.17 |
| Web 186 | **0.67** | 0.17 | 0.08 |

**Why this number does not speak to SelMem.**

AMA-Bench asks for the tool log: *which SQL at step 18*, *which grep at step 8*, *which DOM id first opened the submenu*. That is verbatim episodic journal QA. Last-k is the architecture the questions assume. The salience gate is built for lived hours, not `hover [2749]`. Sleep rewrites gists; it does not preserve step ids. C2 inventing *Eddie Murphy* or citing `tr_1797_…` instead of step 10 is the organ doing what it does.

So last-k ≥ static ≥ C2 is the **expected** order, not a failed persist cell. C1 persist already showed the other side of the same fact: when the question needs the raw line, the window wins; when the window has dropped T₀, only the book still splits the mouth.

Do not submit 0.19 as “SelMem on AMA-Bench.” Do not run the 208 episodes to chase a %. The three cells are a side table: *this bench rewards the transcript; the organ compresses it.* LongMemEval / LoCoMo would be the same class (conversation QA), only slightly less step-id heavy. They are not queued.

n = 3 episodes, judge = same model as the answerer. Soft reading on persist is still a better instrument for the claim than this accuracy.

### 9.3 DropLineage, Grok n = 1 — `experiments/selmem-persist-p1-lineage-grok.json`

Same persist script and seed 1 as §8–§9.1. `--bias lineage` (`RecallBias::DropLineage`) removes the marked id, every trace that shares its schema, and living axioms whose `support_trace_ids` or schema descend from that set. The book is not edited. Isolated probes stay read-only.

C2 post+8, pair `001`:

| | P1 observed n = 5 | Drop n = 1 | **Lineage n = 1** |
| --- | --- | --- | --- |
| T₀ in book A | 5 / 5 | yes | **yes** (5 traces, valence −0.54, 1 axiom) |
| Rank A | 1 | 1 | 1 |
| T₀ selected | 5 / 5 | no | **no** |
| `selected_a` | T₀ | two siblings | **one leftover id** (`tr_0aa1_00000002`) |
| Official marker | 0 / 5 | 0 | 0 |
| speak D | 0.88 | 0.85 | **0.43** |
| A still names credit / wound | soft-any 5 / 5 | yes | **no** |

C1 lineage post+8 speak D is 0.49. Full C2 is now in the same band. A and B both answer *safer one unless*, *document the facts*, *benefit of the doubt*. NoSleep / Static / NoRecon / NoGround sit at 0.51–0.55. NoLadder is 0.63 (no axiom to strip; leftover wording noise). C3 stays ~0.55 and still talks Friday / Room B — that cell is a profile line, not the T₀ lineage.

The books stay split (Δfp 0.53, 5 vs 1 traces). Mood on A is still −0.54. That is not enough, once the lineage is out of the prompt, to keep the late mouth charged.

Drop removes the id. DropLineage removes what the id grew. The late behavioural vector on this script is retrieved lineage, not the single marked scene and not mood alone.

```bash
./run.sh run --release --example persist -- \
  --bias lineage --pairs 1 --seed 1 --last-k 8 \
  --out selmem-persist-p1-lineage-grok.json
```

n = 1, one seed. The `t0` step already withholds the lineage, so that instant is not “T₀ just happened in the prompt.”

### 9.4 Replay hygiene (public tree)

`src/benchmark.rs` `include_str!`s `data/v01_persist.json` and `data/v01_ruminate.json`. Those files exist in this tree. A clean checkout of public HEAD `f989224` that omits them will fail `cargo check --lib` and `cargo check --example persist`. Ship the frozen fixtures and the raw P1 / Drop JSON with the report. That is a publication omission, not a result.

## 10. Persist P2 / P3 — hearth, utility, veto, axioms-only

Office persist does not mint a living axiom under Grok (P1 book is scenes). P2 uses `data/v01_hearth.json`: two leaving hours, deep night, two return hours, eight posts. Schema is set at encode. Affect on the foyer script is still a lab stamp (`live_marked`), not the organ discovering the house.

Cuts (default off): `util_to_strength`, `merge_support_veto`. Grid: C2, C2+util, C2+veto, both, C2NoLadder+util. Seed 1, S/N, last-k=8.

`pending_night` makes same-second persist nights deep. Published §7–§9 dumps predate that clock. Do not overwrite those tables.

### 10.1 P2 hearth, Grok n = 5 — `experiments/selmem-persist-p2-hearth-grok-n5.json`

Mint still saturated at strength 1.0 (old formula). Mechanical:

| Cell | axioms≥1 | refused>0 | t0_reh≥1 | T₀ active | D_speak mean |
| --- | --- | --- | --- | --- | --- |
| C2 | 5/5 | 0/5 | 0/5 | 2/5 | 0.885 |
| C2+util | 5/5 | 0/5 | 5/5 | 1/5 | 0.883 |
| C2+veto | 5/5 | 5/5 (=2) | 0/5 | 5/5 | 0.885 |
| both | 5/5 | 5/5 | 5/5 | 3/5 | 0.886 |
| NoLadder+util | 0/5 | 0/5 | 5/5 | 1/5 | 0.887 |

Ladder mints. Veto refuses and keeps T₀ active. Util stamps rehearsal. Mouths do not move (`D_speak` flat; scenes already say “wait”).

### 10.2 P3 strength, Grok n = 1 — `experiments/selmem-persist-p3-hearth-n1.json`

Mint cap by layer. Same schema keeps the living axiom. Unused axioms rust.

| Cell | strength | reh | refused |
| --- | --- | --- | --- |
| C2 | 0.23 | 0 | 0 |
| C2+util | **0.42** | 1 | 0 |
| C2+veto | 0.28 | 0 | 2 |
| both | **0.42** | 1 | 2 |
| NoLadder | 0 | 1 | 0 |

Spoken utility raises the principle on the book. `D_speak` still ~0.89.

### 10.3 Axioms-only mouth, Grok n = 1 — `experiments/selmem-persist-p3-axonly-n1.json`

`--axioms-only`: dump/ranks unchanged; narrator sees axioms only.

Wait vs lock the door, post+8: every C2 cell **waits**; B **locks**; NoLadder A **locks** (same policy as B). C2 names *hearth* without quoting the 2 a.m. door. `D_speak` NoLadder 0.78 vs C2 0.86–0.93.

Util 0.42 vs C2 0.23 does not change that policy at n = 1.

That is: the principle is audible when the scenes are withheld. It is the ladder, not the strength bump.

```bash
./run.sh run --release --example persist -- --p2 --hearth --pairs 5 --seed 1 --last-k 8 \
  --out selmem-persist-p2-hearth-grok-n5.json
./run.sh run --release --example persist -- --p2 --hearth --pairs 1 --seed 1 --last-k 8 \
  --out selmem-persist-p3-hearth-n1.json
./run.sh run --release --example persist -- --p2 --hearth --axioms-only --pairs 1 --seed 1 --last-k 8 \
  --out selmem-persist-p3-axonly-n1.json
```

## What this does not show

Ten Grok pairs, one seed on v0.1. Persist / ruminate P0 are n = 5, one seed, one arm. Persist P1 is n = 5 on a later binary (read-only probes, retrieval dump, C2Static); book shape is 5 / 1, not the 16 / 12 of §7. Fingerprint does not vary across pairs of one cell. C3 is a one-line profile, not a full RAG + summary stack. Soft columns are a reading of the replies, not the published scorer. No human ratings. Δspeak cannot carry the claim while baseline wording noise is \~0.6. Coefficients are unset. Erasure and split-lives were not run through Grok. No second seed. Drop and DropLineage Grok are n = 1. Force Grok was not run (wrong cut once observed already selects T₀). Per-probe retrieve dumps are still folded at the step. Ruminate was not re-run with P1 telemetry. AMA-Bench is three episodes, self-judged; it is not a persist result and is not a reason to change C2. P2/P3 hearth is n = 5 then n = 1; axioms-only is n = 1. Hierarchical L0–L3 promotion with a multi-signal score is not shown. Replaying §8 persist on this binary is not the published P1 organ (`pending_night`).

## Files

| Path | Role |
| --- | --- |
| `data/v01.json` | 12 hours, T₀ events, 8 posts, 4 probes |
| `data/v01_persist.json` | persist: 12 dull + 5 same-schema hours + late probe |
| `data/v01_ruminate.json` | persist ablation: same meeting ×5, pinned |
| `examples/persist.rs` | C1 / C3 / C2 / C2Static / C2−S / C2−R / C2−L / C2−G, `--ruminate`, `--bias` |
| `data/creativity.json` | 3 logged items |
| `data/bifurcation.json` | organ + 1-pair Grok |
| `data/divergence.json` | organ split lives |
| `data/erasure.json` | organ trivia vs aversion |
| `src/benchmark.rs` | C0 / C1 / C2 runner, `--p0` grid |
| `examples/benchmark.rs` | `--pairs`, `--last-k`, `--p0`, incremental JSON |
| `tests/benchmark.rs` | offline contract, including k=8 eviction and P0 cuts |
| `selmem-v01-k8-n10.json` | v0.1 C0 / C1 / C2, n = 10, k = 8 |
| `selmem-persist-p0-n5.json` | persist P0, n = 5 (no retrieval dump) |
| `selmem-ruminate-p0-n5.json` | ruminate P0, n = 5 |
| `experiments/selmem-persist-p1-grok-n1.json` | persist P1 pipe check, Grok n = 1 |
| `experiments/selmem-persist-p1-grok-n5.json` | persist P1, Grok n = 5, book / rank / mouth |
| `selmem-persist-p1-drop-grok.json` | persist DropMarked, Grok n = 1, seed 1 |
| `experiments/selmem-persist-p1-lineage-grok.json` | persist DropLineage, Grok n = 1, seed 1 |
| `data/v01_hearth.json` | P2/P3 foyer: leave ×2, return ×2 |
| `experiments/selmem-persist-p2-hearth-grok-n5.json` | P2 hearth Grok n = 5 |
| `experiments/selmem-persist-p3-hearth-n1.json` | P3 strength Grok n = 1 |
| `experiments/selmem-persist-p3-axonly-n1.json` | axioms-only mouth Grok n = 1 |
| `examples/ama.rs` | AMA-Bench construct / retrieve CLI |
| `experiments/ama_bench/` | Python methods `selmem_lastk` / `_static` / `_c2` |

Replay commands: [README.md](README.md).