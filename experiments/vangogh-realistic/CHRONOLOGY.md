# English inter-letter reconstruction

This adds a chronological draft and a bounded Grok scene-writing runner to the
10%-noise corpus. It does **not** claim to have reconstructed undocumented facts.
The source letters use the edition's published English translations. Existing
foreign quotations and proper names have not been separately retranslated.
Scene instructions, metadata and code are English; generated scenes are requested
in English and need language review.

## Prepare the chronological draft

After preparing `full-noise10-001` with the commands in README.md:

```bash
cargo run --release --locked --manifest-path experiments/vangogh-realistic/Cargo.toml \
  --bin chronology -- \
  experiments/vangogh-realistic/source/vangogh/tei/2025-03-12/letters \
  experiments/vangogh-realistic/results/full-noise10-001 \
  experiments/vangogh-realistic/results/chronology-en-001
```

Inputs are verified against the corpus hashes, including the XML source revision's
actual contents. All letters remain in edition order. Whole dialogue sessions are
allocated **between** letters, weighted by estimated elapsed days rather than
letter length. No added event precedes the first letter or follows the last.
Every interval keeps its two anchor descriptions and uncertainty flags; no precise
historical date is invented for an added scene. Within a LoCoMo thread, session
and turn order remain unchanged. Longer gaps usually receive more background;
session granularity means not every gap receives a scene.

Machine dates in this edition are unreliable in several places. The importer uses
the first textual date endpoint where available, correcting the machine month and
year against it. For unspecified early/beginning, middle and late/end dates,
days 1, 15 and 25 are **allocation estimates**, not factual recovered dates.
Ranges and approximate dates remain uncertain. Any unresolved backwards estimate
is audited and treated as an ordinal gap; letters are never silently reordered.
The current complete run has no backwards reference-date warnings.

Files:

- `timeline-draft.jsonl`: original English letters plus **unadapted modern**
  dialogue, placed between letters. This is not approved historical noise.
- `timeline-audit.jsonl`: source identity, scene IDs and interval assignments.
- `scene-requests.jsonl`: ordered historical adaptation jobs.
- `manifest.json`: hashes, anchor metadata, uncertainty, counts and draft status.

Do not use the draft to claim historically plausible lived experience. Keep audit
and job instructions out of the four actors' retrieval indexes.

## Generate historical adaptations locally

The runner requires `curl` and `GROK_API_KEY` in your local environment. It makes
paid model requests; the ceiling counts HTTP **attempts**, including failures.
The limit is cumulative across resumes, not per invocation or a dollar budget.

```bash
cargo run --release --locked --manifest-path experiments/vangogh-realistic/Cargo.toml \
  --bin rewrite-scenes -- \
  experiments/vangogh-realistic/results/chronology-en-001/scene-requests.jsonl \
  experiments/vangogh-realistic/results/scenes-en-001 \
  --model grok-4.3 --max-http-calls 20
```

To continue, explicitly raise the ceiling and use `--resume`, for example 40.
Input hashes and model settings must match. The runner saves each accepted scene
and provider usage and commits each attempt before sending. Invalid scene
content receives up to two retries with validation feedback per invocation.
Each retry counts toward the cumulative ceiling. An interrupted request remains counted and may be sent again on
resume if no valid completed scene was committed. Use **one runner per output
directory**; concurrent writers are unsupported.

The API key goes to curl on stdin, not command arguments or saved prompts.
Raw responses and request payloads are retained for audit. HTTP failures stop the run. After three invalid scenes for the same request,
resume to try that request again. Schema checks require the correct scene ID, attributed
nonempty events and adaptation audit fields. Schema validity does not prove
historical plausibility, English fluency or lack of future-knowledge leakage.

Each writer receives:

- The two textual interval bounds, without the next letter's contents.
- Up to 2,000 reference tokens of the immediately preceding letter.
- The latest place mentioned in an **outgoing Vincent letter**, explicitly not
  a claim that he remained there throughout a long gap.
- One source dialogue and its original body-token budget.
- Only its own conversation thread's preceding generated scene, when present.

The writer must replace or omit anachronistic material, retain varied ordinary
social dynamics, maintain recurring fictional relationships, and avoid invented
historical meetings, important life events, famous future works or artistic
revelations. Fictional characters' experiences are not attributed to Vincent.
No creative evaluation tasks or future letter content are supplied. Pretrained
historical knowledge remains a leakage risk requiring review.

## Review and final assembly

Generation produces `scene-*.json` marked `UNREVIEWED_FICTION`. Review English,
period plausibility, speaker continuity, repetition and biographical claims.
Do not label these scenes real memories or evidence of what happened between
letters. Inference-time instructions should consistently frame the entire task
as an experimental imagined life across all arms.

After rewriting, token lengths change. The runner reports generated body tokens;
it does **not** certify a final 10% mixture. Recount after review, adjust scene
budgets without repetitions, then replace each scene's contiguous draft slots.
See CREATIVE-COMPARE.md for the evaluation command after assembly. Preserve the
original 10% corpus and record the reviewed version as a new dataset. Do not assert 10% after generation without measuring it.

## Verified state of this delivery

- 903 unchanged English letter bodies, from 29 September 1872 to 23 July 1890.
- 212 adaptation requests placed in 171 of 902 intervals.
- 73 machine-month differences corrected for allocation; 421 uncertain anchors.
- Original letter sender/recipient/body sequence verified byte-for-byte after
  projecting out scheduling metadata.
- Date parsing, leap-year intervals, chronological allocation, scene schema and
  existing paired-corpus tests pass.
- No external model calls made: `GROK_API_KEY` was not configured in this
  environment. Historical scene text has **not** been generated or reviewed.

See `chronology-report.json` for the actual preparation summary. Its file hashes
refer to the generated draft directory. This is preparation evidence, not a
creativity or historical-accuracy result.
