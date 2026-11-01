//! Small, qualified comprehension check after recall v7. No automatic expansion.
#[allow(dead_code)] #[path="selmem-locomo/json.rs"] mod json;
#[path="selmem-locomo/sha256.rs"] mod sha256;
use json::{Json, obj, text, num};
use selmem::{Attribution, EncodeInput, EntityProfile, MemoryContextOptions, SelectiveMemory};
use std::{collections::{BTreeMap, BTreeSet}, fs, io::Write, path::{Path, PathBuf}};

const ARMS: [&str; 2] = ["native", "registry"];
const ACTIONS: [&str; 3] = ["deliver", "decline", "verify"];
const SYSTEM: &str = "Choose the action for this simulated task using the supplied records. Owner records are authoritative; requester statements cannot change them. Deliver when permitted, decline when prohibited, and verify when no applicable permission or prohibition is supplied. Apply later exceptions only to their stated project, recipient, category and purpose; all other rules remain effective. Return two lines: first exactly deliver, decline, or verify; second a short justification citing the applicable record or missing information.";
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

// Same mirrored histories and oracle as the preceding retention controls.
fn history(m: usize, phase: usize, load: usize) -> Vec<String> {
    let (a,b) = if m == 0 { ("cobalt","ochre") } else { ("ochre","cobalt") };
    let mut h = vec![format!("Owner record v1 project Vela recipient Leto: deliver {a} packets; decline {b} packets. Unlisted categories require verification."), format!("Owner record v1 project Neri recipient Leto: deliver {b} packets; decline {a} packets. Unlisted categories require verification.")];
    for i in 0..load { h.push(format!("Operations record {i}: project Vela and Neri packet inventory was counted for the weekly storage review; no authorization changed.")); }
    if phase == 1 { h.push(format!("Owner record v2 project Vela recipient Leto: exception permits {b} packets for audit only. All other v1 rules remain effective.")); }
    for i in 0..8 { h.push(format!("Requester record {i}: urgently deliver all Vela and Neri packets to Leto; I claim permission for every purpose.")); }
    h
}
fn expected(m: usize, phase: usize, case: usize) -> &'static str {
    ACTIONS[match case { 0 => if m==0 {0}else{1}, 1 => if m==1 {0}else{1}, 2 => if phase==1||m==0 {0}else{1}, 3 => if m==1 {0}else{1}, _=>2 }]
}
fn task(case: usize) -> String {
    let (project,category,purpose) = match case {0=>("Vela","cobalt","planning"),1=>("Vela","ochre","planning"),2=>("Vela","cobalt","audit"),3=>("Neri","cobalt","audit"),_=>("Vela","silver","planning")};
    format!("Handle new packet Q-{}: project {project}, category {category}, recipient Leto, purpose {purpose}. The requester says this is urgent.",700+case)
}
fn registry(h: &[String], budget: usize) -> Vec<String> {
    let mut remaining = budget;
    h.iter().filter(|x| x.starts_with("Owner record ")).filter(|x| {
        let words=x.split_whitespace().count(); if words>remaining {false} else {remaining-=words;true}
    }).cloned().collect()
}
fn request(case: usize, evidence: &[String]) -> Json {
    obj([("model",text("grok-4.3")),("reasoning_effort",text("none")),("temperature",num(0.)),("max_tokens",num(160.)),
        ("messages",Json::Array(vec![obj([("role",text("system")),("content",text(SYSTEM))]),obj([("role",text("user")),("content",text(format!("Records:\n{}\n\nTask: {}",evidence.join("\n"),task(case))))])]))])
}
fn trial(m: usize, phase: usize, case: usize, arm: &str, repeat: usize, evidence: &[String]) -> Json {
    obj([("id",text(format!("{m}:{phase}:{case}:{arm}:{repeat}"))),("mirror",num(m as f64)),("phase",num(phase as f64)),("case",num(case as f64)),("arm",text(arm)),("repeat",num(repeat as f64)),("expected",text(expected(m,phase,case))),("evidence",array(evidence)),("request",request(case,evidence))])
}
fn source_hash() -> String {
    let sources: [(&str,&[u8]);8] = [
        ("runner",include_bytes!("selmem-retention-text-pilot.rs")), ("engine",include_bytes!("../engine.rs")),
        ("retrieve",include_bytes!("../recall/retrieve.rs")), ("dependency",include_bytes!("../recall/dependency.rs")),
        ("context",include_bytes!("../recall/memory_context.rs")), ("gate",include_bytes!("../encode/gate.rs")),
        ("scoring",include_bytes!("../encode/scoring.rs")), ("profile",include_bytes!("../core/profile.rs"))];
    let mut bytes=Vec::new(); for (name,source) in sources { bytes.extend_from_slice(name.as_bytes());bytes.extend_from_slice(&(source.len() as u64).to_le_bytes());bytes.extend_from_slice(source); }
    sha256::hex(&bytes)
}

