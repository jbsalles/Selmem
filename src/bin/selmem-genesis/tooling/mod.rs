//! Offline experiment tooling. No HTTP, model calls, or external language runtime.
use super::{json::{Json,obj,num,text},runner::*,sha256};
use std::{collections::{BTreeMap,BTreeSet},fs,io::Write,path::Path};
mod prepare;
mod ratings;
mod mechanisms;
#[cfg(test)] mod tests;
fn require(ok:bool,msg:&str)->Result<()>{if ok{Ok(())}else{Err(msg.into())}}
fn mean(xs:&[f64])->f64{xs.iter().sum::<f64>()/xs.len() as f64}
fn average(xs:&[f64])->Json{if xs.is_empty(){Json::Null}else{num(mean(xs))}}
fn fresh(path:&Path,value:&Json)->Result<()>{fresh_bytes(path,format!("{}\n",value.encode()).as_bytes())}
fn fresh_bytes(path:&Path,bytes:&[u8])->Result<()>{let mut f=fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|format!("fresh output {}: {e}",path.display()))?;f.write_all(bytes).and_then(|_|f.sync_all()).map_err(|e|e.to_string())}
fn string(v:&Json)->Result<String>{match v{Json::String(s)=>Ok(s.clone()),Json::Number(_)|Json::Bool(_)=>Ok(v.encode()),Json::Array(a)=>Ok(a.iter().map(string).collect::<Result<Vec<_>>>()?.join(",")),Json::Null=>Ok("null".into()),_=>Err("unsupported answer type".into())}}
fn read_lines(path:&Path)->Result<Vec<Json>>{fs::read_to_string(path).map_err(|e|e.to_string())?.lines().filter(|s|!s.trim().is_empty()).map(super::json::parse).collect()}
fn template(dir:&Path,mechanisms:bool)->Result<Json>{let rows=load(&dir.join("blind-proposals.json"))?.array()?.iter().map(|r|{let id=r.get("id")?.clone();Ok(if mechanisms{obj([("id",id),("rater",text("")),("mechanism_summary",text("")),("tags",arr([]))])}else{obj([("id",id),("rater",text("")),("utility",Json::Null),("originality",Json::Null),("constraints_met",Json::Null),("rationale",text(""))])})}).collect::<Result<Vec<_>>>()?;Ok(arr(rows))}
pub(super) fn cli(args:Vec<String>)->Result<()>{
 let action=args.first().ok_or("tools: prepare-creative|prepare-preview|prepare-life|prepare-sleep|rate-creative|rate-preview|rate-life|code-mechanisms|combine-ratings|life-profile")?;
 let p=|i:usize|->Result<&Path>{args.get(i).map(|s|Path::new(s)).ok_or_else(||"missing tool argument".to_string())};
 match action.as_str(){
  "prepare-creative"|"prepare-preview"|"prepare-life"|"prepare-sleep"=>{let input=p(1)?;let out=p(2)?;let tail=&args[3..];let profile=match action.as_str(){"prepare-creative"=>{require(tail.is_empty()||tail==["--all"],"expected optional --all")?;if tail.is_empty(){"pilot"}else{"all"}},"prepare-life"=>{require(tail.is_empty()||tail==["--profile","economy"]||tail==["--profile","full"],"expected --profile economy|full")?;tail.get(1).map(String::as_str).unwrap_or("economy")},_=>{require(tail.is_empty(),"unexpected preparation arguments")?;""}};
   let bytes=fs::read(input).map_err(|e|e.to_string())?;let f=prepare::build(action,profile,&bytes)?;
   // Preserve the byte-pinned preview when exporting the official frozen plan.
   let preview=include_str!("../../../../experiments/genesis/creative-preview.json");
   if action=="prepare-preview"&&f==super::json::parse(preview)?{fresh_bytes(out,preview.as_bytes())?;}else{fresh(out,&f)?;}
   let mut summary=prepare::summary(&f)?;summary.put("output",text(out.to_string_lossy()));summary.put("fixture_sha256",text(file_hash(out)?));summary.put("version",f.get("version")?.clone());println!("{}",summary.encode());
  },
  "rate-creative"|"rate-preview"|"rate-life"|"code-mechanisms"=>{let dir=p(1)?;let cmd=args.get(2).ok_or("template|analyze|summarize required")?;
   if cmd=="template"{require(args.len()==4,"template requires one fresh output")?;fresh(p(3)?,&template(dir,action=="code-mechanisms")?)?;}
   else if cmd=="analyze"{require(args.len()==4,"analyze requires one combined ratings file")?;let data=load(p(3)?)?;let (file,report)=match action.as_str(){"code-mechanisms"=>("life-mechanisms.json",mechanisms::analyze(dir,&data)?),"rate-life"=>("life-ratings.json",ratings::analyze(dir,&data,"life")?),"rate-preview"=>("preview-ratings.json",ratings::analyze(dir,&data,"preview")?),_=>("creative-ratings.json",ratings::analyze(dir,&data,"creative")?)};fresh(&dir.join(file),&report)?;println!("{}",dir.join(file).display());}
   else if cmd=="summarize"&&action=="rate-life"{require(args.len()==3,"summarize takes no extra argument")?;let path=dir.join("life-availability.json");write(&path,&ratings::summarize(dir)?)?;println!("{}",path.display());}else{return Err("unknown analysis operation".into());}
  },
  "combine-ratings"=>{require(args.len()>=4,"combine-ratings OUT INPUT_A INPUT_B [INPUT...]")?;let mut all=vec![];for file in &args[2..]{all.extend(load(Path::new(file))?.array()?.to_vec());}fresh(p(1)?,&arr(all))?;},
  "life-profile"=>{require(args.len()==2,"life-profile FIXTURE")?;let f=load(p(1)?)?;println!("{}",match s(&f,"version")?{"creative-life-3"=>"economy","creative-life-2"=>"full",_=>return Err("not a longitudinal fixture".into())});},
  _=>return Err(format!("unknown offline tool: {action}")),
 }Ok(())
}
