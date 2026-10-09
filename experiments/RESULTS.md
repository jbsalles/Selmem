# LoCoMo strategy observations

These are descriptive pilot observations, pending independent blind behavioral
ratings. The runs share the same organ, protocol, ten conditions and six probes.
Each condition/probe was requested three times at temperature zero with
`reasoning=none`. Three repeated calls on one history are not three independent
histories or evidence of generalization.

| Run | Model | Successful / attempted | Provider errors |
| --- | --- | --- | --- |
| [strategy-grok](strategy-grok/manifest.json) | Grok-4.3 | 179 / 180 | 1 |
| [strategy-gpt](strategy-gpt/manifest.json) | GPT-5.1 | 180 / 180 | 0 |

The missing Grok answer is `full_a`, `launch`, repetition 2. Its failure remains
in `failures.json`; it is not imputed. All successful answers ended with `stop`.
Both preflights passed. The supplied contexts match across models for every
corresponding condition/probe; the binary and protocol hashes match too.
Snapshot byte hashes differ because the runs prepared separate saved states.
No memory or interpretation item was omitted by context packing in these runs.

## Advice

On launch, Grok's full A responses and A-context donor copies propose early
arrival and a group round; full B and B-context copies propose individual
contact or a video call first. There are five available A/donor answers and
six B/donor answers. However, null controls also use the group approach and
two of three raw B answers propose a video call. This does not isolate an
organ-specific advantage.

GPT's A contexts emphasize shared control and collaborative ownership. Its B
contexts more often emphasize identity-related boundaries, safe venues and
sacrificing reach or efficiency for safety. Donor contexts tend to reproduce
that emphasis. Null responses also value safety and collaboration, and some raw
responses contain related safeguards. Differences are therefore graded, not a
clean binary effect unique to the organ.

Disagreement and retry produce largely similar calm interventions or personal
outreach across conditions. A changed supplied context does not necessarily
change the action on every question.

## Game design

All thirty GPT game responses use paper; none of the thirty Grok game responses
explicitly uses paper, a card or a sheet. GPT uses `Web of Twelve` in all 24 game
responses outside full B and B-context donor copies. Those six B-context answers
use other titles, including `Bridge of Twelve`. This is a thematic context
signal, not a creativity score.

Rules still need scrutiny. For example, a GPT full B game requires all twelve
tokens at home bases while its failure rule permanently removes a token. The
recovery does not revise that goal. Other responses leave physical moves or
resource accounting unclear. Distinct wording does not establish playability
or originality.

## Factual controls and limits

The sixty GPT factual-control answers are qualitatively consistent with the
supplied evidence or abstain when it is missing. This is not official LoCoMo
accuracy and was not scored by an independent judge. Nosleep can answer these
controls too, so they do not demonstrate a sleep benefit.

`sampling.json` reports lexical variation, not confidence intervals, statistical
significance or behavioral causality. Grok's missing call makes its global
sampling-availability flag false; valid answers should still be reviewed.
Temperature zero leaves within-context response variation.

The current raw control replaces recalled fragments with complete original
source turns and removes interpretations and disposition simultaneously. A
future matched-fragment ablation is needed to isolate stored interpretations.
The engine remains frozen while that evaluation is designed.

## Evidence and review

Open [Grok strategy.html](strategy-grok/strategy.html) or
[GPT strategy.html](strategy-gpt/strategy.html), rate every repetition, then
reveal conditions and export ratings. The observations above are already
unblinded; reviewers assessing them independently should not read this summary
before rating. The reports randomize response order and hide condition labels,
but hiding a label cannot remove recognizable content from a response.

The saved `manifest.json`, `protocol.json`, `responses*`, `sampling.json`,
`preflight.json`, snapshots and preparation audits remain byte-for-byte unchanged.
Only their containing directories were renamed. Their historical version and
revision identifiers are preserved for hash and provenance verification.
