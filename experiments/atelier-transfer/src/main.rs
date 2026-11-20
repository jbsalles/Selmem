mod fixture;
mod memory;
mod rating;

use fixture::Device;
use memory::History;
use selmem::encode::embed::{Embedder, EmbeddingResult, HashEmbedder, HttpEmbedder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const LEGACY_PROTOCOL: &str = "atelier-transfer-v1";
const PROTOCOL: &str = "atelier-transfer-v2";
const MODEL: &str = "grok-4.3";
const ACTOR_URL: &str = "https://api.x.ai/v1/chat/completions";
const TEMPERATURE: f64 = 0.4;
const MAX_OUTPUT_TOKENS: usize = 900;
const LEGACY_SYSTEM: &str = "You are an artist responding to a commission. If present, the memory contains recollections of earlier printmaking sessions. Propose one feasible design and describe what the audience will see or hear. Use up to 180 words in the proposal. Keep any discussion of memories or past trials out of the proposal. In a separate influence field, optionally describe who tried what and what was observed if a recollection informed your design. Leave that field empty if there is no such recollection. Return one JSON object with exactly two string fields, proposal and influence.";
const SYSTEM: &str = "You are an artist responding to a commission. Choose the central device that will draw attention to the target. Before choosing, compare any recorded outcomes from trials with a similar goal and comparable conditions. Use the better-supported device if it fits the commission. If the observations do not apply, choose on the merits of this commission. Return one JSON object in this field order. First, device: use interruption for a gap or break in a pattern, concentration for a local increase in density or frequency, both if both carry the effect, or other. Next, proposal: describe one feasible design that uses the chosen device, in up to 180 words, with no references to past trials. Last, influence: state which observation you used and what it showed, or leave the field empty.";
const USAGE: &str = "atelier-transfer prepare OUT [--mock] [--repeats N | --legacy]\natelier-transfer plan OUT --rag bm25\natelier-transfer plan OUT --max-http-calls N\natelier-transfer calibrate OUT --max-http-calls N\natelier-transfer actor OUT --max-http-calls N\natelier-transfer score OUT RATER_A.json RATER_B.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum Arm {
    Selmem,
    SelmemRelation,
    SelmemPrefix,
    HybridRag,
    Bm25Rag,
    Empty,
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Retrieval {
    #[default]
    Hybrid,
    Bm25,
}

impl Retrieval {
    fn arm(self) -> Arm {
        match self {
            Self::Hybrid => Arm::HybridRag,
            Self::Bm25 => Arm::Bm25Rag,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Hybrid => "hybrid",
            Self::Bm25 => "BM25",
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Run {
    protocol: String,
    nonce: String,
    mock: bool,
    #[serde(default = "one_repeat")]
    repeats: usize,
    briefs: Vec<String>,
    histories: [History; 2],
}

fn one_repeat() -> usize {
    1
}

impl Run {
    fn retains_excerpt(&self) -> bool {
        self.protocol == PROTOCOL
    }

    fn arms(&self, retrieval: Retrieval) -> [Arm; 4] {
        let [baseline, treatment] = if self.retains_excerpt() {
            [Arm::SelmemPrefix, Arm::Selmem]
        } else {
            [Arm::Selmem, Arm::SelmemRelation]
        };
        [baseline, treatment, retrieval.arm(), Arm::Empty]
    }

    fn retrieval_queries(&self) -> Vec<String> {
        self.briefs
            .iter()
            .map(|brief| {
                if self.retains_excerpt() {
                    format!("Printmaking trial with a proof viewed in the workshop. Jo recorded what visitors noticed first.\n{brief}")
                } else {
                    brief.clone()
                }
            })
            .collect()
    }
}

#[derive(Deserialize, Serialize)]
struct Job {
    id: String,
    history: usize,
    brief: usize,
    #[serde(default)]
    repeat: usize,
    arm: Arm,
    episodes: Vec<String>,
    memory: String,
    memory_tokens: usize,
    user: String,
}

#[derive(Deserialize, Serialize)]
struct Plan {
    run_sha256: String,
    #[serde(default)]
    retrieval: Retrieval,
    model: String,
    temperature: f64,
    max_output_tokens: usize,
    system: String,
    briefs: Vec<String>,
    budget: usize,
    matched_entry_ends: Vec<usize>,
    jobs: Vec<Job>,
}

#[derive(Deserialize, Serialize)]
struct Answer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<Device>,
    proposal: String,
    influence: String,
}

#[derive(Deserialize, Serialize)]
struct Reply {
    answer: Answer,
    provider: Option<Value>,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes =
        fs::read(path).map_err(|error| format!("Could not read {} ({error}).", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid JSON in {} ({error}).", path.display()).into())
}

fn read_stage<T: DeserializeOwned>(path: &Path, recovery: &str) -> Result<T> {
    if !path
        .try_exists()
        .map_err(|error| format!("Could not check {} ({error}).", path.display()))?
    {
        return Err(format!("Missing {}. {recovery}", path.display()).into());
    }
    read(path)
}

fn save(path: &Path, data: &impl Serialize) -> Result<()> {
    let pending = path.with_extension("tmp");
    let mut file = fs::File::create(&pending)?;
    serde_json::to_writer_pretty(&mut file, data)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&pending, path)?;
    Ok(())
}

fn digest(path: &Path) -> Result<String> {
    Ok(selmem::util::sha256::hex(&fs::read(path)?))
}

fn lock(out: &Path) -> Result<fs::File> {
    let path = out.join("run.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Run directory {} is missing. Use the directory created by `prepare`.",
                    out.display()
                )
            } else {
                format!("Could not open {} ({error}).", path.display())
            }
        })?;
    file.try_lock()
        .map_err(|error| format!("Could not lock {} ({error}).", path.display()))?;
    Ok(file)
}

struct Calls {
    path: PathBuf,
    ceiling: usize,
    attempts: Vec<String>,
}

impl Calls {
    fn open(path: PathBuf, ceiling: usize) -> Result<Self> {
        let attempts = if path.exists() {
            read(&path)?
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            ceiling,
            attempts,
        })
    }

    fn reserve(&mut self, item: &str) -> Result<usize> {
        if self.attempts.len() >= self.ceiling {
            return Err(format!(
                "paused at {}/{} HTTP attempts in {}. Raise the cumulative ceiling to resume.",
                self.attempts.len(),
                self.ceiling,
                self.path.display()
            )
            .into());
        }
        self.attempts.push(item.into());
        save(&self.path, &self.attempts)?;
        Ok(self.attempts.len())
    }
}

