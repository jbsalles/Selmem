# Next evaluation: keep the organ frozen

The current baseline and retained Grok/GPT runs are documented in
[../docs/LOCOMO_STRATEGY.md](../docs/LOCOMO_STRATEGY.md) and [RESULTS.md](RESULTS.md).
There is no pending organ tuning plan.

1. Rate the retained responses independently before revealing conditions.
   Separate action choice and tradeoff from wording; judge game originality,
   resource consistency, playability and unsupported attribution separately.
2. Add a matched-context ablation to the experiment adapter: selected recalled
   fragments alone, exactly those fragments plus stored interpretations, and
   no such memories. Freeze fragment selection and the question across these
   comparisons. This is proposed work, not an existing condition.
3. Predeclare the scoring rubric and a new set of situations before generating
   held-out answers. Use repeated calls and null controls; preserve provider
   failures as missing observations.
4. Compare models with the same prepared states and verify exact contexts.
   More response pairs on the same history are not additional histories.

Do not lower global thresholds, edit a probe to mention the marked event, or
inject a preferred action to improve the visible A/B contrast. Any later organ
change must be evaluated against a fresh baseline, with the old evidence retained.
