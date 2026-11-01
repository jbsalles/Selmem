# Frozen native-memory causal ablations

This protocol tests the delivered memory path, after native recall. It does not
retrain the model, replay lives, or change consolidation. The V4 qualified lives
remain synthetic and use a scripted witness: this is not evidence of spontaneous
psychological preference formation.

The previous V4 ON arm supplied a fallible hypothesis **and a predicted action**.
The causal protocol instead calls `open_mouth_scoped` and takes the normal
structured `MemoryContext`, including the native self-view. It never appends the
expectation, picks supporting episodes by ground-truth IDs, or strengthens a
belief's wording. A fresh empty working conversation is identical in every arm;
the native system guide and contract are shared, with the same output constraint.

| Arm | Intervention on the frozen delivered context |
| --- | --- |
| full | Native filtered context, unchanged |
| no_beliefs | Remove conditional self-view entries only; keep episodes and other channels |
| no_episodes | Remove delivered episodes and their association edges |
| swapped | Replace conditional self-view entries with those naturally delivered from the mirror life for the same query and scope |
| bare | Empty memory; keep the shared system instructions and agent identity |
| placebo | Empty memory plus neutral inventory text matched to the full serialized user-payload byte length |

These are delivery interventions. They estimate the marginal contribution of a
channel given the others, rather than the total developmental effect of having
that channel. `no_beliefs` does not erase episodic evidence about a belief.
`no_episodes` retains aggregate affect and other dispositions. The placebo matches
UTF-8 bytes, **not provider tokens**; record token usage separately if exact token
matching is needed. It controls an unrelated text load, not every semantic effect
of reading a memory-shaped narrative.

## Freeze an existing qualified run

Pass the actual directory containing the V4 `*-normal-on.vault` and
`*-reversed-on.vault` files. A breaking-point run has a separate directory for each
load; run each load as a separate experiment. Do not rebuild lives for each arm.

```bash
cargo test --bin selmem-contextual-longlife
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation \
  --snapshots experiments/genesis/results/contextual-v4-grok-pilot-001/snapshots \
  --out experiments/genesis/results/causal-native-001 \
  --pairs 1 --repeats 3 --memory-words 600
```

This stage makes zero API calls. It copies the input snapshots, checks the store
has two qualified conditional beliefs, and freezes the exact requests and native
delivery audit. Review `delivery-audit.jsonl` before the actor run: a belief can be
qualified in storage yet absent from delivery. Missing delivery is an informative
negative result; never add a belief or an episode to rescue it. In particular,
review whether beliefs appear through another channel; the removal is explicitly
of conditional **self-view entries**, not a semantic erasure across the prompt.

```bash
# Set GROK_API_KEY in your shell, without committing it.
cargo run --release --bin selmem-contextual-longlife -- \
  --causal-ablation \
  --snapshots experiments/genesis/results/contextual-v4-grok-pilot-001/snapshots \
  --out experiments/genesis/results/causal-native-001 \
  --pairs 1 --repeats 3 --memory-words 600 \
  --resume --actor --provider grok --model grok-4.3 --max-http-calls 96
```

Repeat the same command to continue. The cap counts HTTP attempts per invocation;
there are no automatic retries. Transport failures are recorded as invalid trials
and are not silently replaced. A process interrupted during an in-flight request
can incur an unrecorded charge; the cap is not a monetary guarantee. Retain the
frozen plan unchanged when resuming. Provider/model and design settings must match.

## Design and interpretation

One domain uses two mirror lives, two scopes, two wordings, **both option orders
for each wording**, six arms and three identical repetitions: **288 calls**.
In general: `96 × domains × repetitions`. The deterministic randomized request
schedule interleaves arms and repetitions. Grok reasoning is explicitly `none`,
temperature is zero; repeat requests are byte-identical and uncached by the harness.
A provider cache or hidden server variation remains possible.

`choices.jsonl` retains exact requests, raw answers and errors. `paired.csv`
compares full to each arm on the same life/scope/wording/order/repetition. It gives
valid pairs, decision flips and directional discordances relative to the original
history. Invalid responses and pending trials remain visible and are excluded
from paired valid-choice calculations. The invalid/pending columns are run-wide,
repeated for convenience, not per-contrast rates.

`repeat_noise` compares full repetitions 1..R-1 against full repetition 0 for the
**identical request**. These comparisons share a reference and are dependent;
they estimate variability, not additional independent samples. Option-order and
wording changes are separate cells and must not be counted as execution noise.

Primary comparisons are full vs no_beliefs, full vs no_episodes, and full vs
swapped. Compare the flip rates with repeat noise, and inspect whether swapped
choices move toward the donor history. Full vs bare/placebo checks that any effect
exceeds the shared prior and an unrelated text load. Null removal effects can mean
redundancy or missing delivery; they do not prove that a channel never matters.
Report results by domain and scope before pooling. Cluster uncertainty by domain
or independently generated history, never treat repeats as independent lives.
Start with one domain as a technical pilot; confirm across the remaining domains
before making causal generalization claims. Do not tune wording after seeing choices.

## Offline implementation check

The regression fixture (workspace domain, 80 distractors, V4 seed 1729) exposes
**zero native self-view beliefs** across its 16 query cells, even though the two
conditional beliefs are qualified in each stored life. This is a delivery finding,
not an actor result and not a measurement of the user's existing run snapshots.
The harness reports exposure in `exposure.csv`. When no conditional self-view is
present in either mirror, full / no_beliefs / swapped preserve identical requests;
any disagreement between them is execution noise. Inspect exposure before spending
actor calls on belief-specific causal claims. No paid actor run was performed.
