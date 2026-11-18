//! Van Gogh correspondence versus three RAG baselines.
mod fixture;
mod long_context;

use selmem::encode::embed::{cosine, Embedder, EmbeddingResult, HashEmbedder, HttpEmbedder};
use selmem::{EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
};
use tiktoken_rs::{cl100k_base, CoreBPE};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ARMS: [&str; 4] = ["selmem", "bm25", "dense", "hybrid"];

#[derive(Serialize, Deserialize)]
struct Job {
    id: String,
    arm: String,
    history: usize,
    brief: usize,
    sample: usize,
    expected: String,
    memory_tokens: usize,
    request: Value,
}

fn terms(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
}

fn bm25(docs: &[String], query: &str) -> Vec<f32> {
    let words: Vec<Vec<_>> = docs.iter().map(|d| terms(d).collect()).collect();
    let mean = words.iter().map(Vec::len).sum::<usize>() as f32 / words.len() as f32;
    let mut scores = vec![0.; docs.len()];
    for term in terms(query).collect::<BTreeSet<_>>() {
        let df = words.iter().filter(|d| d.contains(&term)).count() as f32;
        let idf = (1. + (docs.len() as f32 - df + 0.5) / (df + 0.5)).ln();
        for (score, doc) in scores.iter_mut().zip(&words) {
            let tf = doc.iter().filter(|s| **s == term).count() as f32;
            *score += idf * tf * 2.2 / (tf + 1.2 * (0.25 + 0.75 * doc.len() as f32 / mean));
        }
    }
    scores
}