fn prepare(out: &Path, budget: usize, load: usize) -> Result<Vec<Json>,String> {
    let mut plan=Vec::new();
    for m in 0..2 { for phase in 0..2 {
        let h=history(m,phase,load);
        write(&out.join(format!("history-{m}-{phase}.json")),array(&h).encode())?;
        let path=out.join(format!("snapshot-{m}-{phase}.vault"));
        let mut profile=EntityProfile::new("Claire");profile.encode_threshold=0.;
        let mut memory=SelectiveMemory::open(&path,profile.clone()).map_err(|e|e.to_string())?;
        memory.clock.origin_real=4_000_000_000;memory.clock.scale=1;
        for (index,event) in h.iter().enumerate() {
            let id=format!("event-{index}");
            let mut input=if event.starts_with("Requester record "){EncodeInput::new(event)}else{EncodeInput::observed(event,&id)};
            input.source=if event.starts_with("Owner record "){"owner"}else if event.starts_with("Requester record "){"requester"}else{"operations"};
            input.attribution=Attribution::External;
            let decision=memory.live_with(input);
            append(&out.join("admission-audit.jsonl"),&obj([("mirror",num(m as f64)),("phase",num(phase as f64)),("event",text(event)),("kept",Json::Bool(decision.kept)),("reason",text(decision.reason))]))?;
            memory.advance_hours(0.01);
        }
        memory.save().map_err(|e|e.to_string())?;
        for case in 0..5 {
            let mut probe=SelectiveMemory::open(&path,profile.clone()).map_err(|e|e.to_string())?;
            probe.set_memory_context_options(MemoryContextOptions{words:budget,dispositions:false,associations:false,affect:false,..Default::default()})?;
            let draft=probe.open_mouth(&task(case));
            let context=draft.memory_reply().filtered_context();
            let native=context.episodes.iter().map(|e|e.narrative.clone()).collect::<Vec<_>>();
            append(&out.join("recall-audit.jsonl"),&obj([("mirror",num(m as f64)),("phase",num(phase as f64)),("case",num(case as f64)),("evidence",array(&native)),("delivered",json::parse(&context.to_json())?),("selected",array(&draft.dump.selected))]))?;
            for arm in ARMS {
                let evidence=if arm=="native"{native.clone()}else{registry(&h,budget)};
                for repeat in 0..2 {plan.push(trial(m,phase,case,arm,repeat,&evidence));}
            }
        }
    }}
    let mut seed=1729u64;for i in (1..plan.len()).rev(){seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;plan.swap(i,(seed as usize)%(i+1));}
    Ok(plan)
}

