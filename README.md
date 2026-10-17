# SelMem


Selective reconstructive memory for an LLM entity. v0.5

API reference is the crate rustdoc (`cargo doc --no-deps`). docs.rs builds it on publish. This file is the claim, not the reference.

An LLM maps context to the next token. Adding facts increases coverage, but does not necessarily create divergence: identical contexts tend toward identical continuations. 

SelMem goal is not more memory, but path-dependent memory: selection, reconstruction, sleep, rumination, and identity transform experience into a history that actively shapes future context. The result is not a taller log, but a different past and therefore a different trajectory.

**Hypothesis:** After a year of the same stream, two copies of the same model that retained one hour on one side only no longer remember or forget the same things. Measured on the book, retrieve, and the mouth. Historical exploratory runs motivated this hypothesis; durable behavioral singularity and useful creativity remain to be demonstrated.

**Manifest:** [WHITEPAPER.md](WHITEPAPER.md)\
**Layout:** [ARCHITECTURE.md](ARCHITECTURE.md) — encode / judge / night / snapshot.\
**Benches:** [experiments/RESULTS.md](experiments/RESULTS.md) — observations and limits. Replay from [experiments/README.md](experiments/README.md).\
**Knobs:** [PARAMETERS.md](PARAMETERS.md) — exploratory, not fitted.

**Co-recall:** [Learned associations](docs/CO_RECALL_ASSOCIATIONS.md) — optional organ-level contextual reconstruction, with unchanged retrieval scores and persisted audits.

Rust 1.75. SQLite via system `libsqlite3` (macOS SDK or Linux).

```
src/
  core/       model, profile, store, talk
  encode/     interpret, paint, split, gate, core, intake, scoring, embed, affect
  recall/     retrieve, judge, pull, narrator, http
  dream/      weather, confab, centers, rewrite, merge, ladder, release, night, drift, singularite
  persist/    snapshot (field list), file (SELMEM1), sqlite
  net/        api, httpx, ui
  config.rs   .selmem runtime options
  engine.rs   the loop only
  bin/        selmemd, selmem-chat, selmem-genesis, selmem-locomo-strategy
```

## Loop

```
experience → interpret → paint → split → gate → core
        ↓                         ↑
   lived book + sealed archive    |
        ↓                         |
 remember / speak
 retrieve → reconstruct → judge (DetachKind) → pull
 talk frame keeps the live sitting
        ↓
      sleep (talk goes through the gate, then the frame dies)
 deep:    weather → confab → ladder → refresh centers → gravitate → rewrite → merge → release
 shallow: weather → confab → release
 (deep if new hours or charge clear the budget; sleep_deep forces it)
 `NIGHT_PASSES` still names weather / ladder / rewrite / merge / release. Confab and centers run as hooks, not a sixth named pass.
        ↓
   next experience is already colored
```

The model never sees the archive. Only gist, core, schema, affect, fidelity, confidence, mood, living axioms, schema centers. Access is availability. Confidence is whether the current gist is still trusted. Suppression withholds an hour from default recall without leaving the book.

`tender` / `austere` are two gates on the same corpus, not characters. After nights the books can differ. That is the claim above, not a personality.

**Observation support.** Admission of a new trace and retention of another observation supporting an existing trace are separate operations. With `observation_evidence` enabled, stable caller-provided observation identities accumulate bounded support; replayed identities do not count twice. Support accompanies a selected trace without becoming gist/core rewrite material. Caller identities do not prove statistical independence. See [PARAMETERS.md](PARAMETERS.md).

## Build

Zero Cargo crates. Persistence is a vault, not the memory: the organ lives in RAM (`MemoryStore`); on `save` it is dumped, on `open` it is reloaded. The model never talks to the vault.

Two backends, same `Snapshot` (`profile`, `mood`, `store`). The field list lives once in `persist/snapshot.rs` so SELMEM1 and sqlite cannot drift:

| Path | Backend |
| --- | --- |
| `.db` / `.sqlite` / `.sqlite3` | system `libsqlite3` (prepared statements, `BEGIN IMMEDIATE`) |
| anything else | flat `SELMEM1` file |

IDs are `{prefix}_{pid}_{n}`. The counter is raised on load for both backends. Dropped events leave no archive. Orphans are pruned on sleep and SQLite load.

Link is dynamic against the system `libsqlite3`.

| OS | What you need |
| --- | --- |
| macOS | Xcode Command Line Tools (`xcode-select --install`). The SDK already ships sqlite3. If link still fails: `brew install sqlite` — `./run.sh` adds Homebrew’s lib path. |
| Linux | `libsqlite3` (the `.so.0` runtime is enough). `./run.sh` invents a `libsqlite3.so` stub when `-dev` is missing. |

