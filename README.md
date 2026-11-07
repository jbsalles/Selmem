# SelMem


Selective reconstructive memory for an LLM entity. v0.5

An LLM maps context to the next token. Adding facts increases coverage, but does not necessarily create divergence: identical contexts tend toward identical continuations. 

SelMem goal is not more memory, but path-dependent memory: selection, reconstruction, sleep, rumination, and identity transform experience into a history that actively shapes future context. The result is not a taller log, but a different past and therefore a different trajectory.

**Hypothesis:** After a year of the same stream, two copies of the same model that retained one hour on one side only no longer remember or forget the same things. Measured on the book, retrieve, and the mouth. Historical exploratory runs motivated this hypothesis; durable behavioral singularity and useful creativity remain to be demonstrated.

**Research update — October 2026.** The creative experiments now have recorded proposals, matched controls and compact public evidence. There are early qualitative indications consistent with creative recombination and memory-conditioned variation, but **no proven creativity advantage attributable to SelMem**. Interesting proposals also occur without memory; the guarded preview and fixed-pool selection diagnostic do not establish superiority. Read the [creativity findings](experiments/CREATIVITY_RESULTS.md), inspect the [evidence](experiments/evidence/README.md), or open the [results showcase](docs/index.html).

**Native causal ablations — analysis November 1, 2026.** A completed Grok 4.3 run recorded **3,456 valid choices** across 12 synthetic scenario domains, six delivery conditions, counterbalanced options and three identical-request repetitions. With the explicit `context-cued` recall route, full memory was **479/576 (83.2%)** history-concordant, versus **291/576 (50.5%)** with empty memory, **429/576 (74.5%)** without delivered beliefs and **415/576 (72.0%)** without delivered episodes. Both component cuts changed 12.8% of matched choices, compared with a 2.9% identical-request repeat-disagreement reference. These are **post-recall delivery effects**: beliefs already helped select episodes. The domains share one calm/social axis; order sensitivity, weak donor-history adoption after swaps, and the earlier failed native-delivery run remain documented. [Results, controls and compact answers](experiments/genesis/CAUSAL_NATIVE_RESULTS.md).

**Contextual self update — October 2026.** New Grok 4.3 diagnostics recorded 70 memory-formation HTTP attempts and 96 matched actor decisions. Scripted episodes produced self beliefs that weakened and retired after contradiction; replacing an **explicitly delivered** self proposition changed 6/8 initial paired choices. Adding the native self line to episodes changed only 1/16 choices, and the purported memory-ablation arm accidentally duplicated the episode-only prompts. These results support a limited *self-statement sensitivity*, **not** a learned personal preference, autonomous individuality or a creativity benefit. [Detailed results and provenance](experiments/genesis/SELF_BEHAVIOR_RESULTS.md).

**Contextual transfer update — October 2026.** Across four sequential synthetic Grok 4.3 pilots, contextual self-beliefs were formed, qualified and used at decision time. The v2/v3 controls matched SelMem ON (both 16/16); in the subsequent **v4 long-history pilot**, ON was **16/16** versus **10/16** for a query-independent frequency summary and **10/16** for lexical retrieval, with 14 relevant actions among 240 distractors and a 64-word memory ceiling. This is an **exploratory, two-domain advantage over those specific deterministic baselines**, not a validated gain over semantic RAG, LLM summarization or full-history access. The 12-domain preregistered criterion was **not met** because only two domains ran in that original pilot. [Contextual transfer results and archive hashes](experiments/genesis/CONTEXTUAL_TRANSFER_RESULTS.md).

**Contextual budget sweep — October 28, 2026.** A separate, adaptive two-domain Grok 4.3 follow-up recorded **512/512 valid choices** across 8/16/32/64-word caps and 240/960 distractors (`reasoning_effort=none` logged). At 32 and 64 words, context-qualified SelMem reached **28/32 (87.5%)**, versus lexical retrieval **18/32 (56.25%)**, frequency summary **14/32 (43.75%)**, and scope-disabled memory **12/32 (37.5%)**. At eight words, ON was only 17/32 (53.13%); its predicted action was cut off by the text prefix. Most methods delivered identical payloads across loads, so this is primarily **a compression/serialization and selector pilot**, not independent evidence of distractor robustness. Two synthetic domains and deterministic comparators do not establish general superiority. See [breaking-point results](experiments/genesis/CONTEXTUAL_BREAKING_RESULTS.md) and [compact aggregates](experiments/evidence/contextual/breaking-pilot-001/curve.csv).