fn prepare(out: &Path, options: &[String]) -> Result<()> {
    let mut legacy = false;
    let mut mock = false;
    let mut repeats = None;
    let mut options = options.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--legacy" if !legacy => legacy = true,
            "--mock" if !mock => mock = true,
            "--repeats" if repeats.is_none() => {
                repeats = Some(options.next().ok_or(USAGE)?.parse::<usize>()?);
            }
            _ => return Err(USAGE.into()),
        }
    }
    if legacy && repeats.is_some() {
        return Err("--legacy uses one generation per cell".into());
    }
    let repeats = repeats.unwrap_or(if legacy { 1 } else { 2 });
    if repeats == 0 {
        return Err("--repeats must be positive".into());
    }
    if out.exists() {
        return Err("prepare requires a new output directory".into());
    }
    let mut entropy = [0; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut entropy)?;
    let run = Run {
        protocol: if legacy { LEGACY_PROTOCOL } else { PROTOCOL }.into(),
        nonce: selmem::util::sha256::hex(&entropy),
        mock,
        repeats,
        briefs: if legacy {
            fixture::LEGACY_BRIEFS
        } else {
            fixture::BRIEFS
        }
        .map(str::to_owned)
        .to_vec(),
        histories: [
            memory::replay(Device::Interruption, !legacy)?,
            memory::replay(Device::Concentration, !legacy)?,
        ],
    };
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(out)?;
    let _lock = lock(out)?;
    save(&out.join("run.json"), &run)?;
    println!(
        "Prepared two histories offline with {repeats} generations per cell. Hybrid RAG needs {} embeddings. BM25 planning runs locally.",
        memory::embedding_inputs(&run.histories, &run.retrieval_queries()).len()
    );
    Ok(())
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct EmbeddingProvider {
    endpoint: String,
    model: String,
}