Use `./run.sh` instead of bare `cargo` so those paths are set. A `SELMEM1` vault named `claire.selmem` does not need SQLite. A cwd `.selmem` with `key=value` lines is config, not a vault.

Apple Silicon and Intel are both fine. Bind `127.0.0.1` or `0.0.0.0` as usual.

```bash
./run.sh test --all-targets
./run.sh test --doc
./run.sh run --release --example two_entities
./run.sh run --release --bin selmemd -- --help
```

## Config

Runtime options live in a `.selmem` file in the working directory. That is not the book.

| File | First line | Role |
| --- | --- | --- |
| `.selmem` (cwd) or `selmem.conf` | `llm=…` | options |
| `claire.selmem` / `--path` | `SELMEM1` | vault (traces, axioms, mood) |

```bash
cp data/config.example .selmem
```

```
# .selmem
llm=https://api.x.ai/v1/chat/completions
model=grok-4.3
api_key=
reasoning=none
temp=0
http_timeout=60

# embed=
# embed_model=text-embedding-3-small

# bind=127.0.0.1:7420
# path=claire.db
# name=Claire
# profile=tender
# token=

# ground_overlap=0.18
# ground_strikes=3
# narrator_firmness=0.42

# probes=2
# quick=false
```

`SELMEM_LLM=` and `export SELMEM_API_KEY=` lines are accepted. Quotes are stripped.

**Precedence:** `--flag` &gt; `SELMEM_*` env &gt; file &gt; default.

Another file: `--config path` or `SELMEM_CONFIG`. Discovery otherwise: cwd `.selmem`, then `selmem.conf`.

| Key | Env | Default | Role |
| --- | --- | --- | --- |
| `defaultLlm` | `SELMEM_DEFAULT_LLM` | unset | which block the daemon attaches: `grok` or `gpt` |
| `llm` | `SELMEM_LLM` | unset | chat completions URL; unset and no plug = `RuleNarrator` |
| `model` | `SELMEM_MODEL` | provider-dependent | model id |
| `api_key` | `SELMEM_API_KEY` | unset | `Authorization: Bearer` |
| `reasoning` | `SELMEM_REASONING` | `none` | xAI `reasoning_effort` (`none`/`low`/… or `off` to omit) |
| `temp` | `SELMEM_TEMP` | `0` | sampling temperature |
| `http_timeout` | `SELMEM_HTTP_TIMEOUT` | `60` | curl `--max-time` seconds |
| `embed` | `SELMEM_EMBED` | unset | embeddings URL |
| `embed_model` | `SELMEM_EMBED_MODEL` | `text-embedding-3-small` | embedding model id |
| `bind` | `SELMEM_BIND` | `127.0.0.1:7420` | `selmemd` listen address |
| `path` | `SELMEM_PATH` | `entity.db` / `claire.db` | vault path |
| `name` | `SELMEM_NAME` | `Claire` | entity name |
| `profile` | `SELMEM_PROFILE` | `tender` | `tender` / `austere` |
| `token` | `SELMEM_TOKEN` | unset | HTTP bearer for the daemon |
| `ground_overlap` | `SELMEM_GROUND_OVERLAP` | profile | identity gate on `Hold` only |
| `ground_strikes` | `SELMEM_GROUND_STRIKES` | profile | misses before a pull-back |
| `narrator_firmness` | `SELMEM_NARRATOR_FIRMNESS` | profile | blend strength toward core |
| `probes` | `SELMEM_PROBES` | all / 2 if quick | how many bench questions to speak |
| `quick` | `SELMEM_QUICK` | unset | `1`/`true`/`yes` = salient only, 2 probes, one persist window |

`selmemd`, `selmem-chat`, and the retained experiment runners read this file. `selmemd` prints `config <path>` when it loaded one.

Do not put `api_key` in a committed file. Copy the example, fill the key locally.

## Tests and experiments

Rust only. No Python suite. Always use `./run.sh` (not bare `cargo`) so sqlite link flags are set.

The maintained experiments are [GENESIS](experiments/genesis/README.md) and [LoCoMo strategy](docs/LOCOMO_STRATEGY.md). Historical narrative demos and diagnostic grids have been retired; raw evidence remains in the separate audit archive. Current conclusions: [experiments/RESULTS.md](experiments/RESULTS.md) and [WHITEPAPER.md](WHITEPAPER.md) § Conclusions from the benches.

### GENESIS

