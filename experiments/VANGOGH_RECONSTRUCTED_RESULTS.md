# Van Gogh reconstructed memory and attribution

Analysis of the artifacts supplied on November 17–18, 2026. The memory archive
is named `vangogh-reconstructed-8000-001`. The two generation responses identify
their model as `grok-4.3`. Source revision and generation dates are unavailable
in the memory manifest.

## Corpus and memory artifacts

The Van Gogh preparation pipeline combines chronological letters with realistic
LoCoMo conversation sessions. Its full-corpus configuration uses 903 letters
and requests 10% noise in the final token mixture. That configuration alone does
not establish the noise fraction in the reconstructed-memory archive.

The archive contains two histories, eight chapter files per condition and
history, six final memory strings, a manifest and a lock file. Each history
contains 768 source passages. History 0 covers December 6, 1883 through
October 28, 1885. History 1 covers February 21, 1888 through May 3, 1889.

| Condition | History 0 words | History 1 words |
| --- | ---: | ---: |
| `reconstructed` | 12,774 | 9,120 |
| `summary` | 15,691 | 15,687 |
| `survivor_summary` | 12,432 | 14,925 |

Words are whitespace-delimited. After whitespace normalization, every final
memory equals the concatenation of all its chapter text blocks in order.
This verifies completeness at final assembly. Chapter construction and the
subsequent actor prompt require separate checks.

The manifest records `budget: 8000` and `model: grok-4.3`. It provides no token
usage, tokenizer identity or stop reason. The word counts above must remain
distinct from a verified 8,000-token actor context.

The reconstructed text retains concrete material, including the lemons against
a yellow background (`let612-0015`) and the suggestion of hospital studios for
about thirty painters (`let768-0033`). The latter also survives in
`survivor_summary`. Its presence therefore does not isolate a reconstruction
effect. Reconstruction changes the selection and phrasing of remembered material.

## Delacroix attribution failure

Source passage `let526-0005` explicitly introduces Delacroix writing about his
Pietà. Delacroix describes using Prussian blue for the dead body's shadows and
pure chrome yellow for its lights in a dark chapel.

Trace `tr_fd1e_00000541` retains the first-person color statement while dropping
the introduction identifying Delacroix. Its recorded revision list contains an
`encode` operation. The reconstructed chapter incorporates the statement into
a first-person account of painting peasants. Both summary variants preserve
Delacroix's attribution in the inspected passage.

The attribution loss is already present in the saved trace. This locates an
upstream failure that the final reconstruction carries forward. It does not
identify which encoding substage first removed the speaker boundary.

## Two-response transfer pilot

The pilot selects the chapter block containing Prussian blue from history 0,
takes its first 240 whitespace-delimited words, and gives each condition the
same brief. The brief asks for an injured field worker carried into a dim barn,
with composition, shadow and light colors, paint handling, and up to two
remembered influences identifying who did what. The system prompt identifies
the actor as Vincent and describes memory as fallible. The user prompt asks
the actor to avoid inventing sources.

Both requests set temperature to zero, request 180–230 words, and set
`max_tokens` to 700. Each condition has one response. Both responses report
`finish_reason: stop`.

| Observation | A, `survivor_summary` | B, `reconstructed` |
| --- | --- | --- |
| Prompt tokens reported by xAI | 566 | 558 |
| Main palette | Prussian blue shadows and chrome yellow lights | Same palette |
| Attribution of the remembered color technique | Delacroix's Pietà | A personal recollection of painting a dead figure |
| Composition | Diagonal group carrying the worker into the barn | Diagonal group, five peasants and a lantern |
| Additional influence | Attributes a description of peasant-group brushwork to Millet, beyond the supplied excerpt | Uses the supplied recollection of drawing diggers and reapers as weighted masses |

The reconstructed condition says, “I recall once being made to model a dead
figure’s shadows in Prussian blue”. This carries the misattribution into the
actor's stated reason for a new artistic choice. The control correctly names
Delacroix, while adding a Millet claim that needs independent support.

## Interpretation and limits

This is an observed attribution-transfer failure in one selected case. The two
outputs use the same palette and broadly similar compositions. The responses
provide no measured creativity advantage. The memory excerpts also differ in
their surrounding content, including peasant studies and figure modeling.
An attribution-only intervention would need to hold that content constant.

The case and the dim-barn brief were chosen after the memory error was found.
The brief is close to the dark-chapel example. The pilot therefore estimates
neither spontaneous transfer frequency nor an error rate across histories.
Temperature zero and one response per condition provide no repeatability
estimate. Evaluation was a qualitative assistant review. Although responses
were read before the supplied condition mapping, their content and the prior
case analysis made condition inference possible.

Both 240-word excerpts end mid-sentence because the pilot used a word prefix.
The relevant color statement remains complete in each. This pilot prefix is a
separate operation from assembly of the full memory strings.

The case motivates grounding checks that preserve the speaker and the status
of a statement as a quotation, observation or personal action. The existing
grounding behavior allowed this instance to reach the output. A larger assay
should freeze independent briefs, preserve complete sentences, compare the
same content with attribution corrected, and rate source fidelity alongside
artistic quality.

## Artifact identity

The supplied requests, responses and condition mapping have these SHA-256
digests. They identify the analyzed files and support comparison with the
original run artifacts.

| File | SHA-256 |
| --- | --- |
| `request-A.json` | `a9b79820463398db7f46e82a14c1912adf5fbca900ac9952ea255adf9f63a8c3` |
| `request-B.json` | `ef1b9c88d14de563cd5f737cc2aa353d2f91f6aa4185d961a07a986450f55481` |
| `response-A.json` | `badc8ed2f1a1b7679d31b262cba950fea51998d610eb7bb596f204dd4bb6b783` |
| `response-B.json` | `b6143f808bc1d156572114fb7c490c11b7c1a5facd1175513e9cb1463fdf934d` |
| `conditions.json` | `19d9ebb4a5cce4aafbd80efc59bd62605bd40c478106b4a936415caab3f0a614` |
