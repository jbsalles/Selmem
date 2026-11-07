//! Context budget sweep: synthetic coverage diagnostic, not an alignment benchmark.
#[allow(dead_code)] #[path="selmem-locomo/json.rs"] mod json;
#[path="selmem-locomo/sha256.rs"] mod sha256;
use json::{Json,obj,text,num};
use selmem::{SelectiveMemory,EntityProfile,EncodeInput,Attribution,MemoryContextOptions};
use std::{fs,io::Write,path::{Path,PathBuf},collections::{BTreeMap,BTreeSet}};
fn s<'a>(j: &'a Json, key: &str) -> &'a str { j.get(key).unwrap().string().unwrap() }
fn n(j: &Json, key: &str) -> usize { j.get(key).unwrap().number().unwrap() as usize }
fn strings(j: &Json) -> Result<Vec<String>, String> { j.array()?.iter().map(|v| v.string().map(str::to_owned)).collect() }
fn array(lines: &[String]) -> Json { Json::Array(lines.iter().map(text).collect()) }
fn read(path: &Path) -> Result<Vec<Json>, String> {
    if !path.exists() { return Ok(vec![]); }
    fs::read_to_string(path).map_err(|e| e.to_string())?.lines().filter(|x| !x.trim().is_empty()).map(json::parse).collect()
}
fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> { fs::write(path, content).map_err(|e| e.to_string()) }
fn append(path: &Path, row: &Json) -> Result<(), String> {
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    writeln!(f, "{}", row.encode()).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())
}
fn lines(rows: &[Json]) -> String { rows.iter().map(Json::encode).collect::<Vec<_>>().join("\n") + "\n" }

fn request_hash(p: &Json) -> String {sha256::hex(p.get("request").unwrap().encode().as_bytes())}
fn latest<'a>(plan: &[Json], rows: &'a [Json], attempts: &[Json]) -> Result<BTreeMap<String,&'a Json>,String> {
    let plans=plan.iter().map(|p|(s(p,"id"),p)).collect::<BTreeMap<_,_>>();
    let mut starts=BTreeSet::new();
    for a in attempts {let p=plans.get(s(a,"id")).ok_or("unknown attempt")?;if s(a,"request_sha256")!=request_hash(p)||!starts.insert((s(a,"id"),n(a,"attempt"))){return Err("invalid or duplicate attempt".into());}}
    let mut ends=BTreeSet::new();let mut result=BTreeMap::new();
    for r in rows {
        let p=plans.get(s(r,"id")).ok_or("unknown result")?;
        if s(r,"request_sha256")!=request_hash(p)||!starts.contains(&(s(r,"id"),n(r,"attempt")))||!ends.insert((s(r,"id"),n(r,"attempt"))){return Err("result/attempt mismatch".into());}
        let id=s(r,"id").to_string();
        if result.get(&id).is_some_and(|old: &&Json|n(old,"attempt")>=n(r,"attempt")){return Err("attempt order is not increasing".into());}
        result.insert(id,r);
    }
    if starts!=ends{return Err("interrupted attempt without result: outcome unknown; no automatic retry".into());}
    Ok(result)
}
fn pending<'a>(plan: &'a [Json], last: &BTreeMap<String,&Json>, retry: bool) -> Vec<&'a Json> {
    plan.iter().filter(|p|last.get(s(p,"id")).map_or(true,|r|retry&&!s(r,"error").is_empty())).collect()
}
fn execute(out: &Path, plan: &[Json], cap: usize, retry: bool,
    mut transport: impl FnMut(&Json)->Result<String,String>) -> Result<(),String> {
    let rows=read(&out.join("choices.jsonl"))?;let attempts=read(&out.join("attempts.jsonl"))?;
    let last=latest(plan,&rows,&attempts)?;
    for p in pending(plan,&last,retry).into_iter().take(cap) {
        let attempt=last.get(s(p,"id")).map_or(1,|r|n(r,"attempt")+1);
        let start=obj([("id",text(s(p,"id"))),("attempt",num(attempt as f64)),("request_sha256",text(request_hash(p)))]);
        append(&out.join("attempts.jsonl"),&start)?;
        let (answer,error,raw)=match transport(p.get("request")?) {
            Ok(raw)=>match json::parse(&raw).and_then(|r|r.get("choices")?.array()?.first().ok_or("empty choices")?.get("message")?.get("content")?.string().map(str::to_owned)){
                Ok(answer)=>(answer,String::new(),raw),Err(error)=>(String::new(),error,raw)},
            Err(error)=>(String::new(),error,String::new())};
        let failed=!error.is_empty();
        append(&out.join("choices.jsonl"),&obj([("id",text(s(p,"id"))),("attempt",num(attempt as f64)),("request_sha256",text(request_hash(p))),("answer",text(answer)),("error",text(error)),("raw_response",text(raw))]))?;
        eprintln!("Recorded {} attempt {}{}",s(p,"id"),attempt,if failed{" (API/response error; stopped)"}else{""});
        if failed{break;}
    }
    Ok(())
}