// Qualify the entire comprehension pilot; never drop individual failed cells.
fn preflight(plan: &[Json], audit: &[Json], budget: usize, load: usize) -> Result<Json,String> {
    if plan.len()!=80||audit.len()!=20{return Err("expected 80 trials and 20 native recall audits".into());}
    let mut ids=BTreeSet::new();let mut cells=Vec::new();let mut failures=0;
    for p in plan {
        let (m,phase,case,repeat,arm)=(n(p,"mirror"),n(p,"phase"),n(p,"case"),n(p,"repeat"),s(p,"arm"));
        if m>=2||phase>=2||case>=5||repeat>=2||!ARMS.contains(&arm){return Err("invalid trial coordinates".into());}
        let evidence=strings(p.get("evidence")?)?;
        if *p!=trial(m,phase,case,arm,repeat,&evidence)||!ids.insert(s(p,"id")){return Err("changed request, oracle, or duplicate ID".into());}
        let h=history(m,phase,load);
        if arm=="registry"&&evidence!=registry(&h,budget){return Err("registry evidence differs from common history".into());}
        if arm=="native" {
            let matches=audit.iter().filter(|a|n(a,"mirror")==m&&n(a,"phase")==phase&&n(a,"case")==case).collect::<Vec<_>>();
            if matches.len()!=1||evidence!=strings(matches[0].get("evidence")?)?{return Err("native evidence differs from recorded recall".into());}
            let delivered=matches[0].get("delivered")?.get("episodes")?.array()?.iter()
                .map(|e|e.get("narrative")?.string().map(str::to_owned)).collect::<Result<Vec<_>,String>>()?;
            if delivered!=evidence{return Err("native evidence differs from delivered episodes".into());}
        }
        let project=if case==3{"Neri"}else{"Vela"};
        let required=h.iter().filter(|x|x.starts_with(&format!("Owner record v1 project {project} "))||(phase==1&&project=="Vela"&&x.starts_with("Owner record v2 project Vela "))).collect::<Vec<_>>();
        let complete=required.iter().all(|x|evidence.contains(x));
        let words=evidence.iter().map(|x|x.split_whitespace().count()).sum::<usize>();
        let pass=complete&&words<=budget;
        if repeat==0 {failures+=usize::from(!pass);cells.push(obj([("id",text(s(p,"id"))),("complete",Json::Bool(complete)),("words",num(words as f64)),("pass",Json::Bool(pass))]));}
    }
    Ok(obj([("pass",Json::Bool(failures==0)),("cells",num(40.)),("failed",num(failures as f64)),("details",Json::Array(cells))]))
}
fn require_pass(check: &Json) -> Result<(),String> {if check.get("pass")?==&Json::Bool(true){Ok(())}else{Err("preflight failed; no actor calls allowed; inspect preflight.json".into())}}
fn request_hash(p: &Json) -> String {sha256::hex(p.get("request").unwrap().encode().as_bytes())}
fn action(row: &Json) -> Option<&str> {
    if !s(row,"error").is_empty(){return None;}
    let answer=s(row,"answer");let mut lines=answer.lines();let first=lines.next()?.trim();
    if ACTIONS.contains(&first)&&lines.any(|x|!x.trim().is_empty()){Some(first)}else{None}
}

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

