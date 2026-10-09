# Learned co-recall in the memory organ

Co-recall is an optional, persisted graph in `MemoryStore`. Two independently
observed traces actually recalled together can acquire a stronger association.
The graph supplies contextual cues to later reconstruction; it does not enter
retrieval scores, lower selection thresholds, or retrieve extra traces.

## Activation and ablations

```rust
// After constructing or opening a SelectiveMemory:
mem.set_associations(true, true);   // learn and use associations
mem.set_associations(true, false);  // learn without using them
mem.set_associations(false, true);  // frozen graph, reconstruction only
mem.set_associations(false, false);// original baseline
```

Both switches default to false, including when loading an old snapshot. Existing
LoCoMo preparations therefore retain their original behavior. This change adds
the organ API, not a new launcher flag or an automatic change to the UI.

Learning requires an observed, live recall with reconstruction enabled. Read-only
probes and forced selection ablations do not train the graph. Narrator failures
also prevent reinforcement for that recall. An accepted unchanged gist can count
as activation without claiming that an unknown proposition has been verified.
Each participating trace must independently match the query through its core;
activation combines that relevance with its accessibility before recall.
Verbatim, suppressed, latent, empty, and pulled-back outputs do not train links.
Two traces with the same observation ID do not train each other.

An episode ends after more than ten virtual minutes without a participating live
recall, or after two virtual hours in total. A clock reversal starts a new one.
For controlled experimental trials, call `mem.begin_association_episode()` at
each genuine new episode. Repeating a prompt within an episode cannot strengthen
the same pair twice. The episode counter and timestamps survive saving.

## Reconstruction

Cues are frozen before any trace is reconstructed or the graph is reinforced.
Only other traces already selected for this recall are eligible: at most two
cues per target, with effective weight at least 0.05. A first co-recall cannot use
the link it is about to learn. Each cue carries its ID, current gist, attribution,
and decayed weight. The sealed archive is never read.

`Narrator::reconstruct_associated` has a backwards-compatible default, and
`supports_associations` defaults to false. `HttpNarrator` explicitly supports
the contextual prompt; `RuleNarrator` and the speak-only backend do not claim to
consume these cues. Custom narrators can implement both methods.

The HTTP prompt keeps the target core separate from the associated observations.
They may influence framing or interpretation, but do not establish causality,
shared participants, a shared event, or factual corroboration. External targets
are not associatively reconstructed as autobiographical memories. External cues
remain explicitly external.

Every associated proposal is judged against the target core alone, including
read-only probes and when normal grounding is cut. Misses, unjudged proposals,
and insufficiently grounded holds fall back to the original gist. New wording
normally requires a qualified proposition scorer; `NullScorer` does not license
unknown claims. Normal grounding and reconsolidation still run afterwards.
An association never increases confidence or counts as extra identity evidence.

## Weights and lifecycle

Weights are bounded to [0, 1]. A recall has a total reinforcement budget of 0.15,
divided among its selected pairs and multiplied by both activations and remaining
headroom. Weights decay with a thirty-virtual-day half-life. These constants are
exploratory, not fitted to LoCoMo outcomes.

The graph is capped at 4,096 pairs. Links decayed below 0.001 are pruned on a live
learning call. Reading a weight does not mutate it. Releasing a trace removes its
links. A successful fusion invalidates links of both endpoints rather than
silently transferring their histories to the fused event.

Both flat SELMEM1 and SQLite snapshots use the same strict graph codec. It stores
flags, IDs, weights, episode counts, and timestamps. Old snapshots remain readable
without a migration. Corrupted association records are rejected rather than
silently reset. Existing snapshots are readable by this version; a snapshot saved
with enabled associations should be reopened with this version to retain them.

## Audits and evidence

`RetrievalDump.associations_used` records target IDs, cue IDs/gists/weights, and
whether each proposed reconstruction passed the association guard.
`associations_reinforced` records before/after weights and the episode number.
Accepted changed live narratives add an `associative-reconstruction` operation
with source IDs and before/after text. That operation records the spoken
reconstruction, not a guarantee that reconsolidation changed the stored gist.
Read-only probes write neither graph state nor operation history.

Run `cargo test --test co_recall` for deterministic mechanism tests, including
equal-exposure separate recalls, identical retrieval with changed reconstruction,
claim rejection, episode deduplication, both snapshot formats, merge invalidation,
and a local mock HTTP request. No paid LLM calls are made by this test suite.

These tests demonstrate integration and isolation, not a measured effect on a
real model. A model experiment should compare a frozen learned graph on/off from
the same snapshot, record identical selected IDs and scores, then compare accepted
reconstruction text and downstream replies. Include an equal-exposure condition
where the two traces were recalled separately. Preserve the query, mood, narrator,
scorer, seeds where available, and snapshot for each comparison.
