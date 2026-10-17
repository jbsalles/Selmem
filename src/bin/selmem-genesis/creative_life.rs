//! Paired longitudinal histories. Copy-on-write life states and committed replay.
use super::{creativity,json::{Json,obj,num,text},runner::*,sha256,transport};
use std::{collections::{BTreeMap,BTreeSet},fs,path::{Path,PathBuf},io::Write};
const ARCHS:[&str;4]=["full","replica","no_sleep","no_recon"];
const CONDITIONS:[&str;7]=["full","replica","probe_no_recon","no_sleep","no_recon","rag","empty"];
const LAYERS:[&str;6]=["intact","beliefs","co_recall","latent","mood","residual_only"];
pub(super) fn commit(path:&Path,value:&Json)->Result<()> {super::runner::write(path,value)?;#[cfg(unix)] fs::File::open(path.parent().unwrap_or(Path::new("."))).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;Ok(())}
fn copy(a:&Path,b:&Path)->Result<()> {if let Some(p)=b.parent(){fs::create_dir_all(p).map_err(|e|e.to_string())?;}let bytes=fs::read(a).map_err(|e|e.to_string())?;let mut f=fs::File::create(b).map_err(|e|e.to_string())?;f.write_all(&bytes).map_err(|e|e.to_string())?;f.sync_all().map_err(|e|e.to_string())}
fn remove(p:&Path)->Result<()> {if p.exists(){fs::remove_file(p).map_err(|e|e.to_string())?;}Ok(())}
fn files(dir:&Path)->Result<Vec<PathBuf>> {if !dir.exists(){return Ok(vec![]);}let mut ps=fs::read_dir(dir).map_err(|e|e.to_string())?.map(|e|e.map(|e|e.path()).map_err(|e|e.to_string())).collect::<Result<Vec<_>>>()?;ps.retain(|p|p.extension().is_some_and(|x|x=="json"));ps.sort();Ok(ps)}
fn slim(op:&str,r:Json)->Result<Json>{if op=="live"{return Ok(Json::Object(r.object()?.iter().filter(|(k,_)|k.as_str()!="state").map(|(k,v)|(k.clone(),v.clone())).collect()));}if matches!(op,"init"|"tick"|"sleep"|"recall_cut"){let mut x=obj([("clock",r.get("clock")?.clone()),("traces",num(array(&r,"traces")?.len() as f64)),("axioms",num(array(&r,"axioms")?.len() as f64))]);for key in ["night","lifecycle_audit"]{if let Ok(v)=r.get(key){x.put(key,v.clone());}}return Ok(x);}Ok(r)}
/// Only the latest committed state is authoritative. Each mutation gets a fresh
/// path, and its receipt commits before the old head is collected. An orphaned
/// uncommitted state is overwritten from the committed head on retry.
pub(super) struct Journal{pub(super) memory:Memory,dir:PathBuf,next:usize,hash:Option<String>}
impl Journal{
 pub(super) fn open(mut memory:Memory,dir:PathBuf)->Result<Self>{
  fs::create_dir_all(dir.join("steps")).map_err(|e|e.to_string())?;
  let records=files(&dir.join("steps"))?;
  let mut previous=None;for (i,p) in records.iter().enumerate(){let r=load(p)?;if n(&r,"seq")?!=i as f64{return Err("gap in committed life journal".into());}if let Some(hash)=previous{if r.get("before_sha256")?!=&hash{return Err("broken committed state chain".into());}}previous=Some(r.get("after_sha256")?.clone());}
  if let Some(p)=records.last(){let r=load(p)?;let head=dir.join("heads").join(format!("{:06}.vault",records.len()-1));if file_hash(&head)?!=s(&r,"after_sha256")?{return Err("committed life head changed; preserve evidence".into());}memory.path=head;}
  let hash=if memory.path.exists(){Some(file_hash(&memory.path)?)}else{None};
  let origin=dir.join("origin.json");if records.is_empty(){if origin.exists(){if load(&origin)?.get("sha256")?!=&hash.as_ref().map(text).unwrap_or(Json::Null){return Err("uncommitted birth changed".into());}}else{commit(&origin,&obj([("sha256",hash.as_ref().map(text).unwrap_or(Json::Null))]))?;}}
  Ok(Self{memory,dir,next:0,hash})
 }
 pub(super) fn call(&mut self,key:&str,op:&str,extra:Json)->Result<Json>{
  let seq=self.next;self.next+=1;let signature=sha256::hex(obj([("key",text(key)),("op",text(op)),("extra",extra.clone())]).encode().as_bytes());let receipt=self.dir.join("steps").join(format!("{seq:06}.json"));
  if receipt.exists(){let r=load(&receipt)?;if s(&r,"signature")?!=signature{return Err(format!("changed replay operation: {key}"));}return Ok(r.get("result")?.clone());}
  let actual=if self.memory.path.exists(){Some(file_hash(&self.memory.path)?)}else{None};if actual!=self.hash{return Err(format!("life head changed between operations: {key}"));}
  let old=self.memory.path.clone();let path=self.dir.join("heads").join(format!("{seq:06}.vault"));remove(&path)?;
  if actual.is_some(){copy(&old,&path)?;}else{fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;}
  let candidate=Memory::new("selmem_full",path.clone(),&self.memory_config());
  let result=slim(op,candidate.call(op,extra)?)?;let hash=file_hash(&path)?;
  // Reload verifies the persisted clock, rather than correcting it to fit.
  let state=candidate.call("inspect",obj([]))?;
  if let Ok(clock)=result.get("clock"){if state.get("clock")?!=clock{return Err("saved clock differs from operation result".into());}}
  commit(&receipt,&obj([("seq",num(seq as f64)),("key",text(key)),("signature",text(signature)),("before_sha256",actual.as_ref().map(text).unwrap_or(Json::Null)),("after_sha256",text(&hash)),("result",result.clone())]))?;
  self.memory=candidate;self.hash=Some(hash);
  // Keep birth files and checkpoints; collect only the previous transient head.
  if old.parent()==Some(self.dir.join("heads").as_path())&&old!=path{remove(&old)?;}
  Ok(result)
 }
 fn memory_config(&self)->Json{self.memory.configuration()}
}
fn safe(s:&str)->bool{!s.is_empty()&&s.bytes().all(|x|x.is_ascii_alphanumeric()||x==b'-'||x==b'_')}
fn validate(f:&Json,technical:bool)->Result<()> {
 if !["creative-life-2","creative-life-3"].contains(&s(f,"version")?)||s(f,"dataset_canonical_sha256")?!="c231a86025fc7f6217a2304d794ad66a98fc7dd7c6358f30832eb3d1a5af9ceb"{return Err("unrecognized longitudinal fixture".into());}
 let economy=s(f,"version")?=="creative-life-3";
 let days=n(f,"exposure_days")?;let wash=n(f,"washout_days")?;let repeats=n(f,"repeats")?;
 for x in [days,wash,repeats]{if x.fract()!=0.||x<1.||x>365.{return Err("invalid dimensions".into());}}
 if !technical&&(array(f,"pairs")?.len()!=if economy{2}else{10}||days!=90.||wash!=30.||repeats!=if economy{2.}else{3.}){return Err("real collection requires the complete 90+30 day plan".into());}
 if array(f,"tasks")?.len()!=3||n(f,"context_words")?!=600.{return Err("three tasks and 600-word maximum required".into());}
 let mut seen=BTreeSet::new();let mut total=0;
 for pair in array(f,"pairs")?{if !safe(s(pair,"id")?)||!seen.insert(s(pair,"id")?){return Err("invalid pair id".into());}if array(pair,"prefix")?.is_empty()||array(pair,"days")?.len()!=days as usize{return Err("incomplete life".into());}let mut ids=BTreeSet::new();
  for e in array(pair,"prefix")?{if !flag(e,"external")||!ids.insert(s(e,"id")?){return Err("invalid prefix".into());}total+=1;}
  for (i,d) in array(pair,"days")?.iter().enumerate(){if n(d,"day")?!=(i+1) as f64{return Err("invalid day order".into());}for e in array(d,"background")?{if !flag(e,"external")||!ids.insert(s(e,"id")?){return Err("invalid background".into());}total+=1;}
   if d.get("a")?.get("choices")?!=d.get("b")?.get("choices")?||d.get("a")?.get("situation")?!=d.get("b")?.get("situation")?{return Err("unpaired opportunities".into());}
   for b in ["a","b"]{let r=d.get(b)?;if array(r,"choices")?.len()!=3{return Err("three offered choices required".into());}for c in array(r,"choices")?{for e in array(r.get("outcomes")?.get(s(c,"id")?)?,"events")?.iter().chain(array(r,"extra")?){if flag(e,"external")||flag(e,"critical")||s(e,"observation_id")?.is_empty(){return Err("lived events require episode ids and no supplied salience".into());}}}}
  }
 }
 if !technical&&sha256::hex(f.encode().as_bytes())!=if economy{"2721424dfe2a63c57a121669110e3de2d87d7a8035bff857e42dafa2e91c82b9"}else{"b3ede25d35af21f26753db6986c0ebb73775b690496035469be288cb2a84b358"}{return Err("real collection requires the exact frozen fixture".into());}
 if !technical&&total!=if economy{788}else{5882}{return Err("full source must contain 5882 text turns".into());}
 let cps=array(f,"checkpoints")?.iter().map(|v|v.number().map(|v|v as usize)).collect::<Result<BTreeSet<_>>>()?;
 if cps.len()!=array(f,"checkpoints")?.len()||!cps.contains(&0)||!cps.contains(&(days as usize))||!cps.contains(&((days+wash) as usize))||cps.iter().any(|v|*v>(days+wash) as usize){return Err("invalid checkpoints".into());}
 if !technical&&cps!=if economy{BTreeSet::from([0,90,120])}else{BTreeSet::from([0,30,90,120])}{return Err("fixed checkpoints required".into());}
 for e in array(f,"continuation")?{if n(e,"cycle")?<=0.||n(e,"cycle")?>wash{return Err("invalid continuation".into());}}
 Ok(())
}
fn phase_repeats(f:&Json,phase:&str)->Result<usize>{if phase!="primary"{if let Ok(v)=f.get("intervention_repeats"){return Ok(v.number()? as usize);}}Ok(n(f,"repeats")? as usize)}
fn planned_proposals(f:&Json)->Result<usize>{Ok(array(f,"pairs")?.len()*2*array(f,"tasks")?.len()*(array(f,"checkpoints")?.len()*7*phase_repeats(f,"primary")?+8*phase_repeats(f,"layer")?))}
fn state_items(a:&Json)->Result<Vec<String>>{let mut items=Vec::new();if let Ok(r)=s(a,"reading"){if !r.trim().is_empty(){items.push(r.into());}}items.extend(array(a,"memories")?.iter().map(|v|v.string().map(str::to_owned)).collect::<Result<Vec<_>>>()?);Ok(items)}
pub(super) fn checkpoint(m:&Memory,path:PathBuf,cfg:&Json)->Result<Memory>{let marker=path.with_extension("checksum.json");if marker.exists(){if file_hash(&path)?!=s(&load(&marker)?,"sha256")?{return Err("checkpoint changed".into());}}else{copy(&m.path,&path)?;commit(&marker,&obj([("sha256",text(file_hash(&path)?))]))?;let clone=Memory::new("selmem_full",path.clone(),cfg);commit(&path.with_extension("state.json"),&clone.call("inspect",obj([]))?)?;}Ok(Memory::new("selmem_full",path,cfg))}
pub(super) fn context(m:&Memory,query:&str,reconstruct:bool,out:&Path)->Result<(String,Json)>{
 let hash=file_hash(&m.path)?;let key=sha256::hex(obj([("snapshot",text(&hash)),("query",text(query)),("reconstruct",Json::Bool(reconstruct))]).encode().as_bytes());let path=out.join("readouts").join(format!("{key}.json"));
 if path.exists(){let r=load(&path)?;return Ok((s(&r,"context")?.into(),r.get("audit")?.clone()));}
 let a=m.call("probe",obj([("query",text(query)),("reconstruct",Json::Bool(reconstruct)),("creative_transfer",Json::Bool(true))]))?;
 if file_hash(&m.path)?!=hash{return Err("readonly probe changed source".into());}let ctx=creativity::trim_context(&state_items(&a)?,600);
 let record=obj([("context",text(&ctx)),("audit",a.clone()),("snapshot_sha256",text(hash))]);commit(&path,&record)?;Ok((ctx,a))
}
pub(super) fn generation(p:&mut Provider,task:&Json,ctx:&str,seed:u64,path:&Path)->Result<Json>{
 let prompt=obj([("problem",task.get("prompt")?.clone()),("past_observations",text(ctx)),("output",text("JSON {\"proposal\":\"...\"}; one proposal, maximum 180 words."))]);let sig=sha256::hex(obj([("prompt",prompt.clone()),("seed",num(seed as f64))]).encode().as_bytes());
 if path.exists(){let r=load(path)?;if s(&r,"signature")?!=sig{return Err("changed generation inputs".into());}return Ok(r.get("generation")?.clone());}
 let reply=p.ask_with(prompt.clone(),"life_creative",creativity::SYSTEM,0.7,Some(seed))?;let proposal=s(&reply,"proposal").unwrap_or("");let valid=!proposal.trim().is_empty()&&proposal.split_whitespace().count()<=180&&reply.get("invalid_output").is_err();
 let g=obj([("prompt",prompt),("reply",obj([("proposal",text(proposal))])),("raw_reply",reply),("valid",Json::Bool(valid)),("context_words",num(ctx.split_whitespace().count() as f64)),("seed_requested",num(seed as f64)),("temperature",num(0.7))]);commit(path,&obj([("signature",text(sig)),("generation",g.clone())]))?;Ok(g)
}
fn choose(p:&mut Provider,r:&Json,ctx:&str,path:&Path,day:usize)->Result<String>{
 let prompt=obj([("situation",r.get("situation")?.clone()),("options",r.get("choices")?.clone()),("past_observations",text(ctx))]);let sig=sha256::hex(prompt.encode().as_bytes());
 if path.exists(){let old=load(path)?;if s(&old,"signature")?!=sig{return Err("changed choice input".into());}return Ok(s(&old,"choice")?.into());}
 let reply=p.ask_with(prompt.clone(),"life_choice","Choose one offered approach for the fictional situation using only the supplied fallible memory. Return only JSON {\"choice\":\"offered_id\"}. Do not invent a biography.",0.,Some(2026101400+day as u64))?;let chosen=s(&reply,"choice")?;
 if !array(r,"choices")?.iter().any(|c|s(c,"id").ok()==Some(chosen)){return Err("choice outside offered options".into());}
 commit(path,&obj([("signature",text(sig)),("choice",text(chosen)),("prompt",prompt),("audit_only_world",r.get("audit")?.clone())]))?;Ok(chosen.into())
}
fn proposals(pair:&str,branch:&str,day:usize,phase:&str,condition:&str,m:Option<&Memory>,events:&[Json],f:&Json,p:&mut Provider,out:&Path,all:&mut Vec<Json>,intervention:Json)->Result<()> {
 for task in array(f,"tasks")?{
  let (ctx,audit)=if condition=="empty"{(String::new(),Json::Null)}else if condition=="rag"{creativity::rag(events,s(task,"prompt")?,600)?}else{context(m.ok_or("native memory missing")?,s(task,"prompt")?,!matches!(condition,"probe_no_recon"|"no_recon"),out)?};
  if condition=="probe_no_recon"{let (_,full)=context(m.ok_or("missing full source")?,s(task,"prompt")?,true,out)?;if full.get("selected_ids")?!=audit.get("selected_ids")?{return Err("probe ablation changed selection".into());}}
  for repeat in 0..phase_repeats(f,phase)?{
   let id=format!("{pair}/{branch}/{day}/{phase}/{condition}/{}/{repeat}",s(task,"id")?);let key=sha256::hex(id.as_bytes());let g=generation(p,task,&ctx,2026101400+repeat as u64,&out.join("generations").join(format!("{key}.json")))?;
   all.push(obj([("trial_id",text(id)),("pair",text(pair)),("branch",text(branch)),("day",num(day as f64)),("phase",text(phase)),("condition",text(condition)),("task",task.clone()),("repeat",num(repeat as f64)),("snapshot",m.map(|m|text(m.path.to_string_lossy())).unwrap_or(Json::Null)),("audit",audit.clone()),("intervention",intervention.clone()),("generation",g)]));
  }
 }Ok(())
}
fn run(f:&Json,out:&Path,cfg:&Json,p:&mut Provider)->Result<Vec<Json>>{
 let mut all=Vec::new();let days=n(f,"exposure_days")? as usize;let wash=n(f,"washout_days")? as usize;let cps=array(f,"checkpoints")?.iter().map(|v|v.number().map(|v|v as usize)).collect::<Result<BTreeSet<_>>>()?;
 for pair in array(f,"pairs")?{let id=s(pair,"id")?;super::progress::line(format!("LIFE pair={id} prefix"));
  let mut prefix=Journal::open(Memory::new("selmem_full",out.join("unborn").join(format!("{id}.vault")),cfg),out.join("journals").join(format!("{id}-prefix")))?;
  prefix.call("init","init",obj([("architecture",text("selmem_full")),("associations_learn",Json::Bool(true)),("associations_reconstruct",Json::Bool(true))]))?;
  for e in array(pair,"prefix")?{prefix.call(s(e,"id")?,"live",obj([("event",e.clone())]))?;}
  let birth=prefix.memory.path.clone();
  let mut cut=Journal::open(prefix.memory.clone(),out.join("journals").join(format!("{id}-birth-cut")))?;
  cut.call("recall_cut","recall_cut",obj([("enabled",Json::Bool(false))]))?;let cut_birth=cut.memory.path.clone();
  let mut lives=BTreeMap::new();for b in ["a","b"]{for arch in ARCHS{lives.insert(format!("{b}-{arch}"),Journal::open(Memory::new("selmem_full",if arch=="no_recon"{cut_birth.clone()}else{birth.clone()},cfg),out.join("journals").join(format!("{id}-{b}-{arch}")))?);}}
  let mut exposed:BTreeMap<String,Vec<Json>>=[("a".into(),array(pair,"prefix")?.to_vec()),("b".into(),array(pair,"prefix")?.to_vec())].into_iter().collect();
  for day in 0..=days+wash{
   if day>0{super::progress::line(format!("LIFE pair={id} day={day}/{}",days+wash));for b in ["a","b"]{
    for arch in ARCHS{lives.get_mut(&format!("{b}-{arch}")).unwrap().call(&format!("{day}/tick"),"tick",obj([("hours",num(24.))]))?;}
    if day<=days{let d=&array(pair,"days")?[day-1];let r=d.get(b)?;for e in array(d,"background")?{for arch in ARCHS{lives.get_mut(&format!("{b}-{arch}")).unwrap().call(&format!("{day}/background/{}",s(e,"id")?),"live",obj([("event",e.clone())]))?;}exposed.get_mut(b).unwrap().push(e.clone());}
     let mut ctx=String::new();for arch in ARCHS{let a=lives.get_mut(&format!("{b}-{arch}")).unwrap().call(&format!("{day}/reflect"),"reflect",obj([("query",r.get("recall_query")?.clone()),("creative_transfer",Json::Bool(true))]))?;if arch=="full"{ctx=creativity::trim_context(&state_items(&a)?,600);}}
     let c=choose(p,r,&ctx,&out.join("choices").join(format!("{id}-{b}-{day:03}.json")),day)?;let mut events=array(r.get("outcomes")?.get(&c)?,"events")?.to_vec();events.extend_from_slice(array(r,"extra")?);
     for e in events{for arch in ARCHS{lives.get_mut(&format!("{b}-{arch}")).unwrap().call(&format!("{day}/event/{}",s(&e,"id")?),"live",obj([("event",e.clone())]))?;}exposed.get_mut(b).unwrap().push(e);}
    }
    for arch in ["full","replica","no_recon"]{lives.get_mut(&format!("{b}-{arch}")).unwrap().call(&format!("{day}/sleep"),"sleep",obj([]))?;}
   }}
   if !cps.contains(&day){continue;}
   for b in ["a","b"]{let mut snapshots=BTreeMap::new();let mut clocks=BTreeSet::new();
    for arch in ARCHS{let m=checkpoint(&lives[&format!("{b}-{arch}")].memory,out.join("checkpoints").join(format!("{id}-{b}-{arch}-{day:03}.vault")),cfg)?;clocks.insert(n(&m.call("inspect",obj([]))?,"clock")? as u64);snapshots.insert(arch,m);}
    if clocks.len()!=1{return Err("yoked clocks diverged; no forced correction".into());}
    if day==0&&b=="b"{for arch in ARCHS{if file_hash(&snapshots[arch].path)?!=file_hash(&out.join("checkpoints").join(format!("{id}-a-{arch}-000.vault")))?{return Err("paired birth differs".into());}}}
    for c in CONDITIONS{let arch=match c{"replica"=>"replica","no_sleep"=>"no_sleep","no_recon"=>"no_recon",_=>"full"};proposals(id,b,day,"primary",c,Some(&snapshots[arch]),&exposed[b],f,p,out,&mut all,Json::Null)?;}
    if day==days+wash{for layer in LAYERS{let dir=out.join("journals").join(format!("{id}-{b}-layer-{layer}"));let mut j=Journal::open(snapshots["full"].clone(),dir)?;let lesion=j.call("lesion","lesion",obj([("kind",text(layer)),("roots",arr([]))]))?;proposals(id,b,day,"layer",layer,Some(&j.memory),&exposed[b],f,p,out,&mut all,lesion)?;}}
   }
  }
  // Latent intervention precedes matched new events; origins are day-90 snapshots.
  for b in ["a","b"]{for condition in ["intact","remove_latent"]{let source=Memory::new("selmem_full",out.join("checkpoints").join(format!("{id}-{b}-full-{days:03}.vault")),cfg);let mut j=Journal::open(source,out.join("journals").join(format!("{id}-{b}-continuation-{condition}")))?;
   let lesion=j.call("lesion","lesion",obj([("kind",text(if condition=="intact"{"intact"}else{"latent"})),("roots",arr([]))]))?;
   for cycle in 1..=wash{j.call(&format!("{cycle}/tick"),"tick",obj([("hours",num(24.))]))?;for e in array(f,"continuation")?{if n(e,"cycle")? as usize==cycle{j.call(&format!("{cycle}/new"),"live",obj([("event",obj([("id",text(format!("{id}/{b}/continuation/{cycle}"))),("external",Json::Bool(false)),("text",e.get("text")?.clone())]))]))?;}}
    j.call(&format!("{cycle}/reflect"),"reflect",obj([("query",text("Materials, labels, tasks, containers, assembling displays and participants.")),("creative_transfer",Json::Bool(true))]))?;j.call(&format!("{cycle}/sleep"),"sleep",obj([]))?;
   }
   let snapshot=checkpoint(&j.memory,out.join("continuations").join(format!("{id}-{b}-{condition}.vault")),cfg)?;proposals(id,b,days+wash,"continuation",condition,Some(&snapshot),&[],f,p,out,&mut all,lesion)?;
  }}
 }
 Ok(all)
}
#[cfg(unix)]pub(super) struct Lock{_file:fs::File}
#[cfg(unix)]impl Lock{pub(super) fn acquire(out:&Path)->Result<Self>{use std::os::fd::AsRawFd;extern "C"{fn flock(fd:i32,op:i32)->i32;}let f=fs::OpenOptions::new().read(true).write(true).create(true).open(out.join("run.lock")).map_err(|e|e.to_string())?;if unsafe{flock(f.as_raw_fd(),6)}!=0{return Err("another process owns this collection".into());}Ok(Self{_file:f})}}
#[cfg(not(unix))]pub(super) struct Lock(PathBuf);
#[cfg(not(unix))]impl Lock{pub(super) fn acquire(out:&Path)->Result<Self>{let p=out.join("run.lock");fs::OpenOptions::new().write(true).create_new(true).open(&p).map_err(|_|"run lock exists; verify no process is running before removing a stale lock")?;Ok(Self(p))}}
#[cfg(not(unix))]impl Drop for Lock{fn drop(&mut self){let _=fs::remove_file(&self.0);}}
pub(super) fn code_hash()->Result<String>{fn walk(p:&Path,paths:&mut Vec<PathBuf>)->Result<()>{for e in fs::read_dir(p).map_err(|e|e.to_string())?{let p=e.map_err(|e|e.to_string())?.path();if p.is_dir(){walk(&p,paths)?;}else if p.extension().is_some_and(|s|s=="rs"){paths.push(p);}}Ok(())}let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("src");let mut paths=vec![];walk(&root,&mut paths)?;paths.sort();let mut value=Vec::new();for p in paths{value.push(obj([("path",text(p.strip_prefix(&root).unwrap().to_string_lossy())),("hash",text(file_hash(&p)?))]));}value.push(text(file_hash(&std::env::current_exe().or_else(|_|std::env::args().next().map(PathBuf::from).ok_or_else(||std::io::Error::new(std::io::ErrorKind::NotFound,"missing executable argument"))).map_err(|e|e.to_string())?.canonicalize().map_err(|e|e.to_string())?)?));Ok(sha256::hex(arr(value).encode().as_bytes()))}
pub(super) fn flags(args:Vec<String>)->Result<(BTreeMap<String,String>,BTreeSet<String>)>{let mut flags=BTreeMap::new();let mut switches=BTreeSet::new();let mut i=0;while i<args.len(){let a=&args[i];if ["--mock","--resume","--prepare-only","--technical-smoke"].contains(&a.as_str()){if !switches.insert(a.clone()){return Err("duplicate switch".into());}i+=1;}else{if !["--fixture","--out","--config","--llm","--model","--endpoint","--plug","--max-http-calls","--source","--plan"].contains(&a.as_str()){return Err(format!("unknown flag {a}"));}if flags.insert(a.clone(),args.get(i+1).ok_or("missing flag value")?.clone()).is_some(){return Err("duplicate flag".into());}i+=2;}}Ok((flags,switches))}
pub(super) fn provider(flags:&BTreeMap<String,String>,mock:bool,out:&Path)->Result<Provider>{let mouth=if mock{None}else{Some(configured_mouth(selmem::Config::get(),flags)?.ok_or("configure LLM provider")?)};Ok(Provider::new(mock,mouth.as_ref().map(|m|m.model.clone()).unwrap_or("mock".into()),mouth.as_ref().map(|m|m.url.clone()).unwrap_or_default(),None,out).with_api_key(mouth.and_then(|m|m.api_key)))}
pub(super) fn cli(args:Vec<String>)->Result<()> {
 let (flags,sw)=flags(args)?;let mock=sw.contains("--mock");let technical=sw.contains("--technical-smoke");if technical&&!mock{return Err("contracted plans are mock-only".into());}
 let fixture=Path::new(flags.get("--fixture").ok_or("--fixture required")?);let f=load(fixture)?;validate(&f,technical)?;let economy=s(&f,"version")?=="creative-life-3";let out=Path::new(flags.get("--out").ok_or("--out required")?);let resume=sw.contains("--resume");if out.exists()!=resume{return Err("choose a fresh directory or use --resume for this exact collection".into());}
 if selmem::Config::get().temp().parse::<f64>().map_err(|_|"invalid temperature")?!=0.{return Err("memory temperature must be zero".into());}
 let budget:usize=flags.get("--max-http-calls").map(|s|s.parse()).transpose().map_err(|_|"invalid budget")?.unwrap_or(if economy{12000}else{200000});if !(1..=500000).contains(&budget){return Err("HTTP ceiling 1..500000".into());}
 let mut cfg=campaign_config(selmem::Config::get(),&flags,true,mock)?;cfg.put("associations_learn",Json::Bool(true));cfg.put("associations_reconstruct",Json::Bool(true));cfg.put("observation_evidence",Json::Bool(false));cfg.put("semantic_annotation_cache",Json::Bool(economy));let mut p=provider(&flags,mock,out)?;
 let design=obj([("version",f.get("version")?.clone()),("fixture_sha256",text(file_hash(fixture)?)),("fixture",f.clone()),("config",cfg.clone()),("provider",p.metadata()),("technical",Json::Bool(technical)),("code_sha256",text(code_hash()?)),("cumulative_http_ceiling",num(budget as f64))]);
 if !resume{fs::create_dir_all(out).map_err(|e|e.to_string())?;}let _lock=Lock::acquire(out)?;
 if resume{let old=load(&out.join("life-design.json"))?;let mut comparable=old.clone();comparable.put("cumulative_http_ceiling",num(budget as f64));if comparable!=design||n(&old,"cumulative_http_ceiling")?>budget as f64{return Err("resume requires identical code, fixture, provider, reasoning, config; only a higher HTTP ceiling may change".into());}if old!=design{log(&out.join("ceiling-amendments.jsonl"),&obj([("before",old.get("cumulative_http_ceiling")?.clone()),("after",num(budget as f64))]))?;}}
 commit(&out.join("life-design.json"),&design)?;let _session=transport::resume(out,budget)?;transport::enable_semantic_cache(economy);super::progress::enable(true);
 let result=if sw.contains("--prepare-only"){Ok(Vec::new())}else{run(&f,out,&cfg,&mut p)};
 let planned=planned_proposals(&f)?;
 if let Ok(all)=&result{if !sw.contains("--prepare-only")&&all.len()!=planned{return Err("proposal count differs from plan".into());}if !all.is_empty(){commit(&out.join("all-proposals.json"),&arr(all.clone()))?;creativity::blind(out,all)?;}}
 let completed=result.as_ref().map(|v|v.len()).unwrap_or(0);commit(&out.join("life.json"),&obj([("status",text(if result.is_err(){"paused_with_error"}else if sw.contains("--prepare-only"){"prepared"}else if mock{"technical_rehearsal_complete"}else{"collected_unrated"})),("mock",Json::Bool(mock)),("technical",Json::Bool(technical)),("planned_proposals",num(planned as f64)),("completed_proposals",num(completed as f64)),("http_attempts_cumulative",num(transport::used() as f64)),("semantic_annotation_cache",transport::semantic_cache_stats()),("error",result.as_ref().err().map(text).unwrap_or(Json::Null)),("claim",text("Unrated collection. No automatic creativity, belief or unconsciousness claim."))]))?;result.map(|_|())
}

/// Source-specific interventions require locked blind ratings and a human source audit.
pub(super) fn causal_cli(args:Vec<String>)->Result<()> {
 let (flags,sw)=flags(args)?;if sw.iter().any(|v|v!="--mock"){return Err("causal follow-up supports --mock only; choose fresh output".into());}let mock=sw.contains("--mock");let source=Path::new(flags.get("--source").ok_or("--source required")?);let out=Path::new(flags.get("--out").ok_or("--out required")?);if out.exists(){return Err("fresh causal output required".into());}
 let report=load(&source.join("life.json"))?;if s(&report,"status")?!=if mock{"technical_rehearsal_complete"}else{"collected_unrated"}||flag(&report,"mock")!=mock{return Err("complete matching real/mock collection required".into());}
 let design=load(&source.join("life-design.json"))?;if !mock&&s(&design,"code_sha256")?!=code_hash()?{return Err("source code changed since collection".into());}let cfg=design.get("config")?.clone();let mut p=provider(&flags,mock,out)?;if p.metadata()!=*design.get("provider")?{return Err("same provider/model/reasoning required".into());}
 let binding=campaign_config(selmem::Config::get(),&flags,true,mock)?;for key in ["memory_backend","lifecycle_backend","memory_endpoint","memory_model","scorer_endpoint","scorer_model"]{if binding.get(key).ok()!=cfg.get(key).ok(){return Err("source memory runtime configuration mismatch".into());}}
 if !mock{let ratings=load(&source.join("life-ratings.json"))?;if s(&ratings,"source_proposals_sha256")?!=file_hash(&source.join("all-proposals.json"))?{return Err("locked blind ratings do not match collection".into());}}
 let plan_path=Path::new(flags.get("--plan").ok_or("--plan required")?);let plan=load(plan_path)?;let cases=array(&plan,"cases")?;if cases.is_empty(){return Err("nonempty independently audited source plan required".into());}
 let trials=load(&source.join("all-proposals.json"))?;let mut checked=Vec::new();
 for case in cases{let id=s(case,"trial_id")?;
  let row=trials.array()?.iter().find(|r|s(r,"trial_id").ok()==Some(id)).ok_or("unknown source proposal")?;
  if s(row,"phase")?!="primary"||s(row,"condition")?!="full"{return Err("audit a primary full proposal".into());}
  if s(case,"mechanism")?.trim().is_empty()||s(case,"source_rationale")?.trim().is_empty(){return Err("mechanism and source rationale required".into());}
  let roots=array(case,"roots")?;let sham=array(case,"sham_roots")?;let unique=roots.iter().chain(sham).map(|v|v.string()).collect::<Result<BTreeSet<_>>>()?;if roots.len()!=2||sham.len()!=2||unique.len()!=4{return Err("two distinct sources and two distinct sham roots required".into());}
  let selected=array(row.get("audit")?,"selected_ids")?;let shown=s(row.get("generation")?.get("prompt")?,"past_observations")?;
  for root in roots{if !selected.contains(root){return Err("target source not selected".into());}let trace=array(row.get("audit")?,"recalled_traces")?.iter().find(|t|t.get("id").ok()==Some(root)).ok_or("source not recalled")?;if !shown.contains(s(trace,"narrative")?){return Err("source not shown within word cap".into());}}
  if sham.iter().any(|r|selected.contains(r)){return Err("sham roots must be unselected".into());}
  let path=PathBuf::from(s(row,"snapshot")?);if file_hash(&path)?!=s(&load(&path.with_extension("checksum.json"))?,"sha256")?{return Err("source checkpoint hash changed".into());}let parent=Memory::new("selmem_full",path.clone(),&cfg);let state=parent.call("inspect",obj([]))?;
  if roots.iter().chain(sham).any(|r|!array(&state,"traces").unwrap_or(&[]).iter().any(|t|t.get("id").ok()==Some(r))){return Err("audited root missing from checkpoint".into());}
  let observations=roots.iter().map(|r|array(&state,"traces").unwrap().iter().find(|t|t.get("id").ok()==Some(r)).and_then(|t|s(t,"observation").ok()).unwrap_or("")).collect::<BTreeSet<_>>();if observations.len()!=2||observations.contains(""){return Err("source roots require distinct known observation ids; independent situations still require human audit".into());}
  checked.push((case.clone(),row.clone(),parent,file_hash(&path)?));
 }
 fs::create_dir_all(out).map_err(|e|e.to_string())?;let _lock=Lock::acquire(out)?;let budget=flags.get("--max-http-calls").map(|v|v.parse::<usize>()).transpose().map_err(|_|"invalid budget")?.unwrap_or(3000);if !(1..=50000).contains(&budget){return Err("causal ceiling 1..50000".into());}let _session=transport::begin(out,budget);super::progress::enable(true);
 commit(&out.join("causal-design.json"),&obj([("plan",plan),("plan_sha256",text(file_hash(plan_path)?)),("source_proposals_sha256",text(file_hash(&source.join("all-proposals.json"))?)),("scope",text("Post-selection structural interventions: independent provenance and sham comparability audit required. Not proof of uniquely identified psychological cause."))]))?;
 let mut all=Vec::new();
 for (i,(case,row,parent,hash)) in checked.iter().enumerate(){let roots=array(case,"roots")?;for (arm,removed) in [("intact",vec![]),("remove_a",vec![roots[0].clone()]),("remove_b",vec![roots[1].clone()]),("remove_ab",roots.to_vec()),("sham",array(case,"sham_roots")?.to_vec())]{
  let clone=parent.copy(out.join("interventions").join(format!("{i}-{arm}.vault")))?;let lesion=clone.call("lesion",obj([("kind",text("episode")),("roots",arr(removed))]))?;
  let (ctx,audit)=context(&clone,s(row.get("task")?,"prompt")?,true,out)?;
  for repeat in 0..3{let g=generation(&mut p,row.get("task")?,&ctx,2026101400+repeat,&out.join("generations").join(format!("{i}-{arm}-{repeat}.json")))?;all.push(obj([("trial_id",text(format!("case-{i}/{arm}/{repeat}"))),("source_trial",row.get("trial_id")?.clone()),("arm",text(arm)),("mechanism",case.get("mechanism")?.clone()),("audit",audit.clone()),("lesion",lesion.clone()),("generation",g)]));}
  if file_hash(&parent.path)?!=*hash{return Err("original checkpoint changed during causal follow-up".into());}
 }}
 commit(&out.join("all-proposals.json"),&arr(all.clone()))?;creativity::blind(out,&all)?;commit(&out.join("causal.json"),&obj([("mock",Json::Bool(mock)),("completed_proposals",num(all.len() as f64)),("http_attempts",num(transport::used() as f64))]))
}

#[cfg(test)] mod tests{
 use super::*;
 fn cfg()->Json{let mut c=campaign_config(&selmem::Config::empty(),&BTreeMap::new(),true,true).unwrap();c.put("associations_learn",Json::Bool(true));c.put("associations_reconstruct",Json::Bool(true));c.put("observation_evidence",Json::Bool(false));c}
 fn event(id:&str,body:&str,external:bool)->Json{obj([("id",text(id)),("observation_id",text(id)),("text",text(body)),("external",Json::Bool(external))])}
 fn tiny()->Json{
  let choices=arr(["trial","plan","review"].map(|id|obj([("id",text(id)),("text",text(id))])));
  let days=(1..=2).map(|day|{let branch=|b:&str|{let outcomes=Json::Object(["trial","plan","review"].into_iter().map(|id|(id.into(),obj([("events",arr([event(&format!("{b}/{day}/{id}/action"),"I repaired borrowed workshop tools with neighbors.",false),event(&format!("{b}/{day}/{id}/result"),if b=="a"{"The shared repair worked, everyone thanked me and I was proud."}else{"The repair failed and the borrowed tool was lost in front of everyone."},false)]))]))).collect());obj([("situation",text("Prepare a workshop with borrowed tools.")),("choices",choices.clone()),("recall_query",text("borrowed workshop repair neighbors materials")),("outcomes",outcomes),("extra",arr([])),("audit",obj([]))])};obj([("day",num(day as f64)),("background",arr([])),("a",branch("a")),("b",branch("b"))])});
  let tasks=arr(["one","two","three"].map(|id|obj([("id",text(id)),("prompt",text("Invent a useful process for borrowed workshop tools, repair and neighbors."))])));
  let prefix=arr([event("one","I repaired a broken bicycle with neighbors in a shared workshop.",false),event("two","I lent useful tools to a school group for a public repair session.",false),event("three","I learned to check borrowed workshop materials with a careful written list.",false)]);
  let pair=obj([("id",text("pair")),("prefix",prefix),("days",arr(days))]);
  obj([("version",text("creative-life-2")),("dataset_canonical_sha256",text("c231a86025fc7f6217a2304d794ad66a98fc7dd7c6358f30832eb3d1a5af9ceb")),("exposure_days",num(2.)),("washout_days",num(1.)),("repeats",num(1.)),("context_words",num(600.)),("checkpoints",arr([num(0.),num(2.),num(3.)])),("tasks",tasks),("continuation",arr([obj([("cycle",num(1.)),("text",text("I labeled material samples on a shelf."))])])),("pairs",arr([pair]))])
 }
 #[test]fn copy_on_write_rollback_and_mutation_detection(){
  let out=super::super::lifecycle_tests::root();fs::create_dir_all(&out).unwrap();let c=cfg();let dir=out.join("j");let mut j=Journal::open(Memory::new("selmem_full",out.join("unborn.vault"),&c),dir.clone()).unwrap();let init=obj([("architecture",text("selmem_full"))]);j.call("init","init",init.clone()).unwrap();let hash=file_hash(&j.memory.path).unwrap();
  fs::write(dir.join("heads/000001.vault"),"uncommitted garbage").unwrap();let mut resumed=Journal::open(Memory::new("selmem_full",out.join("unborn.vault"),&c),dir).unwrap();assert_eq!(file_hash(&resumed.memory.path).unwrap(),hash);resumed.call("init","init",init).unwrap();resumed.call("day1","tick",obj([("hours",num(24.))])).unwrap();assert_eq!(n(&resumed.memory.call("inspect",obj([])).unwrap(),"clock").unwrap(),4_102_444_800.+86400.);
  resumed.memory.call("tick",obj([("hours",num(1.))])).unwrap();assert!(resumed.call("day2","tick",obj([("hours",num(24.))])).unwrap_err().contains("between operations"));fs::remove_dir_all(out).unwrap();
 }
 #[test]fn all_controls_interventions_and_resume_preserve_states(){
  let out=super::super::lifecycle_tests::root();fs::create_dir_all(&out).unwrap();let c=cfg();let f=tiny();let mut p=Provider::new(true,"mock".into(),"".into(),None,&out);let first=run(&f,&out,&c,&mut p).unwrap();assert_eq!(first.len(),174);
  let original=first.clone();let checkpoints=files(&out.join("checkpoints")).unwrap().into_iter().map(|p|(p.clone(),file_hash(&p).unwrap())).collect::<Vec<_>>();let second=run(&f,&out,&c,&mut p).unwrap();assert_eq!(second,original);for (p,h) in checkpoints{assert_eq!(file_hash(&p).unwrap(),h);}
  let inputs=|a:&str|files(&out.join(format!("journals/pair-a-{a}/steps"))).unwrap().into_iter().map(|p|load(&p).unwrap()).filter(|r|s(r,"key").unwrap().contains("/event/")).map(|r|s(&r,"signature").unwrap().to_owned()).collect::<Vec<_>>();for a in ["replica","no_sleep","no_recon"]{assert_eq!(inputs("full"),inputs(a));}
  for arch in ARCHS{assert_eq!(file_hash(&out.join("checkpoints").join(format!("pair-a-{arch}-000.vault"))).unwrap(),file_hash(&out.join("checkpoints").join(format!("pair-b-{arch}-000.vault"))).unwrap());}
  fs::remove_dir_all(out).unwrap();
 }
 #[test]fn explicit_reasoning_and_live_transfer_do_not_change_factual_default(){assert_eq!(selmem::config::normalize_reasoning("off"),"none");assert_eq!(selmem::config::normalize_reasoning("LOW"),"low");assert_eq!(selmem::config::normalize_reasoning("none"),"none");assert_ne!(selmem::RecallBias::Transfer,selmem::RecallBias::Observed);}
 #[test]fn cumulative_attempts_and_partial_finish_are_retained(){let out=super::super::lifecycle_tests::root();fs::create_dir_all(&out).unwrap();fs::write(out.join("http.jsonl"),"{\"phase\":\"start\",\"id\":1}\n{\"phase\":\"fin").unwrap();let _b=transport::resume(&out,1).unwrap();assert_eq!(transport::used(),1);assert!(transport::post("test","http://127.0.0.1:1",None,"{}").unwrap_err().contains("budget exhausted"));fs::remove_dir_all(out).unwrap();}
 #[test]fn same_process_double_owner_is_refused(){let out=super::super::lifecycle_tests::root();fs::create_dir_all(&out).unwrap();let lock=Lock::acquire(&out).unwrap();assert!(Lock::acquire(&out).is_err());drop(lock);let _second=Lock::acquire(&out).unwrap();fs::remove_dir_all(out).unwrap();}
 #[test]fn source_specific_followup_has_five_arms_and_preserves_original(){
  let root=super::super::lifecycle_tests::root();let source=root.join("source");let out=root.join("causal");fs::create_dir_all(&source).unwrap();let c=cfg();let path=source.join("checkpoints/test.vault");let parent=Memory::new("selmem_full",path.clone(),&c);parent.init().unwrap();
  let mut organ=selmem::SelectiveMemory::open(&path,selmem::EntityProfile::new("observer")).unwrap();
  for (i,body) in ["Neighbors repaired broken tables together in a workshop.","Adults learned practical skills together with books.","An astronaut observed distant stars.","A submarine explored coral reefs.","A farmer harvested wheat in summer.","A pianist performed a sonata.","A skier crossed snowy mountains.","A jeweler polished a sapphire."].iter().enumerate(){let obs=format!("independent-{i}");let mut input=selmem::EncodeInput::new(body);input.observation_id=Some(&obs);input.self_relevance=1.;input.permanence=1.;input.arousal=0.8;input.schema=Some(format!("test-{i}"));assert!(organ.live_with(input).kept);}
  organ.save().unwrap();let hash=file_hash(&path).unwrap();commit(&path.with_extension("checksum.json"),&obj([("sha256",text(&hash))])).unwrap();
  let task=obj([("id",text("task")),("prompt",text("Neighbors repair broken tables together and adults learn practical skills with books."))]);let (ctx,audit)=context(&parent,s(&task,"prompt").unwrap(),true,&source).unwrap();
  let roots=array(&audit,"selected_ids").unwrap().iter().take(2).cloned().collect::<Vec<_>>();assert_eq!(roots.len(),2);let state=parent.call("inspect",obj([])).unwrap();let sham=array(&state,"traces").unwrap().iter().map(|t|t.get("id").unwrap().clone()).filter(|id|!array(&audit,"selected_ids").unwrap().contains(id)).take(2).collect::<Vec<_>>();assert_eq!(sham.len(),2);
  let p=Provider::new(true,"mock".into(),"".into(),None,&out);commit(&source.join("life-design.json"),&obj([("config",c),("provider",p.metadata())])).unwrap();commit(&source.join("life.json"),&obj([("status",text("technical_rehearsal_complete")),("mock",Json::Bool(true))])).unwrap();
  let trial=obj([("trial_id",text("trial")),("phase",text("primary")),("condition",text("full")),("snapshot",text(path.to_string_lossy())),("task",task),("audit",audit),("generation",obj([("prompt",obj([("past_observations",text(ctx))]))]))]);commit(&source.join("all-proposals.json"),&arr([trial])).unwrap();
  let plan=root.join("plan.json");commit(&plan,&obj([("cases",arr([obj([("trial_id",text("trial")),("roots",arr(roots)),("sham_roots",arr(sham)),("mechanism",text("Engineered fixture mechanism")),("source_rationale",text("Engineered independent sources, not scientific evidence"))])]))])).unwrap();
  causal_cli(vec!["--mock".into(),"--source".into(),source.to_string_lossy().into(),"--plan".into(),plan.to_string_lossy().into(),"--out".into(),out.to_string_lossy().into()]).unwrap();assert_eq!(n(&load(&out.join("causal.json")).unwrap(),"completed_proposals").unwrap(),15.);assert_eq!(file_hash(&path).unwrap(),hash);fs::remove_dir_all(root).unwrap();
 }
 #[test]fn latent_lesion_and_residual_only_record_actual_availability(){
  let root=super::super::lifecycle_tests::root();fs::create_dir_all(&root).unwrap();let c=cfg();let m=Memory::new("selmem_full",root.join("life.vault"),&c);m.init().unwrap();let mut organ=selmem::SelectiveMemory::open(&m.path,selmem::EntityProfile::new("observer")).unwrap();
  for (i,body) in ["I repaired borrowed tools with neighbors.","I learned a useful skill in a workshop."].into_iter().enumerate(){let mut input=selmem::EncodeInput::new(body);input.permanence=1.;let id=organ.live_with(input).trace_id.unwrap();if i==0{organ.store.traces.get_mut(&id).unwrap().status=selmem::TraceStatus::Latent;}}organ.save().unwrap();let hash=file_hash(&m.path).unwrap();
  for (kind,status) in [("latent","Active"),("residual_only","Latent")]{let clone=m.copy(root.join(format!("{kind}.vault"))).unwrap();let r=clone.call("lesion",obj([("kind",text(kind)),("roots",arr([]))])).unwrap();let traces=array(r.get("after").unwrap(),"traces").unwrap();assert_eq!(traces.len(),1);if kind=="residual_only"{assert_eq!(s(&traces[0],"status").unwrap(),status);}else{assert_ne!(s(&traces[0],"status").unwrap(),"Latent");}}
  assert_eq!(file_hash(&m.path).unwrap(),hash);fs::remove_dir_all(root).unwrap();
 }

}