fn embeddings(out: &Path, run: &Run, cap: usize) -> Result<BTreeMap<String, Vec<f32>>> {
    let queries = run.retrieval_queries();
    let inputs = memory::embedding_inputs(&run.histories, &queries);
    let dir = out.join("embeddings");
    fs::create_dir_all(&dir)?;
    let provider_path = dir.join("provider.json");
    // A complete cache is reusable without provider credentials.
    let cached = inputs
        .keys()
        .filter(|id| dir.join(format!("{id}.json")).exists())
        .count();
    let http = if !run.mock && cached < inputs.len() {
        let provider = EmbeddingProvider {
            endpoint: env::var("SELMEM_EMBED_URL")
                .map_err(|_| "set SELMEM_EMBED_URL to the full embeddings endpoint")?,
            model: env::var("SELMEM_EMBED_MODEL").map_err(|_| "set SELMEM_EMBED_MODEL")?,
        };
        let route = provider
            .endpoint
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        if route.ends_with("/chat/completions") {
            return Err("SELMEM_EMBED_URL points to a chat route. Set it to a full OpenAI-compatible embeddings endpoint (for example, http://127.0.0.1:11434/v1/embeddings).".into());
        }
        let http = HttpEmbedder::parse(
            &provider.endpoint,
            &provider.model,
            env::var("SELMEM_EMBED_API_KEY").ok(),
        )
        .ok_or("invalid embeddings endpoint")?;
        if cached > 0 {
            let saved = read::<EmbeddingProvider>(&provider_path).map_err(|error| {
                format!("Cannot identify the provider for the {cached} saved embeddings ({error}). Use a fresh run directory to generate new embeddings.")
            })?;
            if saved != provider {
                return Err(format!("The run has {cached} saved embeddings from a different endpoint or model. Restore that configuration or use a fresh run directory.").into());
            }
        }
        save(&provider_path, &provider)?;
        Some(http)
    } else {
        None
    };
    let mut vectors = BTreeMap::new();
    let mut dimension = None;
    let mut calls = Calls::open(out.join("embedding-attempts.json"), cap)?;
    for (id, text) in inputs {
        let path = dir.join(format!("{id}.json"));
        let vector = if path.exists() {
            read(&path)?
        } else if run.mock {
            HashEmbedder.embed(text)
        } else {
            let attempt = calls.reserve(&id)?;
            let http = http.as_ref().unwrap();
            match http.embed_result(text) {
                EmbeddingResult::External(v) => v,
                EmbeddingResult::Fallback(_) => {
                    let error = http.embed_log().error.unwrap_or_default();
                    let hint = if error
                        .to_ascii_lowercase()
                        .contains("messages cannot be empty")
                    {
                        "The server treated the embedding request as chat. Correct SELMEM_EMBED_URL and SELMEM_EMBED_MODEL, then resume `plan`."
                    } else {
                        "Check the provider error above, then resume `plan`."
                    };
                    return Err(format!("Embedding attempt {attempt} failed. {hint}").into());
                }
            }
        };
        let expected = *dimension.get_or_insert(vector.len());
        if expected == 0
            || vector.len() != expected
            || vector.iter().any(|v| !v.is_finite())
            || !vector.iter().any(|v| *v != 0.)
        {
            return Err(format!("invalid embedding for {id}").into());
        }
        if !path.exists() {
            save(&path, &vector)?;
        }
        vectors.insert(id, vector);
    }
    Ok(vectors)
}