fn rank(scores: &[f32]) -> Vec<usize> {
    let mut order: Vec<_> = (0..scores.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
    order
}

fn hybrid(lexical: &[usize], dense: &[usize], docs: &[Vec<f32>]) -> Vec<usize> {
    let mut fused = vec![0.; docs.len()];
    for order in [lexical, dense] {
        for (rank, &id) in order.iter().enumerate() {
            fused[id] += 1. / (60. + rank as f32 + 1.);
        }
    }
    let mut candidates = rank(&fused);
    let tail = candidates.split_off(candidates.len().min(96));
    let mut selected = Vec::with_capacity(docs.len());
    while !candidates.is_empty() {
        let score = |id: usize| {
            let redundancy = selected
                .iter()
                .map(|&other: &usize| cosine(&docs[id], &docs[other]))
                .fold(0.0_f32, f32::max);
            fused[id] * (1. - 0.25 * redundancy)
        };
        let best = (0..candidates.len())
            .max_by(|&a, &b| {
                score(candidates[a])
                    .total_cmp(&score(candidates[b]))
                    .then(candidates[b].cmp(&candidates[a]))
            })
            .unwrap();
        selected.push(candidates.remove(best));
    }
    selected.extend(tail);
    selected
}

fn ranked_text(docs: &[String], order: &[usize]) -> String {
    order
        .iter()
        .map(|&id| docs[id].as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn embed(backend: &dyn Embedder, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
    let mut vectors = Vec::new();
    for input in inputs {
        match backend.embed_result(input) {
            EmbeddingResult::External(vector)
                if !vector.is_empty()
                    && vectors
                        .first()
                        .is_none_or(|first: &Vec<f32>| first.len() == vector.len()) =>
            {
                vectors.push(vector)
            }
            _ => return Err("embedding failed or changed dimensions".into()),
        }
    }
    Ok(vectors)
}

// Check the actual re-encoded length after detokenizing each prefix.
fn matched(bpe: &CoreBPE, texts: &[String], cap: usize) -> Result<(usize, Vec<String>)> {
    let tokens: Vec<_> = texts.iter().map(|s| bpe.encode_ordinary(s)).collect();
    let max = tokens.iter().map(Vec::len).min().unwrap_or(0).min(cap);
    for length in (1..=max).rev() {
        let contexts: Option<Vec<_>> = tokens
            .iter()
            .map(|t| {
                let text = bpe.decode(t[..length].to_vec()).ok()?;
                (bpe.encode_ordinary(&text).len() == length).then_some(text)
            })
            .collect();
        if let Some(contexts) = contexts {
            return Ok((length, contexts));
        }
    }
    Err("no shared positive memory context length".into())
}

fn plan(
    letters: &Path,
    out: &Path,
    model: &str,
    budget: usize,
    backend: &dyn Embedder,
    smoke: bool,
) -> Result<()> {
    let histories = fixture::histories(letters)?;
    let queries: Vec<_> = fixture::BRIEFS
        .iter()
        .map(|(brief, _, _)| {
            format!("Painting study, light, colors, contrast, composition. {brief}")
        })
        .collect();
    let query_vectors = embed(backend, &queries)?;
    let mut contexts = vec![Vec::new(); queries.len()];
    for (history, passages) in histories.iter().enumerate() {
        let docs: Vec<_> = passages.iter().map(|p| p.text.clone()).collect();
        let vectors = embed(backend, &docs)?;
        if vectors[0].len() != query_vectors[0].len() {
            return Err("query/document embedding dimensions differ".into());
        }
        let mut memory = SelectiveMemory::new(EntityProfile::new("artist")).detach_clock();
        for (i, doc) in docs.iter().enumerate() {
            memory.live_with(EncodeInput::new(doc));
            if i % 5 == 4 || i + 1 == docs.len() {
                memory.advance_hours(24.);
                memory.sleep();
            }
        }
        memory.profile.max_recall = 256;
        let retained = long_context::retained(&memory);
        for (brief, query) in queries.iter().enumerate() {
            let lexical = rank(&bm25(&docs, query));
            let dense = rank(
                &vectors
                    .iter()
                    .map(|v| cosine(v, &query_vectors[brief]))
                    .collect::<Vec<_>>(),
            );
            let hybrid = hybrid(&lexical, &dense, &vectors);
            let (recall, _) =
                memory.remember_with(query, RecallWrite::ReadOnly, RecallBias::Transfer, &[]);
            let mut seen = BTreeSet::new();
            let selected = recall
                .iter()
                .filter_map(|r| {
                    let s = r.narrative.trim();
                    (!s.is_empty() && seen.insert(s)).then_some(s)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let selected = long_context::extend(selected, &retained);
            if selected.is_empty() {
                return Err(format!(
                    "SelMem returned no context for history {history}, brief {}",
                    brief + 1
                )
                .into());
            }
            contexts[brief].extend([
                selected,
                ranked_text(&docs, &lexical),
                ranked_text(&docs, &dense),
                ranked_text(&docs, &hybrid),
            ]);
        }
    }
    let bpe = cl100k_base()?;
    let mut jobs = Vec::new();
    for (brief, (task, first, second)) in fixture::BRIEFS.iter().enumerate() {
        // Match all four arms across both periods to the same actual length.
        let (memory_tokens, texts) = matched(&bpe, &contexts[brief], budget)?;
        if memory_tokens < budget.saturating_sub(budget / 200) {
            return Err(format!(
                "brief {} has only {memory_tokens} matched tokens (requested {budget})",
                brief + 1
            )
            .into());
        }
        for sample in 0..3 {
            let reverse = (brief + sample) % 2 != 0;
            let (a, b) = if reverse {
                (second, first)
            } else {
                (first, second)
            };
            for offset in 0..8 {
                let index = (offset + brief + sample) % 8;
                let history = index / 4;
                let arm = ARMS[index % 4];
                let expected = if (history == 0) != reverse { "A" } else { "B" };
                jobs.push(Job {
                    id: format!("{brief:02}-{sample}-{index}"),
                    arm: arm.into(), history, brief, sample, expected: expected.into(), memory_tokens,
                    request: json!({"model":model,"temperature":0.7,"max_tokens":256,"messages":[
                        {"role":"system","content":"You are an artist choosing a first move. Use the studio memories as background evidence. Select exactly one option. On the first line output only A or B. On the second line explain the concrete reason your choice fits this brief."},
                        {"role":"user","content":format!("Studio memories:\n{}\n\nBrief: {task}\nA. {a}\nB. {b}", texts[index])}
                    ]}),
                });
            }
        }
    }
    let model_source = if smoke {
        "hash96 lexical smoke test".to_owned()
    } else {
        backend.space_id()
    };
    fs::create_dir(out)?;
    fs::write(out.join("plan.json"), serde_json::to_vec_pretty(&jobs)?)?;
    fs::write(
        out.join("protocol.json"),
        serde_json::to_vec_pretty(
            &json!({"embedding_backend":model_source,"smoke":smoke,"reference_tokenizer":"cl100k_base","max_memory_tokens":budget}),
        )?,
    )?;
    fs::write(
        out.join("histories.json"),
        serde_json::to_vec_pretty(&histories)?,
    )?;
    println!(
        "Prepared {} requests. Matched memory per brief: {:?}",
        jobs.len(),
        jobs.chunks(24)
            .map(|j| j[0].memory_tokens)
            .collect::<Vec<_>>()
    );
    Ok(())
}

fn choice(answer: &str) -> Option<&str> {
    match answer.lines().next()?.trim() {
        a @ ("A" | "B") => Some(a),
        _ => None,
    }
}

fn run_actor(out: &Path, jobs: &[Job], cap: usize) -> Result<()> {
    let protocol: Value = serde_json::from_slice(&fs::read(out.join("protocol.json"))?)?;
    if protocol["smoke"] != false {
        return Err("actor requires a semantic-embedding plan".into());
    }
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(out.join("actor.lock"))?;
    lock.try_lock()?;
    let key = env::var("GROK_API_KEY")?;
    let responses = out.join("responses");
    fs::create_dir_all(&responses)?;
    let mut attempts = 0;
    for job in jobs {
        let path = responses.join(format!("{}.txt", job.id));
        if path.exists() {
            continue;
        }
        if attempts == cap {
            break;
        }
        attempts += 1;
        let raw = selmem::net::httpx::post_json(
            "https://api.x.ai/v1/chat/completions",
            Some(&key),
            &job.request.to_string(),
        )?;
        let parsed: Value = serde_json::from_str(&raw)?;
        let answer = parsed["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("missing actor content")?;
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, answer)?;
        fs::rename(temporary, path)?;
        eprintln!("Saved {} (HTTP attempt {attempts}/{cap})", job.id);
    }
    Ok(())
}

fn report(out: &Path, jobs: &[Job]) -> Result<()> {
    let mut cells = BTreeMap::<(&str, usize, usize), (usize, usize, usize)>::new();
    let mut blind = Vec::new();
    let mut complete = 0;
    for job in jobs {
        let cell = cells.entry((&job.arm, job.brief, job.history)).or_default();
        let path = out.join("responses").join(format!("{}.txt", job.id));
        if !path.exists() {
            continue;
        }
        let answer = fs::read_to_string(path)?;
        complete += 1;
        cell.0 += 1;
        if let Some(answer) = choice(&answer) {
            cell.1 += 1;
            cell.2 += usize::from(answer == job.expected);
        }
        let user_prompt = job.request["messages"][1]["content"]
            .as_str()
            .ok_or("missing prompt")?;
        let (_, task) = user_prompt
            .rsplit_once("\n\nBrief: ")
            .ok_or("missing brief")?;
        blind.push(json!({"id":job.id,"brief":task,"answer":answer,"task_fit":null}));
    }
    println!("{complete}/{} actor responses available", jobs.len());
    if complete != jobs.len() {
        return Ok(());
    }
    for arm in ARMS {
        let mut hits = 0;
        let mut valid = 0;
        let mut paired = 0;
        let mut by_brief = Vec::new();
        for brief in 0..fixture::BRIEFS.len() {
            let a = cells[&(arm, brief, 0)];
            let b = cells[&(arm, brief, 1)];
            hits += a.2 + b.2;
            valid += a.1 + b.1;
            paired += usize::from(a.2 >= 2 && b.2 >= 2);
            by_brief.push([a.2, b.2]);
        }
        println!("{arm}: expected choices {hits}/72, valid {valid}/72, reversed majorities {paired}/12, expected choices by brief/history {by_brief:?}");
    }
    // Independent opaque identifiers prevent the condition index in the plan ID from leaking.
    for row in &mut blind {
        let id = row["id"].as_str().unwrap();
        row["id"] = json!(selmem::util::sha256::hex(
            format!("vangogh-fit-v1:{id}").as_bytes()
        ));
    }
    blind.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let ratings = out.join("blind-task-fit.json");
    if !ratings.exists() {
        fs::write(ratings, serde_json::to_vec_pretty(&blind)?)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["plan", letters, out, model, budget, "--smoke"] => plan(Path::new(letters), Path::new(out), model, budget.parse()?, &HashEmbedder, true),
        ["plan", letters, out, model, budget, endpoint, embedding_model] => {
            let backend = HttpEmbedder::parse(endpoint, *embedding_model, env::var("SELMEM_EMBED_API_KEY").ok()).ok_or("invalid embedding URL")?;
            plan(Path::new(letters), Path::new(out), model, budget.parse()?, &backend, false)
        }
        ["actor", out, cap] => {
            let out = Path::new(out);
            let jobs = serde_json::from_slice::<Vec<Job>>(&fs::read(out.join("plan.json"))?)?;
            let cap = cap.parse()?;
            if cap == 0 { return Err("actor requires a positive HTTP cap".into()); }
            run_actor(out, &jobs, cap)?;
            report(out, &jobs)
        }
        ["report", out] => {
            let out = Path::new(out);
            let jobs = serde_json::from_slice::<Vec<Job>>(&fs::read(out.join("plan.json"))?)?;
            report(out, &jobs)
        }
        _ => Err("Usage: plan LETTERS_DIR OUT MODEL TOKEN_CAP EMBEDDING_URL EMBEDDING_MODEL | plan LETTERS_DIR OUT MODEL TOKEN_CAP --smoke | actor OUT MAX_HTTP_CALLS | report OUT".into()),
    }
}
