//! Causal delivery v2: audit every stage and fail closed before actor HTTP.
//! Original snapshots and decision questions remain unchanged.
use super::*;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use selmem::{MemoryContext, MemoryContextOptions};
const ARMS: [&str; 6] = ["full","no_beliefs","no_episodes","swapped","bare","placebo"];

pub(super) struct Causal {
    source:PathBuf,out:PathBuf,pairs:usize,repeats:usize,words:usize,seed:u64,
    actor:bool,resume:bool,cap:usize,provider:String,model:String,route:String,prepare:bool,rag:bool,
}
impl Causal {
    pub(super) fn parse()->Result<Self,String>{
        let mut c=Self{source:PathBuf::new(),out:PathBuf::new(),pairs:12,repeats:3,words:600,seed:1729,
            actor:false,resume:false,cap:0,provider:"grok".into(),model:"grok-4.3".into(),route:"legacy".into(),prepare:false,rag:false};
        let mut it=std::env::args().skip(1);
        while let Some(f)=it.next(){
            match f.as_str(){"--causal-ablation"=>continue,"--actor"=>{c.actor=true;continue},"--resume"=>{c.resume=true;continue},"--prepare-snapshots"=>{c.prepare=true;continue},"--rag"=>{c.rag=true;continue},
                "--help"|"-h"=>{println!("Native causal v2 (offline by default)\n--causal-ablation --snapshots DIR --out NEW_DIR --pairs 1..12\n--prepare-snapshots DIR   (offline: write workspace vaults and exit)\n--recall-route legacy|context|context-cued|context-cued-ablated [--repeats 3 --memory-words 600 --seed 1729]\n[--actor --provider grok|openai --model MODEL --max-http-calls N] [--resume]\nAll cells must pass frozen delivery checks BEFORE keys or HTTP. Failed preflights cannot be resumed into actors.\nCalls = 96 * pairs * repeats; actor cap is per invocation, no automatic retries.");std::process::exit(0)},_=>{}}
            let v=it.next().ok_or_else(||format!("missing value for {f}"))?;
            match f.as_str(){"--snapshots"=>c.source=v.into(),"--out"=>c.out=v.into(),"--pairs"=>c.pairs=v.parse().map_err(|_|"invalid pairs")?,
                "--repeats"=>c.repeats=v.parse().map_err(|_|"invalid repeats")?,"--memory-words"=>c.words=v.parse().map_err(|_|"invalid words")?,
                "--seed"=>c.seed=v.parse().map_err(|_|"invalid seed")?,"--recall-route"=>c.route=v,
                "--provider"=>c.provider=v,"--model"=>c.model=v,"--max-http-calls"=>c.cap=v.parse().map_err(|_|"invalid cap")?,_=>return Err(format!("unknown flag {f}"))}
        }
        if !(1..=12).contains(&c.pairs){return Err("pairs 1..12 required".into());}
        if c.prepare {
            if c.actor||c.resume{return Err("snapshot preparation is offline and requires a new directory".into());}
            if c.source.exists(){return Err("snapshot directory already exists".into());}
            if c.source.as_os_str().is_empty(){return Err("--prepare-snapshots requires --snapshots DIR".into())}
            return Ok(c);
        }
        if c.source.as_os_str().is_empty()||c.out.as_os_str().is_empty(){return Err("--snapshots and --out required".into())}
        if !(1..=12).contains(&c.pairs)||!(2..=100).contains(&c.repeats){return Err("pairs 1..12, repeats 2..100 required".into())}
        MemoryContextOptions{words:c.words,..Default::default()}.validate()?;
        if !["legacy","context","context-cued","context-cued-ablated"].contains(&c.route.as_str()){return Err("invalid recall route".into())}
        if !["grok","openai"].contains(&c.provider.as_str())||c.model.trim().is_empty(){return Err("invalid provider/model".into())}
        if c.actor&&c.cap==0{return Err("--actor requires a positive HTTP cap".into())}
        if c.resume != c.out.is_dir() || (!c.resume&&c.out.exists()){return Err("use a new --out, or --resume on an existing v2 run".into())}
        Ok(c)
    }
    fn manifest(&self)->String{obj([("version",text("native-causal-v2")),("protocol",text("qualified-ablation-v3")),("rag",Json::Bool(self.rag)),("source",text(self.source.to_string_lossy())),
        ("pairs",num(self.pairs as f64)),("repeats",num(self.repeats as f64)),("words",num(self.words as f64)),("seed",text(self.seed.to_string())),
        ("route",text(&self.route)),("provider",text(&self.provider)),("model",text(&self.model)),
        ("reasoning",text(if self.provider=="grok"{"none"}else{"provider_default"})),("planned_calls",num((96*self.pairs*self.repeats)as f64))]).encode()}
}
fn read_lines(p:&Path)->Result<Vec<Json>,String>{fs::read_to_string(p).map_err(|e|e.to_string())?.lines().map(json::parse).collect()}
fn conditionals(ctx:&MemoryContext)->usize{ctx.self_model.iter().filter(|s|s.scope_status==ScopeStatus::Conditional.token()).count()}

