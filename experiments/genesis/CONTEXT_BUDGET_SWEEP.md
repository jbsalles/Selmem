# Context budget sweep

This opt-in change makes the total serialized-memory token ceiling, episode count
and per-episode word ceiling configurable together. Ordinary defaults remain the
historical 600 words, four episodes and 160 words per episode until evidence
supports a new default. Full prompt size also includes system instructions, the
current query and working conversation; the memory ceiling does not cap those.

## Configuration

```ini
memory_context=structured
memory_context_tokens=4000
memory_context_words=64000
memory_context_max_episodes=24
memory_context_episode_words=800
```

The word ceiling remains an independent bound. Setting it high lets the token
ceiling control delivery. Supported ranges: 512–64000 token units, 32–64000 words,
1–64 episodes and 1–4000 words per episode. The mouth's selection cap follows an
explicit episode-count change, and the stored profile cap is restored afterwards.
Legacy delivery rejects expanded options because it cannot honor this contract.

`MemoryContextOptions::token_counter` is a replaceable `fn(&str)->usize`. A host
with the selected model's tokenizer should provide a wrapper around that
**deterministic** tokenizer. Without it, the default counts UTF-8 bytes of the
entire serialized memory JSON, including provenance, metadata and punctuation.
For byte-based tokenizers this is a conservative upper bound, **not an estimate
of actual model token use**. It can leave substantial room unused. Word or
character ratios are not presented as exact token counts.

The final delivery filter enforces the configured counter, removing whole items
instead of clipping JSON. It removes associations, supplied context statements,
dispositions, episode tails and finally self-perspectives as needed. Episode
limits and scoped-record safeguards apply before that. Parent-first record
ordering ensures removing an episode tail does not leave a later exception alone.
The caller's counter must count the empty memory envelope below the chosen budget
and remain deterministic; use the provider's actual tokenizer for precise limits.
No provider tokenizer or external dependency is downloaded automatically.

## Bounded synthetic pilot

```bash
cargo test --release --test context_budget --bin selmem-context-budget-sweep
cargo run --release --bin selmem-context-budget-sweep -- \
  --out experiments/genesis/results/context-budget-001
```

The runner admits 24 distinct workshop constraints, freezes a common snapshot and
probes four tasks at ceilings of 1000, 2000, 4000 and 8000 units. Episode count (24),
per-episode ceiling (800 words), ranking and model instructions are held constant.
Each probe reopens the same snapshot. Two identical repeats produce **32 planned
requests**; offline preparation makes no HTTP calls. Each arm explicitly uses
UTF-8 byte bounds. Therefore this first pilot measures a conservative budget
sweep; it must not be described as using exactly 1k/2k/4k/8k model tokens.

With `GROK_API_KEY` configured:

```bash
cargo run --release --bin selmem-context-budget-sweep -- \
  --out experiments/genesis/results/context-budget-001 \
  --resume --actor --max-http-calls 32
```

Requests use Grok-4.3, reasoning none and temperature zero. Results include raw
responses and provider usage, frozen requests and memory payloads, selected
coverage, delivered episodes and byte counts. `comparison.json` reports mentioned
expected W-codes as a diagnostic only: mentioning a code does not prove its
condition is correct. Inspect rules and omissions in the original answers.
The small synthetic fixture is a capacity regression, not evidence of real-world
alignment or an optimal budget. No larger budget is automatically filled with
irrelevant content, and the archive remains unavailable to recall.

HTTP starts/results are append-only; the cap is per invocation. Execution stops
on the first API/response error. Ordinary resume skips recorded errors; explicit
`--retry-errors --resume --actor` retries errors and missing trials, never successful
responses. An interrupted attempt with unknown outcome blocks further calls.
A lock prevents concurrent execution; only remove stale locks after verifying the
process has stopped. Frozen plan SHA-256 and a fingerprint of the runner, engine,
retrieval and context serializer block accidental mixed-source resumes. This is
not a fingerprint of every source file. Keep the repository revision with results.

```bash
tar -czf context-budget-001.tar.gz \
  -C experiments/genesis/results context-budget-001
```