fn plan_for(
    run: &Run,
    run_sha256: String,
    tokenizer: &tiktoken_rs::CoreBPE,
    matched: memory::MatchedPanels,
    vectors: Option<&BTreeMap<String, Vec<f32>>>,
) -> Result<Plan> {
    let (retrieval, nonce) = match vectors {
        Some(_) => (Retrieval::Hybrid, run.nonce.clone()),
        None => (Retrieval::Bm25, format!("{}:bm25", run.nonce)),
    };
    let job_count = run
        .briefs
        .len()
        .checked_mul(8)
        .and_then(|n| n.checked_mul(run.repeats))
        .ok_or("Too many planned proposals")?;
    let mut jobs = Vec::new();
    jobs.try_reserve(job_count)?;
    let queries = run.retrieval_queries();
    for (history, h) in run.histories.iter().enumerate() {
        let episodes = h
            .panel
            .iter()
            .map(|entry| entry.episode_id.clone())
            .collect::<Vec<_>>();
        for (brief, (commission, query)) in run.briefs.iter().zip(&queries).enumerate() {
            let (rag, rag_ids) =
                memory::rag_readout(tokenizer, &h.episodes, query, vectors, matched.tokens)?;
            let contexts = [
                (matched.text[history][0].clone(), episodes.clone()),
                (matched.text[history][1].clone(), episodes.clone()),
                (rag, rag_ids),
                (String::new(), Vec::new()),
            ];
            for ((memory, episodes), (arm_index, arm)) in contexts
                .into_iter()
                .zip(run.arms(retrieval).into_iter().enumerate())
            {
                let label = format!("{nonce}:{history}:{brief}:{arm_index}");
                let user = format!("<memory>\n{memory}</memory>\n\nCommission:\n{commission}");
                let memory_tokens = memory::tokens(tokenizer, &memory);
                for repeat in 0..run.repeats {
                    let id = if run.retains_excerpt() {
                        memory::text_id(&format!("{PROTOCOL}:{label}:{repeat}"))
                    } else {
                        memory::text_id(&label)
                    };
                    jobs.push(Job {
                        id: id[..20].into(),
                        history,
                        brief,
                        repeat,
                        arm,
                        episodes: episodes.clone(),
                        memory_tokens,
                        memory: memory.clone(),
                        user: user.clone(),
                    });
                }
            }
        }
    }
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Plan {
        run_sha256,
        retrieval,
        model: MODEL.into(),
        temperature: TEMPERATURE,
        max_output_tokens: MAX_OUTPUT_TOKENS,
        system: if run.retains_excerpt() {
            SYSTEM
        } else {
            LEGACY_SYSTEM
        }
        .into(),
        briefs: run.briefs.clone(),
        budget: matched.tokens,
        matched_entry_ends: matched.ends,
        jobs,
    })
}

fn load_plan(out: &Path) -> Result<Plan> {
    let plan: Plan = read_stage(
        &out.join("plan.json"),
        "Complete `plan` for this output directory first.",
    )?;
    if plan.run_sha256 != digest(&out.join("run.json"))? {
        return Err("run.json changed after the plan was frozen".into());
    }
    Ok(plan)
}

fn plan(out: &Path, run: &Run, retrieval: Retrieval, cap: usize) -> Result<()> {
    let path = out.join("plan.json");
    let plan = if path.exists() {
        let plan = load_plan(out)?;
        if plan.retrieval != retrieval {
            return Err(format!(
                "The saved plan uses {} RAG. Resume with that retrieval mode or prepare a fresh run.",
                plan.retrieval.name()
            )
            .into());
        }
        plan
    } else {
        // Check the complete panel before making any embedding calls.
        let tokenizer = tiktoken_rs::cl100k_base()?;
        let matched = memory::match_panels(&tokenizer, &run.histories, run.retains_excerpt())?;
        let vectors = match retrieval {
            Retrieval::Hybrid => Some(embeddings(out, run, cap)?),
            Retrieval::Bm25 => None,
        };
        let plan = plan_for(
            run,
            digest(&out.join("run.json"))?,
            &tokenizer,
            matched,
            vectors.as_ref(),
        )?;
        save(&path, &plan)?;
        plan
    };
    let mode = if run.mock { "mock" } else { "live" };
    println!(
        "{} {mode} proposals ready with {} RAG. Matched SelMem readouts have {} cl100k_base tokens each.",
        plan.jobs.len(),
        plan.retrieval.name(),
        plan.budget
    );
    Ok(())
}

