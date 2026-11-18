# Van Gogh with 10% realistic conversational distractors

Current protocol: all 903 letters with 10% added noise in the final corpus.
The default is now `--mode full --noise-percent 10`. This requires approximately
one noise token per nine letter tokens; LoCoMo alone is sufficient.
The percentage is not computed relative to the letters alone.


This standalone Rust package prepares the corpus, schedules inter-letter scenes,
and provides bounded, resumable English scene generation.
See CHRONOLOGY.md for the scene generation commands.

## Preparation behavior

- Read only LoCoMo's raw `conversation.session_N[].text`, speakers and turn IDs.
  Exclude QA, answers, evidence, observations, summaries, personas, images and
  captions. Text that references an image is retained without inventing its contents.
- Each source session is used at most once. Reject duplicate turn IDs within a
  conversation; remove exact duplicate sessions. Preserve naturally repeated
  utterances inside otherwise distinct sessions and report their count.
- Shuffle only among the **next available sessions** of conversations. Numeric
  session order and turn order within each conversation are preserved. Sessions
  stay contiguous in the mixed stream. The final selected session can be a
  prefix, explicitly flagged in the audit; only its final turn may be shortened.
- Preserve speaker attribution in both sources and give every arm the same
  fields (`event_id`, ordinal `period`, `sender`, `recipient`, `text`). Do not
  promote another speaker's life events to the agent's autobiography.
- No three-way synthetic noise categories and no filtering on emotional salience.
  Salient distractor experiences are part of this test, not automatically errors
  for SelMem to suppress. Repetition, emotional importance and task relevance are
  distinct properties.
- Freeze a deterministic seed and record hashes, source inventories, actual token
  fraction, partial-session count and checkpoint offsets. Source labels are kept
  in audit sidecars, never in model input.

Tokens mean `cl100k_base` reference tokens, summed over independently tokenized
bodies of at most 256 tokens. Speaker names, JSON and prompt wrappers are excluded.
These are not Grok's billing tokens. Use the same convention for all arms.
A strict requested-percentage run allows at most 0.1 percentage-point deviation at a word boundary;
report the actual value. It never pads a shortfall by resampling.

## Reproduce

From the repository root, with Git and Rust stable:

```bash
bash experiments/vangogh-realistic/fetch-sources.sh
cargo test --locked --manifest-path experiments/vangogh-realistic/Cargo.toml

cargo run --release --locked --manifest-path experiments/vangogh-realistic/Cargo.toml -- \
  inspect experiments/vangogh-realistic/source/locomo/data/locomo10.json

cargo run --release --locked --manifest-path experiments/vangogh-realistic/Cargo.toml -- \
  experiments/vangogh-realistic/source/vangogh/tei/2025-03-12/letters \
  experiments/vangogh-realistic/source/locomo/data/locomo10.json \
  experiments/vangogh-realistic/results/full-noise10-001 --mode full --noise-percent 10 --seed 42
```

Modes must not be confused:

| Mode | Letters | Noise |
| --- | --- | --- |
| `full` (default) | All letters | Require enough unique LoCoMo material for the requested percentage; otherwise fail before writing |
| `pilot` | Largest whole-letter chronological prefix that fits | Requested percentage, without replacement |
| `available` | All letters | All available LoCoMo sessions; report the resulting smaller fraction |

Use a fresh output directory. Output is staged and renamed after preparation.
On interruption, inspect/remove only the sibling `OUT.preparing` directory before
rerunning. This tool makes **zero model calls**. It does not implement actor-run
resume, consolidation, retrieval, creative generation or judging.

The resulting `stream-clean.jsonl` and `stream-mixed.jsonl` are paired inputs.
Audit sidecars and `manifest.json` are for experiment administration only. All
four proposed arms (empty, strong raw RAG, faithful compression and full SelMem)
must receive the same selected stream at the same checkpoint. Memory construction
must remain blind to held-out tasks. See CREATIVE-COMPARE.md for the four-arm evaluation on the assembled timeline.

## Measured corpus, seed 42

Current full preparation: 1,157,464 letter tokens plus 128,607 noise tokens
(1,286,071 total): 9.999992% noise, or 10% after rounding. It uses 212
sessions without replacement, with at most one boundary prefix. See
`full-noise10-manifest.json`. All 903 letters are retained.

