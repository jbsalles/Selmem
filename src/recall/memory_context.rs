//! Attributed, bounded delivery of already selected lived state, never archives.
use crate::{Attribution,AxiomLayer,Channel,Config,MemoryStore,Mood,TraceStatus,WorkingTalk};
use crate::net::httpx::json_esc;
use std::collections::BTreeSet;
pub const MEMORY_CONTEXT_VERSION:&str="selmem-memory-context-1";
pub const MEMORY_USE_CONTRACT:&str=include_str!("../../data/memory-context-contract.txt");
const GUIDE:&str=include_str!("../../data/memory-context-guide.txt");
const NEUTRAL:&str=include_str!("../../data/memory-context-neutral.txt");
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MemoryContextFormat { Legacy, Structured }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MemoryUse { Legacy, Neutral, Experience }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MemoryContextOptions {
 pub format:MemoryContextFormat,pub usage:MemoryUse,
 pub episodes:bool,pub dispositions:bool,pub associations:bool,pub affect:bool,pub words:usize,
}
impl Default for MemoryContextOptions {
 fn default()->Self { Self {format:MemoryContextFormat::Structured,usage:MemoryUse::Experience,
  episodes:true,dispositions:true,associations:true,affect:true,words:600} }
}
impl MemoryContextOptions {
 pub fn validate(&self)->Result<(),String>{
  if !(32..=4000).contains(&self.words){return Err("memory_context_words must be in 32..4000".into());}
  if self.format==MemoryContextFormat::Legacy&&!(self.episodes&&self.dispositions&&self.associations&&self.affect){return Err("component cuts require memory_context=structured".into());}Ok(())
 }
 pub fn from_config(c:&Config)->Result<Self,String>{
  fn flag(c:&Config,k:&str)->Result<bool,String>{match c.resolve_or(None,k,"true").to_ascii_lowercase().as_str(){"true"|"on"|"1"=>Ok(true),"false"|"off"|"0"=>Ok(false),_=>Err(format!("{k} must be true or false"))}}
  let format=match c.resolve_or(None,"memory_context","structured").as_str(){"legacy"=>MemoryContextFormat::Legacy,"structured"=>MemoryContextFormat::Structured,_=>return Err("memory_context must be legacy or structured".into())};
  let usage=match c.resolve_or(None,"memory_use","experience").as_str(){"legacy"=>MemoryUse::Legacy,"neutral"=>MemoryUse::Neutral,"experience"=>MemoryUse::Experience,_=>return Err("memory_use must be legacy, neutral or experience".into())};
  let o=Self{format,usage,episodes:flag(c,"memory_context_episodes")?,dispositions:flag(c,"memory_context_dispositions")?,associations:flag(c,"memory_context_associations")?,affect:flag(c,"memory_context_affect")?,words:c.resolve_or(None,"memory_context_words","600").parse().map_err(|_|"invalid memory_context_words")?};o.validate()?;Ok(o)
 }
 pub fn to_json(&self)->String{format!("{{\"format\":{},\"usage\":{},\"episodes\":{},\"dispositions\":{},\"associations\":{},\"affect\":{},\"word_budget\":{}}}",q(match self.format{MemoryContextFormat::Legacy=>"legacy",MemoryContextFormat::Structured=>"structured"}),q(match self.usage{MemoryUse::Legacy=>"legacy",MemoryUse::Neutral=>"neutral",MemoryUse::Experience=>"experience"}),self.episodes,self.dispositions,self.associations,self.affect,self.words)}
}
#[derive(Clone,Debug)]
pub struct MemoryEpisode{pub trace_id:String,pub narrative:String,pub attribution:Attribution,pub channel:Channel,pub fidelity:f32,pub confidence:f32,pub affect:Option<Mood>,pub scope_facets:Vec<crate::ContextFacet>,pub truncated:bool}
#[derive(Clone,Debug)]
pub struct AcquiredDisposition{pub axiom_id:String,pub statement:String,pub layer:AxiomLayer,pub strength:f32,pub attribution:String,pub support_trace_ids:Vec<String>,pub truncated:bool}
#[derive(Clone,Debug)]
pub struct ActiveAssociation{pub a:String,pub b:String,pub weight:f32}
#[derive(Clone,Debug)]
pub struct SelfPerspective{pub trace_id:String,pub role:String,pub context:String,pub statement:String,pub certainty:f32,pub importance:f32,pub foundations_missing:bool,pub counterexamples:usize,pub scope_status:String,pub scope_parent:Option<String>,pub scope_facets:Vec<crate::ContextFacet>,pub truncated:bool}
#[derive(Clone,Debug)]
pub struct ContextStatement{pub statement:String,pub truncated:bool}
#[derive(Clone,Debug,Default)]
pub struct MemoryContext{pub self_model:Vec<SelfPerspective>,pub agent_name:Option<String>,pub episodes:Vec<MemoryEpisode>,pub dispositions:Vec<AcquiredDisposition>,pub contextual_statements:Vec<ContextStatement>,pub associations:Vec<ActiveAssociation>,pub affect:Option<Mood>,pub content_words:usize}
impl MemoryContext {
 pub fn from_selected(store:&MemoryStore,ids:&[String],narratives:&[String],axioms:&[String],mood:&Mood,o:&MemoryContextOptions,now:u64)->Self{
  let mut c=Self::default();let mut remaining=o.words.min(4000);let mut seen=BTreeSet::new();
  if o.episodes {for (id,narrative) in ids.iter().zip(narratives).take(4){let Some(t)=store.traces.get(id)else{continue};if t.suppressed||t.status==TraceStatus::Latent||!seen.insert(id){continue;}
   let (narrative,truncated)=clip(narrative,&mut remaining,160);if narrative.is_empty(){continue;}
   c.episodes.push(MemoryEpisode{trace_id:id.clone(),narrative,attribution:t.attribution,channel:t.channel,fidelity:t.fidelity,confidence:t.confidence,affect:o.affect.then(||Mood{valence:t.valence,arousal:t.arousal,disgust:t.disgust}),scope_facets:t.personal.scope_facets.clone(),truncated});}}
  let mut seen=BTreeSet::new();if o.dispositions{for line in axioms.iter().take(4){if !seen.insert(line){continue;}
   let mut matches:Vec<_>=store.living_axioms().into_iter().filter(|a|a.statement==*line).collect();matches.sort_by(|a,b|a.id.cmp(&b.id));
   if let Some(a)=matches.first(){let support:Vec<_>=a.support_trace_ids.iter().filter_map(|id|store.traces.get(id)).collect();if support.len()!=a.support_trace_ids.len()||support.iter().any(|t|t.suppressed||t.status==TraceStatus::Latent){continue;}
    let origin=if support.is_empty(){"unspecified"}else if support.iter().all(|t|t.attribution==Attribution::Internal){"internal"}else if support.iter().all(|t|t.attribution==Attribution::External){"external"}else{"mixed_or_unspecified"};
    let(statement,truncated)=clip(line,&mut remaining,80);if !statement.is_empty(){c.dispositions.push(AcquiredDisposition{axiom_id:a.id.clone(),statement,layer:a.layer,strength:a.strength,attribution:origin.into(),support_trace_ids:a.support_trace_ids.clone(),truncated});}
   }else{if store.axioms.values().any(|a|a.statement==*line){continue;}let(statement,truncated)=clip(line,&mut remaining,80);if !statement.is_empty(){c.contextual_statements.push(ContextStatement{statement,truncated});}}
  }}
  if o.associations&&store.associations.reconstruct{for (i,a) in c.episodes.iter().enumerate(){for b in c.episodes.iter().skip(i+1){let weight=store.associations.weight(&a.trace_id,&b.trace_id,now);if weight.is_finite()&&weight>0.0{c.associations.push(ActiveAssociation{a:a.trace_id.clone(),b:b.trace_id.clone(),weight});}}}}
  c.affect=o.affect.then(||mood.clone());c.content_words=o.words.min(4000)-remaining;c
 }
 pub fn attach_self_view(&mut self,store:&MemoryStore,query:&str,o:&MemoryContextOptions){
  self.attach_self_view_scoped(store,query,o,&[])
 }
 pub fn attach_self_view_scoped(&mut self,store:&MemoryStore,query:&str,o:&MemoryContextOptions,facets:&[crate::ContextFacet]){
  self.attach_self_view_in_context(store,query,None,o,facets)
 }
 pub fn attach_self_view_in_context(&mut self,store:&MemoryStore,query:&str,context:Option<&str>,o:&MemoryContextOptions,facets:&[crate::ContextFacet]){
  if !o.dispositions{return;}
  self.self_model=crate::core::self_model::view_scoped(store,query,context,facets).into_iter().take(4).map(|t|SelfPerspective{trace_id:t.id.clone(),role:t.personal.role.token().into(),context:t.personal.context.clone(),statement:t.gist.clone(),certainty:t.personal.certainty,importance:t.personal.importance,foundations_missing:t.personal.foundations_missing,counterexamples:t.personal.counter_ids.len(),scope_status:t.personal.scope_status.token().into(),scope_parent:t.personal.scope_parent.clone(),scope_facets:t.personal.scope_facets.clone(),truncated:false}).collect();
  *self=self.filtered(o);
 }
 pub fn filtered(&self,o:&MemoryContextOptions)->Self{
  let mut c=self.clone();let mut remaining=o.words.min(4000);if !o.dispositions{c.self_model.clear();}c.self_model.truncate(4);for s in &mut c.self_model{let(t,x)=clip(&s.context,&mut remaining,20);s.context=t;s.truncated|=x;let(t,x)=clip(&s.statement,&mut remaining,60);s.statement=t;s.truncated|=x;}c.self_model.retain(|s|!s.statement.is_empty());if !o.episodes{c.episodes.clear();}c.episodes.truncate(4);
  for e in &mut c.episodes{let(t,x)=clip(&e.narrative,&mut remaining,160);e.narrative=t;e.truncated|=x;if !o.affect{e.affect=None;}}c.episodes.retain(|e|!e.narrative.is_empty());
  if !o.dispositions{c.dispositions.clear();c.contextual_statements.clear();}c.dispositions.truncate(4);c.contextual_statements.truncate(4-c.dispositions.len());
  for a in &mut c.dispositions{let(t,x)=clip(&a.statement,&mut remaining,80);a.statement=t;a.truncated|=x;}for a in &mut c.contextual_statements{let(t,x)=clip(&a.statement,&mut remaining,80);a.statement=t;a.truncated|=x;}
  c.dispositions.retain(|a|!a.statement.is_empty());c.contextual_statements.retain(|a|!a.statement.is_empty());if !o.associations{c.associations.clear();}
  c.associations.retain(|a|c.episodes.iter().any(|e|e.trace_id==a.a)&&c.episodes.iter().any(|e|e.trace_id==a.b));c.associations.truncate(6);if !o.affect{c.affect=None;}c.content_words=o.words.min(4000)-remaining;c
 }
 pub fn to_json(&self)->String{
  let self_model=self.self_model.iter().map(|s|format!("{{\"trace_id\":{},\"role\":{},\"context\":{},\"statement\":{},\"certainty\":{},\"importance\":{},\"foundations_missing\":{},\"counterexamples\":{},\"scope_status\":{},\"scope_parent\":{},\"scope_facets\":{},\"truncated\":{}}}",q(&s.trace_id),q(&s.role),q(&s.context),q(&s.statement),number(s.certainty),number(s.importance),s.foundations_missing,s.counterexamples,q(&s.scope_status),s.scope_parent.as_ref().map(|x|q(x)).unwrap_or_else(||"null".into()),strings(&s.scope_facets.iter().map(|f|format!("{}={}",f.dimension,f.value)).collect::<Vec<_>>()),s.truncated)).collect::<Vec<_>>().join(",");
  let episodes=self.episodes.iter().map(|e|format!("{{\"trace_id\":{},\"narrative\":{},\"attribution\":{},\"channel\":{},\"fidelity\":{},\"confidence\":{},\"affect\":{},\"scope_facets\":{},\"truncated\":{}}}",q(&e.trace_id),q(&e.narrative),q(e.attribution.token()),q(match e.channel{Channel::Selfhood=>"selfhood",Channel::World=>"world",Channel::Log=>"log"}),number(e.fidelity),number(e.confidence),mood_json(e.affect.as_ref()),strings(&e.scope_facets.iter().map(|f|format!("{}={}",f.dimension,f.value)).collect::<Vec<_>>()),e.truncated)).collect::<Vec<_>>().join(",");
  let dispositions=self.dispositions.iter().map(|a|format!("{{\"axiom_id\":{},\"statement\":{},\"layer\":{},\"strength\":{},\"attribution\":{},\"support_trace_ids\":{},\"support_trace_count\":{},\"support_ids_truncated\":{},\"truncated\":{}}}",q(&a.axiom_id),q(&a.statement),q(match a.layer{AxiomLayer::Motif=>"motif",AxiomLayer::Belief=>"belief",AxiomLayer::Trait=>"trait"}),number(a.strength),q(&a.attribution),strings(&a.support_trace_ids[..a.support_trace_ids.len().min(32)]),a.support_trace_ids.len(),a.support_trace_ids.len()>32,a.truncated)).collect::<Vec<_>>().join(",");
  let statements=self.contextual_statements.iter().map(|s|format!("{{\"statement\":{},\"status\":\"supplied_context_not_acquired_disposition\",\"truncated\":{}}}",q(&s.statement),s.truncated)).collect::<Vec<_>>().join(",");
  let associations=self.associations.iter().map(|a|format!("{{\"a\":{},\"b\":{},\"weight\":{}}}",q(&a.a),q(&a.b),number(a.weight))).collect::<Vec<_>>().join(",");
  format!("{{\"self_model\":[{self_model}],\"version\":{},\"episodes\":[{}],\"dispositions\":[{}],\"contextual_statements\":[{}],\"associations\":[{}],\"affect\":{},\"content_words\":{}}}",q(MEMORY_CONTEXT_VERSION),episodes,dispositions,statements,associations,mood_json(self.affect.as_ref()),self.content_words)
 }
}
pub struct MemoryReply<'a>{pub user:&'a str,pub memories:&'a[String],pub axioms:&'a[String],pub mood:&'a Mood,pub talk:&'a WorkingTalk,pub reading:&'a str,pub context:&'a MemoryContext,pub options:&'a MemoryContextOptions}
impl MemoryReply<'_>{
 pub fn filtered_context(&self)->MemoryContext{self.context.filtered(self.options)}
 pub fn system_prompt(&self)->String{
  let base=match self.options.usage{MemoryUse::Legacy=>format!("{}\n{}",crate::lexicon::prompts().reply,crate::net::httpx::READING_CONSTRAINT),_=>NEUTRAL.into()};
  let guide=if self.options.format==MemoryContextFormat::Structured||self.options.usage!=MemoryUse::Legacy{format!("\n{GUIDE}")}else{String::new()};
  let contract=if self.options.usage==MemoryUse::Experience{format!("\n{MEMORY_USE_CONTRACT}")}else{String::new()};
  // Constitutive identity is not a recollection or a human-name assertion.
  let identity=self.context.agent_name.as_deref().filter(|n|!n.is_empty())
   .map(|name|format!("\nYou are the agent named {:?}. That is YOUR own name, NOT the human's. Do not infer the human's name from yours. Only use an explicitly supplied, currently remembered human name; otherwise say you do not know.",name)).unwrap_or_default();
  format!("{base}{guide}{contract}{identity}")
 }
 pub fn user_prompt(&self)->String{
  if self.options.format==MemoryContextFormat::Structured{return format!("{{\"working_conversation\":{},\"memory\":{},\"human\":{}}}",q(&self.talk.render()),self.filtered_context().to_json(),q(self.user));}
  let mut c=self.talk.render();if !self.reading.is_empty(){c.push_str("reading: ");c.push_str(self.reading);c.push('\n');}
  for(i,m)in self.memories.iter().take(4).enumerate(){c.push_str(if i==0{"this hour: "}else{"- memory: "});c.push_str(m);c.push('\n');}for a in self.axioms.iter().take(4){c.push_str("- axiom: ");c.push_str(a);c.push('\n');}
  format!("mood v={:.2} a={:.2} d={:.2}\n{c}\nhuman: {}",self.mood.valence,self.mood.arousal,self.mood.disgust,self.user)
 }
 pub fn audit_json(&self,supports:bool)->String{let c=self.filtered_context();format!("{{\"version\":{},\"requested\":{},\"backend_supports_context\":{},\"structured_delivery_supported\":{},\"experience_contract_supported\":{},\"trace_ids\":{},\"axiom_ids\":{},\"association_count\":{},\"content_words\":{}}}",q(MEMORY_CONTEXT_VERSION),self.options.to_json(),supports,supports&&self.options.format==MemoryContextFormat::Structured,supports&&self.options.usage==MemoryUse::Experience,strings(&c.episodes.iter().map(|e|e.trace_id.clone()).collect::<Vec<_>>()),strings(&c.dispositions.iter().map(|a|a.axiom_id.clone()).collect::<Vec<_>>()),c.associations.len(),c.content_words)}
}
fn q(s:&str)->String{format!("\"{}\"",json_esc(s))}
fn number(n:f32)->String{if n.is_finite(){n.to_string()}else{"null".into()}}
fn strings(xs:&[String])->String{format!("[{}]",xs.iter().map(|s|q(s)).collect::<Vec<_>>().join(","))}
fn mood_json(m:Option<&Mood>)->String{m.map(|m|format!("{{\"valence\":{},\"arousal\":{},\"disgust\":{}}}",number(m.valence),number(m.arousal),number(m.disgust))).unwrap_or_else(||"null".into())}
fn clip(s:&str,remaining:&mut usize,cap:usize)->(String,bool){let n=s.split_whitespace().count();let k=n.min(*remaining).min(cap);*remaining-=k;if k==n{(s.into(),false)}else{(s.split_whitespace().take(k).collect::<Vec<_>>().join(" "),true)}}
