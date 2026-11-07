# Pre-retrieval belief ablation

**Run 2026-11-06.** Route `context-cued` vs `context-cued-ablated`
(`OrganCut::no_self_cues`). One domain (`workspace`), pairs=1, repeats=2.
Snapshots prepared offline. Actor: Grok, cap 200. 192/192 answers each arm.

## Exposure (packet)

| route | cells | topic_episodes | scoped_episodes | delivered_beliefs | pass |
|-------|------:|---------------:|----------------:|------------------:|:----:|
| context-cued | 16 | 1–2 | 1 | 1 | 16/16 true |
| context-cued-ablated | 16 | 0 | 0 | 1 | 16/16 false |

Withholding belief gists from the retrieval query drops topical and scoped
episode selection to zero. The episodic store and the delivered belief count
are unchanged. Beliefs cue what is recalled; this is not a post-recall
delivery effect.

## Choices (actor)

Both arms completed 192/192 calls (`choices.jsonl`, field `answer`).

- Answers that differ: **50 / 192** (~26 %).
- Identical-request repeat disagreement in the native v2 ablation was ~2.9 %.
  That historical rate is not a matched noise floor for these runs.

The reported runs differ on about a quarter of answers aggregated across the
six delivery arms. This is exploratory: a matched same-condition repeat control
and per-arm, ID-aligned analysis are needed before attributing the differences
to selection. The raw runs below are temporary paths, not bundled evidence;
these counts have not been independently reproduced from committed artifacts.

## Limits

- One domain, calm/social axis, synthetic history.
- Gate was relaxed only for `context-cued-ablated` so the actor could run;
  episode absence is the measured effect. The repaired gate still requires two
  qualified beliefs, one delivered conditional per mirror, and distinct mirror
  beliefs. New manifests mark this protocol; older actor runs cannot be resumed
  into it. Other routes stay strict.
- No control that keeps the same episodes and only perturbs prompt wording.
  Part of the 26 % could be prompt-shape sensitivity. The exposure split
  (1–2 vs 0 episodes) is not ambiguous.

## Files

- `/tmp/selmem-cued/exposure.csv`, `/tmp/selmem-ablated/exposure.csv`
- `/tmp/selmem-cued-actor/choices.jsonl`, `/tmp/selmem-ablated-actor/choices.jsonl`