const BUDGETS:[usize;4]=[1000,2000,4000,8000];
fn task(k:usize)->String{format!("List remembered workshop constraints concerning {}. Include each constraint's W-code and its condition. Do not invent missing constraints.",if k==0{"all topics"}else{["","safety","schedule","materials"][k]})}
fn prepare(out:&Path)->Result<Vec<Json>,String>{
 let mut p=EntityProfile::new("Claire");p.encode_threshold=0.;let path=out.join("snapshot.vault");let mut m=SelectiveMemory::new(p.clone());m.path=Some(path.clone());m.clock.origin_real=4_000_000_000;m.clock.scale=1;
 for i in 0..24{let topic=["safety","schedule","materials"][i%3];let event=format!("Workshop constraint W{i:02} concerns {topic}: station number {i} requires checklist item {} before work can begin. If that item is unavailable, pause this station and ask its supervisor. This condition applies to this station only; completion elsewhere cannot substitute for its own checklist.",100+i);let id=format!("observation-{i}");let mut input=EncodeInput::observed(&event,&id);input.source="workshop-supervisor";input.attribution=Attribution::External;let d=m.live_with(input);if !d.kept{return Err(format!("fixture admission failed: {i}"));}m.advance_hours(0.01);}
 m.save().map_err(|e|e.to_string())?;let mut plan=Vec::new();
 for case in 0..4 {for budget in BUDGETS{
 let mut probe=SelectiveMemory::open(&path,p.clone()).map_err(|e|e.to_string())?;
 let options=MemoryContextOptions{words:64000,token_budget:Some(budget),max_episodes:24,episode_words:800,dispositions:false,associations:false,affect:false,..Default::default()};probe.set_memory_context_options(options)?;
 let draft=probe.open_mouth(&task(case));let c=draft.memory_reply().filtered_context();let payload=c.to_json();if payload.len()>budget{return Err("memory exceeds conservative token bound".into());}
 let expected=(0..24).filter(|i|case==0||i%3==case-1).map(|i|format!("W{i:02}")).collect::<Vec<_>>();let visible=expected.iter().filter(|code|payload.contains(code.as_str())).count();
 let request=obj([("model",text("grok-4.3")),("reasoning_effort",text("none")),("temperature",num(0.)),("max_tokens",num(1200.)),("messages",Json::Array(vec![obj([("role",text("system")),("content",text("Use the supplied remembered evidence. Report conditions accurately and state when evidence is missing."))]),obj([("role",text("user")),("content",text(format!("Memory: {payload}\nTask: {}",task(case))))])]))]);
 for repeat in 0..2{plan.push(obj([("id",text(format!("{case}:{budget}:{repeat}"))),("case",num(case as f64)),("budget",num(budget as f64)),("repeat",num(repeat as f64)),("expected_codes",array(&expected)),("exposed_codes",num(visible as f64)),("episodes",num(c.episodes.len() as f64)),("memory_bytes",num(payload.len() as f64)),("memory",json::parse(&payload)?),("request",request.clone())]));}
 }}
 plan.sort_by_key(|p|sha256::hex(s(p,"id").as_bytes()));Ok(plan)
}
fn report(out:&Path,plan:&[Json])->Result<(),String>{let rows=read(&out.join("choices.jsonl"))?;let attempts=read(&out.join("attempts.jsonl"))?;let last=latest(plan,&rows,&attempts)?;let mut details=Vec::new();for p in plan{let r=last.get(s(p,"id"));let expected=strings(p.get("expected_codes")?)?;let mentioned=r.map(|r|expected.iter().filter(|code|s(r,"answer").contains(code.as_str())).count());details.push(obj([("id",text(s(p,"id"))),("budget",p.get("budget")?.clone()),("expected_codes",num(expected.len() as f64)),("exposed_codes",p.get("exposed_codes")?.clone()),("episodes",p.get("episodes")?.clone()),("memory_bytes",p.get("memory_bytes")?.clone()),("mentioned_expected_codes",mentioned.map_or(Json::Null,|n|num(n as f64))),("answer",r.map_or(Json::Null,|r|text(s(r,"answer")))),("error",r.map_or(Json::Null,|r|text(s(r,"error"))))]));}write(&out.join("comparison.json"),Json::Array(details).encode())}
struct Lock(PathBuf);impl Drop for Lock{fn drop(&mut self){let _=fs::remove_file(&self.0);}}
fn run()->Result<(),String>{
 let(mut out,mut resume,mut actor,mut retry,mut cap)=(PathBuf::new(),false,false,false,0usize);let mut args=std::env::args().skip(1);while let Some(a)=args.next(){match a.as_str(){"--out"=>out=args.next().ok_or("missing output")?.into(),"--resume"=>resume=true,"--actor"=>actor=true,"--retry-errors"=>retry=true,"--max-http-calls"=>cap=args.next().ok_or("missing cap")?.parse().map_err(|_|"invalid cap")?,_=>return Err(format!("unknown flag {a}"))}}
 if out.as_os_str().is_empty()||actor&&(!resume||cap==0)||retry&&(!actor||!resume){return Err("require --out; actor requires --resume and positive cap; retries require actor".into());}
 if !resume{if let Some(parent)=out.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}fs::create_dir(&out).map_err(|_|"output exists or cannot be created")?;}
 fs::OpenOptions::new().write(true).create_new(true).open(out.join("run.lock")).map_err(|_|"output unavailable or locked")?;let _lock=Lock(out.join("run.lock"));
 let fingerprint=sha256::hex(concat!(include_str!("selmem-context-budget-sweep.rs"),include_str!("../engine.rs"),include_str!("../recall/memory_context.rs"),include_str!("../recall/retrieve.rs")).as_bytes());
 let plan=if resume{if fs::read_to_string(out.join("source.sha256")).map_err(|e|e.to_string())?!=fingerprint{return Err("compiled source changed".into());}let bytes=fs::read(out.join("plan.jsonl")).map_err(|e|e.to_string())?;if sha256::hex(&bytes)!=fs::read_to_string(out.join("plan.sha256")).map_err(|e|e.to_string())?{return Err("plan changed".into());}read(&out.join("plan.jsonl"))?}else{let plan=prepare(&out)?;let bytes=lines(&plan);write(&out.join("plan.sha256"),sha256::hex(bytes.as_bytes()))?;write(&out.join("plan.jsonl"),bytes)?;write(&out.join("source.sha256"),fingerprint)?;plan};
 if plan.len()!=32{return Err("expected 32 requests".into());}
 if actor{let key=std::env::var("GROK_API_KEY").map_err(|_|"GROK_API_KEY missing")?;if key.trim().is_empty(){return Err("empty API key".into());}execute(&out,&plan,cap,retry,|r|selmem::net::httpx::post_json("https://api.x.ai/v1/chat/completions",Some(&key),&r.encode()).map(|s|s.replace(&key,"[REDACTED]")).map_err(|e|e.replace(&key,"[REDACTED]")))?;}
 report(&out,&plan)?;eprintln!("32 frozen requests; accounting: UTF-8 byte upper bound, not exact model tokens. See comparison.json.");Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("BUDGET SWEEP: {e}");std::process::exit(1);}}
#[cfg(test)] mod tests{
 use super::*;
 #[test]fn frozen_grid_exposes_budget_effect_without_http(){let out=std::env::temp_dir().join(format!("budget-sweep-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&out).unwrap();let p=prepare(&out).unwrap();assert_eq!(p.len(),32);for row in &p{let count=n(row,"memory_bytes");assert!(count<=n(row,"budget"));let twin=p.iter().find(|q|n(q,"case")==n(row,"case")&&n(q,"budget")==n(row,"budget")&&n(q,"repeat")!=n(row,"repeat")).unwrap();assert_eq!(row.get("request").unwrap(),twin.get("request").unwrap());}let large=p.iter().find(|p|n(p,"case")==0&&n(p,"budget")==8000).unwrap();let small=p.iter().find(|p|n(p,"case")==0&&n(p,"budget")==1000).unwrap();assert!(n(large,"episodes")>4);assert!(n(large,"episodes")>n(small,"episodes"));assert!(!out.join("attempts.jsonl").exists());fs::remove_dir_all(out).unwrap();}
}
