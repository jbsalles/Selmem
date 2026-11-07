# Normative memory (experimental, opt-in)

Normative focus keeps caller-validated permissions, prohibitions, uncertainty,
conditions and revision links attached to living traces. It protects their typed
semantics from ordinary prose reconstruction and composes complete applicable
rule families for the actor. It does not establish general LLM alignment or
validate the actor's final answer.

## Trust and intake

The host authenticates an issuer and supplies a `NormRecord` annotation. SelMem
checks an explicit runtime source allowlist and an active external carrier with
an identical source. A sentence claiming to be an operator has no such authority.
This is not authentication: applications must not let user text control source
IDs, the allowlist, annotations or query scope. Extraction/verification of norms
from arbitrary natural language is deliberately outside this initial API.

```rust
use selmem::{Attribution, EncodeInput, EntityProfile, Modality, NormClause,
    NormRecord, NormRelation, NormScope, SelectiveMemory};
use std::collections::BTreeMap;

let mut memory = SelectiveMemory::new(EntityProfile::new("Claire"));
memory.set_normative_authorities(&["operator"])?;
let mut input = EncodeInput::observed(
    "Cedar allows amber exports to Mira; teal exports are prohibited; other categories need verification.",
    "authenticated-event-42",
);
input.source = "operator";
input.attribution = Attribution::External;
let decision = memory.live_with(input);
if let Some(id) = decision.trace_id {
    let scope: NormScope = BTreeMap::from([
        ("project".into(), "Cedar".into()),
        ("recipient".into(), "Mira".into()),
    ]);
    memory.register_normative(&id, NormRecord {
        source: "operator".into(), scope: scope.clone(), action: "export".into(),
        clauses: vec![
            NormClause { when: BTreeMap::from([("category".into(), "amber".into())]), modality: Modality::Permit },
            NormClause { when: BTreeMap::from([("category".into(), "teal".into())]), modality: Modality::Forbid },
        ],
        otherwise: Some(Modality::Verify), relation: NormRelation::Root,
    })?;
    let draft = memory.open_mouth_normative(
        "Export amber to Mira for project Cedar?", &scope, "export",
    )?;
    // Use draft.memory_reply() with a context-aware narrator, then close/cancel.
    memory.cancel_mouth(&draft);
}
```

The ordinary encode gate still decides whether a carrier is admitted. Register
only after a successful admission; the API does not silently pin or resurrect it.
The annotation is protected semantic memory, not access to archived verbatim.
Identifiers are case-sensitive ASCII letters/digits plus `_ - . /`, max 96 bytes.
At most 256 normative records, 16 clauses per record and 8 fields per scope or
condition are accepted. Conditions within a clause are conjunctive. Scope and
condition fields are supplied separately: scope selects the rule family; all
category/purpose conditions remain available to the actor.

## Grounding and revisions

`NormRecord::grounds(candidate)` is a conservative exact comparison of typed
semantics. It detects changed modality, scope, conditions, source, fallback or
parent. It is not a semantic classifier for arbitrary prose. Actual normative
readout deterministically renders the immutable anchor, so an altered gist never
becomes a changed permission. Ordinary reconsolidation may still modify that
trace's prose; it cannot rewrite the normative anchor.

An existing anchor cannot be overwritten. Admit a new authenticated event and
register `Exception(parent_trace_id)` for a conditional change, or
`Revision(parent_trace_id)` for a replacement in the same scope. Source, scope
and action must agree with the parent. Exceptions inherit the parent's fallback.
Dependencies are traversed parent-first; missing parents and cycles block rule
exposure. The initial implementation requires exact parent scope equality;
narrower conditions belong in clauses, not a different scope.

Multiple roots, overlapping opposing clauses and competing sibling changes are
reported conservatively for review. SelMem does not infer authority hierarchy or
choose the most recent timestamp as a winner. Conflict detection can over-report:
logically equivalent or disjoint domain-specific conditions are not reasoned over.
A full revision retains the parent for provenance and labels the replacement;
the actor must apply that explicit relationship. Final action enforcement is not
implemented.

## Delivery and lifecycle

Use `open_mouth_normative` explicitly. It selects by supplied scope/action from
living anchors, not by the desired answer or an archive. This is a separate,
bounded normative retrieval path; it does not claim unchanged episodic ranking.
Suppressed, latent, low-access (< 0.15), source-mismatched or unauthorized carriers
are unavailable. A missing scope field is uncertainty, not a wildcard match.

Normative focus supplies only the normative block, the current user request and
agent identity. Ordinary narratives, dispositions, affect, reading and prior
working conversation are excluded from that draft, preventing them from silently
re-entering through another prompt field. They remain in memory for ordinary
`open_mouth`; the feature does not globally censor or delete observations.

The complete applicable bundle must fit the configured word ceiling. Otherwise,
only an explicit incomplete-context message is delivered. No clause is clipped
and no dependent exception is delivered alone. JSON includes trace IDs and issue
codes for auditing; the evidence word count covers the rendered text, including
its provenance and conditions, not JSON punctuation/metadata. Cutting episodes
also cuts normative delivery. Legacy context mode is rejected by the focus API.

The configurable serialized-memory token ceiling also applies to normative
delivery. If the complete bundle does not fit, it is replaced atomically with an
incomplete-context marker and `token_budget_exceeded`, with no trace IDs or rule
fragments. Word counts include the replacement text. If even that marker cannot
fit the configured counter, the focus API returns an error before opening a
mouth. The default counter is a conservative UTF-8 byte bound; applications may
provide their own tokenizer. Expanded episode and word settings remain available.

Generic merge cannot combine a normative carrier with another trace. Forgetting
still applies: releasing a carrier removes its anchor; any surviving dependent
becomes incomplete. File and SQLite snapshots persist anchors. Trust allowlists
are runtime-only and must be reapplied after opening a vault. Old snapshots have
no normative anchors. The sealed archive is never used for recovery.

## Evaluation

See [the bounded normative pilot](../experiments/genesis/NORMATIVE_PILOT_V10.md).
The completed wide-context run recorded 192 valid Grok 4.3 responses. Normative
delivery scored **46/48**, compared with **41/48** for native recall, **40/48** for
unfiltered anchors and **46/48** for the scope-filtered registry. Native recall
already contained all applicable rules. The five corrections concern explicit
prohibitions; authorized delivery accuracy stayed 18/20 and unauthorized deliveries
were zero in every arm. The actor still misapplied a planning-only prohibition to
inspection. This is a small rule-following pilot, not proof of general alignment
or a unique reconstruction benefit. [Results, limitations and source hashes](../experiments/genesis/NORMATIVE_WIDE_RESULTS.md).

Existing v8/v9 runs remain historical controls and should not be resumed using
a changed source fingerprint. The confidentiality assay is a separate protocol;
these results must not be presented as its execution.
