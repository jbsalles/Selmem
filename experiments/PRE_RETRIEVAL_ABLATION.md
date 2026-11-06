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
  50/192 is well above that floor.

Removing the cues (and therefore the episodes they select) changes about a
quarter of the held-out binary choices. Selection reaches the decision, not
only the delivered packet.

## Limits

- One domain, calm/social axis, synthetic history.
- Gate was relaxed only for `context-cued-ablated` so the actor could run;
  episode absence is the measured effect. Other routes stay strict.
- No control that keeps the same episodes and only perturbs prompt wording.
  Part of the 26 % could be prompt-shape sensitivity. The exposure split
  (1–2 vs 0 episodes) is not ambiguous.

## Files

- `/tmp/selmem-cued/exposure.csv`, `/tmp/selmem-ablated/exposure.csv`
- `/tmp/selmem-cued-actor/choices.jsonl`, `/tmp/selmem-ablated-actor/choices.jsonl`
