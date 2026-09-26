# SelMem

Selective reconstructive memory for an LLM entity. v0.5

An LLM maps context to the next token. Adding facts increases coverage, but does not necessarily create divergence: identical contexts tend toward identical continuations. 

SelMem instead sculpts a particular past so that otherwise identical instances can follow different paths and develop a singular identity..

Its goal is not more memory, but path-dependent memory: selection, reconstruction, sleep, rumination, and identity transform experience into a history that actively shapes future context. The result is not a taller log, but a different past and therefore a different trajectory.

**Manifest:** [WHITEPAPER.md](WHITEPAPER.md)\
**Layout:** [ARCHITECTURE.md](ARCHITECTURE.md) — encode / judge / night / snapshot.\
**Benches:** experiments/REPORT.md — method, tables, P0 / P1, Drop / Lineage n=1, hearth P2/P3. Replay from experiments/README.md.\
**Knobs:** [PARAMETERS.md](PARAMETERS.md) — exploratory, not fitted.

Rust 1.75. SQLite via system `libsqlite3` (macOS SDK or Linux).

```
src/
  core/       model, profile, store, talk
  encode/     interpret, paint, split, gate, core, intake, scoring, embed, affect
  recall/     retrieve, judge, pull, narrator, http
  dream/      weather, rewrite, merge, ladder, release, night, drift, singularite
  persist/    snapshot (field list), file (SELMEM1), sqlite
  net/        api, httpx, ui
  config.rs   .selmem runtime options
  engine.rs   the loop only
  bin/        selmemd, selmem-chat
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
 deep:    weather → rewrite → merge → ladder → release
 shallow: weather → release
 (deep if new hours or charge clear the budget; sleep_deep forces it)
        ↓
   next experience is already colored
```

The model never sees the archive. Only gist, core, schema, affect, fidelity, mood, living axioms.

Claire and Silas are not characters. They are two sensitivities (`tender` / `austere`) on the same corpus. After nights they are not the same past.

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
./run.sh test
./run.sh run --release --example demo
./run.sh run --release --example compare
./run.sh run --release --example llm_night
./run.sh run --release --example bifurcation
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
| `llm` | `SELMEM_LLM` | unset | chat completions URL; unset = `RuleNarrator` |
| `model` | `SELMEM_MODEL` | bin: `llama3`, benches: `gpt-4o-mini` | model id |
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

`selmemd`, `selmem-chat`, and the experiment examples all read this file. `selmemd` prints `config <path>` when it loaded one.

Do not put `api_key` in a committed file. Copy the example, fill the key locally.

## Tests and experiments

Rust only. No Python suite. Always use `./run.sh` (not bare `cargo`) so sqlite link flags are set.

Scripts in `data/*.json` are the frozen stimuli. Edit those if you change a protocol; do not rewrite them mid-run. Later prompts in a script never name the marked event.

Published report (method + tables): experiments/REPORT.md. Conclusions only: [WHITEPAPER.md](WHITEPAPER.md) § Conclusions from the benches.

### Persist (interpretation after last-k eviction)

C1 k=8 vs C3 vs C2 vs C2Static vs C2−S/R/L/G. Default script: 12 dull days, five same-schema hours, late probe. `--ruminate` is the same-meeting ablation (hours pinned so merge cannot collapse them). `--bias drop|force` withholds or pins the marked scene at recall.

JSON reports book (`t0_in_book_*`), retrieval (`t0_rank_*`, `t0_selected_*`), and behavior (`marker_*`) separately. Probes are read-only.

```bash
./run.sh run --release --example persist -- --pairs 1 --last-k 8 --out selmem-persist-repeat.json
./run.sh run --release --example persist -- --ruminate --pairs 1 --last-k 8 --out selmem-persist-ruminate.json
./run.sh run --release --example persist -- --pairs 5 --seed 1 --last-k 8 --out selmem-persist-p1-grok-n5.json
```

Grok persist P1 is n = 5. DropMarked n = 1 (`--bias drop`): T₀ leaves the prompt, C2 mouth stays charged. DropLineage n = 1 (`--bias lineage`): mouth falls to C1 (~0.43); books stay split. Hearth P2 n = 5 / P3 n = 1: ladder, veto, util-to-strength. `--axioms-only` n = 1: C2 waits, NoLadder locks. See experiments/REPORT.md §8–§10.

AMA-Bench (`examples/ama`, experiments/ama_bench/) is a **side table**, not a SelMem score. It asks for step ids in agent logs. last-k 0.50 / static 0.28 / C2 0.19 on 3 episodes. Expected; do not submit. Why: experiments/REPORT.md §9.2.

### Benchmark v0.1 (H2)

Does `D_fp` stay above pre-T₀ after 8 identical later hours?

C0 = no book. C1 = last-k verbatim (default k = 24; `--last-k 8` drops T0 after the eight posts). C2 = SelMem. Two arms: salient/neutral, and two different salient events. 12 shared hours, 8 posts, 4 behavior probes via `speak_isolated`. Phase 0 invalidates on the book (`D_fp > 0.02` or unequal traces), never on `D_speak`. Three creative items in `data/creativity.json` are recorded, not claimed.

```bash
./run.sh test --test benchmark
./run.sh run --release --example benchmark -- --out selmem-v01.json
./run.sh run --release --example benchmark -- --pairs 10 --out selmem-v01-n10.json
```

`RuleNarrator` is deterministic: ten pairs repeat. A live model: same command with `llm=` in `.selmem`. JSON is rewritten after every cell.

