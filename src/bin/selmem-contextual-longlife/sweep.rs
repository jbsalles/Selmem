//! GENESIS v4 breaking-point map: rerun the SAME qualified lives at multiple
//! memory budgets, with independent lives at each distractor load.
//! Included as a child module of selmem-contextual-longlife.rs to reuse its
//! exact history generator, actor, controls and synthetic oracle.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};

const SYSTEM: &str = "You are choosing for yourself. Previously recorded decisions may be relevant but are fallible. They do not establish feelings, preferences, or causal explanations. Decide in the NEW situation using the available history and present circumstances. Reply with only the digit 1 or 2, no explanation.";

#[derive(Clone, Debug)]
pub(super) struct Sweep {
    out: PathBuf,
    pairs: usize,
    seed: u64,
    budgets: Vec<usize>,
    loads: Vec<usize>,
    arms: Vec<Arm>,
    actor: bool,
    resume: bool,
    provider: String,
    model: String,
    reasoning: Option<String>,
    max_http_calls: usize,
}

fn axis(raw: &str, name: &str, low: usize, high: usize) -> Result<Vec<usize>, String> {
    if raw.is_empty() { return Err(format!("{name} may not be empty")); }
    let nums: Vec<usize> = raw.split(',').map(|s| {
        let v: usize = s.parse().map_err(|_| format!("bad {name}: {s}"))?;
        if !(low..=high).contains(&v) { return Err(format!("{name} outside {low}..={high}: {v}")); }
        Ok(v)
    }).collect::<Result<_, String>>()?;
    if nums.windows(2).any(|w| w[0] >= w[1]) {
        return Err(format!("{name} must be strictly ascending and unique"));
    }
    Ok(nums)
}
fn arms_arg(raw: &str) -> Result<Vec<Arm>, String> {
    let mut a = Vec::new();
    for s in raw.split(',') {
        let arm = Arm::ALL.iter().copied().find(|x| x.name() == s)
            .ok_or_else(|| format!("unknown arm {s}"))?;
        if a.contains(&arm) { return Err(format!("duplicate arm {s}")); }
        a.push(arm);
    }
    if !a.contains(&Arm::On) || !a.contains(&Arm::Summary) || !a.contains(&Arm::Retrieval) {
        return Err("sweep arms must include on,summary,retrieval".into());
    }
    Ok(a)
}
impl Sweep {
    pub(super) fn parse() -> Result<Self, String> {
        let mut c = Self {
            out: PathBuf::new(), pairs: 2, seed: 1729, budgets: vec![], loads: vec![],
            arms: vec![Arm::On, Arm::Off, Arm::Summary, Arm::Retrieval],
            actor: false, resume: false, provider: "grok".into(),
            model: "grok-4.3".into(), reasoning: None, max_http_calls: 0,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            match flag.as_str() {
                "--actor" => { c.actor = true; continue; }
                "--resume" => { c.resume = true; continue; }
                "--help" | "-h" => {
                    println!("GENESIS v4 contextual memory breaking-point map\n\
--out DIR --sweep-budgets 8,16,24,32,64,128 --sweep-loads 240,480,960\n\
[--pairs 1..12] [--seed N] [--sweep-arms on,off,summary,retrieval]\n\
[--actor --provider grok|openai --model MODEL --max-http-calls N]\n\
[--reasoning none|low|medium|high] (Grok 4.3; default none)\n\
[--resume] reuse the identical offline payload plan / append actor calls.\n\
A strict per-invocation HTTP cap pauses the run; resume with the SAME flags.\n\
Default: offline, two domains, four arms. No API calls without --actor.\n\
Actor calls in the complete grid = 8 * pairs * arms * budgets * loads.\n\
Report in comparison.csv, curve.csv, summary.jsonl and exact payloads.jsonl.");
                    std::process::exit(0);
                }
                _ => {}
            }
            let v = it.next().ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--out" => c.out = PathBuf::from(v),
                "--pairs" => c.pairs = v.parse().map_err(|_| "invalid pairs")?,
                "--seed" => c.seed = v.parse().map_err(|_| "invalid seed")?,
                "--sweep-budgets" => c.budgets = axis(&v, "budgets", 1, 256)?,
                "--sweep-loads" => c.loads = axis(&v, "loads", 80, 2000)?,
                "--sweep-arms" => c.arms = arms_arg(&v)?,
                "--provider" => c.provider = v,
                "--model" => c.model = v,
                "--reasoning" => c.reasoning = Some(v),
                "--max-http-calls" => c.max_http_calls = v.parse().map_err(|_| "invalid max-http-calls")?,
                _ => return Err(format!("unknown sweep flag {flag}")),
            }
        }
        if c.out.as_os_str().is_empty() { return Err("--out is required".into()); }
        if c.budgets.is_empty() || c.loads.is_empty() {
            return Err("--sweep-budgets and --sweep-loads are both required".into());
        }
        if !(1..=DOMAINS.len()).contains(&c.pairs) { return Err("--pairs must be 1..=12".into()); }
        if c.provider != "grok" && c.provider != "openai" { return Err("provider must be grok or openai".into()); }
        if c.model.trim().is_empty() { return Err("empty model".into()); }
        if c.provider == "grok" {
            // Grok 4.3 defaults to low if omitted. Never silently rely on it.
            let effort = c.reasoning.get_or_insert_with(|| "none".into());
            if !matches!(effort.as_str(), "none" | "low" | "medium" | "high") {
                return Err("Grok reasoning must be none, low, medium or high".into());
            }
        } else if c.reasoning.is_some() {
            return Err("--reasoning is supported only with --provider grok in this sweep".into());
        }
        if c.actor && c.max_http_calls == 0 { return Err("--actor requires a positive --max-http-calls (per-invocation cap)".into()); }
        if !c.resume && c.out.exists() { return Err("--out must name a NEW directory; use --resume for the same plan".into()); }
        if c.resume && !c.out.is_dir() { return Err("--resume requires an existing sweep directory".into()); }
        Ok(c)
    }
    fn calls(&self) -> usize {
        self.pairs * 8 * self.arms.len() * self.budgets.len() * self.loads.len()
    }
    fn manifest(&self) -> String {
        obj([
            ("version", text("genesis-contextual-breaking-points-v1")),
            ("source", text("genesis-contextual-longlife-v4; exact same synthetic lives and controls")),
            ("synthetic", Json::Bool(true)),
            ("pairs", num(self.pairs as f64)),
            ("independent_domains", num(self.pairs as f64)),
            ("seed", num(self.seed as f64)),
            ("budgets", Json::Array(self.budgets.iter().map(|v| num(*v as f64)).collect())),
            ("distractors", Json::Array(self.loads.iter().map(|v| num(*v as f64)).collect())),
            ("arms", Json::Array(self.arms.iter().map(|v| text(v.name())).collect())),
            ("actor_provider", text(&self.provider)),
            ("actor_model", text(&self.model)),
            ("planned_actor_calls", num(self.calls() as f64)),
            ("comparison_note", text("Word caps are ceilings, NOT matched delivered lengths or token/cost budgets. Original V4 truncation and deterministic controls; no semantic RAG, independent judge or no-pattern arm.")),
        ]).encode()
    }
}

