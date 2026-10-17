//! Intermediate creative screening; source narratives are encoded once, never replayed as RAG to native arms.
use super::{creative_life::{self as life,Journal,Lock},creativity,json::{Json,obj,num,text},runner::*,transport};
use std::{collections::BTreeSet,fs,path::Path};
const CONDITIONS:[&str;5]=["empty","rag","before_sleep","after_no_recon","after_full"];
const FROZEN:&str="af468c3b2a08171e636b4dd4b621bb5d56a2eb609e9df3258292f9c5cc971ba2";
fn validate(f:&Json)->Result<()> {
 if s(f,"version")?!="creative-preview-1"||n(f,"nights")?!=10.||n(f,"repeats")?!=2.||n(f,"context_words")?!=600.||array(f,"tasks")?.len()!=3||array(f,"histories")?.len()!=2{return Err("requires frozen two-history, ten-night preview".into());}
 if *f.get("conditions")?!=arr(CONDITIONS.map(text)){return Err("five preview controls required".into());}
 let mut histories=BTreeSet::new();let mut tasks=BTreeSet::new();
 for t in array(f,"tasks")?{if !safe(s(t,"id")?)||!tasks.insert(s(t,"id")?)||s(t,"prompt")?.trim().is_empty(){return Err("invalid task".into());}}
 for h in array(f,"histories")?{if !safe(s(h,"id")?)||!histories.insert(s(h,"id")?)||array(h,"events")?.is_empty(){return Err("invalid history".into());}let mut seen=BTreeSet::new();for e in array(h,"events")?{if !seen.insert(s(e,"id")?)||!flag(e,"external")||flag(e,"critical")||s(e,"text")?.trim().is_empty(){return Err("external unboosted source turns required".into());}}}
 Ok(())
}
fn safe(s:&str)->bool{!s.is_empty()&&s.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_')}
fn run(f:&Json,out:&Path,cfg:&Json,p:&mut Provider)->Result<Vec<Json>> {
 let mut cells=Vec::new();let mut states=Vec::new();
 for h in array(f,"histories")?{
  let id=s(h,"id")?;let events=array(h,"events")?;
  let mut j=Journal::open(Memory::new("selmem_full",out.join("unborn").join(format!("{id}.vault")),cfg),out.join("journals").join(format!("{id}-encode")))?;
  j.call("init","init",obj([("architecture",text("selmem_full")),("associations_learn",Json::Bool(true)),("associations_reconstruct",Json::Bool(true))]))?;
  for (i,e) in events.iter().enumerate(){super::progress::line(format!("PREVIEW ENCODE history={id} event={}/{}",i+1,events.len()));j.call(&format!("{i}/tick"),"tick",obj([("hours",num(1.))]))?;j.call(&format!("{i}/live"),"live",obj([("event",e.clone())]))?;}
  let birth=life::checkpoint(&j.memory,out.join("checkpoints").join(format!("{id}-encoded.vault")),cfg)?;
  let mut awake=Journal::open(birth.clone(),out.join("journals").join(format!("{id}-no-sleep")))?;
  let mut asleep=Journal::open(birth.clone(),out.join("journals").join(format!("{id}-sleep")))?;
  for night in 1..=n(f,"nights")? as usize{
   super::progress::line(format!("PREVIEW SLEEP history={id} night={night}/10"));
   for j in [&mut awake,&mut asleep]{j.call(&format!("night-{night}/tick"),"tick",obj([("hours",num(24.))]))?;}
   asleep.call(&format!("night-{night}/sleep"),"sleep",obj([]))?;
  }
  let before=life::checkpoint(&awake.memory,out.join("checkpoints").join(format!("{id}-no-sleep.vault")),cfg)?;
  let after=life::checkpoint(&asleep.memory,out.join("checkpoints").join(format!("{id}-after-sleep.vault")),cfg)?;
  if before.call("inspect",obj([]))?.get("clock")?!=after.call("inspect",obj([]))?.get("clock")?{return Err("sleep/no-sleep clocks differ".into());}
  for m in [&birth,&before,&after]{states.push((m.path.clone(),file_hash(&m.path)?));}
  for task in array(f,"tasks")?{
   let query=s(task,"prompt")?;
   let (full_ctx,full_audit)=life::context(&after,query,true,out)?;
   let (cut_ctx,cut_audit)=life::context(&after,query,false,out)?;
   if full_audit.get("selected_ids")?!=cut_audit.get("selected_ids")?{return Err("probe reconstruction cut changed selection".into());}
   for condition in CONDITIONS{
    let source=match condition{"before_sleep"=>Some(&before),"after_full"|"after_no_recon"=>Some(&after),_=>None};
    let (ctx,audit)=match condition{
     "empty"=>(String::new(),Json::Null),"rag"=>creativity::rag(events,query,600)?,
     "before_sleep"=>life::context(&before,query,true,out)?,
     "after_full"=>(full_ctx.clone(),full_audit.clone()),_=>(cut_ctx.clone(),cut_audit.clone())};
    super::progress::line(format!("PREVIEW CONTEXT {id}/{}/{condition} words={}",s(task,"id")?,ctx.split_whitespace().count()));
    let record=obj([("history",text(id)),("task",task.clone()),("condition",text(condition)),("context",text(&ctx)),("audit",audit),
     ("snapshot",source.map(|m|text(m.path.to_string_lossy())).unwrap_or(Json::Null)),("snapshot_sha256",source.map(|m|file_hash(&m.path).map(text)).transpose()?.unwrap_or(Json::Null))]);
    let key=format!("{id}-{}-{condition}",s(task,"id")?);life::commit(&out.join("contexts").join(format!("{key}.json")),&record)?;
    for repeat in 0..2{cells.push(merge(&record,&obj([("trial_id",text(format!("{id}/{}/{condition}/{repeat}",s(task,"id")?))),("repeat",num(repeat as f64)),("generation_file",text(format!("{key}-{repeat}.json")))])));}
   }
  }
 }
 Rng::new(2026101701).shuffle(&mut cells);let mut all=Vec::new();
 for cell in cells{super::progress::line(format!("PREVIEW CREATIVE proposal={}/60",all.len()+1));let g=life::generation(p,cell.get("task")?,s(&cell,"context")?,2026101700+n(&cell,"repeat")? as u64,&out.join("generations").join(s(&cell,"generation_file")?))?;all.push(merge(&cell,&obj([("generation",g)])));}
 for (path,hash) in states{if file_hash(&path)?!=hash{return Err("source checkpoint changed during creative evaluation".into());}}
 Ok(all)
}
pub(super) fn cli(args:Vec<String>)->Result<()> {
 let (flags,sw)=life::flags(args)?;if flags.keys().any(|k|["--source","--plan"].contains(&k.as_str())){return Err("preview accepts --fixture, not --source/--plan".into());}
 let mock=sw.contains("--mock");let technical=sw.contains("--technical-smoke");if technical&&!mock{return Err("contracted preview fixtures are mock-only".into());}
 let fixture=Path::new(flags.get("--fixture").ok_or("--fixture required")?);let f=load(fixture)?;validate(&f)?;
 if !technical&&file_hash(fixture)?!=FROZEN{return Err("requires exact frozen creative-preview.json; do not edit tasks after collection".into());}
 let out=Path::new(flags.get("--out").ok_or("--out required")?);let resume=sw.contains("--resume");if out.exists()!=resume{return Err("choose fresh output or --resume this exact preview".into());}
 if selmem::Config::get().temp().parse::<f64>().map_err(|_|"invalid temperature")?!=0.{return Err("memory temperature must be zero".into());}
 let budget=flags.get("--max-http-calls").map(|s|s.parse::<usize>()).transpose().map_err(|_|"invalid HTTP ceiling")?.unwrap_or(3000);if !(1..=50000).contains(&budget){return Err("HTTP ceiling 1..50000".into());}
 let mut cfg=campaign_config(selmem::Config::get(),&flags,true,mock)?;for key in ["associations_learn","associations_reconstruct","semantic_annotation_cache"]{cfg.put(key,Json::Bool(true));}cfg.put("observation_evidence",Json::Bool(false));
 let mut p=life::provider(&flags,mock,out)?;
 let design=obj([("version",text("creative-preview-1")),("fixture",f.clone()),("fixture_sha256",text(file_hash(fixture)?)),("config",cfg.clone()),("provider",p.metadata()),("code_sha256",text(life::code_hash()?)),("technical",Json::Bool(technical)),("cumulative_http_ceiling",num(budget as f64)),("planned_proposals",num(60.)),("system",text(creativity::SYSTEM))]);
 if !resume{fs::create_dir_all(out).map_err(|e|e.to_string())?;}let _lock=Lock::acquire(out)?;
 if resume{let old=load(&out.join("preview-design.json"))?;let mut comparable=old.clone();comparable.put("cumulative_http_ceiling",num(budget as f64));if comparable!=design||n(&old,"cumulative_http_ceiling")?>budget as f64{return Err("resume requires identical fixture/code/provider/reasoning/config; only a higher HTTP ceiling is allowed".into());}if old!=design{log(&out.join("ceiling-amendments.jsonl"),&obj([("before",old.get("cumulative_http_ceiling")?.clone()),("after",num(budget as f64))]))?;}}
 life::commit(&out.join("preview-design.json"),&design)?;let _session=transport::resume(out,budget)?;transport::enable_semantic_cache(true);super::progress::enable(true);
 let result=if sw.contains("--prepare-only"){Ok(vec![])}else{run(&f,out,&cfg,&mut p)};
 if let Ok(all)=&result{if !sw.contains("--prepare-only"){if all.len()!=60{return Err("preview proposal count differs from plan".into());}life::commit(&out.join("all-proposals.json"),&arr(all.clone()))?;creativity::blind(out,all)?;}}
 let completed=if let Ok(all)=&result{all.len()}else{fs::read_dir(out.join("generations")).map(|xs|xs.filter_map(|e|e.ok()).filter(|e|e.path().extension().is_some_and(|x|x=="json")).count()).unwrap_or(0)};
 life::commit(&out.join("preview.json"),&obj([("status",text(if result.is_err(){"paused_with_error"}else if sw.contains("--prepare-only"){"prepared"}else if mock{"technical_rehearsal_complete"}else{"collected_unrated"})),("mock",Json::Bool(mock)),("technical",Json::Bool(technical)),("planned_proposals",num(60.)),("completed_proposals",num(completed as f64)),("http_attempts_cumulative",num(transport::used() as f64)),("semantic_annotation_cache",transport::semantic_cache_stats()),("error",result.as_ref().err().map(text).unwrap_or(Json::Null)),("claim",text("Exploratory unrated screening; no automatic creativity, source causality or unconsciousness claim."))]))?;result.map(|_|())
}

#[cfg(test)]mod tests{
 use super::*;
 fn tiny()->Json{let mut f=super::super::json::parse(include_str!("../../../experiments/genesis/creative-preview.json")).unwrap();let mut hs=array(&f,"histories").unwrap().to_vec();for h in &mut hs{h.put("events",arr(array(h,"events").unwrap().iter().take(3).cloned()));}f.put("histories",arr(hs));f}
 #[test]fn matched_clocks_same_sources_readonly_and_resume(){
  let out=super::super::lifecycle_tests::root();fs::create_dir_all(&out).unwrap();let cfg=super::super::json::parse(include_str!("../../../experiments/genesis/config.json")).unwrap();let f=tiny();validate(&f).unwrap();let mut p=Provider::new(true,"mock".into(),"".into(),None,&out);
  let first=run(&f,&out,&cfg,&mut p).unwrap();assert_eq!(first.len(),60);
  for h in array(&f,"histories").unwrap(){let id=s(h,"id").unwrap();let state=|suffix:&str|load(&out.join("checkpoints").join(format!("{id}-{suffix}.state.json"))).unwrap();assert_eq!(state("no-sleep").get("clock").unwrap(),state("after-sleep").get("clock").unwrap());assert_eq!(n(&state("no-sleep"),"clock").unwrap()-n(&state("encoded"),"clock").unwrap(),240.*3600.);}
  let hashes=fs::read_dir(out.join("checkpoints")).unwrap().map(|p|{let p=p.unwrap().path();let h=file_hash(&p).unwrap();(p,h)}).collect::<Vec<_>>();assert_eq!(run(&f,&out,&cfg,&mut p).unwrap(),first);for(p,h)in hashes{assert_eq!(file_hash(&p).unwrap(),h);}
  fs::remove_dir_all(out).unwrap();
 }
 #[test]fn preview_rejects_missing_controls_and_boosts(){let mut f=tiny();f.put("conditions",arr([text("after_full")]));assert!(validate(&f).is_err());let mut f=tiny();let mut hs=array(&f,"histories").unwrap().to_vec();let mut es=array(&hs[0],"events").unwrap().to_vec();es[0].put("critical",Json::Bool(true));hs[0].put("events",arr(es));f.put("histories",arr(hs));assert!(validate(&f).is_err());}
}