**Manifest:** [WHITEPAPER.md](WHITEPAPER.md)\
**Layout:** [ARCHITECTURE.md](ARCHITECTURE.md) — encode / judge / night / snapshot.\
**Benches:** [experiments/RESULTS.md](experiments/RESULTS.md) — observations and limits. Replay from [experiments/README.md](experiments/README.md).\
**Knobs:** [PARAMETERS.md](PARAMETERS.md) — exploratory, not fitted.

**Experimental contextual self-beliefs:** With explicit opt-in, attested circumstances can propose narrower, provenance-linked *action hypotheses*. Prospective observations must qualify them before delivery to the actor; confounded evidence leads to abstention. A synthetic v4 pilot observed better decisions than frequency-summary and lexical-retrieval controls under heavy distraction, **not** an established general performance advantage or autonomous preference learning. See [contextual self](docs/TRACE_SELF_MODEL.md), [contextual transfer results](experiments/genesis/CONTEXTUAL_TRANSFER_RESULTS.md) and the later [native delivery ablations](experiments/genesis/CAUSAL_NATIVE_RESULTS.md).

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

`selmem-genesis positive` qualifies the decision model against a faithful history; `learning` tests observation support OFF/ON. The same executable also provides sleep, creative preview, transfer and longitudinal controls. Separate Rust binaries retain fixed-selection replays and memory-influence diagnostics. Runtime configuration comes from `.selmem`, `--config` or `SELMEM_CONFIG`.

`positive-001`: Grok 48/48 oracle choices; GPT-4o-mini 28/48. `evidence-001`: native and strict recall match the oracle on 8/16 with support OFF and 16/16 with support ON. The effect comes from explicit observation counts, not demonstrated reconstruction superiority, singularity or creativity. One history pair is exploratory, not population replication.

```bash
./run.sh run --release --bin selmem-genesis -- --help
bash experiments/genesis/run-positive-control.sh experiments/genesis/results/positive-new
bash experiments/genesis/run-evidence-learning.sh \
  experiments/genesis/results/positive-new/grok \
  experiments/genesis/results/evidence-new
```

Real runs make paid API calls. Scripts print progress and cap HTTP attempts including retries. Choose fresh output paths. `--mock` tests plumbing only and cannot qualify a model. See the [protocol](experiments/genesis/protocol.md) for controls, probe isolation and limitations.

### GENESIS contextual transfer (v1–v4, October 2026)

The four synthetic Grok 4.3 pilots are recorded in [the contextual-transfer result ledger](experiments/genesis/CONTEXTUAL_TRANSFER_RESULTS.md). In v2 and v3, SelMem ON **did not** outperform the better memory controls (16/16 each); these negative comparisons motivated the stronger v4 distraction test. V4 produced 16/16 ON versus 10/16 each for a generic frequency summary and lexical retrieval over the full stream (96 model responses, two independent domains, 254 observations per history, 64-word cap). The 12-domain confirmation threshold is still unmet. A subjective preference or durable individuality was **not** demonstrated.

Run the v4 Rust harness with `./run.sh run --release --bin selmem-contextual-longlife -- --out experiments/genesis/results/contextual-v4-new --pairs 2 --distractors 240 --memory-words 64` (offline) and add `--actor --provider grok --model grok-4.3 --max-http-calls 96` for the paid pilot. The output directory must be new. [Protocol and controls](experiments/genesis/CONTEXTUAL_LONGLIFE_V4.md).

### Creativity publication

The guarded preview freshly encodes 788 source turns, applies twenty sleep cycles across two histories, and collects 60 proposals under five conditions. The recorded run completed in 887 HTTP attempts. It uses task-blind guarded retelling of task-selected gists, with matched direct generation; it does not invoke response-choice selection or deliver the reading profile and axioms.