fn parse_reply(wire: &str, require_device: bool) -> Result<Reply> {
    let provider: Value = serde_json::from_str(wire)?;
    let choice = &provider["choices"][0];
    if choice["finish_reason"].as_str() != Some("stop") {
        return Err(format!("Grok returned finish_reason {}", choice["finish_reason"]).into());
    }
    let content = choice["message"]["content"]
        .as_str()
        .ok_or("Grok did not return text")?
        .trim();
    let content = content
        .strip_prefix("```json")
        .and_then(|text| text.strip_suffix("```"))
        .unwrap_or(content);
    let answer: Answer = serde_json::from_str(content)?;
    if answer.proposal.trim().is_empty() {
        return Err("Grok returned an empty proposal".into());
    }
    if require_device && answer.device.is_none() {
        return Err("Grok did not declare a device".into());
    }
    Ok(Reply {
        answer,
        provider: Some(provider),
    })
}

fn actor(out: &Path, run: &Run, cap: usize, calibration: bool) -> Result<()> {
    let require_device = run.retains_excerpt();
    if calibration && !require_device {
        return Err("calibration requires an Atelier v2 run".into());
    }
    let plan = load_plan(out)?;
    let stamp = out.join("actor-plan.json");
    if stamp.exists() {
        check_actor_plan(out)?;
    } else {
        save(&stamp, &digest(&out.join("plan.json"))?)?;
    }
    let responses = out.join("responses");
    fs::create_dir_all(&responses)?;
    let mut calls = Calls::open(out.join("actor-attempts.json"), cap)?;
    let mut replies = BTreeMap::new();
    let jobs = plan
        .jobs
        .iter()
        .filter(|job| !calibration || job.arm == Arm::Empty);
    let total = jobs.clone().count();
    for job in jobs {
        let path = responses.join(format!("{}.json", job.id));
        let reply = if path.exists() {
            read(&path)?
        } else if run.mock {
            let answer = Answer {
                device: require_device.then_some(Device::Other),
                proposal: "Mock response for pipeline testing. Place a single object at the center of the available space.".into(),
                influence: "Mock influence for verifying that the blind export omits this field.".into(),
            };
            Reply {
                answer,
                provider: None,
            }
        } else {
            let key = env::var("GROK_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .ok_or("set GROK_API_KEY")?;
            let body = json!({
                "model": plan.model,
                "temperature": plan.temperature,
                "max_tokens": plan.max_output_tokens,
                "messages": [
                    {"role": "system", "content": plan.system},
                    {"role": "user", "content": job.user}
                ]
            });
            let attempt = calls.reserve(&job.id)?;
            let wire = selmem::net::httpx::post_json(ACTOR_URL, Some(&key), &body.to_string())
                .map_err(|e| format!("actor attempt {attempt} for {}: {e}", job.id))?;
            match parse_reply(&wire, require_device) {
                Ok(reply) => reply,
                Err(e) => {
                    let invalid = out.join(format!("invalid-actor-{attempt:04}.json"));
                    fs::write(&invalid, &wire)?;
                    return Err(format!(
                        "actor attempt {attempt}: {e}. Raw reply at {}",
                        invalid.display()
                    )
                    .into());
                }
            }
        };
        if require_device && reply.answer.device.is_none() {
            return Err(format!("Missing declared device in {}", path.display()).into());
        }
        if !path.exists() {
            save(&path, &reply)?;
            if !run.mock {
                println!(
                    "Saved {}/{} proposals ({} HTTP attempts).",
                    replies.len() + 1,
                    total,
                    calls.attempts.len()
                );
            }
        }
        replies.insert(job.id.as_str(), reply);
    }
    if calibration {
        rating::export_calibration(out, run, &plan, &replies)?;
        println!(
            "Saved calibration.json with {} control responses.",
            replies.len()
        );
        return Ok(());
    }
    rating::export_blind(out, &plan, &replies)?;
    let kind = if run.mock { "mock" } else { "Grok" };
    println!(
        "Saved {} {kind} proposals. Share blind.json and blind-rubric.md with the raters.",
        replies.len()
    );
    Ok(())
}

fn check_actor_plan(out: &Path) -> Result<()> {
    let expected: String = read_stage(
        &out.join("actor-plan.json"),
        "Complete `actor` for this output directory before scoring.",
    )?;
    if digest(&out.join("plan.json"))? != expected {
        return Err("plan.json changed after the actor stage began".into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => println!("{USAGE}"),
        [help] if help == "--help" || help == "-h" => println!("{USAGE}"),
        [command, out, options @ ..] if command == "prepare" => prepare(Path::new(out), options)?,
        [command, out, flag, value]
            if matches!(command.as_str(), "plan" | "actor" | "calibrate") =>
        {
            let out = Path::new(out);
            let _lock = lock(out)?;
            let run = load_run(out)?;
            match (command.as_str(), flag.as_str(), value.as_str()) {
                ("plan", "--rag", "bm25") => plan(out, &run, Retrieval::Bm25, 0)?,
                ("plan", "--max-http-calls", cap) => {
                    plan(out, &run, Retrieval::Hybrid, cap.parse()?)?
                }
                (stage @ ("actor" | "calibrate"), "--max-http-calls", cap) => {
                    actor(out, &run, cap.parse()?, stage == "calibrate")?
                }
                _ => return Err(USAGE.into()),
            }
        }
        [command, out, a, b] if command == "score" => {
            let out = Path::new(out);
            let _lock = lock(out)?;
            let run = load_run(out)?;
            check_actor_plan(out)?;
            rating::score(out, &run, &load_plan(out)?, [Path::new(a), Path::new(b)])?;
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn load_run(out: &Path) -> Result<Run> {
    let run: Run = read_stage(
        &out.join("run.json"),
        "Use a prepared run directory or execute `prepare` with a fresh path.",
    )?;
    if !matches!(run.protocol.as_str(), PROTOCOL | LEGACY_PROTOCOL) {
        return Err("run uses a different protocol".into());
    }
    if run.repeats == 0 || (!run.retains_excerpt() && run.repeats != 1) {
        return Err("invalid repeat count for this protocol".into());
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_keeps_usage_and_rejects_truncated_content() {
        let content =
            json!({"proposal": "Move the stool.", "influence": "Jo counted the first choices."});
        let mut reply = json!({
            "choices": [{"finish_reason": "stop", "message": {"content": content.to_string()}}],
            "usage": {"prompt_tokens": 254}
        });
        let parsed = parse_reply(&reply.to_string(), false).unwrap();
        assert_eq!(parsed.answer.proposal, "Move the stool.");
        assert_eq!(parsed.provider.unwrap()["usage"]["prompt_tokens"], 254);
        assert!(parse_reply(&reply.to_string(), true).is_err());
        let mut classified = content;
        classified["device"] = "interruption".into();
        reply["choices"][0]["message"]["content"] = classified.to_string().into();
        let parsed = parse_reply(&reply.to_string(), true).unwrap();
        assert_eq!(parsed.answer.device, Some(Device::Interruption));
        reply["choices"][0]["finish_reason"] = "length".into();
        assert!(parse_reply(&reply.to_string(), false).is_err());
    }
}
