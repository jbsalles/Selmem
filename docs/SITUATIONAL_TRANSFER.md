# Scoped situational interpretations

These mechanisms are part of the frozen organ used by the active
[LoCoMo strategy experiment](LOCOMO_STRATEGY.md). The retained pilots contain
nonempty transfer contexts but do not establish a benefit of interpretation
or improved creative answers. [Baseline](ORGAN_AUDIT_FIXES.md).

## Evidence and affect

Reported turns are split when successive sentences change affect direction.
An incident and its related consequences stay together until another affective
episode begins. Later support can therefore keep its own positive affect instead
of inheriting the incident's negative valence. Long reported turns are also
bounded, and every fragment keeps the original speaker and observation ID.
Lexical slices now receive their own valence, arousal, disgust and schema.
Explicit multipart semantic annotations still require annotations for every slice.
No global encoding or retrieval threshold is lowered.

## Interpretation during sleep

The deep-night ladder records `situational-interpretation` operations. Each
operation contains the original core, the hypothesis, confidence and support
trace ID. Its statement names the subject, observation, evidence and scope.
File and SQLite snapshots already persist operations; no new snapshot format or
migration is introduced. Deep sleep without the ladder cannot produce these
operations. Repeating consolidation does not count an observation twice.

The deterministic baseline recognizes two bounded kinds of social evidence:
acceptance/support associated with a helpful outcome, and hurtful interaction
associated with distress. It requires both a relation and a consequence. It
does not infer a personality from valence, assign recommended actions, or promote
a singleton to a belief. It conservatively excludes negated and hypothetical
sentences. This is a small lexical semantic baseline, not a learned general
interpreter; new domains and less explicit language can still yield no analogy.

## Transfer and factual recall

Advice or design queries may activate a stored, relevant situational hypothesis
despite absent literal topic overlap. Its score depends on evidence confidence,
fidelity and accessibility, with the existing eligibility gates. A shared word
such as `people` alone cannot establish topic relevance. Subject matches use
observed names to prevent mixing named people.

The reading profile labels these hypotheses as tentative observed analogies.
They do not become Claire's personal mood or lived experience. The real mouth
also filters unrelated externally supported axioms before replying. Factual
queries cannot activate the transfer path. A support-group question requires
that relation in the evidence, rather than assembling `group` and `support`
from different clauses. Gists, factual cores and sealed evidence are not
rewritten by this interpretation pass.

## Offline experiment gate

The strategy launcher writes `interpretations.json` with operation evidence and
`preflight.json` with per-probe nonempty-context counts, unique-context counts
and full/nosleep context differences. It includes creative probes as transfer
probes. Unknown factual controls may remain empty without failing the gate.

A dry run reports warnings and makes no provider calls. A paid run stops before
its first provider call if a transfer probe has no evidence, all conditions have
identical transfer contexts, or sleep changes no transfer context. Provider
errors after a successful gate retain the existing continue-and-record behavior.
Context differences are necessary for comparison, not proof of a useful effect.

Prepare memories from the frozen organ, or reuse a completed run with matching
protocol, dataset, organ revision and snapshot hashes. Snapshots from earlier
organs are incompatible. Existing `.selmem` configuration, including
`reasoning=none`, is reused. The launcher packs whole context items and audits
omissions; interpretation caveats are not cut mid-string. See the active
experiment guide for commands and sampling controls.
