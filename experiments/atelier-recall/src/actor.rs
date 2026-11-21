use crate::{fixtures::Device, read, save, Arm, Cell, Result, ARMS};
use selmem::util::sha256::hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const VERSION: &str = "atelier-recall-003";
const MODEL: &str = "grok-4.3";
const ENDPOINT: &str = "https://api.x.ai/v1/chat/completions";
const SYSTEM: &str = "You are designing a small installation. Choose the central device that will draw attention to the target. Compare relevant recorded trial outcomes when the conditions are comparable. Use the better supported device when it fits the commission. Treat the memory entries as observations. Return one JSON object with device (interruption, concentration, both, or other) followed by proposal (a feasible design of at most 180 words with no mention of prior trials). Explain where every supplied element goes and give the total.";

#[derive(Deserialize, Serialize)]
struct Plan {
    version: String,
    cells: Vec<Cell>,
    jobs: Vec<Job>,
}

#[derive(Deserialize, Serialize)]
struct Job {
    id: String,
    cell: usize,
    repeat: usize,
    request: Value,
}

pub fn prepare(out: &Path, cells: Vec<Cell>, repeats: usize) -> Result<()> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let mut jobs = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let user = format!(
            "<memory>\n{}</memory>\n\nCommission:\n{}",
            cell.packet.memory, cell.brief,
        );
        for repeat in 0..repeats {
            jobs.push(Job {
                id: hex(format!("{VERSION}/{nonce}/{index}/{repeat}").as_bytes())[..24].into(),
                cell: index,
                repeat,
                request: json!({
                    "model": MODEL, "temperature": 0.4, "max_tokens": 900,
                    "messages": [
                        {"role": "system", "content": SYSTEM},
                        {"role": "user", "content": user},
                    ],
                }),
            });
        }
    }
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    let plan = Plan {
        version: VERSION.into(),
        cells,
        jobs,
    };
    let path = out.join("plan.json");
    save(&path, &plan)?;
    fs::write(out.join("plan.sha256"), hex(&fs::read(path)?))?;
    println!(
        "Prepared {} held-out requests. No HTTP requests were made.",
        plan.jobs.len(),
    );
    Ok(())
}

fn load_plan(out: &Path) -> Result<(Plan, String)> {
    let bytes = fs::read(out.join("plan.json"))?;
    let fingerprint = hex(&bytes);
    if fingerprint != fs::read_to_string(out.join("plan.sha256"))?.trim() {
        return Err("plan.json has changed since preparation".into());
    }
    let plan: Plan = serde_json::from_slice(&bytes)?;
    if plan.version != VERSION {
        return Err("unsupported plan version".into());
    }
    Ok((plan, fingerprint))
}

fn lock(out: &Path) -> Result<fs::File> {
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(out.join("run.lock"))?;
    lock.try_lock()?;
    Ok(lock)
}

#[derive(Deserialize, Serialize)]
struct Proposal {
    device: Device,
    proposal: String,
}

#[derive(Deserialize, Serialize)]
struct Attempt {
    id: String,
    reply: Option<Proposal>,
    error: Option<String>,
    usage: Value,
}

#[derive(Deserialize, Serialize)]
struct Journal {
    plan_sha256: String,
    mock: bool,
    attempts: Vec<Attempt>,
}

impl Journal {
    fn replies(&self) -> BTreeMap<&str, &Proposal> {
        self.attempts
            .iter()
            .filter_map(|attempt| {
                attempt
                    .reply
                    .as_ref()
                    .map(|reply| (attempt.id.as_str(), reply))
            })
            .collect()
    }
}

fn decode(raw: &str) -> Result<(Proposal, Value)> {
    let data: Value = serde_json::from_str(raw)?;
    let choice = &data["choices"][0];
    if choice["finish_reason"] != "stop" {
        return Err(format!("incomplete provider reply: {}", choice["finish_reason"]).into());
    }
    let message = choice["message"]["content"]
        .as_str()
        .ok_or("missing provider text")?
        .trim();
    let message = message
        .strip_prefix("```json")
        .and_then(|text| text.trim_end().strip_suffix("```"))
        .unwrap_or(message)
        .trim();
    let proposal: Proposal = serde_json::from_str(message)?;
    if proposal.proposal.trim().is_empty() {
        return Err("empty proposal".into());
    }
    Ok((proposal, data["usage"].clone()))
}

fn check_journal(journal: &Journal, fingerprint: &str) -> Result<()> {
    if journal.plan_sha256 != fingerprint {
        return Err("attempts belong to a different plan".into());
    }
    Ok(())
}