One executable, two commands: `selmem-genesis positive` qualifies the decision model against a faithful history; `selmem-genesis learning` tests cumulative learning with observation support OFF/ON. Both read provider configuration from `.selmem`, or the file selected by `--config` / `SELMEM_CONFIG`.

`positive-001`: Grok 48/48 oracle choices; GPT-4o-mini 28/48. `evidence-001`: native and strict recall match the oracle on 8/16 with support OFF and 16/16 with support ON. The effect comes from explicit observation counts, not demonstrated reconstruction superiority, singularity or creativity. One history pair is exploratory, not population replication.

```bash
./run.sh run --release --bin selmem-genesis -- --help
bash experiments/genesis/run-positive-control.sh experiments/genesis/results/positive-new
bash experiments/genesis/run-evidence-learning.sh \
  experiments/genesis/results/positive-new/grok \
  experiments/genesis/results/evidence-new
```

Real runs make paid API calls. Scripts print progress and cap HTTP attempts including retries. Choose fresh output paths. `--mock` tests plumbing only and cannot qualify a model. See the [protocol](experiments/genesis/protocol.md) for controls, probe isolation and limitations.

### Unit tests (no paid LLM calls)

```bash
./run.sh test --all-targets
./run.sh test --doc
./run.sh test --test scenes
./run.sh test --test engine
./run.sh test --test ground
./run.sh test --test dream_order
./run.sh test --test horizon
./run.sh test --test talk
./run.sh test --test config
```

These cover selection, reconstruction, grounding, consolidation, persistence, observation support and experiment plumbing. `examples/two_entities.rs` is the retained minimal library example.

## Run

Same keys can sit in `.selmem`. Flags still win.

```bash
./run.sh run --release --bin selmemd -- \
  --bind 0.0.0.0:7420 \
  --path claire.db \
  --name Claire \
  --profile tender \
  --ground-overlap 0.18 \
  --ground-strikes 3 \
  --narrator-firmness 0.42 \
  --token secret \
  --llm https://api.openai.com/v1/chat/completions \
  --model gpt-4o-mini \
  --api-key "$SELMEM_API_KEY"
```

UI: `http://IP:7420/` — paste the token at the top, talk.

### UI controls

The composer (field, spinner, **send**) stays pinned to the bottom.

| Control | What it does |
| --- | --- |
| **token** | Bearer for this tab (`localStorage`). Empty if the daemon has no token. |
| **tender / austere / neutral** | Sleep-time voice. Sets embellish vs disgust and the retell table. Does not wipe the book. |
| **who are you** | Living axioms (`GET /who`). Empty until sleep minted a belief. |
| **pin** | Test comfort only (`POST /pin`). Forces the current line into the book. **Avoid it.** The organ should keep or drop on its own (gate + salience-weighted night). Pin is so someone can try the UI without waiting for a charged hour — not how a deployed instance should live. |
| **sleep** | One night (`POST /sleep`): sculpt, merge, extinguish, maybe mint axioms. Then the salon thread is dropped. Click once. |
| **Speak to it… / send** | One turn (`POST /turn`): encode attempt + reply. Spinner while the model runs. `kept` = wrote a trace; `left` = stayed in the thread only. |
| `kept` / `left` + S | Gate score. `left` is not amnesia until you sleep. |

Do not sleep to “refresh.” Sleep ends the sitting. Prefer a charged hour over **pin**. Pin is a debug override.

### Why the chat can feel weird

This is not a chatbot with extra context. `/turn` asks the model to continue from two short piles: the live thread (WorkingTalk) and a few recalled gists. The reply prompt is four lines. There is no persona script.

What that produces:

- `left` **is not forgetting.** The line stayed in the salon. It dies when you sleep, unless you pin it or it already passed the gate.
- `kept` **is not “it understood you”.** It means a trace was written. The next sentence still comes from Grok looking at that gist, not from a stored Q&A.
- **Empty book + “hello”** → the model fills the hole. Velvet greetings, “I know your name” without saying it, a politician who “never quite landed.” That is the prior, not SelMem.
- **Sleep wipes the thread.** “What did we talk about yesterday?” only sees what survived the night. A pinned name can come back; small talk cannot.
- **Sleep also retells.** Tender can soften a fact (“JB became quieter”). Austere can harden it. The organ is allowed to warp; the UI will look inconsistent if you expect a CRM.
- **Two clocks.** The salon lasts ten minutes. A “day” is one Sleep click, not 24 hours.

If the replies feel like a well-prompted Grok, the book is thin. Pin the hour that should last, sleep once, ask again. If they still feel like theater, the model is padding an empty recall — that is expected, not a bug in the buttons.