// Freeze the *actor* settings separately so an already-qualified offline
// payload plan remains reusable without rewriting its historical manifest.
// Reject any existing responses whose reasoning settings were not audited.
fn verify_actor_settings(c: &Sweep) -> Result<(), String> {
    let path = c.out.join("actor_config.json");
    let cfg = obj([
        ("provider", text(&c.provider)),
        ("model", text(&c.model)),
        ("temperature", num(0.0)),
        ("reasoning_effort", text(c.reasoning.as_deref().unwrap_or("provider_default"))),
        ("max_tokens", num(16.0)),
    ]).encode();
    if path.exists() {
        if fs::read_to_string(path).map_err(|e| e.to_string())? != cfg {
            return Err("actor settings differ (including reasoning); use a fresh output directory".into());
        }
    } else {
        let choices = c.out.join("choices.jsonl");
        if choices.exists() && fs::metadata(choices).map_err(|e| e.to_string())?.len() > 0 {
            return Err("existing choices have no reasoning audit; start a new output directory".into());
        }
        fs::write(path, cfg).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Only sweep actor calls use this payload. Original V4 remains unchanged.
// xAI's Chat Completions API accepts reasoning_effort=none for grok-4.3.
fn actor_request(c: &Sweep, prompt: &str) -> Json {
    let mut data = obj([
        ("model", text(&c.model)),
        ("temperature", num(0.0)),
        ("max_tokens", num(16.0)),
        ("messages", Json::Array(vec![
            obj([("role", text("system")), ("content", text(SYSTEM))]),
            obj([("role", text("user")), ("content", text(prompt))]),
        ])),
    ]);
    if let Some(ref effort) = c.reasoning {
        data.put("reasoning_effort", text(effort));
    }
    data
}
fn sweep_chat(c: &Sweep, key: &str, prompt: &str) -> Result<String, String> {
    let endpoint = if c.provider == "grok" {
        "https://api.x.ai/v1/chat/completions"
    } else {
        "https://api.openai.com/v1/chat/completions"
    };
    let raw = selmem::net::httpx::post_json(endpoint, Some(key), &actor_request(c, prompt).encode())?;
    let response = json::parse(&raw)?;
    let choices = response.get("choices")?.array()?;
    choices.first().ok_or("no choices")?.get("message")?.get("content")?
        .string().map(|s| s.trim().to_string())
}

#[derive(Clone, Copy)]
struct CellTrial { load: usize, budget: usize, trial: Trial }
impl CellTrial {
    fn key(self) -> String { format!("load{}:budget{}:{}", self.load, self.budget, self.trial.id()) }
}
fn plan(c: &Sweep) -> Vec<CellTrial> {
    let trials = trials(c.pairs);
    let mut p = Vec::with_capacity(c.calls());
    for &load in &c.loads { for &budget in &c.budgets { for &trial in &trials {
        if c.arms.contains(&trial.arm) { p.push(CellTrial { load, budget, trial }); }
    }}}
    p
}
fn snapshot_dir(c: &Sweep, load: usize) -> PathBuf {
    c.out.join("snapshots").join(format!("load-{load}"))
}
fn payload(p: CellTrial, c: &Sweep) -> Result<Json, String> {
    let dir = snapshot_dir(c, p.load);
    let (uncapped, ids, full_words) = material(p.trial, &dir, usize::MAX)?;
    let memo = cap_words(&uncapped, p.budget);
    let used = memo.split_whitespace().count();
    if used > p.budget || used > full_words || (used == full_words && memo != uncapped) {
        return Err(format!("word cap or prefix mismatch: {}", p.key()));
    }
    let t = p.trial;
    Ok(obj([
        ("id", text(p.key())), ("distractors", num(p.load as f64)),
        ("budget", num(p.budget as f64)), ("domain", text(DOMAINS[t.domain].id)),
        ("history", text(if t.reversed {"reversed"} else {"normal"})),
        ("context", text(if t.company {"friends"} else {"alone"})),
        ("arm", text(t.arm.name())), ("wording", num(t.wording as f64)),
        ("order", num(t.order as f64)), ("expected_a", Json::Bool(t.target_a())),
        ("question", text(t.question())), ("memory", text(memo)),
        ("memory_words", num(used as f64)),
        ("uncapped_words", num(full_words as f64)),
        ("clipped", Json::Bool(used < full_words)),
        ("selected_ids", Json::Array(ids.into_iter().map(text).collect())),
    ]))
}
fn write_payloads(c: &Sweep, p: &[CellTrial]) -> Result<(), String> {
    let mut s = String::new();
    for &t in p { s.push_str(&payload(t, c)?.encode()); s.push('\n'); }
    let file = c.out.join("payloads.jsonl");
    if c.resume {
        let old = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        if old != s { return Err("payloads differ from original run; refusing resume".into()); }
    } else { fs::write(file, s).map_err(|e| e.to_string())?; }
    Ok(())
}
fn qualify(c: &Sweep) -> Result<(), String> {
    if !c.resume {
        fs::create_dir(&c.out).map_err(|e| e.to_string())?;
        fs::write(c.out.join("manifest.json"), c.manifest()).map_err(|e| e.to_string())?;
    } else {
        let existing = fs::read_to_string(c.out.join("manifest.json")).map_err(|e| e.to_string())?;
        if existing != c.manifest() { return Err("manifest mismatch; cannot resume with changed flags or code protocol".into()); }
    }
    for &load in &c.loads {
        let dir = snapshot_dir(c, load);
        if !c.resume {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut log = BufWriter::new(File::create(dir.join("organ.jsonl")).map_err(|e| e.to_string())?);
            for d in DOMAINS.into_iter().take(c.pairs) { for reversed in [false, true] { for on in [false, true] {
                build_life(&dir, d, reversed, on, &mut log, load, c.seed)?;
            }}}
            log.flush().map_err(|e| e.to_string())?;
        }
        for d in DOMAINS.into_iter().take(c.pairs) { for reversed in [false, true] {
            let on = load_memory(&dir, d, reversed, true)?;
            let off = load_memory(&dir, d, reversed, false)?;
            let rows = |m: &SelectiveMemory| observations(m).iter()
                .map(|t| (facet_company(t).to_string(), t.gist.clone()))
                .collect::<Vec<_>>();
            if rows(&on) != rows(&off) {
                return Err(format!("ON/OFF observation mismatch: {} load={load} reversed={reversed}", d.id));
            }
        }}
    }
    Ok(())
}
// Distinct function name so the `load` axis can coexist with the V4 loader.
fn load_memory(dir: &Path, d: Domain, reversed: bool, on: bool) -> Result<SelectiveMemory, String> {
    super::load(dir, d, reversed, on)
}
fn previous(c: &Sweep, p: &[CellTrial]) -> Result<BTreeMap<String, Json>, String> {
    let file = c.out.join("choices.jsonl");
    if !file.exists() { return Ok(BTreeMap::new()); }
    let known: BTreeSet<String> = p.iter().map(|t| t.key()).collect();
    let reader = BufReader::new(File::open(file).map_err(|e| e.to_string())?);
    let mut existing = BTreeMap::new();
    for (n, row) in reader.lines().enumerate() {
        let row = row.map_err(|e| e.to_string())?;
        let j = json::parse(&row).map_err(|e| format!("invalid choices line {}: {e}", n+1))?;
        let key = j.get("id")?.string()?.to_string();
        if !known.contains(&key) { return Err(format!("choice not in frozen plan: {key}")); }
        if existing.insert(key.clone(), j).is_some() { return Err(format!("duplicate choice: {key}")); }
    }
    Ok(existing)
}
fn bool_field(j: &Json, key: &str) -> Result<bool, String> {
    match j.get(key)? { Json::Bool(x) => Ok(*x), _ => Err(format!("not boolean: {key}")) }
}
fn report(c: &Sweep, p: &[CellTrial], done: &BTreeMap<String, Json>) -> Result<(), String> {
    #[derive(Default)]
    struct Stat { total: usize, correct: usize, valid: usize, words: usize, clipped: usize }
    let mut domain: BTreeMap<(usize, usize, usize, Arm), Stat> = BTreeMap::new();
    let mut overall: BTreeMap<(usize, usize, Arm), Stat> = BTreeMap::new();
    for &ct in p {
        let j = done.get(&ct.key()).ok_or_else(|| format!("incomplete: {}", ct.key()))?;
        let data = payload(ct, c)?;
        // The payload is frozen at qualification: derived metrics are never read
        // from the model response or guessed from the declared ceiling.
        let n = data.get("memory_words")?.number()? as usize;
        let clipped = bool_field(&data, "clipped")?;
        let correct = bool_field(j, "concordant")?;
        let valid = bool_field(j, "valid")?;
        let d = ct.trial.domain;
        for s in [domain.entry((ct.load, ct.budget, d, ct.trial.arm)).or_default(),
                  overall.entry((ct.load, ct.budget, ct.trial.arm)).or_default()] {
            s.total += 1; s.correct += usize::from(correct); s.valid += usize::from(valid);
            s.words += n; s.clipped += usize::from(clipped);
        }
    }
    let mut jsonl = String::new();
    let mut csv = String::from("distractors,budget,domain,arm,attempts,correct,valid,score,mean_words,clipped_fraction\n");
    for (&(load, budget, idx, arm), s) in &domain {
        let rate = s.correct as f64 / s.total as f64;
        jsonl.push_str(&obj([
            ("kind", text("domain")), ("distractors", num(load as f64)),
            ("budget", num(budget as f64)), ("domain", text(DOMAINS[idx].id)),
            ("arm", text(arm.name())), ("attempts", num(s.total as f64)),
            ("correct", num(s.correct as f64)), ("valid", num(s.valid as f64)),
            ("score", num(rate)),
        ]).encode()); jsonl.push('\n');
    }
    for (&(load, budget, arm), s) in &overall {
        let rate = s.correct as f64 / s.total as f64;
        let mean = s.words as f64 / s.total as f64;
        let clipped = s.clipped as f64 / s.total as f64;
        jsonl.push_str(&obj([
            ("kind", text("aggregate")), ("distractors", num(load as f64)),
            ("budget", num(budget as f64)), ("arm", text(arm.name())),
            ("attempts", num(s.total as f64)), ("correct", num(s.correct as f64)),
            ("valid", num(s.valid as f64)), ("score", num(rate)),
            ("mean_words", num(mean)), ("clipped_fraction", num(clipped)),
        ]).encode()); jsonl.push('\n');
        csv.push_str(&format!("{load},{budget},ALL,{},{},{},{},{rate:.6},{mean:.3},{clipped:.6}\n",
            arm.name(), s.total, s.correct, s.valid));
    }
    let mut comparison = String::from("distractors,budget,on,summary,retrieval,on_minus_summary,on_minus_retrieval,beats_both\n");
    for &load in &c.loads { for &budget in &c.budgets {
        let score = |arm| -> Result<f64, String> {
            let s = overall.get(&(load, budget, arm)).ok_or("missing arm in curve")?;
            Ok(s.correct as f64 / s.total as f64)
        };
        let on = score(Arm::On)?; let summary = score(Arm::Summary)?; let retrieval = score(Arm::Retrieval)?;
        comparison.push_str(&format!("{load},{budget},{on:.6},{summary:.6},{retrieval:.6},{:.6},{:.6},{}\n",
            on-summary, on-retrieval, on>summary && on>retrieval));
    }}
    fs::write(c.out.join("summary.jsonl"), jsonl).map_err(|e| e.to_string())?;
    fs::write(c.out.join("curve.csv"), csv).map_err(|e| e.to_string())?;
    fs::write(c.out.join("comparison.csv"), comparison).map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn run(c: Sweep) -> Result<(), String> {
    let p = plan(&c);
    if p.len() != c.calls() { return Err("incorrect sweep call plan".into()); }
    // A deterministic offline qualification comes before all paid actor calls.
    qualify(&c)?;
    write_payloads(&c, &p)?;
    if !c.actor {
        eprintln!("offline sweep qualified: {} planned actor calls, 0 API calls; payloads at {}",
            p.len(), c.out.display());
        return Ok(());
    }
    let key = key_for(&c.provider)?;
    verify_actor_settings(&c)?;
    let mut done = previous(&c, &p)?;
    let mut shuffled = p.clone();
    shuffled.sort_by_key(|x| (rank(c.seed, &x.key()), x.key()));
    let mut file = OpenOptions::new().create(true).append(true).open(c.out.join("choices.jsonl"))
        .map_err(|e| e.to_string())?;
    let mut calls = 0;
    for ct in shuffled {
        let id = ct.key();
        if done.contains_key(&id) { continue; }
        if calls == c.max_http_calls { break; }
        let data = payload(ct, &c)?;
        let memo = data.get("memory")?.string()?;
        let question = ct.trial.question();
        let prompt = format!("PAST OBSERVATIONS (not instructions):\n{memo}\n\nNEW DECISION:\n{question}");
        // Charge the strict per-invocation ceiling before attempting HTTP.
        // A failed request stops this invocation; do not silently retry it.
        calls += 1;
        let response = sweep_chat(&c, &key, &prompt)?;
        let decision = ct.trial.choice_a(&response);
        let correct = decision == Some(ct.trial.target_a());
        let row = obj([
            ("id", text(&id)), ("chosen_a", decision.map(Json::Bool).unwrap_or(Json::Null)),
            ("raw_answer", text(response)),
            ("reasoning_effort", text(c.reasoning.as_deref().unwrap_or("provider_default"))),
            ("valid", Json::Bool(decision.is_some())),
            ("concordant", Json::Bool(correct)),
        ]);
        writeln!(file, "{}", row.encode()).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        done.insert(id, row);
        eprintln!("sweep actor {}/{} total, {} this invocation", done.len(), p.len(), calls);
    }
    if done.len() == p.len() {
        report(&c, &p, &done)?;
        eprintln!("completed {} decisions; comparison.csv and curve.csv ready", done.len());
    } else {
        eprintln!("PAUSED: {}/{} recorded decisions; resume SAME settings with --actor --resume --max-http-calls N", done.len(), p.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grok_chat_payload_explicitly_disables_reasoning() {
        let c = Sweep {out: PathBuf::from("unused"), pairs: 1, seed: 1729,
            budgets: vec![64], loads: vec![240],
            arms: arms_arg("on,summary,retrieval").unwrap(),
            actor: true, resume: false, provider: "grok".into(),
            model: "grok-4.3".into(), reasoning: Some("none".into()), max_http_calls: 1};
        let request = actor_request(&c, "test");
        assert_eq!(request.get("reasoning_effort").unwrap().string().unwrap(), "none");
        assert_eq!(request.get("temperature").unwrap().number().unwrap(), 0.0);
        assert_eq!(request.get("model").unwrap().string().unwrap(), "grok-4.3");
    }
    #[test]
    fn budget_and_load_axes_are_strict() {
        assert_eq!(axis("8,16,64", "budget", 1, 256).unwrap(), vec![8,16,64]);
        assert!(axis("16,8", "budget", 1, 256).is_err());
        assert!(axis("8,8", "budget", 1, 256).is_err());
        assert!(axis("0,64", "budget", 1, 256).is_err());
        assert!(arms_arg("on,summary,retrieval").is_ok());
        assert!(arms_arg("on,summary").is_err());
    }
    #[test]
    fn paired_plan_has_exact_size_and_unique_cells() {
        let c = Sweep {out: PathBuf::from("unused"), pairs: 2, seed: 1729,
            budgets: vec![8,16,64], loads: vec![240,960],
            arms: arms_arg("on,off,summary,retrieval").unwrap(),
            actor:false,resume:false,provider:"grok".into(),model:"grok-4.3".into(),reasoning:Some("none".into()),max_http_calls:0};
        let p=plan(&c);
        assert_eq!(c.calls(),384);
        assert_eq!(p.len(),384);
        assert_eq!(p.iter().map(|t|t.key()).collect::<BTreeSet<_>>().len(),384);
        assert_eq!(p.iter().filter(|t| t.load==240 && t.budget==16 && t.trial.arm==Arm::On).count(),16);
    }
    #[test]
    fn offline_sweep_reuses_life_for_each_budget() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let out = std::env::temp_dir().join(format!("selmem-budget-sweep-{}-{n}",std::process::id()));
        let c = Sweep {out:out.clone(),pairs:1,seed:1729,budgets:vec![8,64],loads:vec![80],
            arms:arms_arg("on,summary,retrieval").unwrap(),actor:false,resume:false,
            provider:"grok".into(),model:"grok-4.3".into(),reasoning:Some("none".into()),max_http_calls:0};
        let r=run(c.clone());
        if r.is_ok() {
            let lines=fs::read_to_string(out.join("payloads.jsonl")).unwrap();
            assert_eq!(lines.lines().count(),c.calls());
            for row in lines.lines() {
                let j=json::parse(row).unwrap();
                assert!(j.get("memory_words").unwrap().number().unwrap() <= j.get("budget").unwrap().number().unwrap());
            }
            let mut resume=c.clone();resume.resume=true;
            run(resume).unwrap();
        }
        let _=fs::remove_dir_all(&out);
        r.unwrap();
    }
}