The table below records the previous 90%-target investigation; those historical
results and manifests are unchanged. Reproduce its first two rows with
`--mode available` and `--mode pilot --noise-percent 90`, respectively.


The pinned raw LoCoMo release contains 10 conversations, 272 unique sessions,
5,882 turns and **166,284 reference body tokens**. Ten exact duplicate turn-text
occurrences were observed; no entire session was duplicated. These natural
utterance recurrences remain, rather than artificially flattening the corpus.

| Preparation actually run | Letters | Letter tokens | Noise tokens | Noise fraction |
| --- | ---: | ---: | ---: | ---: |
| Full-life, available noise | 903 | 1,157,464 | 166,284 | 12.56161% |
| Chronological-prefix pilot | 38 | 18,123 | 163,107 | 90.00000% |
| Full-life, 90% requirement | 903 | 1,157,464 | 10,417,176 required | Rejected |

The strict full run reports a **10,250,892-token shortfall** and writes no corpus.
The pilot uses 267 sessions (including its boundary prefix). It represents only
an early prefix of the correspondence, **not Van Gogh's entire creative life**.
Do not scatter 38 selected letters over 18 years and call that equivalent.
The entire-life variant preserves the trajectory but is not a 90% experiment.
`pilot-manifest.json` and `available-manifest.json` record actual preparation
results, not creative-performance evidence.

Validation: five unit tests, one end-to-end test checking shortfall refusal,
deterministic paired inputs, hidden audit labels and exclusion of QA/editorial
content; full source preparation in both usable modes; strict full-run refusal;
Clippy with warnings denied. No creativity scores or SelMem advantage claimed.

## Interpretation and remaining controls

LoCoMo provides **generated conversations edited by humans**, not recordings of
real lives: https://snap-research.github.io/locomo/ . Its event continuity is more
realistic than a nine-template noise bank, but the background can itself contain
useful creative material. Here “noise” means externally designated background,
not known irrelevance to every held-out task.

Modern chats and nineteenth-century letters have obvious lexical, temporal and
format differences. Matching envelope fields does not eliminate those cues.
Keep an identity/source-aware raw RAG control; measure how easily source identity
alone separates the corpora. An advantage over weak lexical RAG is insufficient.
Use at least a second noise family and multiple seeds before a general claim.
Do not remove all natural repetition or emotional events to make the result easier.

Both timelines keep their internal order, but interleaving is an experimental
schedule. Ordinal periods are not real dates or years of learning. The Van Gogh
XML has known date-field inconsistencies; filename edition order is retained.
No claim is made that the author met these modern conversational participants.

The former full-life target of 90% realistic background would require another **large,
diverse corpus**, not repeated LoCoMo or paraphrases of the same ten stories.
Multi-Session Chat (MSC) is a candidate with human-human multi-session dialogues:
https://parl.ai/projects/msc/ . Its unique volume, licensing and duplicated history
prefixes must be checked before adding it; no MSC importer or validated MSC
mixture is included in this patch.

## Provenance and licensing

The fetch script pins and verifies clean source checkouts:

- LoCoMo: `snap-research/locomo`, commit
  `3eb6f2c585f5e1699204e3c3bdf7adc5c28cb376`, `data/locomo10.json`.
  Attribution: Adyasha Maharana, Dong-Ho Lee, Sergey Tulyakov, Mohit Bansal,
  Francesco Barbieri and Yuwei Fang, *Evaluating Very Long-Term Conversational
  Memory of LLM Agents*, ACL 2024. Data license: **CC BY-NC 4.0** (LICENSE.txt).
- Letters: `HuygensING/vangogh`, commit
  `ab158199731faf965b4312ff98815f687b8a6811`, `tei/2025-03-12/letters`.
  Attribution: Leo Jansen, Hans Luijten and Nienke Bakker (editors), Van Gogh
  Museum and Huygens Institute. Corpus license: **CC BY-NC-SA 4.0** (XML
  publication statements). Read English bodies, retaining editorially supplied
  transcription text but excluding annotations, figures and stand-off material.

The importer also accepts local source directories; hashes identify their actual
contents but do not certify an arbitrary directory's Git revision. Corpus licenses
are distinct from SelMem's code license. Source texts and generated corpora are
ignored by Git; manifests contain only counts, identifiers and hashes.