```bash
bash experiments/genesis/run-creative-preview-guarded.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-new --prepare-only
# Inspect the design, then start paid collection:
bash experiments/genesis/run-creative-preview-guarded.sh \
  experiments/genesis/creative-preview.json \
  experiments/genesis/results/creative-preview-new --resume
```

Earlier 72-proposal replay lots and a frozen-pool response-selection diagnostic are retained as separate interventions. Both long creative-life plans remain available. [Commands and controls](experiments/README.md), [results and limitations](experiments/CREATIVITY_RESULTS.md), [structured context defaults](docs/MEMORY_CONTEXT_DEFAULTS.md), [response selection](docs/RESPONSE_SELECTION.md).

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

## Outside the demonstrated claim

Do not read these as results. They are outside the demonstrated result.

- Personality, character, “Claire / Silas as people.”
- Creativity, originality, intelligence, “more human.”
- General superiority over semantic RAG, strong summaries or full history; v4 beat the tested deterministic selectors in a two-domain pilot.
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

[GENESIS](experiments/genesis/README.md) retains qualification, cumulative learning, sleep, creative preview/replay and longitudinal creative-life protocols. Recorded creative collections are available; the long economy/full plans remain for future runs. Probes use isolated states. Durable singularity, a useful creativity advantage and causal attribution of original mechanisms remain unproved. See the [findings](experiments/CREATIVITY_RESULTS.md) and [reproduction policy](experiments/REPRODUCIBILITY.md).

### Contextual self model

Beliefs, aspirations and commitments can live as typed memory traces. The prior
self view influences admission; observed outcomes can revise it during sleep.
See [architecture, Rust API and limitations](docs/TRACE_SELF_MODEL.md).

The Grok v3 pilot completed 128 choices across two scripted histories. The
contextual self changed which routine notes were admitted; evolving beliefs
retired after contradiction and new actions supported later beliefs. Decision
effects were small: full loop vs memory-only differed on 2/32 matched pairs,
and full loop vs evolution without admission feedback on 0/32. This supports
the admission/revision mechanism in this fixture, without establishing durable
behavioral individuality or a creativity advantage. [Results and provenance](experiments/genesis/SELF_MODEL_RESULTS.md),
[replay protocol](experiments/genesis/SELF_MODEL_PILOT_V3.md).

### GENESIS self-model-to-decision diagnostics (October 2026)

[Self-Bridge v4](experiments/genesis/SELF_BRIDGE.md) found a strong first-option
bias despite retrieved episodes and explicit self statements (84/84 choices;
positive actor-sensitivity control 12/12). The independent prompt-only
[Self-Preference](experiments/genesis/SELF_PREFERENCE.md) diagnostic found
6/8 decisions consistent with a past-action report versus 8/8 with an
explicit autobiographical preference, but this does not show SelMem created
one. In [Preference Emergence](experiments/genesis/SELF_EMERGENCE.md), the organ
formed evidence-guarded *action-pattern beliefs* from scripted histories and
retired them after three counterexamples (certainty ~0.35 → ~0.05). On 96
matched Grok decisions, substituting a self proposition changed 6/8 initial
choices but 0/8 after retirement; adding one's own self model to the same
episodes changed 1/16. **The supposed memory-ablation prompts were identical
to the episode-only prompts and cannot support an ablation claim.**

[Measurements, limitations, archive hashes and next tests](experiments/genesis/SELF_BEHAVIOR_RESULTS.md).
No persistent preference reversal or autonomous behavioral individuality has
been established.

### Experimental normative memory

An opt-in Rust API attaches caller-authorized permission, prohibition and verification
anchors to living traces, preserves explicit dependencies and composes bounded rule
families without archive access. Ordinary recall remains unchanged. The host must
authenticate sources and validate annotations; this does not enforce model outputs.
See [the API and limits](docs/NORMATIVE_MEMORY.md) and the
[bounded four-condition pilot](experiments/genesis/NORMATIVE_PILOT_V10.md).
