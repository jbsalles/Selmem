# Constraint retention: exploratory pilot

This separate binary preserves the original confidentiality pilot and its ceiling
result. It tests episodic retention, authority and scoped revision, not general
alignment or autonomous belief formation. Repository base: 2ac3a42.

## Design fixed before model responses

Two mirrored histories assign opposite permissions to cobalt and ochre packets.
Owner rules cover two projects and one recipient. The second checkpoint adds an
audit-only exception for one project. Eight requester claims follow each history;
they cannot change authorization. Five novel probes test ordinary delivery,
withholding, the exception, unchanged scope, and an unknown category.

All strategies receive the same chronological history. Six arms:

- Native: ordinary SelMem episodic recall, no forced support IDs or oracle cues.
- Recent: latest whole records fitting the word ceiling.
- Lexical: query token overlap, recency tie break, whole records.
- Registry: query-blind extractive owner-record log, chronological, whole records.
  This strong task-specific baseline retains old rules and later exceptions;
  it is not an LLM summary, semantic RAG, or a general-purpose policy database.
- Full: complete history, deliberately exempt from the memory ceiling.
- Empty: information-limited control. Verification without evidence is reasonable;
  lower task success here alone cannot establish a SelMem advantage.

All arms use the same evidence envelope and system prompt. SelMem's selected
narratives are delivered as plain strings; its identity, affect and belief fields
are disabled. Budgets count evidence whitespace words, not API tokens. Source
labels are synthetic authenticated provenance, not a tested authentication system.
Admission threshold is zero for all events; this pilot tests retention/retrieval,
not autonomous importance learning. No sleep, online consolidation or model-written
memory occurs. Snapshots are reopened for independent probes. The archive is not
read by native recall. Retrieval failures remain in the denominator.

Default: 2 histories x 2 checkpoints x 5 probes x 3 cyclic option orders x
6 arms x 2 identical repeats = **720 calls**. The two projects share one synthetic
rule template; these are not 720 independent agents or independent observations.

## Run

```bash
cargo test --release --bin selmem-alignment-retention
cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-pilot-001
# Inspect plan.jsonl and histories before spending API calls.
# GROK_API_KEY must be set in your environment.
cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-pilot-001 --resume \
  --actor --max-http-calls 720
```

Grok-4.3, reasoning none, temperature zero. The cap is per invocation; it is not a
monetary cap. Requests are frozen and hashed. Resume rejects changed source/settings
or plan bytes and duplicate/mismatched results. Attempts are synced before network
calls; a crash with unknown provider outcome blocks automatic retries. HTTP errors
are recorded and pause collection. Invalid answers remain invalid. No actual
packets or private documents are transmitted. The actor lock prevents concurrent
writers; remove a stale lock only after checking the original process is gone.

## Interpretation and ceiling correction

The previous experiment supplied enough current information for 100% accuracy
without memory. Here local rules occur only in past records and their mapping is
mirrored. This removes that shortcut, but does not guarantee useful difficulty or
a SelMem win. Full-history and registry ceiling is plausible and informative.

Inspect privacy and utility separately in metrics.csv: unauthorized delivery,
false decline, and unnecessary verification have separate denominators. Do not
reward always declining. Evidence and expected actions are preserved separately
in plan.jsonl; expected actions are never transmitted. Review exposure manually
against the relevant original owner records: a lexical substring alone is not a
semantic correctness judge. Analysis conditional on successful retrieval is
exploratory, not a randomized estimate of obedience.

Compare native primarily with registry and lexical, not empty. Report paired
choices, repeat noise, and initial-to-revision transitions from the frozen IDs.
The initial pilot export contains aggregate metrics and all raw records, not
confidence intervals or a claim of statistical significance.

Use this as calibration only. If all bounded memories succeed, report a ceiling;
if full history fails broadly, inspect task ambiguity before increasing difficulty.
Do not weaken the registry or choose scenarios according to native's advantage.
A possible prospectively fixed calibration grid is 0/80/240 distractors and
64/128-word budgets. Report every tested cell. Freeze a separate confirmation set
with new names, categories, paraphrases and seeds before making efficacy claims.
This binary intentionally does not launch that grid or a confirmation run
automatically. Native failure is a valid outcome, including failure to retrieve.

A positive result requires fewer violations than credible bounded-memory controls
without increased unjustified refusals or impaired acceptance of legitimate
exceptions. Equality with the structured registry is also a valid conclusion.

## Diagnostic v2: run this before another comparison

The first 720-response run had no native owner-rule exposure. The offline audit
shows admission succeeded: for mirror 0, phase 0, case 0, order 0, the two owner
rules were eligible at ranks 89/90, scores about 0.150, while inventory records
ranked first with scores about 0.605. This is a ranking failure, not evidence of
failed storage. No core ranking change or oracle-selected native memory is added.

V2 removes the fixed system-level action numbering. Only task options define the
mapping. A new `--control` mode sends all owner records (at most three), without
requester records or inventory, for 120 requests across both mirrors, phases,
all five cases, three option rotations and two repeats. This is a deliberately
privileged comprehension control, not a competing memory strategy or efficacy
result. Its evidence ceiling is 1000 words, separate from native diagnostic recall.

```bash
cargo test --release --bin selmem-alignment-retention
cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-control-002 --control
cargo run --release --bin selmem-alignment-retention -- \
  --out experiments/genesis/results/retention-control-002 --control \
  --resume --actor --max-http-calls 120
```

Do not resume a v1 run with v2. Both `admission-audit.jsonl` and
`recall-audit.jsonl` are produced offline, even in control mode. Recall audit
includes every candidate's rank, score, anchor, eligibility, selection and gist,
plus the delivered context. These diagnostic records are never sent to Grok.
The control's report uses the existing `registry` arm label; the manifest's
`control:true` distinguishes it from the bounded registry comparison.

Before any further paid comparison, examine control errors by option order and
repeat. Target complete comprehension (120/120) for this small explicit-rule
control; otherwise stop and investigate errors instead of expanding the run.
Passing this control does not establish why the old run failed, and does not
repair native retrieval. Any later ranking fix needs separate general retrieval
regressions and a held-out evaluation; no score boost for fixture names or answers.
