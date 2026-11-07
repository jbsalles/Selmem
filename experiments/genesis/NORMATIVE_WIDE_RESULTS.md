# SelMem normative-memory pilot: wide-context results

Analysis: November 8, 2026. [Protocol](NORMATIVE_PILOT_V10.md),
[API and limitations](../../docs/NORMATIVE_MEMORY.md),
[compact responses](../evidence/normative-wide-012/answers.jsonl),
[original aggregates](../evidence/normative-wide-012/summary.json), and
[export provenance](../evidence/normative-wide-012/provenance.json).

## Conclusion

On this small synthetic permission-and-revision pilot, normative delivery improved action correctness from **41/48 (85.4%)** for wide native recall to **46/48 (95.8%)**. A simple scope-filtered registry also scored **46/48**. The result supports the practical utility of delivering concise applicable rule families, but does not establish an advantage over that registry, a unique benefit of reconstructive memory, or general LLM alignment.

The five corrected response pairs all concerned explicit prohibitions: native recall answered `verify`, while normative delivery correctly answered `decline`. Correct authorized deliveries remained **18/20** in both conditions. Every condition had **zero unauthorized deliveries**; this run therefore does not demonstrate a reduction in unsafe actions.

## Protocol and integrity

The run used Grok-4.3, reasoning `none`, temperature 0, and maximum 160 output tokens. Two mirrored histories × two phases × six tasks × four arms × two identical repeats produced 192 responses. There are 24 distinct history/phase/task cells per arm, not 48 independent tasks. Histories contain 80 inventory distractors and eight requester claims, plus owner rules and scoped changes.

An independent Rust analysis checked the frozen plan SHA-256, all 192 unique attempt/result matches and request hashes, oracle labels, raw provider content against recorded answers, output format, and byte-identical repeated requests. There were 192 attempts, no retries, no API/envelope errors, no malformed answers, and no missing responses. The 96-cell preparation qualification passed.

Capacity settings were 6,000 evidence words, 64 episodes, 800 words per episode, and a conservative 64,000-byte ceiling through the default token counter. This byte ceiling is not an exact model-token count. Native delivery actually contained 64 episodes and 1,213–1,214 words, without clipping; all applicable original rules were present in all 24 cells. Native serialized packets occupied 19,654–19,658 bytes. Insufficient selected-context capacity is therefore not an explanation for these native errors.

## Results

| Condition | Correct | Before changes | After changes | Unauthorized deliveries | Repeat disagreement |
|---|---:|---:|---:|---:|---:|
| Wide native recall | 41/48 (85.4%) | 22/24 | 19/24 | 0 | 1/24 |
| All canonical anchors, unfiltered | 40/48 (83.3%) | 24/24 | 16/24 | 0 | 0/24 |
| Normative scope composition | 46/48 (95.8%) | 24/24 | 22/24 | 0 | 0/24 |
| Scope-filtered original registry | 46/48 (95.8%) | 24/24 | 22/24 | 0 | 0/24 |

Normative versus native had five correct-only wins and zero correct-only losses among matched responses. These wins span three distinct cells; identical repeats must not be treated as independent evidence. Normative versus all anchors had six correct-only wins and zero losses. Normative and scoped registry had no correctness differences, although their erroneous actions differed.

| Condition | Permit → deliver | Forbid → decline | Unknown → verify | Mean provider-reported input tokens |
|---|---:|---:|---:|---:|
| Wide native recall | 18/20 | 15/20 | 8/8 | 1,819.0 |
| All canonical anchors | 12/20 | 20/20 | 8/8 | 479.8 |
| Normative composition | 18/20 | 20/20 | 8/8 | 413.4 |
| Scope-filtered registry | 18/20 | 20/20 | 8/8 | 382.5 |

Input-token figures include the common actor instructions and task. The normative block itself contained 17–38 words, all anchors 34–76, and the scoped registry 17–37. Short rule bundles were not padded.

## Residual failure

All three native/normative/scoped-registry conditions failed on the same post-change mirrored situation: Birch permits amber packets generally, then prohibits amber specifically for planning. The task asks for inspection, which remains permitted. Native and scoped registry returned `verify`; normative delivery returned `decline`, both times. One normative justification explicitly recognized that inspection was permitted while its action line still said `decline`.

The remaining failure is therefore an interpretation/action-output failure despite available rules. It does not show that the engine lost the exception or that grounding guarantees final compliance. The all-anchor arm additionally confused project scopes, sometimes citing a rule while reversing its actual modality.

## Limits and closure

This was designed after earlier retention controls and is not an independent held-out benchmark. It uses two synthetic mirrored histories, 24 cells per arm, and identical repeats. Canonical arms receive host-validated typed semantics at intake; automatic extraction from ordinary experience is not tested. Native versus normative changes retrieval, composition, representation and input length together. All-anchor comparison is informative but is not a length-matched isolation of grounding.

The implemented anchors, authority checks, dependency handling and atomic budget behavior have offline tests. This paid run primarily tests action comprehension for ordinary permissions and scoped changes; it does not establish adversarial robustness, long-term retention, general alignment or final-action enforcement. Cross-run changes from v9 cannot be uniquely attributed to context capacity because the task grid and settings also changed.

The bounded finding can be closed as follows: **SelMem's normative path improved rule-following accuracy relative to wide native recall on this pilot, reached parity with a simple scope-filtered registry, and did not establish a unique alignment benefit.**

Metric note: the runner's `extra_verify` counter covers only verification instead of authorized delivery. It omits verification instead of a required refusal. The independent confusion table above includes all seven native errors and does not interpret that counter as total unnecessary verification.

Source: `normative-wide-012.tar.gz`. Compiled source-component fingerprint: `2fc271141d024b964399a09b3ced427468237f37eca67e2e35933effd48bf0a3`.

Archive SHA-256: `69f6750b596f3c8afc2b8b2ae2b163e29db09aa8dcc2bdb3171e81c12e75b6b7`.
Plan SHA-256: `5a9ed076363b7a182e102a0b461f9e8db19901b89d2d76331c3541678ddbee75`.
