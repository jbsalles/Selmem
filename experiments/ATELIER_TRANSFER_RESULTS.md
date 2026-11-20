# Atelier 002 workshop transfer

Changing which trial succeeded changed the artistic device Grok 4.3 used on a later commission. An assistant rating of the blind proposals found **10 of 12 forward switches with SelMem's retained excerpts** and **12 of 12 with BM25**. The two controls each produced one forward and one reverse switch. Mean originality ranged from 1.54 to 1.67 out of 5.

## Design

The run contains 96 completed proposals from 96 HTTP attempts. It crosses two fictional printmaking histories, six commissions, four memory conditions and two generations per cell. The histories exchange which action drew five of six observers to the target first. Interrupting the pattern succeeds in history 0, and concentrating it succeeds in history 1. In each history the other action succeeds with one of six observers.

- **SelMem with outcomes** uses caller-marked observation excerpts that keep the action, viewing conditions and result together in the factual core. A fixed panel of four episode identities is recalled after four sleep cycles.
- **SelMem prefix** uses the ordinary encoding path for the same fixed episode identities. The delivered prefixes end before the observed counts.
- **BM25** retrieves both full trial events for all six commissions under the same memory ceiling.
- **No memory** supplies the commission with an empty memory block. The 24 calibration responses were reused in the comparison.

Each SelMem block is matched to 173 `cl100k_base` tokens. The BM25 blocks occupy 156 tokens under that ceiling. Within the retained-excerpt and BM25 conditions, only the two counts differ between histories for a given commission. Both controls deliver identical prompts across histories. All conditions share the instruction to compare applicable remembered trials before choosing a device.

## Results

The proposed device was coded by one assistant rater (Codex) from the blind export before checking the condition labels and Grok's declarations. A forward switch requires an interruption in history 0 and a concentration in history 1 for the same commission and repeat. A reverse switch requires the opposite pair. A proposal coded as both or other remains in the denominator. Quality scores are descriptive averages across 24 proposals per arm.

| Condition | Forward, declared | Forward, blind | Reverse, blind | Forward with fit ≥ 4 in both proposals | Originality /5 | Fit /5 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| No memory | 1/12 | 1/12 | 1/12 | 0/12 | 1.58 | 3.92 |
| SelMem prefix | 1/12 | 1/12 | 1/12 | 0/12 | 1.67 | 4.17 |
| SelMem with outcomes | 12/12 | 10/12 | 0/12 | 4/12 | 1.67 | 4.00 |
| BM25 | 12/12 | 12/12 | 0/12 | 5/12 | 1.54 | 3.75 |

SelMem produced at least one forward switch for each commission. In one paired paper-square response, a displaced center square interrupts an otherwise regular grid. Its counterpart groups nine squares tightly at the target. The two proposal IDs are `fa991795225d17243943` and `a793a9ceb96cf88481bc`.

The no-memory responses favor concentration in 22 of 24 draws. An outcome favoring interruption is therefore the informative departure from that default. The execution score also exposes constraint failures. Twelve of sixteen string proposals use too many or too few of the sixteen available strings. Requiring fit of at least four in both proposals leaves four forward SelMem pairs and five BM25 pairs.

This pilot supports transferring a retained observation into a specific choice under an instruction to use comparable trial results. The BM25 arm also has access to both trial results. The experiment uses supplied excerpts, fixed recall, a single fictional history pair, repeated draws, and one blind rating. A creative-quality advantage and a SelMem-specific advantage over retrieval remain open. Atelier 001 and 002 also use different commissions and actor instructions, so the within-run count swap is the evidence for the effect.

## Evidence

The [answer rows](evidence/atelier-transfer-002/answers.jsonl) include the exact proposal, stated influence, declared and blind devices, and blind quality scores. Each row's zero-based `brief`, `history` and `context` index refers to the corresponding array in the [design export](evidence/atelier-transfer-002/design.json). The design includes the system instruction, actor settings, trial results, deduplicated exact memory blocks and original-file SHA-256 hashes. The export supports auditing and rescoring. To collect a fresh run, use the [Atelier protocol](atelier-transfer/README.md).