Local Ollama: `--llm http://127.0.0.1:11434/v1/chat/completions --model llama3`

Behind nginx:

```
location / { proxy_pass http://127.0.0.1:7420; proxy_read_timeout 90s; }
```

Or only the file:

```
llm=https://api.openai.com/v1/chat/completions
model=gpt-4o-mini
api_key=sk-...
embed=https://api.openai.com/v1/embeddings
token=secret
```

No `llm`: `RuleNarrator` + hashed vectors. The organ still runs.

One thread per connection. `/health` and `/` do not take the memory lock. `/turn` is serialized on the organ (one writer). Auth is `Authorization: Bearer` only — not `?token=`.

## HTTP

| Method | Route | Role |
| --- | --- | --- |
| GET | `/health` | liveness (no token, no lock) |
| GET | `/who` | living axioms (trait &gt; belief &gt; motif) |
| GET | `/lineage?schema=` | history of a belief |
| GET | `/mood` | mood |
| GET | `/profile` | knobs (`τ`, embellish, ground, …) |
| POST | `/profile` | set knobs |
| GET | `/audit?id=` | sealed verbatim (human debug, never the model) |
| GET | `/talk` | live thread (topic + turns of the active conversation) |
| POST | `/live` | encode |
| POST | `/remember` | reconstruct |
| POST | `/speak` | embodied reply (holds the thread) |
| POST | `/turn` | live + reply |
| POST | `/pin` | keep the current thread topic as a lived trace |
| POST | `/talk/clear` | drop the session frame |
| POST | `/sleep` | consolidate |
| POST | `/save` | flush |

## Rules that hold

- Two books: lived narrative / archive. The model never reads the second. Grounding rewrites the gist toward the *core*; it never injects the journal.
- Fading traces (cold / myth / low hold) may warp without a ceiling. Living traces are pulled back in proportion to `narrator_firmness`.
- Latent forgetting: the scene drops out of recall; schema and affect still color the next event.
- Two channels: `self` is sculpted, `world` is not.
- Forgetting by default. Encoding threshold.
- Unlabelled events get lexical affect (FR+EN), then identity, then optional LLM `interpret` on the live sentence.
- Living axioms color the next encoding before the gate.
- Cherished memories embellish. Recalled disgust amplifies. Neglected disgust extinguishes.
- Recall can shift *meaning* (valence under current mood), not only wording.
- Nearby episodes can fuse. Heavy anchors do not merge.
- Axioms climb: 2 traces → motif, 3+ → belief, aligned beliefs → trait. Lineage stays.
- Two profiles on the same corpus can keep different books. Fingerprint is a lab metric on the book, not a personality score.
- Persistence: `.db` (SQLite) or `.selmem` (flat file).

## Rust API

```rust
let mut mem = SelectiveMemory::open("claire.db", EntityProfile::tender("Claire"))?;
mem.live_with(input);
let recalled = mem.remember("that evening");
let _ = mem.speak("do you remember?");
mem.sleep();
mem.who_am_i();
mem.lineage("loyalty");
singularity_distance(&fingerprint(&a), &fingerprint(&b));
```

## Hors claim

Do not read these as results. They are outside the demonstrated result.

- Personality, character, “Claire / Silas as people.”
- Creativity, originality, intelligence, “more human.”
- Superiority over RAG, summaries or full history.
- Durable singularity, long-term persistence or transfer to new actors and tasks.
- Wording distance as a behavioral validity test.
- A benefit caused specifically by reconstruction rather than retained evidence.

## What this is not

Not RAG. Not a vector database. Not a personality in a system prompt.\
Not a brain. A loop (select → reconstruct → sleep) around a next-token transducer.\
No local neural encoder ships in-tree: pass `--embed` if you have one.

Clock isolation, durable saves, reinterpretation and snapshot migration guarantees are documented in [Organ integrity](docs/ORGAN_INTEGRITY.md).

## How it's built

Humans own architecture, concepts and governance. Models draft code, tests, and prose. Generated patches are reviewed.
## Test deployment

Deploy the Rust server and UI together on Render Free using [the deployment guide](deploy/README.md).
The `main` branch deploys after CI passes; the public demo isolates each visitor’s temporary memory and provider key.

### GENESIS experimental protocol

[GENESIS](experiments/genesis/README.md) retains model qualification and a controlled cumulative-learning assay. Probes run on isolated memory states. Transplantation, structural lesions, long-horizon singularity and constrained creativity remain further experiments, not active campaigns or demonstrated results. See the [scope and limitations](experiments/genesis/protocol.md).
