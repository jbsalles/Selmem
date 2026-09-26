# LoCoMo adapter (P4 external)

Not the persist claim. See [../EXTERNAL.md](../EXTERNAL.md).

SelMem does not ship the LoCoMo corpus. Point `LOCOMO_JSON` at their release. Score only the subset listed in EXTERNAL.md before the run.

Suggested mapping:

- each session turn → `live` (world channel if it is narration)
- each question → isolated `speak` / last-k / static readout
- official score = exact or LoCoMo’s own F1, declared on the page

```bash
# placeholder — fill after the corpus path exists
export LOCOMO_JSON=/path/to/locomo.json
# last-k / static / C2 runners are the persist cells, not a new organ
```

Subset and scores live in [../EXTERNAL.md](../EXTERNAL.md). Do not vendor `locomo10.json`.

P4 picked LoCoMo. One bench only.