The four Grok dumps and the exact replay lines: experiments/README.md § Replay.

### Unit tests (no network)

```bash
./run.sh test
./run.sh test --test scenes
./run.sh test --test engine
./run.sh test --test ground
./run.sh test --test dream_order
./run.sh test --test bifurcation
./run.sh test --test divergence
./run.sh test --test erasure
./run.sh test --test json_parse
./run.sh test --test talk
./run.sh test --test config
./run.sh test --test benchmark
```

| What | File | Stimulus |
| --- | --- | --- |
| Narrative scenes | `tests/scenes.rs` | `tests/cases/*.json` |
| Decay, anchors, SQLite, Ebbinghaus | `tests/engine.rs` | inline |
| DetachKind vs core | `tests/ground.rs` | inline |
| Night pass order | `tests/dream_order.rs` | `NIGHT_PASSES` |
| Bifurcation A/B (rules) | `tests/bifurcation.rs` | `data/bifurcation.json` |
| Split lives (rules) | `tests/divergence.rs` | `data/divergence.json` |
| Erasure: trivia vs repeated aversion | `tests/erasure.rs` | `data/erasure.json` |
| Chat JSON walker | `tests/json_parse.rs` | fixtures in the test |
| Working talk (session frame) | `tests/talk.rs` | inline |
| `.selmem` config parser | `tests/config.rs` | inline |
| Benchmark v0.1 H2 | `tests/benchmark.rs` | `data/v01.json` |

These use `RuleNarrator`. They must stay green offline.

### Replay a protocol (print the probes)

Same scripts as the unit tests, with the full report on stdout:

```bash
./run.sh run --release --example bifurcation
./run.sh run --release --example divergence
./run.sh run --release --example erasure
```

`Finished in 0.00s` means Cargo reused an old binary. After pulling code:

```bash
touch src/experiment.rs examples/bifurcation.rs
./run.sh build --release --example bifurcation
```

You should see `Compiling selmem`.

### Same protocols with a live model

Only `speak` / `reply` hits the HTTP API (`SpeakOnlyHttp`). Encode, sleep and reconstruct stay on the organ.

Options live in a `.selmem` file in the working directory (`data/config.example`). CLI flags and `SELMEM_*` env still override the file.

```
# .selmem  — not a SELMEM1 vault
llm=https://api.x.ai/v1/chat/completions
model=grok-4.3
api_key=xai-…
reasoning=none
temp=0
http_timeout=60
```

```bash
cp data/config.example .selmem
./run.sh run --release --example bifurcation
./run.sh run --release --example divergence
```

`--config path` or `SELMEM_CONFIG` selects another file. A `claire.selmem` vault starts with `SELMEM1` and is never read as config.

OpenAI-compatible endpoints work the same. Ollama:

```
llm=http://127.0.0.1:11434/v1/chat/completions
model=llama3
```

Check the endpoint before a 100-call run:

```bash
curl -sS --max-time 30 "$SELMEM_LLM" \
  -H "Authorization: Bearer $SELMEM_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"$SELMEM_MODEL\",\"reasoning_effort\":\"none\",\"messages\":[{\"role\":\"user\",\"content\":\"dis: ok\"}]}"
```

| Variable | Default | Role |
| --- | --- | --- |
| `SELMEM_LLM` | unset | chat URL; unset = rules only |
| `SELMEM_MODEL` | — | model id |
| `SELMEM_API_KEY` | unset | `Authorization: Bearer` |
| `SELMEM_REASONING` | `none` | xAI `reasoning_effort` (`none`/`low`/… or `off` to omit) |
| `SELMEM_TEMP` | `0` | sampling temperature |
| `SELMEM_HTTP_TIMEOUT` | `60` | curl `--max-time` seconds |
| `SELMEM_QUICK` | unset | `1` = salient condition only, 2 probes, one persist window |
| `SELMEM_PROBES` | all / 2 if quick | how many probe questions to speak |

A line `narrator: 0/10 answers are the RuleNarrator template` means the model answered. `Cela me revient` means the HTTP call failed and the rules ran. `selmem LLM reply failed:` prints the error.

Full bifurcation ≈ 100 `reply` calls (5 probes × 2 agents × 5 snapshots × 2 LLM conditions). Quick mode ≈ 12. Ablation `salient-no-consolidation` never calls the model.

Fingerprint distance is on the book (0 = clones). Speak distance is wording overlap; two clones with the same book already differ under Grok (\~0.7). Do not read H₁ off Δspeak alone.

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
- `kept` **is not “it understood you\`.** It means a trace was written. The next sentence still comes from Grok looking at that gist, not from a stored Q&A.
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
- Nearby episodes fuse into myth. Heavy anchors do not merge.
- Axioms climb: 2 traces → motif, 3+ → belief, aligned beliefs → trait. Lineage stays.
- Two profiles on the same corpus diverge. Fingerprint uses founders, traits, contradictions.
- Proxy for originality: reconstruction sticks less than the unmodified log; clones are not interchangeable; `world` facts survive (`originality_is_path_dependent_not_a_taller_log`).
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

## What this is not

Not RAG. Not a vector database. Not a personality in a system prompt.\
Not a neocortex and not a brain. An executive–autobiographical loop around a next-token transducer.\
No local neural encoder ships in-tree: pass `--embed` if you have one.

## How it's built

Humans own architecture, concepts and governance. Models draft code, tests, and prose. Generated patches are reviewed.