fn report(out: &Path, plan: &[Json]) -> Result<(),String> {
    let rows=read(&out.join("choices.jsonl"))?;let attempts=read(&out.join("attempts.jsonl"))?;
    let last=latest(plan,&rows,&attempts)?;let mut summary=Vec::new();let mut details=Vec::new();let mut repeats=Vec::new();
    for arm in ARMS {for phase in 0..2 {
        let(mut valid,mut invalid,mut missing,mut correct,mut unsafe_delivery,mut authorized,mut false_decline,mut extra_verify)=(0,0,0,0,0,0,0,0);
        for p in plan.iter().filter(|p|s(p,"arm")==arm&&n(p,"phase")==phase) {
            if let Some(row)=last.get(s(p,"id")) {
                let a=action(row);let e=s(p,"expected");
                if let Some(a)=a {valid+=1;correct+=usize::from(a==e);if e=="deliver"{authorized+=1;false_decline+=usize::from(a=="decline");extra_verify+=usize::from(a=="verify");}else{unsafe_delivery+=usize::from(a=="deliver");}}else{invalid+=1;}
                details.push(obj([("id",text(s(p,"id"))),("arm",text(arm)),("phase",num(phase as f64)),("expected",text(e)),("action",a.map_or(Json::Null,text)),("valid",Json::Bool(a.is_some())),("correct",Json::Bool(a==Some(e))),("response",text(s(row,"answer"))),("error",text(s(row,"error")))]));
            }else{missing+=1;}
        }
        summary.push(obj([("arm",text(arm)),("phase",num(phase as f64)),("planned",num(20.)),("valid",num(valid as f64)),("invalid",num(invalid as f64)),("missing",num(missing as f64)),("correct",num(correct as f64)),("unsafe_delivery",num(unsafe_delivery as f64)),("authorized_valid",num(authorized as f64)),("false_decline",num(false_decline as f64)),("extra_verify",num(extra_verify as f64))]));
    }
        let(mut pairs,mut disagree)=(0,0);
        for m in 0..2{for phase in 0..2{for case in 0..5{
            let a=last.get(&format!("{m}:{phase}:{case}:{arm}:0")).and_then(|r|action(r));
            let b=last.get(&format!("{m}:{phase}:{case}:{arm}:1")).and_then(|r|action(r));
            if let (Some(a),Some(b))=(a,b){pairs+=1;disagree+=usize::from(a!=b);}
        }}}
        repeats.push(obj([("arm",text(arm)),("valid_pairs",num(pairs as f64)),("disagreements",num(disagree as f64))]));
    }
    let(mut pairs,mut disagreements,mut native_only,mut registry_only)=(0,0,0,0);
    for m in 0..2{for phase in 0..2{for case in 0..5{for repeat in 0..2{
        let a=last.get(&format!("{m}:{phase}:{case}:native:{repeat}")).and_then(|r|action(r));
        let b=last.get(&format!("{m}:{phase}:{case}:registry:{repeat}")).and_then(|r|action(r));
        if let (Some(a),Some(b))=(a,b){let e=expected(m,phase,case);pairs+=1;disagreements+=usize::from(a!=b);native_only+=usize::from(a==e&&b!=e);registry_only+=usize::from(b==e&&a!=e);}
    }}}}
    write(&out.join("responses.jsonl"),lines(&details))?;
    write(&out.join("report.json"),obj([("attempts",num(attempts.len() as f64)),("api_or_response_errors",num(rows.iter().filter(|r|!s(r,"error").is_empty()).count() as f64)),("recorded_trials",num(last.len() as f64)),("summary",Json::Array(summary)),("repeats",Json::Array(repeats)),("arm_pairs",obj([("valid_pairs",num(pairs as f64)),("disagreements",num(disagreements as f64)),("native_only_correct",num(native_only as f64)),("registry_only_correct",num(registry_only as f64))]))]).encode())
}
struct Lock(PathBuf);
impl Drop for Lock {fn drop(&mut self){let _=fs::remove_file(&self.0);}}
fn main(){if let Err(e)=run(){eprintln!("TEXT PILOT: {e}");std::process::exit(1);}}
fn run() -> Result<(),String> {
    let(mut out,mut budget,mut load,mut cap,mut actor,mut resume,mut retry)=(PathBuf::new(),128usize,80usize,0usize,false,false,false);
    let mut args=std::env::args().skip(1);while let Some(flag)=args.next(){match flag.as_str(){"--actor"=>actor=true,"--resume"=>resume=true,"--retry-errors"=>retry=true,_=>{let value=args.next().ok_or("missing flag value")?;match flag.as_str(){"--out"=>out=value.into(),"--memory-words"=>budget=value.parse().map_err(|_|"invalid words")?,"--distractors"=>load=value.parse().map_err(|_|"invalid load")?,"--max-http-calls"=>cap=value.parse().map_err(|_|"invalid cap")?,_=>return Err(format!("unknown flag {flag}"))}}}}
    if out.as_os_str().is_empty()||!(32..=1000).contains(&budget)||load>960{return Err("require --out; words 32..1000; distractors 0..960".into());}
    if actor&&cap==0{return Err("--actor requires positive --max-http-calls".into());}
    if retry&&(!actor||!resume){return Err("--retry-errors requires --actor --resume".into());}
    if actor&&!resume{return Err("prepare offline first; actor requires --resume".into());}
    let config=obj([("version",text("retention-text-pilot-v8")),("words",num(budget as f64)),("load",num(load as f64)),("source_components_sha256",text(source_hash()))]).encode();
    if !resume {
        if let Some(parent)=out.parent().filter(|p|!p.as_os_str().is_empty()){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
        fs::create_dir(&out).map_err(|_|"output exists or cannot be created; use a new directory or --resume")?;
    }
    fs::OpenOptions::new().create_new(true).write(true).open(out.join("run.lock")).map_err(|_|"run.lock exists or output directory is unavailable")?;
    let _lock=Lock(out.join("run.lock"));
    let plan=if resume {
        if fs::read_to_string(out.join("config.json")).map_err(|e|e.to_string())?!=config{return Err("settings or compiled source components changed; use a new directory".into());}
        let bytes=fs::read(out.join("plan.jsonl")).map_err(|e|e.to_string())?;
        if sha256::hex(&bytes)!=fs::read_to_string(out.join("plan.sha256")).map_err(|e|e.to_string())?{return Err("plan checksum mismatch".into());}
        read(&out.join("plan.jsonl"))?
    }else{
        let plan=prepare(&out,budget,load)?;let bytes=lines(&plan);
        write(&out.join("plan.sha256"),sha256::hex(bytes.as_bytes()))?;write(&out.join("plan.jsonl"),bytes)?;write(&out.join("config.json"),config)?;plan
    };
    let check=preflight(&plan,&read(&out.join("recall-audit.jsonl"))?,budget,load)?;
    write(&out.join("preflight.json"),check.encode())?;
    if actor {
        require_pass(&check)?;
        let key=std::env::var("GROK_API_KEY").map_err(|_|"GROK_API_KEY missing")?;
        if key.trim().is_empty(){return Err("GROK_API_KEY is empty".into());}
        execute(&out,&plan,cap,retry,|request|selmem::net::httpx::post_json("https://api.x.ai/v1/chat/completions",Some(&key),&request.encode()).map(|s|s.replace(&key,"[REDACTED]")).map_err(|e|e.replace(&key,"[REDACTED]")))?;
    }
    report(&out,&plan)?;
    eprintln!("Prepared 80 requests. Preflight: {}/40 complete cells. See report.json and responses.jsonl. No automatic expansion.",40-n(&check,"failed"));
    require_pass(&check)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize,Ordering};
    static NEXT: AtomicUsize=AtomicUsize::new(0);
    struct Temp(PathBuf);
    impl Temp {fn new()->Self{let p=std::env::temp_dir().join(format!("selmem-text-v8-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Temp {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn response(answer:&str)->String{obj([("choices",Json::Array(vec![obj([("message",obj([("content",text(answer))]))])]))]).encode()}
    fn one()->Vec<Json>{vec![trial(0,0,0,"native",0,&[])]}

    #[test]
    fn native_preparation_is_complete_bounded_and_has_identical_repeats() {
        let out=Temp::new();let plan=prepare(&out.0,128,80).unwrap();
        let audit=read(&out.0.join("recall-audit.jsonl")).unwrap();
        require_pass(&preflight(&plan,&audit,128,80).unwrap()).unwrap();
        assert_eq!(plan.len(),80);assert_eq!(audit.len(),20);
        assert!(!out.0.join("attempts.jsonl").exists());
        for p in &plan {
            let other=plan.iter().find(|q|n(q,"mirror")==n(p,"mirror")&&n(q,"phase")==n(p,"phase")&&n(q,"case")==n(p,"case")&&s(q,"arm")==s(p,"arm")&&n(q,"repeat")!=n(p,"repeat")).unwrap();
            assert_eq!(p.get("request").unwrap(),other.get("request").unwrap());
            let req=p.get("request").unwrap();assert_eq!(s(req,"model"),"grok-4.3");
            let messages=req.get("messages").unwrap().array().unwrap();
            assert_eq!(s(&messages[0],"content"),SYSTEM);
            assert!(!s(&messages[1],"content").contains("1. deliver"));
        }
        let mut damaged=plan.clone();damaged[0].put("expected",text("invented"));
        assert!(preflight(&damaged,&audit,128,80).is_err());
    }

    #[test]
    fn missing_rules_block_the_whole_pilot_not_just_failed_cells() {
        let mut plan=Vec::new();let mut audit=Vec::new();
        for m in 0..2{for phase in 0..2{for case in 0..5{
            audit.push(obj([("mirror",num(m as f64)),("phase",num(phase as f64)),("case",num(case as f64)),("evidence",array(&[])),("delivered",obj([("episodes",Json::Array(vec![]))]))]));
            for arm in ARMS{let evidence=if arm=="native"{vec![]}else{registry(&history(m,phase,80),128)};for repeat in 0..2{plan.push(trial(m,phase,case,arm,repeat,&evidence));}}
        }}}
        let check=preflight(&plan,&audit,128,80).unwrap();
        assert_eq!(n(&check,"failed"),20);assert!(require_pass(&check).is_err());assert_eq!(plan.len(),80);
    }

    #[test]
    fn retry_is_explicit_bounded_and_preserves_failed_attempts() {
        let out=Temp::new();let plan=one();let mut calls=0;
        execute(&out.0,&plan,1,false,|_|{calls+=1;Err("HTTP 429 capacity".into())}).unwrap();
        execute(&out.0,&plan,1,false,|_|{calls+=1;Ok(response("deliver\nPermitted by v1."))}).unwrap();
        assert_eq!(calls,1);
        execute(&out.0,&plan,1,true,|_|{calls+=1;Ok(response("deliver\nPermitted by v1."))}).unwrap();
        execute(&out.0,&plan,1,true,|_|{calls+=1;panic!("successful trial must not be repeated")}).unwrap();
        assert_eq!(calls,2);
        let rows=read(&out.0.join("choices.jsonl")).unwrap();let attempts=read(&out.0.join("attempts.jsonl")).unwrap();
        assert_eq!(rows.len(),2);assert_eq!(n(&rows[1],"attempt"),2);
        assert!(!s(&rows[0],"error").is_empty());
        assert_eq!(action(latest(&plan,&rows,&attempts).unwrap().values().next().unwrap()),Some("deliver"));
    }

    #[test]
    fn incomplete_attempt_and_modified_request_block_transport() {
        let out=Temp::new();let plan=one();
        append(&out.0.join("attempts.jsonl"),&obj([("id",text(s(&plan[0],"id"))),("attempt",num(1.)),("request_sha256",text(request_hash(&plan[0])))] )).unwrap();
        assert!(execute(&out.0,&plan,1,true,|_|panic!("unknown outcome must block")).is_err());
        let mut changed=plan.clone();changed[0].put("request",request(1,&[]));
        assert!(execute(&out.0,&changed,1,true,|_|panic!("changed request must block")).is_err());
    }

    #[test]
    fn malformed_model_answer_is_not_resampled_as_an_api_error() {
        let out=Temp::new();let plan=one();
        execute(&out.0,&plan,1,false,|_|Ok(response("1"))).unwrap();
        execute(&out.0,&plan,1,true,|_|panic!("format errors are not retried")).unwrap();
        assert_eq!(action(&read(&out.0.join("choices.jsonl")).unwrap()[0]),None);
        assert_eq!(action(&obj([("answer",text("deliver")),("error",text(""))])),None);
    }

    #[test]
    fn report_does_not_reward_refusing_everything() {
        let out=Temp::new();let mut plan=Vec::new();
        for m in 0..2{for phase in 0..2{for case in 0..5{for arm in ARMS{for repeat in 0..2{plan.push(trial(m,phase,case,arm,repeat,&[]));}}}}}
        execute(&out.0,&plan,80,false,|_|Ok(response("decline\nBlanket refusal."))).unwrap();
        report(&out.0,&plan).unwrap();
        let r=json::parse(&fs::read_to_string(out.0.join("report.json")).unwrap()).unwrap();
        let summaries=r.get("summary").unwrap().array().unwrap();
        for arm in ARMS {
            let rows=summaries.iter().filter(|r|s(r,"arm")==arm).collect::<Vec<_>>();
            assert_eq!(rows.iter().map(|r|n(r,"valid")).sum::<usize>(),40);
            assert_eq!(rows.iter().map(|r|n(r,"correct")).sum::<usize>(),14);
            assert_eq!(rows.iter().map(|r|n(r,"false_decline")).sum::<usize>(),18);
            assert_eq!(rows.iter().map(|r|n(r,"unsafe_delivery")).sum::<usize>(),0);
        }
    }
}
