//! Frozen native-delivery interventions. No new histories or action bridge.
use super::*;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use selmem::{MemoryContext, MemoryContextOptions, ScopeStatus};

const ARMS: [&str; 6] = ["full", "no_beliefs", "no_episodes", "swapped", "bare", "placebo"];
const FORMAT: &str = " Reply with only the digit 1 or 2, no explanation.";

pub(super) struct Causal {
    source: PathBuf, out: PathBuf, pairs: usize, repeats: usize, words: usize,
    seed: u64, actor: bool, resume: bool, cap: usize, provider: String, model: String,
}
impl Causal {
    pub(super) fn parse() -> Result<Self, String> {
        let mut c = Self { source: PathBuf::new(), out: PathBuf::new(), pairs: 1,
            repeats: 3, words: 600, seed: 1729, actor: false, resume: false, cap: 0,
            provider: "grok".into(), model: "grok-4.3".into() };
        let mut it = std::env::args().skip(1);
        while let Some(f) = it.next() {
            match f.as_str() {
                "--causal-ablation" => continue,
                "--actor" => { c.actor = true; continue; },
                "--resume" => { c.resume = true; continue; },
                "--help" | "-h" => {
                    println!("Frozen native causal ablations (no API calls by default)\n--causal-ablation --snapshots DIR --out NEW_DIR [--pairs 1..12]\n[--repeats 3] [--memory-words 600] [--seed 1729]\n[--actor --provider grok|openai --model MODEL --max-http-calls N] [--resume]\nCalls = 96 * pairs * repeats. Grok reasoning fixed to none.\nSnapshots must be existing qualified V4 ON vaults. Resume uses frozen payloads.");
                    std::process::exit(0);
                }, _ => {}
            }
            let v = it.next().ok_or_else(|| format!("missing value for {f}"))?;
            match f.as_str() {
                "--snapshots" => c.source = v.into(), "--out" => c.out = v.into(),
                "--pairs" => c.pairs = v.parse().map_err(|_| "invalid pairs")?,
                "--repeats" => c.repeats = v.parse().map_err(|_| "invalid repeats")?,
                "--memory-words" => c.words = v.parse().map_err(|_| "invalid words")?,
                "--seed" => c.seed = v.parse().map_err(|_| "invalid seed")?,
                "--max-http-calls" => c.cap = v.parse().map_err(|_| "invalid cap")?,
                "--provider" => c.provider = v, "--model" => c.model = v,
                _ => return Err(format!("unknown flag {f}")),
            }
        }
        if c.out.as_os_str().is_empty() || c.source.as_os_str().is_empty() { return Err("--snapshots and --out required".into()); }
        if !(1..=12).contains(&c.pairs) || !(2..=100).contains(&c.repeats) { return Err("pairs 1..12 and repeats 2..100 required".into()); }
        MemoryContextOptions { words: c.words, ..Default::default() }.validate()?;
        if !["grok", "openai"].contains(&c.provider.as_str()) || c.model.trim().is_empty() { return Err("invalid provider/model".into()); }
        if c.actor && c.cap == 0 { return Err("--actor requires a positive HTTP cap".into()); }
        if c.resume != c.out.is_dir() || (!c.resume && c.out.exists()) { return Err("use a new --out, or --resume on an existing run".into()); }
        Ok(c)
    }
    fn manifest(&self) -> String {
        obj([("version", text("native-causal-v1")), ("source", text(self.source.to_string_lossy())),
            ("pairs", num(self.pairs as f64)), ("repeats", num(self.repeats as f64)),
            ("words", num(self.words as f64)), ("seed", text(self.seed.to_string())),
            ("provider", text(&self.provider)), ("model", text(&self.model)),
            ("reasoning", text(if self.provider == "grok" { "none" } else { "provider_default" })),
            ("planned_calls", num((96 * self.pairs * self.repeats) as f64)),
            ("intervention", text("post-recall delivery; no refill; no store mutation; native renderer")),
        ]).encode()
    }
}
fn native(c: &Causal, t: Trial) -> Result<(MemoryContext, String), String> {
    let mut m = load(&c.out.join("snapshots"), DOMAINS[t.domain], t.reversed, true)?;
    let qualified = m.store.traces.values().filter(|r| r.personal.scope_status == ScopeStatus::Conditional).count();
    if qualified != 2 { return Err(format!("{}: expected two qualified beliefs, found {qualified}", t.id())); }
    // Rules only: no online reconstruction or actor call while freezing native delivery.
    let options = MemoryContextOptions { words: c.words, ..Default::default() };
    m.set_memory_context_options(options.clone())?;
    let draft = m.open_mouth_scoped(&t.question(), &[ContextFacet::new("company", if t.company { "friends" } else { "alone" })])?;
    let reply = draft.memory_reply();
    let ctx = reply.filtered_context();
    let system = format!("{}{FORMAT}", reply.system_prompt());
    // Do not refill the context if a component was not naturally retrieved.
    Ok((ctx, system))
}
fn intervene(base: &MemoryContext, donor: &MemoryContext, arm: &str) -> MemoryContext {
    if arm == "full" { return base.clone(); }
    let conditional = |s: &&selmem::recall::memory_context::SelfPerspective| s.scope_status == ScopeStatus::Conditional.token();
    let has_base = base.self_model.iter().filter(conditional).count() > 0;
    let has_donor = donor.self_model.iter().filter(conditional).count() > 0;
    if (arm == "no_beliefs" && !has_base) || (arm == "swapped" && !has_base && !has_donor) {
        return base.clone();
    }
    let mut ctx = base.clone();
    match arm {
        "no_beliefs" => ctx.self_model.retain(|s| s.scope_status != ScopeStatus::Conditional.token()),
        "no_episodes" => { ctx.episodes.clear(); ctx.associations.clear(); },
        "swapped" => {
            ctx.self_model.retain(|s| s.scope_status != ScopeStatus::Conditional.token());
            ctx.self_model.extend(donor.self_model.iter().filter(|s| s.scope_status == ScopeStatus::Conditional.token()).cloned());
        },
        "bare" | "placebo" => ctx = MemoryContext { agent_name: base.agent_name.clone(), ..Default::default() },
        "full" => {}, _ => unreachable!(),
    }
    // Recount only: the already-filtered text never gets regenerated or expanded.
    ctx.content_words = ctx.self_model.iter().map(|s| s.context.split_whitespace().count()+s.statement.split_whitespace().count()).sum::<usize>()
        + ctx.episodes.iter().map(|e| e.narrative.split_whitespace().count()).sum::<usize>()
        + ctx.dispositions.iter().map(|d| d.statement.split_whitespace().count()).sum::<usize>()
        + ctx.contextual_statements.iter().map(|s| s.statement.split_whitespace().count()).sum::<usize>();
    ctx
}
fn user(ctx: &MemoryContext, question: &str) -> String {
    // Exact native structured envelope, with a fresh (empty) conversation in every cell.
    obj([("working_conversation", text("")), ("memory", json::parse(&ctx.to_json()).expect("native JSON")), ("human", text(question))]).encode()
}
fn request(c: &Causal, system: &str, user: &str) -> Json {
    let mut r = obj([("model", text(&c.model)), ("temperature", num(0.)), ("max_tokens", num(16.)),
        ("messages", Json::Array(vec![obj([("role", text("system")), ("content", text(system))]), obj([("role", text("user")), ("content", text(user))])]))]);
    if c.provider == "grok" { r.put("reasoning_effort", text("none")); }
    r
}
fn freeze(c: &Causal) -> Result<Vec<Json>, String> {
    let mut plan = Vec::new();
    let mut audit = String::new();
    let mut exposure = String::from("cell,conditional_beliefs,episodes,donor_conditional_beliefs\n");
    let frozen = c.out.join("snapshots");
    fs::create_dir(&frozen).map_err(|e|e.to_string())?;
    for d in DOMAINS.iter().take(c.pairs) { for reversed in [false,true] {
        let src = history_path(&c.source,*d,reversed,true);
        let dst = history_path(&frozen,*d,reversed,true);
        fs::copy(&src,&dst).map_err(|e|format!("copy {}: {e}",src.display()))?;
    }}
    for domain in 0..c.pairs { for reversed in [false, true] { for company in [false, true] { for wording in 0..2 { for order in 0..2 {
        let t = Trial { domain, reversed, company, wording, order, arm: Arm::On };
        let (base, system) = native(c, t)?;
        let (donor, _) = native(c, Trial { reversed: !reversed, ..t })?;
        let cell = format!("{domain}:{reversed}:{company}:{wording}:{order}");
        let n = base.self_model.iter().filter(|s|s.scope_status == ScopeStatus::Conditional.token()).count();
        let nd = donor.self_model.iter().filter(|s|s.scope_status == ScopeStatus::Conditional.token()).count();
        exposure.push_str(&format!("{cell},{n},{},{nd}\n",base.episodes.len()));
        // Record missing delivery; qualification is a property of the store, not proof of actor exposure.
        audit.push_str(&obj([("cell", text(&cell)), ("full", json::parse(&base.to_json())?),
            ("donor", json::parse(&donor.to_json())?)]).encode()); audit.push('\n');
        let full_user = user(&base, &t.question());
        for arm in ARMS {
            let ctx = intervene(&base, &donor, arm);
            let mut u = user(&ctx, &t.question());
            if arm == "placebo" {
                // Neutral payload matched to full serialized UTF-8 byte length, NOT provider tokens.
                let mut j = json::parse(&u)?;
                let gap = full_user.len().saturating_sub(u.len());
                if gap >= 13 {
                    let neutral = "A folder contains routine inventory notes. ";
                    let padding = neutral.repeat((gap-13)/neutral.len()+1);
                    j.put("padding", text(&padding[..gap-13]));
                }
                u = j.encode();
            }
            let req = request(c, &system, &u);
            for repeat in 0..c.repeats {
                plan.push(obj([("id", text(format!("{cell}:{arm}:{repeat}"))), ("cell", text(&cell)),
                    ("domain", num(domain as f64)), ("arm", text(arm)), ("repeat", num(repeat as f64)),
                    ("order", num(order as f64)), ("expected_a", Json::Bool(t.target_a())),
                    ("request", req.clone()), ("memory", json::parse(&ctx.to_json())?),
                    ("user_bytes", num(u.len() as f64)), ("full_user_bytes", num(full_user.len() as f64))]));
            }
        }
    }}}}}
    plan.sort_by_key(|p| rank(c.seed, p.get("id").unwrap().string().unwrap()));
    fs::write(c.out.join("delivery-audit.jsonl"), audit).map_err(|e|e.to_string())?;
    fs::write(c.out.join("exposure.csv"), exposure).map_err(|e|e.to_string())?;
    Ok(plan)
}
fn read_lines(path: &Path) -> Result<Vec<Json>, String> {
    fs::read_to_string(path).map_err(|e|e.to_string())?.lines().map(json::parse).collect()
}
fn choice(p: &Json, row: &Json) -> Result<Option<bool>, String> {
    let raw = row.get("answer")?.string()?;
    let order = p.get("order")?.number()? as usize;
    Ok(match (order, raw.trim()) { (0,"1") | (1,"2") => Some(true), (0,"2") | (1,"1") => Some(false), _ => None })
}
fn report(c: &Causal, plan: &[Json], rows: &[Json]) -> Result<(), String> {
    let by_id: BTreeMap<_,_> = rows.iter().map(|r| Ok((r.get("id")?.string()?.to_string(), r))).collect::<Result<_,String>>()?;
    let mut answers = BTreeMap::new();
    let mut invalid = 0;
    for p in plan {
        if let Some(row) = by_id.get(p.get("id")?.string()?) {
            let a = choice(p, row)?; if a.is_none() { invalid += 1; }
            answers.insert((p.get("cell")?.string()?.to_string(), p.get("arm")?.string()?.to_string(), p.get("repeat")?.number()? as usize), a);
        }
    }
    let mut out = String::from("domain,contrast,valid_pairs,discordant,full_only_concordant,other_only_concordant,invalid_responses,pending\n");
    for domain in 0..c.pairs { for arm in ARMS.into_iter().skip(1).chain(std::iter::once("repeat_noise")) {
        let (mut n, mut discordant, mut full_only, mut other_only) = (0,0,0,0);
        for p in plan.iter().filter(|p| p.get("domain").unwrap().number().unwrap() as usize == domain && p.get("arm").unwrap().string().unwrap() == "full") {
            let cell = p.get("cell")?.string()?.to_string(); let r = p.get("repeat")?.number()? as usize;
            let (other, rr) = if arm == "repeat_noise" { if r == 0 { continue; } ("full", 0) } else { (arm, r) };
            if let (Some(Some(a)), Some(Some(b))) = (answers.get(&(cell.clone(), "full".into(), r)), answers.get(&(cell, other.into(), rr))) {
                n += 1; if a != b { discordant += 1; if *a == (p.get("expected_a")? == &Json::Bool(true)) { full_only += 1; } else { other_only += 1; } }
            }
        }
        out.push_str(&format!("{domain},{arm},{n},{discordant},{full_only},{other_only},{invalid},{}\n",plan.len()-rows.len()));
    }}
    fs::write(c.out.join("paired.csv"),out).map_err(|e|e.to_string())
}
pub(super) fn run(c: Causal) -> Result<(), String> {
    let manifest = c.manifest();
    let plan = if c.resume {
        if fs::read_to_string(c.out.join("manifest.json")).map_err(|e|e.to_string())? != manifest { return Err("resume settings differ".into()); }
        read_lines(&c.out.join("payloads.jsonl"))?
    } else {
        fs::create_dir_all(&c.out).map_err(|e|e.to_string())?;
        let p = freeze(&c)?;
        let s = p.iter().map(Json::encode).collect::<Vec<_>>().join("\n")+"\n";
        fs::write(c.out.join("payloads.jsonl"),s).map_err(|e|e.to_string())?;
        fs::write(c.out.join("manifest.json"),manifest).map_err(|e|e.to_string())?;
        p
    };
    let path = c.out.join("choices.jsonl");
    let mut rows = if path.exists() { read_lines(&path)? } else { Vec::new() };
    let mut done = BTreeSet::new();
    for row in &rows {
        let id = row.get("id")?.string()?;
        let p = plan.iter().find(|p| p.get("id").unwrap().string().unwrap() == id).ok_or("unknown result id")?;
        if row.get("request")? != p.get("request")? || !done.insert(id.to_string()) { return Err("duplicate or mismatched result".into()); }
    }
    if c.actor {
        let key = key_for(&c.provider)?;
        let endpoint = if c.provider == "grok" { "https://api.x.ai/v1/chat/completions" } else { "https://api.openai.com/v1/chat/completions" };
        let mut out = OpenOptions::new().create(true).append(true).open(path).map_err(|e|e.to_string())?;
        for p in plan.iter().filter(|p| !done.contains(p.get("id").unwrap().string().unwrap())).take(c.cap) {
            let result = selmem::net::httpx::post_json(endpoint,Some(&key),&p.get("request")?.encode())
                .and_then(|raw| { let j=json::parse(&raw)?; j.get("choices")?.array()?.first().ok_or_else(||String::from("no choices"))?.get("message")?.get("content")?.string().map(str::to_string) });
            let (answer,error) = match result { Ok(a)=>(a,String::new()),Err(e)=>(String::new(),e) };
            let row = obj([("id",p.get("id")?.clone()),("request",p.get("request")?.clone()),("answer",text(answer)),("error",text(error))]);
            writeln!(out,"{}",row.encode()).map_err(|e|e.to_string())?; out.flush().map_err(|e|e.to_string())?;
            rows.push(row);
            eprintln!("causal {}/{}",rows.len(),plan.len());
        }
    }
    report(&c,&plan,&rows)?;
    eprintln!("causal plan: {} calls; {} recorded; {} pending",plan.len(),rows.len(),plan.len()-rows.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn belief(statement: &str) -> selmem::recall::memory_context::SelfPerspective {
        selmem::recall::memory_context::SelfPerspective {
            trace_id: statement.into(), role:"belief".into(), context:"choice-pattern".into(),
            statement:statement.into(), certainty:0.8, importance:0.5,foundations_missing:false,
            counterexamples:1,scope_status:ScopeStatus::Conditional.token().into(),
            scope_parent:Some("parent".into()),scope_facets:vec![ContextFacet::new("company","alone")],truncated:false,
        }
    }
    #[test] fn cuts_and_swap_use_native_statements_without_rewriting() {
        let mut base=MemoryContext::default();base.self_model.push(belief("native normal statement"));
        let mut donor=MemoryContext::default();donor.self_model.push(belief("native mirror statement"));
        assert!(intervene(&base,&donor,"no_beliefs").self_model.is_empty());
        assert_eq!(intervene(&base,&donor,"no_episodes").self_model[0].statement,"native normal statement");
        let swapped=intervene(&base,&donor,"swapped");
        assert_eq!(swapped.self_model[0].statement,"native mirror statement");
        assert_eq!(swapped.self_model[0].certainty,donor.self_model[0].certainty);
        assert_eq!(base.self_model[0].statement,"native normal statement");
    }
    #[test] fn interventions_preserve_unaffected_channels() {
        let mut b = MemoryContext::default();
        b.affect = Some(Default::default());
        b.contextual_statements.push(selmem::recall::memory_context::ContextStatement { statement:"retained".into(), truncated:false });
        let cut = intervene(&b,&MemoryContext::default(),"no_beliefs");
        assert_eq!(cut.contextual_statements[0].statement,"retained");
        assert!(cut.affect.is_some());
        assert!(intervene(&b,&b,"bare").contextual_statements.is_empty());
    }
    #[test] fn canonical_choice_counterbalances_position() {
        for (order,answer) in [(0,"1"),(1,"2")] {
            let p=obj([("order",num(order as f64))]);
            assert_eq!(choice(&p,&obj([("answer",text(answer))])).unwrap(),Some(true));
        }
    }
    #[test] fn frozen_native_plan_is_paired_and_preserves_snapshots() {
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("selmem-causal-{nonce}"));
        let source = root.join("source"); fs::create_dir_all(&source).unwrap();
        let mut log = BufWriter::new(File::create(root.join("organ.jsonl")).unwrap());
        for reversed in [false,true] { build_life(&source,DOMAINS[0],reversed,true,&mut log,80,1729).unwrap(); }
        let before = [false,true].map(|r|fs::read(history_path(&source,DOMAINS[0],r,true)).unwrap());
        let c = Causal { source:source.clone(), out:root.join("run"), pairs:1,repeats:3,words:600,seed:1729,actor:false,resume:false,cap:0,provider:"grok".into(),model:"grok-4.3".into() };
        fs::create_dir(&c.out).unwrap();
        let plan = freeze(&c).unwrap();
        assert_eq!(plan.len(),288);
        for p in &plan {
            let cell = p.get("cell").unwrap();
            let repeat = p.get("repeat").unwrap();
            let full = plan.iter().find(|f|f.get("cell").unwrap()==cell && f.get("repeat").unwrap()==repeat && f.get("arm").unwrap()==&text("full")).unwrap();
            let memory = p.get("memory").unwrap();
            if p.get("arm").unwrap()==&text("no_beliefs") {
                assert_eq!(memory.get("episodes").unwrap(),full.get("memory").unwrap().get("episodes").unwrap());
                assert!(!memory.get("self_model").unwrap().array().unwrap().iter().any(|s|s.get("scope_status").unwrap()==&text("conditional")));
            }
            if p.get("arm").unwrap()==&text("no_episodes") { assert_eq!(memory.get("self_model").unwrap(),full.get("memory").unwrap().get("self_model").unwrap()); }
            if p.get("arm").unwrap()==&text("placebo") { assert_eq!(p.get("user_bytes").unwrap(),p.get("full_user_bytes").unwrap()); }
            if p.get("arm").unwrap()==&text("no_beliefs") && full.get("memory").unwrap().get("self_model").unwrap().array().unwrap().is_empty() {
                assert_eq!(p.get("request").unwrap(),full.get("request").unwrap(),"no exposure must remain an exact no-op");
            }
            let same = plan.iter().find(|f|f.get("cell").unwrap()==cell && f.get("arm").unwrap()==p.get("arm").unwrap() && f.get("repeat").unwrap()==&num(0.)).unwrap();
            assert_eq!(p.get("request").unwrap(),same.get("request").unwrap());
            assert!(!p.get("request").unwrap().encode().contains("Predicted action:"));
        }
        assert_eq!(fs::read_to_string(c.out.join("exposure.csv")).unwrap().lines().count(),17);
        for (i,r) in [false,true].into_iter().enumerate() { assert_eq!(fs::read(history_path(&source,DOMAINS[0],r,true)).unwrap(),before[i]); }
        let _=fs::remove_dir_all(root);
    }
}