pub fn run(out: &Path, cap: usize, mock: bool) -> Result<()> {
    let _lock = lock(out)?;
    let (plan, fingerprint) = load_plan(out)?;
    let path = out.join("attempts.json");
    let mut journal = if path.exists() {
        read::<Journal>(&path)?
    } else {
        Journal {
            plan_sha256: fingerprint.clone(),
            mock,
            attempts: Vec::new(),
        }
    };
    check_journal(&journal, &fingerprint)?;
    if journal.mock != mock {
        return Err("use a fresh plan to change between mock and Grok".into());
    }
    for job in &plan.jobs {
        if journal.replies().contains_key(job.id.as_str()) {
            continue;
        }
        if journal.attempts.len() >= cap {
            break;
        }
        let key = if mock {
            String::new()
        } else {
            env::var("GROK_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or("GROK_API_KEY is not configured")?
        };
        journal.attempts.push(Attempt {
            id: job.id.clone(),
            reply: None,
            error: Some("interrupted or pending call".into()),
            usage: Value::Null,
        });
        save(&path, &journal)?;
        let response = if mock {
            Ok((
                Proposal {
                    device: Device::Other,
                    proposal: "Mock proposal for offline pipeline verification.".into(),
                },
                Value::Null,
            ))
        } else {
            selmem::net::httpx::post_json(ENDPOINT, Some(&key), &job.request.to_string())
                .map_err(Into::into)
                .and_then(|raw| decode(&raw))
        };
        let latest = journal.attempts.last_mut().unwrap();
        match response {
            Ok((proposal, usage)) => {
                latest.reply = Some(proposal);
                latest.usage = usage;
                latest.error = None;
                save(&path, &journal)?;
            }
            Err(error) => {
                latest.error = Some(error.to_string());
                save(&path, &journal)?;
                return Err(error);
            }
        }
    }
    let replies = journal.replies();
    println!(
        "Recorded {}/{} proposals with {} {}attempts (cumulative cap {}).",
        replies.len(),
        plan.jobs.len(),
        journal.attempts.len(),
        if mock { "mock " } else { "" },
        cap,
    );
    if replies.len() == plan.jobs.len() {
        let rows: Vec<_> = plan
            .jobs
            .iter()
            .map(|job| {
                json!({
                    "id": job.id, "brief": plan.cells[job.cell].brief,
                    "proposal": replies[job.id.as_str()].proposal,
                    "device": null, "feasible": null,
                })
            })
            .collect();
        save(&out.join("blind.json"), &rows)?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct Rating {
    id: String,
    device: Device,
    feasible: bool,
}

#[derive(Default, Serialize)]
struct Scores {
    pairs: usize,
    declared_forward: usize,
    blind_forward: usize,
    blind_reverse: usize,
    feasible_forward: usize,
    paired_intact_evidence: usize,
}

fn forward(devices: [Device; 2]) -> bool {
    devices == [Device::Interruption, Device::Concentration]
}

pub fn score(out: &Path, path: &Path) -> Result<()> {
    let _lock = lock(out)?;
    let (plan, fingerprint) = load_plan(out)?;
    let journal: Journal = read(&out.join("attempts.json"))?;
    check_journal(&journal, &fingerprint)?;
    let responses = journal.replies();
    let mut ratings = BTreeMap::new();
    for rating in read::<Vec<Rating>>(path)? {
        if ratings.insert(rating.id.clone(), rating).is_some() {
            return Err("duplicate blind rating ID".into());
        }
    }
    if ratings.len() != plan.jobs.len() || responses.len() != plan.jobs.len() {
        return Err("scoring requires all proposals and one rating per job".into());
    }
    let mut pairs = BTreeMap::new();
    for job in &plan.jobs {
        let cell = &plan.cells[job.cell];
        let rating = ratings.get(&job.id).ok_or("missing blind rating ID")?;
        let proposal = responses
            .get(job.id.as_str())
            .ok_or("missing proposal ID")?;
        pairs
            .entry((&cell.workshop, cell.commission, job.repeat, cell.arm))
            .or_insert([None, None])[cell.mirror] = Some((
            rating,
            proposal.device,
            cell.packet.intact_trials.len() == 2,
        ));
    }
    let mut totals = BTreeMap::new();
    let mut per_workshop = BTreeMap::<&str, BTreeMap<Arm, Scores>>::new();
    for ((workshop, _, _, arm), pair) in pairs {
        let [Some(first), Some(second)] = pair else {
            return Err("a matched pair is incomplete".into());
        };
        let blind = [first.0.device, second.0.device];
        for scores in [
            totals.entry(arm).or_insert_with(Scores::default),
            per_workshop
                .entry(workshop)
                .or_default()
                .entry(arm)
                .or_default(),
        ] {
            scores.pairs += 1;
            scores.declared_forward += usize::from(forward([first.1, second.1]));
            scores.blind_forward += usize::from(forward(blind));
            scores.blind_reverse += usize::from(forward([blind[1], blind[0]]));
            scores.feasible_forward +=
                usize::from(forward(blind) && first.0.feasible && second.0.feasible);
            scores.paired_intact_evidence += usize::from(first.2 && second.2);
        }
    }
    for arm in ARMS {
        let metrics = &totals[&arm];
        println!(
            "{arm:?}: {}/{} feasible forward pairs",
            metrics.feasible_forward, metrics.pairs
        );
    }
    save(
        &out.join("scores.json"),
        &json!({
            "version": VERSION, "plan_sha256":fingerprint, "mock":journal.mock,
            "one_blind_rating_per_job": true, "totals":totals, "workshops":per_workshop,
        }),
    )
}
