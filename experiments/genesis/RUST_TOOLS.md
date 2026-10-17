# Offline Rust experiment tools

All GENESIS data preparation, blind rating, availability summaries, and mechanism coding tools now run in the existing Rust binary. Bash only launches Cargo. The former JavaScript files and their Node test files are removed. No new Cargo dependency, JavaScript runtime, or Python runtime is required.

Install this update only after an active collection has completed. Source and binary changes alter collection fingerprints and prevent resuming a run created with the earlier executable. Keep that executable and source environment until the run is complete. Finished 007 preview outputs remain readable by the new offline rating tools; they do not need to be collected again. No existing result folder is rewritten by installation or by generating rating templates.

## Preparation

Run from the repository root:

```bash
bash experiments/genesis/tools.sh prepare-preview /path/to/locomo10.json preview.json
bash experiments/genesis/tools.sh prepare-life /path/to/locomo10.json economy.json --profile economy
bash experiments/genesis/tools.sh prepare-life /path/to/locomo10.json full.json --profile full
bash experiments/genesis/tools.sh prepare-creative /path/to/locomo10.json pilot.json
bash experiments/genesis/tools.sh prepare-creative /path/to/locomo10.json all-histories.json --all
bash experiments/genesis/tools.sh prepare-sleep /path/to/locomo10.json sleep.json
```

Output paths must be fresh. The same official source fingerprints are enforced. The sleep preparer retains its historical byte-exact source requirement; creative preparers retain the pinned normalized-content requirement. Unicode escape and whitespace normalization preserve source key order for that fingerprint. The byte-pinned preview fixture is exported exactly for the accepted frozen plan. Longitudinal fixture JSON object keys are serialized in deterministic Rust order; parsed content and canonical protocol hashes match the former files. The bundled original fixtures are unchanged.

## Blind ratings

Only send `blind-proposals.json` to the raters. Do not share the private key, condition names, contexts, or raw responses. For the preview:

```bash
bash experiments/genesis/tools.sh rate-preview RUN template judge-a.json
bash experiments/genesis/tools.sh rate-preview RUN template judge-b.json
# Fill the files independently, including distinct rater identifiers.
bash experiments/genesis/tools.sh combine-ratings ratings.json judge-a.json judge-b.json
bash experiments/genesis/tools.sh rate-preview RUN analyze ratings.json
```

This writes `RUN/preview-ratings.json`. The same operations work with `rate-life` or `rate-creative`, writing `life-ratings.json` or `creative-ratings.json`. Analysis requires complete collections, valid blind mappings, full rating coverage, at least two distinct raters per proposal, ordinal scores, constraint compliance, and rationales. Invalid generation and constraint failures receive zero in the existing exploratory score. Templates, combined files, and analysis outputs are never silently overwritten.

The schemas, metric definitions, and per-history/per-pair contrasts are preserved. Aggregated cells and contrasts may appear in deterministic sorted order instead of their former insertion order; this does not change their values. These measures are exploratory and do not automatically establish creativity, source causality, or statistical significance.

## Longitudinal diagnostics and mechanism coding

```bash
bash experiments/genesis/tools.sh rate-life RUN summarize
bash experiments/genesis/tools.sh code-mechanisms RUN template coder-a.json
bash experiments/genesis/tools.sh code-mechanisms RUN template coder-b.json
# Fill independently with the fixed vocabulary documented in CREATIVE_LIFE.md.
bash experiments/genesis/tools.sh combine-ratings coding.json coder-a.json coder-b.json
bash experiments/genesis/tools.sh code-mechanisms RUN analyze coding.json
```

These write `life-availability.json` and `life-mechanisms.json`. The availability summary can be regenerated; rating and mechanism outputs require fresh paths. Functional mechanism-tag distance is not a creativity score.

The offline tools never invoke HTTP or a model. Their implementation contains no collection mutation calls. They can analyze finished results created by the earlier JavaScript-based tooling. Data preparation and rating do not consume the API budget. Launchers for sleep, historical creativity, and both long-life profiles use these Rust commands instead of Node.

## Tests

```bash
bash run.sh test --bin selmem-genesis -- --test-threads=1
GENESIS_TOOL_TEST_DATASET=/path/to/locomo10.json \
  bash run.sh test --bin selmem-genesis \
  tooling::prepare::tests::complete_official_plans_match_frozen_protocols \
  -- --ignored --test-threads=1
```

The optional dataset test checks both frozen longitudinal protocol fingerprints, preview content, the historical pilot size, and sleep panel construction. The sleep constructor test uses the historical source-hash label with normalized source content; the production CLI continues to reject a different raw source hash. This test does not weaken that production check.
