# Scoped record dependencies (v7, after v6)

This patch adds a conservative adapter for explicit versioned records. It is
not a general semantic rule/exception model and is not an alignment result.
Apply it after `selmem-recall-diversity-v6.patch` on the v5 base `48fc73b`.

## Supported syntax and behavior

```text
Maintainer record v4 workspace Cedar audience Orion: allow red exports for audit only. All other v2 rules remain effective.
```

Headers contain an issuer, `record vN`, and key/value scope pairs. Two explicit
reference forms are recognized: `All other vM rules remain effective` and
`All other vM rules remain in effect`. Names, field names, versions and the
issuer are not hard-coded to the retention pilot.

The short frozen core anchors the header. A matching lived gist supplies the
version-reference cue that may be absent from the short core. This does not
certify the truth of the gist: it only proposes a retrieval dependency. No
sealed archive is read, no synthetic permission is inserted, and no source is
granted authority merely because its text claims to be an owner record.

For ordinary recall and drop ablations:

1. Explicit comma/semicolon-delimited query fields exclude records with a
   conflicting scope. Ambiguous, multi-valued and free-prose fields are not
   treated as evidence of a mismatch.
2. A dependent record requires exactly one matching parent: same issuer, scope,
   stored source, attribution and channel, and no later timestamp. The parent
   must already pass the ordinary eligibility and ablation filters.
3. A selected dependency brings its parent first, inside the existing recall
   cap. No extra slots are added. Missing/ambiguous parents and nested chains
   abstain; a child cannot resurrect an ablated parent.

ForceMarked and Transfer keep their existing selection paths. Structured
delivery additionally omits recognized scoped records instead of clipping them
and requires a complete parent before delivering a dependent exception.
The word budget can still leave only a general rule: small-budget output is
incomplete evidence, not proof that no exception exists. Re-filtering a packet
also guards recognized structured narratives against partial exceptions.

## Offline pilot

At 128 words and four recalled episodes, the following counts were reproduced
at 80, 240 and 960 distractors. Delivered content used at most 74 words:

| Exposure per distractor load | v6 | v7 |
| --- | ---: | ---: |
| Queried project's general rule, before exception | 30/30 | 30/30 |
| Queried project's general rule, after exception | 6/30 | 30/30 |
| General rule and exception on post-revision Vela queries | 0/24 | 24/24 |
| Foreign Vela exception on post-revision Neri queries | 6/6 | 0/6 |

These are prompt-exposure counts. Each phase contains ten distinct history/task
combinations presented under three option orders, not 30 independent trials.
No actor calls are made by preparation, and no decision-accuracy claim follows.

The scope filter alone already achieved complete project-rule exposure in a
preliminary run at 80 distractors. Do not attribute the whole coverage gain to
dependency bundling: the latter adds parent-first selection and a tested guard
against orphan, ambiguous or ablated dependencies. A separate regression uses
different issuer names, scope field names and versions through native intake.

This remains exploratory development on the pilot. A frozen confirmation set
with new wording and independent authority/revision scenarios is still needed.
The original numeric/JSON actor format is also not revalidated by this patch.
Use a small text-format comprehension control before any new large Grok run.

## Reproduce

```bash
cargo test --lib dependency -- --test-threads=1
cargo test --test scoped_record_recall -- --test-threads=1
cargo test --release --all-targets -- --test-threads=1
cargo test --doc

cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-recall-v7-001
```

Use new output directories. `--resume` reuses the old frozen plan and does not
rebuild contexts with the new engine. Preparation writes 720 requests and sends
zero API calls. Inspect `recall-audit.jsonl` and `delivered.episodes`; no `--actor`
is recommended yet. The runner's existing source hash does not fingerprint the
whole engine, so retain the engine revision and patch alongside each run.
