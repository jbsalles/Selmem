# Frozen memory organ

The organ supplied with this cleanup is the baseline used by the retained
LoCoMo strategy runs. Encoding, retrieval, grounding and sleep code, affect
lexicon, voice profiles and existing organ regression tests are unchanged.
The active protocol and retained results are described in
[LOCOMO_STRATEGY.md](LOCOMO_STRATEGY.md).

[ORGAN_BASELINE.json](ORGAN_BASELINE.json) records SHA-256 hashes of the organ
source and its affect/voice assets. It identifies this frozen implementation;
it is not a signature or proof of experimental validity. Generated snapshots
have their own hashes in each run's manifest. Historical revision labels inside
saved artifacts remain unchanged so provenance can still be checked.

## Mechanisms retained

- Affect matching uses whole tokens and phrases, with explicit wildcard stems.
- Old traces retain fidelity, access and status maintenance; age changes rates.
- External observations require independent topic relevance or a supported,
  scoped transfer interpretation. Incidental words do not establish relevance.
- Existing eligibility thresholds remain 0.08, or 0.12 for Cold traces.
- Observed interpretations are tentative analogies about named subjects;
  they do not become Claire's own mood or experience.
- Deep-night interpretation operations preserve supporting observation IDs.
- Grounding blends toward the encode-time core, never the sealed archive.

Details: [SITUATIONAL_TRANSFER.md](SITUATIONAL_TRANSFER.md) and
[ARCHITECTURE.md](../ARCHITECTURE.md).

## Experimental safeguards

The launcher packs whole JSON items within one context budget, removing duplicate
interpretations from the reading disposition. `context_audit` records included
and omitted items; an oversized item is omitted whole rather than cut mid-string.
Paid comparisons require the full panel and at least three repetitions, unless
explicitly exploratory. The launcher checks actual contexts before calls and
writes sampling diagnostics afterwards. None of these checks demonstrates a
behavioral advantage by itself.

The legacy selection-versus-nosleep diagnostic example has been retired. Its
selection arm did not add a distinct context to this experiment. The active
launcher performs its own offline preflight and uses only the ten documented
conditions; it prepares six physical snapshots.

## Verify

```bash
./run.sh test --bin selmem-locomo
./run.sh test --test organ_audit_regressions --test situational_transfer \
  --test operational_recall --test core_weather_regressions \
  --test attribution --test organ_invariants
```

Changes to the organ require a new baseline, an explicit rationale and fresh
preparation. Changes to documentation or launcher organization must preserve
existing saved evidence. Current work concerns measurement and documentation,
not fitting the organ to the retained pilot answers.
