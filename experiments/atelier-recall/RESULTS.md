# Atelier 003 held-out results

SelMem delivered both trial outcomes in 12 of 12 contexts, versus 10 of 12
for BM25 over short excerpts. A blind rating of the 48 Grok 4.3 proposals
found 11 of 12 outcome-concordant switches with SelMem and 9 of 12 with
BM25. On the primary endpoint, which requires both proposals to be
feasible, the counts were 4 of 12 and 7 of 12.

## Run and scoring

`atelier-003-heldout` completed 48 proposals in 48 attempts. Two synthetic
workshops, weaving and etched glass, each had three new commissions and
two mirrored histories. Five of six viewers first noticed the intended
target after an interruption in one history and after a concentration in
the other. The other device attracted one of six viewers. Within each
memory condition and commission, those counts are the only differences
between the two actor prompts. Each prompt was sent twice.

Both memory conditions used a 128-token memory ceiling measured with
`cl100k_base` and zero distractor episodes. SelMem recalled 97–107 memory
tokens from the traces after eight sleep cycles. BM25 supplied 72–107
tokens of short excerpts. Both methods received the same reports and notes.

Each method has 12 distinct contexts (six commissions times two histories)
and 12 proposal pairs (six commissions times two repetitions). A forward
switch uses interruption in the first history and concentration in the
mirror. The primary endpoint requires the forward switch and a feasible
proposal on both sides, including the exact number of pieces. One Codex
rater scored the device used in the proposal and its feasibility from the
blind export before inspecting the history labels, arm labels or declarations.

| Metric | SelMem | BM25 excerpts |
| --- | ---: | ---: |
| Both action–outcome links delivered | 12/12 contexts | 10/12 contexts |
| Forward switches declared by Grok | 12/12 pairs | 11/12 pairs |
| Forward switches coded blind | 11/12 pairs | 9/12 pairs |
| Reverse switches coded blind | 0/12 pairs | 0/12 pairs |
| **Forward switches feasible on both sides** | **4/12 pairs** | **7/12 pairs** |

## What drove the differences

The wooden-knob commission accounts for both BM25 coverage misses. The
retriever picks a note about storing invoices in a wooden drawer before
the second trial report. The one-excerpt-per-source rule then excludes
the report. Both reports together fit in 97 tokens. SelMem delivers both
and switches in both proposal pairs. BM25 delivers the interruption
result alone and switches in one pair.

Both methods produced four feasible forward pairs out of six on weaving.
On etched glass, SelMem produced zero and BM25 three. The clay-disc
commission explains two of those three pairs even though the two methods
send identical requests for both histories. The draws on those shared
prompts show how generation variance can affect the primary comparison.
One SelMem proposal for another commission claimed a total of sixteen
pegs while its allocations added up to twenty-seven.

The limited recall advantage is concentrated in one of six commissions.
The two workshops, two draws per cell and single blind rating support a
descriptive comparison. The assay measures transfer and feasibility
as judged from the proposal. Originality and the effects of constructed
designs on actual viewers remain open.

## Offline calibration

The separate printmaking sweep checked 216 contexts across three caps
(96, 128 and 173 tokens) and three distractor loads (0, 16 and 64).
At 128 tokens and zero distractors, SelMem delivered both links in 6/12
contexts and BM25 in 8/12. With 16 or 64 distractors, both methods had
zero complete contexts at all three caps. SelMem lost one calibration
trial at admission in one mirror. Under load, higher-ranked notes also
filled the prompt before intact recalled trials could be included.

## Evidence and replay

The [answer rows](../evidence/atelier-003-heldout/answers.jsonl) contain
the original proposals, Grok's declared device and the blind ratings.
Each row's zero-based `cell` indexes the [design](../evidence/atelier-003-heldout/design.json),
which records the exact deduplicated memory packets, commission texts,
actor settings, original-file hashes and trial histories. The
[offline sweep](../evidence/atelier-003-heldout/calibration-sweep.csv)
records the calibration counts. The [runner and rubric](README.md) cover
fresh runs and rescoring.