#[derive(Clone)]
struct Delivery { context:MemoryContext, system:String, audit:Json, pass:bool }
fn native(c:&Causal,t:Trial)->Result<Delivery,String>{
    let mut m=load(&c.out.join("snapshots"),DOMAINS[t.domain],t.reversed,true)?;
    let d=DOMAINS[t.domain];let q=t.question();
    let facets=vec![ContextFacet::new("company",if t.company{"friends"}else{"alone"})];
    let plain=view_scoped(&m.store,&q,None,&facets).iter().map(|t|t.id.clone()).collect::<Vec<_>>();
    let typed=view_scoped(&m.store,&q,Some("choice-pattern"),&facets).iter().map(|t|t.id.clone()).collect::<Vec<_>>();
    let qualified=m.store.traces.values().filter(|t|t.personal.scope_status==ScopeStatus::Conditional).count();
    m.set_memory_context_options(MemoryContextOptions{words:c.words,..Default::default()})?;
    if c.route=="context-cued-ablated"{m.cut=selmem::OrganCut::no_self_cues();}
    let draft=match c.route.as_str(){
        "legacy"=>m.open_mouth_scoped(&q,&facets)?,
        "context"=>m.open_mouth_in_context(&q,"choice-pattern",&facets)?,
        "context-cued"|"context-cued-ablated"=>m.open_mouth_in_context_cued(&q,"choice-pattern",&facets)?,_=>unreachable!()};
    let reply=draft.memory_reply();let ctx=reply.filtered_context();
    // Validation ONLY may inspect fixture labels. None enter retrieval or actor instructions.
    let topic_episodes=ctx.episodes.iter().filter(|e|m.store.traces.get(&e.trace_id).is_some_and(|t|[d.a,d.b].contains(&t.gist.as_str()))).count();
    let scoped_episodes=ctx.episodes.iter().filter(|e|m.store.traces.get(&e.trace_id).is_some_and(|t|[d.a,d.b].contains(&t.gist.as_str())&&t.personal.scope_facets.contains(&facets[0]))).count();
    let pass=qualified==2&&conditionals(&ctx)==1&&topic_episodes>0&&scoped_episodes>0;
    let candidates=draft.dump.candidates.iter().take(8).map(|s|obj([("id",text(&s.trace_id)),("score",num(s.score as f64)),("anchor",num(s.anchor as f64)),
        ("gist",text(m.store.traces.get(&s.trace_id).map(|t|t.gist.as_str()).unwrap_or("")))])).collect();
    let audit=obj([("qualified",num(qualified as f64)),("plain_view_ids",Json::Array(plain.into_iter().map(text).collect())),
        ("typed_view_ids",Json::Array(typed.into_iter().map(text).collect())),("delivered_beliefs",num(conditionals(&ctx)as f64)),
        ("topic_episodes",num(topic_episodes as f64)),("scoped_episodes",num(scoped_episodes as f64)),("pass",Json::Bool(pass)),
        ("candidates",Json::Array(candidates)),("memory",json::parse(&ctx.to_json())?)]);
    Ok(Delivery{context:ctx,system:format!("{} Reply with only the digit 1 or 2, no explanation.",reply.system_prompt()),audit,pass})
}
fn cut(base:&MemoryContext,donor:&MemoryContext,arm:&str)->MemoryContext{
    if arm=="full"{return base.clone()}
    let mut x=base.clone();
    match arm{
        "full"=>{},"no_beliefs"=>x.self_model.retain(|s|s.scope_status!=ScopeStatus::Conditional.token()),
        "no_episodes"=>{x.episodes.clear();x.associations.clear()},
        "swapped"=>{let mut donor=donor.self_model.iter().filter(|s|s.scope_status==ScopeStatus::Conditional.token());
            x.self_model=x.self_model.into_iter().filter_map(|s|if s.scope_status==ScopeStatus::Conditional.token(){donor.next().cloned()}else{Some(s)}).collect();},
        "bare"|"placebo"=>x=MemoryContext{agent_name:base.agent_name.clone(),..Default::default()},_=>unreachable!()}
    x.content_words=x.self_model.iter().map(|s|s.context.split_whitespace().count()+s.statement.split_whitespace().count()).sum::<usize>()
        +x.episodes.iter().map(|e|e.narrative.split_whitespace().count()).sum::<usize>()+x.dispositions.iter().map(|s|s.statement.split_whitespace().count()).sum::<usize>()
        +x.contextual_statements.iter().map(|s|s.statement.split_whitespace().count()).sum::<usize>();x
}
fn user(ctx:&MemoryContext,q:&str)->Json{obj([("working_conversation",text("")),("memory",json::parse(&ctx.to_json()).expect("native JSON")),("human",text(q))])}
fn request(c:&Causal,system:&str,user:Json)->Json{
    let mut x=obj([("model",text(&c.model)),("temperature",num(0.)),("max_tokens",num(16.)),
        ("messages",Json::Array(vec![obj([("role",text("system")),("content",text(system))]),obj([("role",text("user")),("content",text(user.encode()))])]))]);
    if c.provider=="grok"{x.put("reasoning_effort",text("none"))}x
}
fn freeze(c:&Causal)->Result<Vec<Json>,String>{
    // Validate ALL source files before creating an output directory.
    for d in DOMAINS.iter().take(c.pairs){for r in [false,true]{let p=history_path(&c.source,*d,r,true);if !p.is_file(){return Err(format!("missing snapshot {}; no output created",p.display()))}}}
    fs::create_dir_all(c.out.join("snapshots")).map_err(|e|e.to_string())?;
    for d in DOMAINS.iter().take(c.pairs){for r in [false,true]{fs::copy(history_path(&c.source,*d,r,true),history_path(&c.out.join("snapshots"),*d,r,true)).map_err(|e|e.to_string())?;}}
    let mut plan=Vec::new();let mut audit=String::new();let mut exposure=String::from("cell,qualified,plain_view,typed_view,delivered_beliefs,topic_episodes,scoped_episodes,pass\n");
    let mut failed=0;
    let mut deliveries=BTreeMap::new();
    for domain in 0..c.pairs{for reversed in [false,true]{for company in [false,true]{for wording in 0..2{for order in 0..2{
        let t=Trial{domain,reversed,company,wording,order,arm:Arm::On};
        deliveries.insert((domain,reversed,company,wording,order),native(c,t)?);
    }}}}}
    for domain in 0..c.pairs{for reversed in [false,true]{for company in [false,true]{for wording in 0..2{for order in 0..2{
        let t=Trial{domain,reversed,company,wording,order,arm:Arm::On};
        let base=deliveries[&(domain,reversed,company,wording,order)].clone();
        let donor=&deliveries[&(domain,!reversed,company,wording,order)];
        let cell=format!("{domain}:{reversed}:{company}:{wording}:{order}");
        let donor_diff=base.context.self_model.iter().filter(|s|s.scope_status==ScopeStatus::Conditional.token()).map(|s|&s.statement).collect::<Vec<_>>()!=donor.context.self_model.iter().filter(|s|s.scope_status==ScopeStatus::Conditional.token()).map(|s|&s.statement).collect::<Vec<_>>();
        let pass=if c.route=="context-cued-ablated" {beliefs_valid(&base.audit)&&beliefs_valid(&donor.audit)&&donor_diff} else {base.pass&&donor.pass&&donor_diff};if !pass{failed+=1}
        exposure.push_str(&format!("{cell},{},{},{},{},{},{},{pass}\n",base.audit.get("qualified")?.number()?,base.audit.get("plain_view_ids")?.array()?.len(),base.audit.get("typed_view_ids")?.array()?.len(),conditionals(&base.context),base.audit.get("topic_episodes")?.number()?,base.audit.get("scoped_episodes")?.number()?));
        audit.push_str(&obj([("cell",text(&cell)),("base",base.audit), ("donor",donor.audit.clone()),("donor_differs",Json::Bool(donor_diff)),("pass",Json::Bool(pass))]).encode());audit.push('\n');
        let full=user(&base.context,&t.question()).encode();
        for arm in ARMS{
            let ctx=cut(&base.context,&donor.context,arm);let mut u=user(&ctx,&t.question());
            if arm=="placebo"{let gap=full.len().saturating_sub(u.encode().len());if gap>=13{let s="A folder contains routine inventory notes. ";let pad=s.repeat((gap-13)/s.len()+1);u.put("padding",text(&pad[..gap-13]));}}
            let req=request(c,&base.system,u);
            for repeat in 0..c.repeats{plan.push(obj([("id",text(format!("{cell}:{arm}:{repeat}"))),("cell",text(&cell)),("domain",num(domain as f64)),("arm",text(arm)),("repeat",num(repeat as f64)),("order",num(order as f64)),("expected_a",Json::Bool(t.target_a())),("request",req.clone())]));}
        }
    }}}}}
    plan.sort_by_key(|p|rank(c.seed,p.get("id").unwrap().string().unwrap()));
    let payloads=plan.iter().map(Json::encode).collect::<Vec<_>>().join("\n")+"\n";
    fs::write(c.out.join("delivery-audit.jsonl"),&audit).map_err(|e|e.to_string())?;
    fs::write(c.out.join("exposure.csv"),exposure).map_err(|e|e.to_string())?;
    fs::write(c.out.join("manifest.json"),c.manifest()).map_err(|e|e.to_string())?;
    fs::write(c.out.join("preflight.json"),obj([("version",text("native-causal-v2")),("pass",Json::Bool(failed==0)),("cells",num((16*c.pairs)as f64)),("failed_cells",num(failed as f64)),
        ("payload_checksum",text(checksum(&payloads))),("audit_checksum",text(checksum(&audit)))]).encode()).map_err(|e|e.to_string())?;
    fs::write(c.out.join("payloads.jsonl"),payloads).map_err(|e|e.to_string())?;
    if c.rag {
        let mut lines = String::new();
        for domain in 0..c.pairs {
            for reversed in [false, true] {
                let m = load(&c.out.join("snapshots"), DOMAINS[domain], reversed, true)?;
                let q = "choice";
                let ctx = rag_baseline(&m.store, q, 8, c.words);
                lines.push_str(&obj([("domain", text(DOMAINS[domain].id)), ("reversed", Json::Bool(reversed)), ("rag", text(&ctx))]).encode());
                lines.push('\n');
            }
        }
        fs::write(c.out.join("rag.jsonl"), lines).map_err(|e| e.to_string())?;
        eprintln!("wrote {}", c.out.join("rag.jsonl").display());
    }
    Ok(plan)
}
fn beliefs_valid(a:&Json)->bool {a.get("qualified").and_then(Json::number).ok()==Some(2.) && a.get("delivered_beliefs").and_then(Json::number).ok()==Some(1.)}
fn ablated_cell_valid(a:&Json)->bool {a.get("donor_differs").ok()==Some(&Json::Bool(true)) && a.get("base").is_ok_and(beliefs_valid) && a.get("donor").is_ok_and(beliefs_valid)}
fn gate(c:&Causal,plan:&[Json])->Result<(),String>{
    let preflight=json::parse(&fs::read_to_string(c.out.join("preflight.json")).map_err(|e|e.to_string())?)?;
    for (file,key) in [("payloads.jsonl","payload_checksum"),("delivery-audit.jsonl","audit_checksum")]{
        let bytes=fs::read_to_string(c.out.join(file)).map_err(|e|e.to_string())?;
        if preflight.get(key)?.string()?!=checksum(&bytes){return Err(format!("PRECHECK BLOCKED: {file} changed after qualification"))}
    }
    if plan.len()!=96*c.pairs*c.repeats{return Err("invalid/incomplete frozen plan".into())}
    let audit=read_lines(&c.out.join("delivery-audit.jsonl"))?;
    // Ablated route is expected to fail the episode gate: that is the measurement.
    // Still require a frozen plan and checksums. Do not weaken other routes.
    let ablated = c.route == "context-cued-ablated";
    let cells_ok = preflight.get("cells")?.number()? as usize == 16*c.pairs && audit.len() == 16*c.pairs;
    let pass_ok = if ablated { audit.iter().all(ablated_cell_valid) } else {
        preflight.get("pass")? == &Json::Bool(true)
            && audit.iter().all(|a| a.get("pass").ok() == Some(&Json::Bool(true)))
    };
    if preflight.get("version")?.string()? != "native-causal-v2" || !cells_ok || !pass_ok {
        return Err("PRECHECK BLOCKED: at least one cell lacks a delivered belief, relevant scoped episode, or distinct mirror belief. See exposure.csv and delivery-audit.jsonl. ZERO actor HTTP calls made.".into());
    }
    if ablated { eprintln!("gate relaxed for context-cued-ablated (episode absence is the measured effect)"); }
    Ok(())
}
// Detect accidental plan edits on resume; this is not an authentication signature.
fn checksum(s:&str)->String{format!("{:016x}",rank(0,s))}
pub(super) fn prepare(c:&Causal)->Result<(),String>{
    fs::create_dir_all(&c.source).map_err(|e|e.to_string())?;
    let mut log=BufWriter::new(File::create(c.source.join("organ.jsonl")).map_err(|e|e.to_string())?);
    for d in DOMAINS.iter().take(c.pairs){for r in [false,true]{
        build_life(&c.source,*d,r,true,&mut log,80,c.seed)?;
        eprintln!("wrote {}", history_path(&c.source,*d,r,true).display());
    }}
    Ok(())
}
pub(super) fn run(c:Causal)->Result<(),String>{
    if c.prepare { return prepare(&c); }
    let plan=if c.resume{
        if fs::read_to_string(c.out.join("manifest.json")).map_err(|e|e.to_string())?!=c.manifest(){return Err("resume settings/version differ; v1 actor results cannot be reused".into())}
        read_lines(&c.out.join("payloads.jsonl"))?
    }else{freeze(&c)?};
    gate(&c,&plan)?;
    eprintln!("preflight PASS: {}/{} cells, route {}; {} planned calls",16*c.pairs,16*c.pairs,c.route,plan.len());
    if !c.actor{return Ok(())}
    // Gate precedes key lookup, choices-file creation and all network access.
    let key=key_for(&c.provider)?;let path=c.out.join("choices.jsonl");
    let mut rows=if path.exists(){read_lines(&path)?}else{Vec::new()};let mut done=BTreeSet::new();
    for row in &rows{let id=row.get("id")?.string()?;let p=plan.iter().find(|p|p.get("id").unwrap().string().unwrap()==id).ok_or("unknown response id")?;
        if row.get("request")?!=p.get("request")?||!done.insert(id.to_string()){return Err("duplicate/mismatched response".into())}}
    let endpoint=if c.provider=="grok"{"https://api.x.ai/v1/chat/completions"}else{"https://api.openai.com/v1/chat/completions"};
    let mut out=OpenOptions::new().create(true).append(true).open(path).map_err(|e|e.to_string())?;
    for p in plan.iter().filter(|p|!done.contains(p.get("id").unwrap().string().unwrap())).take(c.cap){
        let result=selmem::net::httpx::post_json(endpoint,Some(&key),&p.get("request")?.encode()).and_then(|raw|{let j=json::parse(&raw)?;j.get("choices")?.array()?.first().ok_or_else(||String::from("no choices"))?.get("message")?.get("content")?.string().map(str::to_string)});
        let(a,e)=match result{Ok(a)=>(a,String::new()),Err(e)=>(String::new(),e)};
        let row=obj([("id",p.get("id")?.clone()),("request",p.get("request")?.clone()),("answer",text(a)),("error",text(e))]);
        writeln!(out,"{}",row.encode()).map_err(|e|e.to_string())?;out.flush().map_err(|e|e.to_string())?;rows.push(row);
        eprintln!("causal {}/{}",rows.len(),plan.len());
    }
    summarize(&c,&plan,&rows)?;Ok(())
}
fn chosen(p:&Json,r:&Json)->Result<Option<bool>,String>{Ok(match(p.get("order")?.number()? as usize,r.get("answer")?.string()?.trim()){(0,"1")|(1,"2")=>Some(true),(0,"2")|(1,"1")=>Some(false),_=>None})}
fn summarize(c:&Causal,plan:&[Json],rows:&[Json])->Result<(),String>{
    let by_id=rows.iter().map(|r|Ok((r.get("id")?.string()?.to_string(),r))).collect::<Result<BTreeMap<_,_>,String>>()?;
    let mut values=BTreeMap::new();let mut invalid=0;
    for p in plan{if let Some(r)=by_id.get(p.get("id")?.string()?){let v=chosen(p,r)?;if v.is_none(){invalid+=1}values.insert((p.get("cell")?.string()?.to_string(),p.get("arm")?.string()?.to_string(),p.get("repeat")?.number()? as usize),v);}}
    let mut report=String::from("domain,contrast,valid_pairs,discordant,full_only_concordant,other_only_concordant,invalid_responses,pending\n");
    for domain in 0..c.pairs{for arm in ARMS.into_iter().skip(1).chain(std::iter::once("repeat_noise")){
        let(mut n,mut flip,mut plus,mut minus)=(0,0,0,0);
        for p in plan.iter().filter(|p|p.get("domain").unwrap().number().unwrap() as usize==domain&&p.get("arm").unwrap().string().unwrap()=="full"){
            let cell=p.get("cell")?.string()?.to_string();let r=p.get("repeat")?.number()? as usize;
            let(other,rr)=if arm=="repeat_noise"{if r==0{continue}("full",0)}else{(arm,r)};
            if let(Some(Some(a)),Some(Some(b)))=(values.get(&(cell.clone(),"full".into(),r)),values.get(&(cell,other.into(),rr))){n+=1;if a!=b{flip+=1;if *a==(p.get("expected_a")?==&Json::Bool(true)){plus+=1}else{minus+=1}}}
        }
        report.push_str(&format!("{domain},{arm},{n},{flip},{plus},{minus},{invalid},{}\n",plan.len()-rows.len()));
    }}fs::write(c.out.join("paired.csv"),report).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(route:&str)->(PathBuf,Causal){
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root=std::env::temp_dir().join(format!("selmem-causal-v2-{nonce}"));
        let source=root.join("source");fs::create_dir_all(&source).unwrap();
        let mut log=BufWriter::new(File::create(root.join("organ.jsonl")).unwrap());
        for r in [false,true]{build_life(&source,DOMAINS[0],r,true,&mut log,80,1729).unwrap();}
        let c=Causal{source,out:root.join("run"),pairs:1,repeats:3,words:600,seed:1729,actor:false,resume:false,cap:0,provider:"grok".into(),model:"grok-4.3".into(),route:route.into(),prepare:false,rag:false};
        (root,c)
    }
    #[test]
    fn ablated_gate_retains_belief_and_mirror_requirements(){
        let valid=obj([("qualified",num(2.)),("delivered_beliefs",num(1.)),("topic_episodes",num(0.)),("scoped_episodes",num(0.))]);
        let mut cell=obj([("base",valid.clone()),("donor",valid.clone()),("donor_differs",Json::Bool(true))]);assert!(ablated_cell_valid(&cell));
        cell.put("donor_differs",Json::Bool(false));assert!(!ablated_cell_valid(&cell));cell.put("donor_differs",Json::Bool(true));let mut broken=valid;broken.put("delivered_beliefs",num(0.));cell.put("base",broken);assert!(!ablated_cell_valid(&cell));
    }
    #[test]
    fn rag_budget_and_ties_are_deterministic_and_missing_snapshots_fail(){
        let mut p=EntityProfile::new("Claire");p.encode_threshold=0.;let mut m=SelectiveMemory::new(p);
        for (i,event) in ["token alpha","token beta","token gamma"].iter().enumerate(){let id=format!("obs-{i}");let input=EncodeInput::observed(event,&id);m.live_with(input);}
        let first=rag_baseline(&m.store,"token",8,2);assert!(first.split_whitespace().count()<=2);assert_eq!(rag_baseline(&m.store,"token",8,1),"");
        let traces=m.store.traces.drain().collect::<Vec<_>>();for(id,t)in traces.into_iter().rev(){m.store.traces.insert(id,t);}assert_eq!(first,rag_baseline(&m.store,"token",8,2));
        for t in m.store.traces.values_mut(){t.suppressed=true;}assert_eq!(rag_baseline(&m.store,"token",8,100),"");
        let dir=std::env::temp_dir().join("definitely-missing-selmem-review");assert!(load(&dir,DOMAINS[0],false,true).is_err());assert!(!dir.exists());
    }
    #[test]
    fn failed_exposure_blocks_actor_and_resume_before_key_lookup(){
        let(root,mut c)=fixture("legacy");c.actor=true;c.cap=1;
        let error=run(c).unwrap_err();assert!(error.contains("PRECHECK BLOCKED"),"{error}");
        assert!(!root.join("run/choices.jsonl").exists());
        let c=Causal{source:root.join("source"),out:root.join("run"),pairs:1,repeats:3,words:600,seed:1729,actor:true,resume:true,cap:1,provider:"grok".into(),model:"grok-4.3".into(),route:"legacy".into(),prepare:false,rag:false};
        assert!(run(c).unwrap_err().contains("PRECHECK BLOCKED"));
        assert!(!root.join("run/choices.jsonl").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn qualified_plan_is_frozen_and_component_cuts_preserve_other_channels(){
        let(root,c)=fixture("context-cued");
        let source=fs::read(history_path(&c.source,DOMAINS[0],false,true)).unwrap();
        let plan=freeze(&c).unwrap();assert_eq!(plan.len(),288);gate(&c,&plan).unwrap();
        let mut keys=BTreeSet::new();
        for p in &plan{
            assert!(keys.insert(p.get("id").unwrap().string().unwrap().to_string()));
            let cell=p.get("cell").unwrap();let arm=p.get("arm").unwrap();
            let reference=plan.iter().find(|q|q.get("cell").unwrap()==cell&&q.get("arm").unwrap()==arm&&q.get("repeat").unwrap()==&num(0.)).unwrap();
            assert_eq!(p.get("request").unwrap(),reference.get("request").unwrap());
        }
        let t=Trial{domain:0,reversed:false,company:false,wording:0,order:0,arm:Arm::On};
        let a=native(&c,t).unwrap();let b=native(&c,Trial{reversed:true,..t}).unwrap();
        assert_eq!(cut(&a.context,&b.context,"no_beliefs").episodes[0].narrative,a.context.episodes[0].narrative);
        assert!(cut(&a.context,&b.context,"no_beliefs").self_model.is_empty());
        assert!(cut(&a.context,&b.context,"no_episodes").episodes.is_empty());
        assert_eq!(cut(&a.context,&b.context,"no_episodes").self_model[0].statement,a.context.self_model[0].statement);
        assert_eq!(cut(&a.context,&b.context,"swapped").self_model[0].statement,b.context.self_model[0].statement);
        assert_eq!(source,fs::read(history_path(&c.source,DOMAINS[0],false,true)).unwrap());
        // An accidental request edit must invalidate a previously successful preflight.
        let path=c.out.join("payloads.jsonl");let mut text=fs::read_to_string(&path).unwrap();text.push(' ');fs::write(path,text).unwrap();
        assert!(gate(&c,&plan).unwrap_err().contains("changed after qualification"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_source_does_not_leave_a_partial_run_directory(){
        let(root,mut c)=fixture("legacy");c.pairs=2;assert!(freeze(&c).unwrap_err().contains("missing snapshot"));assert!(!c.out.exists());fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn canonical_choice_is_independent_of_option_position(){
        for(order,digit)in[(0.,"1"),(1.,"2")]{assert_eq!(chosen(&obj([("order",num(order))]),&obj([("answer",text(digit))])).unwrap(),Some(true));}
        assert_eq!(chosen(&obj([("order",num(0.))]),&obj([("answer",text("invalid"))])).unwrap(),None);
    }
}

/// Lexical RAG baseline: rank traces by token overlap with the query,
/// return the top-k gists as a flat context. Same word budget as memory arms.
/// Scaffold for an equal-metadata embedding RAG; no HTTP.
#[allow(dead_code)]
pub(super) fn rag_baseline(store: &selmem::MemoryStore, query: &str, k: usize, words: usize) -> String {
    let q: std::collections::BTreeSet<_> = query.split_whitespace().map(|w| w.to_ascii_lowercase()).collect();
    let mut scored: Vec<_> = store.traces.values().filter(|t| !t.suppressed && t.status==selmem::TraceStatus::Active && t.personal.role==SelfRole::Episode && t.personal.evidence==selmem::SelfEvidence::Observation).map(|t| {
        let g: std::collections::BTreeSet<_> = t.gist.split_whitespace().map(|w| w.to_ascii_lowercase()).collect();
        let overlap = q.intersection(&g).count();
        (overlap, t.id.clone(), t.gist.clone())
    }).collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out = String::new();
    let mut n = 0;
    for (_, _, gist) in scored.into_iter().take(k) {
        let add = gist.split_whitespace().count();
        if n + add > words { continue; }
        if !out.is_empty() { out.push(' '); }
        out.push_str(&gist);
        n += add;
    }
    out
}
