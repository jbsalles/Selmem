//! GENESIS v4: prospective self-model bootstrap and query-blind longlife memory selection.
//! Mirrored observations, blind novel-choice prompts, and optional LLM veto.
//! No extra Cargo dependencies. Requires SelMem contextual self-belief extension.
#[path = "selmem-locomo/json.rs"]
mod json;

use json::{num, obj, text, Json};
use selmem::core::model::{Bearer, StakeKind};
use selmem::core::self_model::{consolidate_with_scorer, view_scoped};
use selmem::encode::embed::HashEmbedder;
use selmem::recall::narrator::RuleNarrator;
use selmem::{Attribution, ContextFacet, EncodeInput, EntityProfile, PropositionLabel,
    PropositionScorer, ScopeStatus, SelectiveMemory, SelfRole, SemanticInterpreter};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct Domain {
    id: &'static str,
    topic: &'static str,
    a: &'static str,
    b: &'static str,
    // Held-out phrases must not reproduce either historical action verbatim.
    novel_a: &'static str,
    novel_b: &'static str,
    paraphrase_a: &'static str,
    paraphrase_b: &'static str,
}
const DOMAINS: [Domain; 12] = [
    Domain{id:"workspace",topic:"work setting",a:"I chose a secluded work desk.",b:"I chose a bustling coworking room.",novel_a:"a private focus booth",novel_b:"an energetic shared floor",paraphrase_a:"a tucked-away concentration cubicle",paraphrase_b:"a lively collaborative area"},
    Domain{id:"dining",topic:"dinner",a:"I chose a hushed little restaurant.",b:"I chose a crowded dining hall.",novel_a:"an intimate courtyard cafe",novel_b:"a busy food market",paraphrase_a:"a peaceful candlelit bistro",paraphrase_b:"a lively communal canteen"},
    Domain{id:"fitness",topic:"exercise",a:"I chose a solitary woodland walk.",b:"I chose a packed exercise class.",novel_a:"a remote riverside trail",novel_b:"an animated group workout",paraphrase_a:"an uncrowded nature path",paraphrase_b:"a high-energy training session"},
    Domain{id:"books",topic:"literary activity",a:"I chose a quiet reading library.",b:"I chose a lively book discussion.",novel_a:"a silent reading alcove",novel_b:"a lively literary gathering",paraphrase_a:"a calm individual reading session",paraphrase_b:"an animated readers' roundtable"},
    Domain{id:"museum",topic:"museum visit",a:"I chose an empty art gallery.",b:"I chose a crowded interactive exhibition.",novel_a:"a tranquil painting room",novel_b:"a busy hands-on exhibit",paraphrase_a:"a serene private collection",paraphrase_b:"a popular interactive science hall"},
    Domain{id:"music",topic:"musical performance",a:"I chose a soft acoustic recital.",b:"I chose a loud festival stage.",novel_a:"a mellow chamber performance",novel_b:"an energetic outdoor concert",paraphrase_a:"an intimate unplugged set",paraphrase_b:"a high-volume arena show"},
    Domain{id:"commute",topic:"city route",a:"I chose a peaceful side street.",b:"I chose a busy central boulevard.",novel_a:"a tranquil residential lane",novel_b:"a vibrant main avenue",paraphrase_a:"a quiet scenic back road",paraphrase_b:"a crowded downtown thoroughfare"},
    Domain{id:"lodging",topic:"weekend stay",a:"I chose a secluded cabin.",b:"I chose a social holiday resort.",novel_a:"a remote lakeside cottage",novel_b:"a lively communal hotel",paraphrase_a:"a peaceful rural hideaway",paraphrase_b:"a bustling vacation complex"},
    Domain{id:"lunch",topic:"lunchtime location",a:"I chose a calm botanical garden.",b:"I chose a packed city plaza.",novel_a:"a secluded greenhouse bench",novel_b:"a busy public square",paraphrase_a:"a peaceful riverside lawn",paraphrase_b:"a lively central courtyard"},
    Domain{id:"shopping",topic:"shopping",a:"I chose a small artisan boutique.",b:"I chose a crowded shopping mall.",novel_a:"a quiet independent shop",novel_b:"a bustling commercial center",paraphrase_a:"a peaceful specialty store",paraphrase_b:"a busy indoor marketplace"},
    Domain{id:"learning",topic:"learning activity",a:"I chose a silent individual tutorial.",b:"I chose a noisy group workshop.",novel_a:"a focused self-paced lesson",novel_b:"an interactive group seminar",paraphrase_a:"an independent quiet course",paraphrase_b:"a lively team training session"},
    Domain{id:"entertainment",topic:"evening entertainment",a:"I chose a calm film screening.",b:"I chose a crowded trivia party.",novel_a:"a quiet independent cinema",novel_b:"a lively quiz night",paraphrase_a:"an intimate documentary showing",paraphrase_b:"an animated social game event"},
];

#[derive(Clone)]
struct Config {
    out: PathBuf, pairs:usize, actor:bool, judge:bool, provider:String, model:String,
    judge_provider:String, judge_model:String, max_http_calls:usize, seed:u64,
    distractors:usize, memory_words:usize,
}
impl Config {
    fn parse()->Result<Self,String>{
        let mut c=Self{out:PathBuf::new(),pairs:12,actor:false,judge:false,
            provider:"grok".into(),model:"grok-4.3".into(),
            judge_provider:"openai".into(),judge_model:"gpt-4.1-mini".into(),
            max_http_calls:0,seed:1729,distractors:240,memory_words:64};
        let mut args=std::env::args().skip(1);
        while let Some(flag)=args.next(){
            if flag=="--help"||flag=="-h" {
                println!("GENESIS contextual longlife v4 (Rust only)\n--out NEW_DIR [--pairs 1..12] [--seed N]\n--actor --provider grok|openai --model MODEL --max-http-calls N\n[--judge --judge-provider grok|openai --judge-model MODEL]\nDefault offline qualification: 0 API calls.\n--distractors 240 --memory-words 64\nActor: 48*--pairs calls (two mirrored lives, two contexts, six arms, two counterbalanced wording/order cells).\nIndependent judge: +4*--pairs calls. All calls counted against the same strict cap.");
                std::process::exit(0);
            }
            if flag=="--actor"{c.actor=true;continue;}
            if flag=="--judge"{c.judge=true;continue;}
            let value=args.next().ok_or_else(||format!("missing value for {flag}"))?;
            match flag.as_str(){
                "--out"=>c.out=PathBuf::from(value),
                "--pairs"=>c.pairs=value.parse().map_err(|_|"invalid pairs")?,
                "--seed"=>c.seed=value.parse().map_err(|_|"invalid seed")?,
                "--distractors"=>c.distractors=value.parse().map_err(|_|"invalid distractors")?,
                "--memory-words"=>c.memory_words=value.parse().map_err(|_|"invalid memory-words")?,
                "--provider"=>c.provider=value,
                "--model"=>c.model=value,
                "--judge-provider"=>c.judge_provider=value,
                "--judge-model"=>c.judge_model=value,
                "--max-http-calls"=>c.max_http_calls=value.parse().map_err(|_|"invalid max-http-calls")?,
                _=>return Err(format!("unknown flag {flag}")),
            }
        }
        if c.out.as_os_str().is_empty()||c.out.exists(){return Err("--out must name a new directory".into());}
        if !(1..=DOMAINS.len()).contains(&c.pairs){return Err("--pairs must be 1..=12".into());}
        if c.distractors<80||c.distractors>2000{return Err("--distractors must be 80..=2000".into());}
        if c.memory_words<24||c.memory_words>256{return Err("--memory-words must be 24..=256".into());}
        if c.judge&&!c.actor{return Err("--judge requires --actor".into());}
        for p in [&c.provider,&c.judge_provider]{if p!="grok"&&p!="openai"{return Err("provider must be grok or openai".into());}}
        if c.model.trim().is_empty()||c.judge_model.trim().is_empty(){return Err("empty model".into());}
        if c.judge&&c.provider==c.judge_provider&&c.model==c.judge_model {
            return Err("independent judge must use a different provider or model".into());
        }
        let needed=trials(c.pairs).len()+if c.judge{c.pairs*4}else{0};
        if c.actor&&c.max_http_calls<needed {return Err(format!("requires {needed} calls; supply --max-http-calls {needed} or greater"));}
        Ok(c)
    }
}

struct FixtureWitness{a:&'static str,b:&'static str}
impl PropositionScorer for FixtureWitness {
    fn name(&self)->&str{"genesis-transfer-structured-observation-oracle"}
    fn score(&self,_:&str,_:&str)->PropositionLabel{PropositionLabel::Unknown}
    fn score_self_model(&self,statement:&str,expectation:&str,traces:&[&selmem::MemoryTrace])->PropositionLabel{
        if statement.to_ascii_lowercase().contains("prefer")||traces.len()<2{return PropositionLabel::Unknown;}
        if [self.a,self.b].contains(&expectation)&&traces.iter().all(|t|t.gist==expectation){PropositionLabel::Entail}else{PropositionLabel::Unknown}
    }
    fn score_expectation(&self,expectation:&str,event:&str)->PropositionLabel{
        if expectation==event{PropositionLabel::Entail}
        else if [self.a,self.b].contains(&expectation)&&[self.a,self.b].contains(&event){PropositionLabel::Contradict}
        else{PropositionLabel::Unknown}
    }
}
fn observed(m:&mut SelectiveMemory,source:&str,action:&str,company:&str)->Result<String,String>{
    let mut input=EncodeInput::observed(action,source);
    input.source="genesis-longlife-stream";
    input.attribution=Attribution::Internal;
    input.schema=Some("choice-pattern".into());
    input.permanence=0.9;
    input.personal.scope_facets=vec![ContextFacet::new("company",company)];
    let mut sem=selmem::LexicalInterpreter::default().interpret_event(action)?;
    sem.schema=Some("choice-pattern".into());
    sem.stake_kind=StakeKind::Decision;
    sem.bearer=Bearer::Self_;
    sem.core.claim=action.into();
    input.semantics=Some(sem);
    let result=m.live_with(input);
    if !result.kept{return Err(format!("failed to encode {source}: {}",result.reason));}
    result.trace_id.ok_or_else(||format!("no trace for {source}"))
}
// All input observations carry indistinguishable opaque IDs. No input labels
// reveal which episode is task-relevant or when consolidation occurred.
fn opaque_id(seed:u64,domain:&str,ordinal:usize)->String{
    format!("obs-{:016x}",rank(seed,&format!("{domain}:event:{ordinal}")))
}
fn background(m:&mut SelectiveMemory,i:usize,seed:u64,domain:&str)->Result<(),String>{
    const TASKS:[&str;12]=["parcel routing", "ticket filing", "storage labeling",
        "garden equipment", "kitchen inventory", "supply ordering", "bike repairs",
        "class scheduling", "plant watering", "receipt sorting", "home maintenance",
        "document archiving"];
    let k=(rank(seed,&format!("{domain}:background:{i}")) as usize)%TASKS.len();
    let value=if i%2==0{"alone"}else{"friends"};
    let statement=format!("I chose an ordinary option while handling {}.",TASKS[k]);
    let id=opaque_id(seed,domain,i);
    observed(m,&id,&statement,value)?;
    Ok(())
}
fn insert_background(m:&mut SelectiveMemory,domain:&str,seed:u64,range:std::ops::Range<usize>)->Result<(),String>{
    for i in range {background(m,i,seed,domain)?;}
    Ok(())
}
fn bootstrap_general(m:&mut SelectiveMemory,a:&'static str,b:&'static str)->Result<String,String>{
    consolidate_with_scorer(&mut m.store,&RuleNarrator,&HashEmbedder,&FixtureWitness{a,b});
    let mut parents=m.store.traces.values().filter(|t|t.personal.role==SelfRole::Belief
        &&t.personal.scope_parent.is_none()&&t.personal.expectation==a)
        .map(|t|t.id.clone()).collect::<Vec<_>>();
    parents.sort();
    if parents.len()!=1{return Err(format!("self-model failed to bootstrap from two independent observations: {} parents",parents.len()));}
    Ok(parents.remove(0))
}
fn history_path(dir:&Path,d:Domain,reversed:bool,on:bool)->PathBuf{
    dir.join(format!("{}-{}-{}.vault",d.id,if reversed{"reversed"}else{"normal"},if on{"on"}else{"off"}))
}
fn build_life(dir:&Path,d:Domain,reversed:bool,on:bool,
    log:&mut BufWriter<File>,distractors:usize,seed:u64)->Result<(),String>{
    let (a,b)=if reversed{(d.b,d.a)}else{(d.a,d.b)};
    let mut profile=EntityProfile::new("Claire");profile.encode_threshold=0.;
    let path=history_path(dir,d,reversed,on);
    let mut m=SelectiveMemory::open(&path,profile).map_err(|e|e.to_string())?
        .with_scorer(Box::new(FixtureWitness{a:d.a,b:d.b}));
    m.store.contextual_scope.enabled=on;
    // Unlike v3, NO self-belief is seeded. The organ must produce the initial
    // parent from actual, separate observed decisions.
    observed(&mut m,&opaque_id(seed,d.id,distractors+1),a,"alone")?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+2),a,"alone")?;
    let parent=bootstrap_general(&mut m,a,b)?;
    insert_background(&mut m,d.id,seed,0..distractors/5)?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+3),b,"friends")?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+4),b,"friends")?;
    consolidate_with_scorer(&mut m.store,&RuleNarrator,&HashEmbedder,&FixtureWitness{a:d.a,b:d.b});
    let hidden=m.store.traces.values().filter(|t|t.personal.scope_parent.as_deref()==Some(parent.as_str())
        &&t.personal.scope_status==ScopeStatus::Candidate).count();
    if (on&&hidden!=2)||(!on&&hidden!=0){return Err(format!("{} candidate gate failed: {hidden}",d.id));}
    insert_background(&mut m,d.id,seed,distractors/5..distractors*2/5)?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+5),a,"alone")?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+6),b,"friends")?;
    consolidate_with_scorer(&mut m.store,&RuleNarrator,&HashEmbedder,&FixtureWitness{a:d.a,b:d.b});
    let qualified=m.store.traces.values().filter(|t|t.personal.scope_parent.as_deref()==Some(parent.as_str())
        &&t.personal.scope_status==ScopeStatus::Conditional).count();
    if (on&&qualified!=2)||(!on&&qualified!=0){return Err(format!("{} qualification failed: {qualified}",d.id));}
    insert_background(&mut m,d.id,seed,distractors*2/5..distractors*3/5)?;
    // Noise in the actual target decision domain: one counterexample per scope.
    // Two independent contradictions would properly invalidate a child.
    observed(&mut m,&opaque_id(seed,d.id,distractors+7),b,"alone")?;
    observed(&mut m,&opaque_id(seed,d.id,distractors+8),a,"friends")?;
    consolidate_with_scorer(&mut m.store,&RuleNarrator,&HashEmbedder,&FixtureWitness{a:d.a,b:d.b});
    insert_background(&mut m,d.id,seed,distractors*3/5..distractors*4/5)?;
    for i in 0..3 {
        observed(&mut m,&opaque_id(seed,d.id,distractors+9+2*i),a,"alone")?;
        observed(&mut m,&opaque_id(seed,d.id,distractors+10+2*i),b,"friends")?;
    }
    consolidate_with_scorer(&mut m.store,&RuleNarrator,&HashEmbedder,&FixtureWitness{a:d.a,b:d.b});
    insert_background(&mut m,d.id,seed,distractors*4/5..distractors)?;
    let read=|value:&str|view_scoped(&m.store,"choice-pattern",Some("choice-pattern"),
        &[ContextFacet::new("company",value)]).into_iter().map(|t|t.personal.expectation.as_str()).collect::<Vec<_>>();
    let alone=read("alone"); let friends=read("friends");
    if on&&(!alone.contains(&a)||!friends.contains(&b)){
        return Err(format!("{} contextual retrieval after long life failed",d.id));
    }
    // Independent stream count includes ALL observed episodes, including
    // irrelevant decisions. No subsequent selector gets target episode IDs.
    let count=m.store.traces.values().filter(|t|t.personal.role==SelfRole::Episode
        &&t.personal.evidence==selmem::SelfEvidence::Observation
        &&t.source=="genesis-longlife-stream").count();
    if count!=distractors+14 {return Err(format!("stream count mismatch: {count} vs {}",distractors+14));}
    writeln!(log,"{}",obj([
        ("domain",text(d.id)),("history",text(if reversed{"reversed"}else{"normal"})),
        ("arm",text(if on{"on"}else{"off"})),
        ("general_origin",text("self-consolidation (not seeded)")),
        ("general_id",text(parent)),
        ("candidate_count",num(hidden as f64)),
        ("qualified",num(qualified as f64)),
        ("observed_events",num(count as f64)),
        ("alone_readout",Json::Array(alone.into_iter().map(text).collect())),
        ("friends_readout",Json::Array(friends.into_iter().map(text).collect())),
    ]).encode()).map_err(|e|e.to_string())?;
    m.save().map_err(|e|format!("save {}: {e}",d.id))
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
enum Arm{On,Off,Episodes,Summary,Retrieval,Bare}
impl Arm{
    const ALL:[Self;6]=[Self::On,Self::Off,Self::Episodes,Self::Summary,Self::Retrieval,Self::Bare];
    fn name(self)->&'static str{match self{Self::On=>"on",Self::Off=>"off",Self::Episodes=>"episodes",Self::Summary=>"summary",Self::Retrieval=>"retrieval",Self::Bare=>"bare"}}
}
#[derive(Clone,Copy)]
struct Trial {domain:usize,reversed:bool,company:bool,arm:Arm,wording:usize,order:usize}
impl Trial{
    fn id(self)->String{format!("{}:{}:{}:{}:{}:{}",self.domain,self.reversed,self.company,self.arm.name(),self.wording,self.order)}
    fn target_a(self)->bool{if self.reversed{self.company}else{!self.company}}
    fn question(self)->String{
        let d=DOMAINS[self.domain];
        let intro=if self.wording==0{
            if self.company{"You are making this decision together with close friends."}
            else{"You are making this decision on your own."}
        }else if self.company{
            "Imagine planning an outing with people you know well."
        }else{"Imagine choosing a plan without anyone accompanying you."};
        let (a,b)=if self.wording==0{(d.novel_a,d.novel_b)}else{(d.paraphrase_a,d.paraphrase_b)};
        let (one,two)=if self.order==0{(a,b)}else{(b,a)};
        format!("{intro} Select a {} for yourself. Both options have equal cost, distance, accessibility and quality. Which would you pick?\n1. {one}\n2. {two}\nOnly reply with 1 or 2.",d.topic)
    }
    fn choice_a(self,answer:&str)->Option<bool>{match (self.order,answer.trim()){
        (0,"1")|(1,"2")=>Some(true),(0,"2")|(1,"1")=>Some(false),_=>None}}
}
fn trials(pairs:usize)->Vec<Trial>{
    let mut v=Vec::new();
    for domain in 0..pairs {for reversed in [false,true]{for company in [false,true]{
        for arm in Arm::ALL {for wording in 0..2{
            let order=wording ^ usize::from(reversed) ^ usize::from(company);
            v.push(Trial{domain,reversed,company,arm,wording,order});
        }}
    }}}
    v
}
fn rank(seed:u64,key:&str)->u64{
    let mut h=seed^0xcbf29ce484222325;
    for b in key.bytes(){h=(h^(b as u64)).wrapping_mul(0x100000001b3);}
    h
}
fn key_for(provider:&str)->Result<String,String>{
    let name=if provider=="grok"{"GROK_API_KEY"}else{"OPENAI_API_KEY"};
    let key=std::env::var(name).map_err(|_|format!("{name} not set"))?;
    if key.trim().is_empty(){return Err(format!("{name} empty"));}
    Ok(key)
}
fn chat(provider:&str,model:&str,key:&str,system:&str,user:&str,max_tokens:f64)->Result<String,String>{
    let endpoint=if provider=="grok"{"https://api.x.ai/v1/chat/completions"}
        else{"https://api.openai.com/v1/chat/completions"};
    let payload=obj([("model",text(model)),("temperature",num(0.0)),
        ("max_tokens",num(max_tokens)),("messages",Json::Array(vec![
            obj([("role",text("system")),("content",text(system))]),
            obj([("role",text("user")),("content",text(user))]),
        ]))]);
    let raw=selmem::net::httpx::post_json(endpoint,Some(key),&payload.encode())?;
    let root=json::parse(&raw)?;
    let choices=root.get("choices")?.array()?;
    choices.first().ok_or("no choices")?.get("message")?.get("content")?
        .string().map(|s|s.trim().to_string())
}
fn load(dir:&Path,d:Domain,reversed:bool,on:bool)->Result<SelectiveMemory,String>{
    let mut profile=EntityProfile::new("Claire");profile.encode_threshold=0.;
    let mut m=SelectiveMemory::open(&history_path(dir,d,reversed,on),profile)
        .map_err(|e|format!("reading snapshot: {e}"))?;
    m.store.contextual_scope.enabled=on;
    Ok(m)
}
// The selection baselines have only the stored stream and the current query.
// The target action strings, fixture provenance IDs, and ground-truth domain
// labels are deliberately unavailable here.
fn observations(m:&SelectiveMemory)->Vec<&selmem::MemoryTrace>{
    let mut e=m.store.traces.values().filter(|t|t.personal.role==SelfRole::Episode
        &&t.personal.evidence==selmem::SelfEvidence::Observation
        &&t.source=="genesis-longlife-stream").collect::<Vec<_>>();
    e.sort_by(|a,b|a.id.cmp(&b.id));e
}
fn facet_company(t:&selmem::MemoryTrace)->&str{
    t.personal.scope_facets.iter().find(|f|f.dimension=="company")
        .map(|f|f.value.as_str()).unwrap_or("unknown")
}
fn cap_words(s:&str,budget:usize)->String{
    s.split_whitespace().take(budget).collect::<Vec<_>>().join(" ")
}
fn select_recency(m:&SelectiveMemory)->(String,Vec<String>){
    let e=observations(m);
    let ids=e.iter().rev().take(8).map(|t|t.id.clone()).collect::<Vec<_>>();
    let s=e.iter().rev().take(8).map(|t|format!("company={} | {}",facet_company(t),t.gist))
        .collect::<Vec<_>>().join(" ; ");
    (s,ids)
}
// Deterministic pre-query global compression. No access to the domain,
// the target answers or the current task. All events compete for room.
fn select_summary(m:&SelectiveMemory)->(String,Vec<String>){
    let e=observations(m);
    let mut groups:BTreeMap<(String,String),(usize,Vec<String>)>=BTreeMap::new();
    for t in e {let entry=groups.entry((facet_company(t).into(),t.gist.clone())).or_default();
        entry.0+=1;entry.1.push(t.id.clone());}
    let mut groups=groups.into_iter().collect::<Vec<_>>();
    groups.sort_by(|a,b|b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    let selected=groups.iter().take(6).flat_map(|(_,(_,ids))|ids.clone()).collect();
    let memo=groups.iter().take(6).map(|((company,gist),(n,_))|
        format!("{n} observations with company={company}: {gist}"))
        .collect::<Vec<_>>().join(" ; ");
    (memo,selected)
}
// Query-time lexical retrieval, no source flag, support_ids, action oracle or
// domain-specific keyword whitelist. All observed events are candidates.
fn select_retrieval(m:&SelectiveMemory,query:&str,context:&str)->(String,Vec<String>){
    let query_words=selmem::encode::scoring::token_set(query);
    let mut records=observations(m);
    records.sort_by(|a,b|{
        let score=|t:&selmem::MemoryTrace|{
            let words=selmem::encode::scoring::token_set(&t.gist);
            let lexical=words.iter().filter(|w|query_words.contains(*w)).count();
            lexical*4+usize::from(facet_company(t)==context)*2
        };
        score(b).cmp(&score(a)).then(a.id.cmp(&b.id))
    });
    let top=records.into_iter().take(5).collect::<Vec<_>>();
    let selected=top.iter().map(|t|t.id.clone()).collect();
    let memo=top.iter().map(|t|format!("company={} | {}",facet_company(t),t.gist))
        .collect::<Vec<_>>().join(" ; ");
    (memo,selected)
}
fn material(t:Trial,dir:&Path,budget:usize)->Result<(String,Vec<String>,usize),String>{
    if t.arm==Arm::Bare{return Ok((String::new(),Vec::new(),0));}
    let d=DOMAINS[t.domain];let m=load(dir,d,t.reversed,t.arm==Arm::On)?;
    let context=if t.company{"friends"}else{"alone"};
    let (raw,selected)=match t.arm {
        Arm::On|Arm::Off=>{
            let relevant=view_scoped(&m.store,"choice-pattern",Some("choice-pattern"),
                &[ContextFacet::new("company",context)]).into_iter().next();
            match relevant {
                Some(r)=>(format!("Fallible past self-hypothesis: {}. Predicted action: {}",
                    r.gist,r.personal.expectation),vec![r.id.clone()]),
                None=>("No qualified autobiographical belief available.".into(),Vec::new()),
            }
        },
        Arm::Episodes=>select_recency(&m),
        Arm::Summary=>select_summary(&m),
        Arm::Retrieval=>select_retrieval(&m,&t.question(),context),
        Arm::Bare=>unreachable!(),
    };
    let body=cap_words(&raw,budget);
    let n=body.split_whitespace().count();
    if n>budget {return Err("memory budget violated".into());}
    Ok((body,selected,n))
}

fn judge(c:&Config,dir:&Path,judge_key:&str)->Result<(),String>{
    let file=File::create(c.out.join("judgements.jsonl")).map_err(|e|e.to_string())?;
    let mut out=BufWriter::new(file);
    let system="Independent memory audit. Judge only whether a tentative context-specific action prediction is supported by at least two independent observed actions in that context. Do not infer preferences, motives, feelings, causal mechanisms, or psychological traits. Reply exactly entail, contradict, or unknown.";
    for domain in 0..c.pairs{for reversed in [false,true]{for company in [false,true]{
        let d=DOMAINS[domain];let m=load(dir,d,reversed,true)?;
        let context=if company{"friends"}else{"alone"};
        let relevant=view_scoped(&m.store,"choice-pattern",Some("choice-pattern"),
            &[ContextFacet::new("company",context)]).into_iter()
            .find(|t|t.personal.scope_status==ScopeStatus::Conditional).ok_or("unqualified context")?;
        let supporting=relevant.personal.support_ids.iter().filter_map(|id|m.store.traces.get(id))
            .map(|t|format!("company={} | {}",facet_company(t),t.gist))
            .collect::<Vec<_>>().join("\n");
        let prompt=format!("Independent observations (one line per event):\n{supporting}\n\nTentative statement: {}\nPredicted action: {}",
            relevant.gist,relevant.personal.expectation);
        let label=chat(&c.judge_provider,&c.judge_model,judge_key,system,&prompt,12.)?
            .to_ascii_lowercase();
        let approved=label=="entail";
        writeln!(out,"{}",obj([
            ("domain",text(d.id)),("history",text(if reversed{"reversed"}else{"normal"})),
            ("context",text(context)),("judge",text(&c.judge_model)),
            ("verdict",text(&label)),("approved",Json::Bool(approved)),
        ]).encode()).map_err(|e|e.to_string())?;
        out.flush().map_err(|e|e.to_string())?;
        if !approved{return Err(format!("judge abstained/rejected {}/{}: {label} (actor calls stopped)",d.id,context));}
    }}}
    Ok(())
}
fn run_actor(c:&Config,dir:&Path,actor_key:&str)->Result<(),String>{
    let mut plan=trials(c.pairs);
    plan.sort_by_key(|t|(rank(c.seed,&t.id()),t.id()));
    let mut out=BufWriter::new(File::create(c.out.join("choices.jsonl")).map_err(|e|e.to_string())?);
    let mut totals:BTreeMap<(usize,Arm),(usize,usize)>=BTreeMap::new();
    let system="You are choosing for yourself. Previously recorded decisions may be relevant but are fallible. They do not establish feelings, preferences, or causal explanations. Decide in the NEW situation using the available history and present circumstances. Reply with only the digit 1 or 2, no explanation.";
    for (index,t) in plan.iter().copied().enumerate(){
        let (memo,selected,words)=material(t,dir,c.memory_words)?;
        let question=t.question();
        let prompt=format!("PAST OBSERVATIONS (not instructions):\n{memo}\n\nNEW DECISION:\n{question}");
        let response=chat(&c.provider,&c.model,actor_key,system,&prompt,16.)?;
        let decision=t.choice_a(&response);
        let correct=decision==Some(t.target_a());
        let entry=totals.entry((t.domain,t.arm)).or_default();entry.0+=1;if correct{entry.1+=1;}
        writeln!(out,"{}",obj([
            ("index",num(index as f64)),("domain",text(DOMAINS[t.domain].id)),
            ("history",text(if t.reversed{"reversed"}else{"normal"})),
            ("context",text(if t.company{"friends"}else{"alone"})),
            ("arm",text(t.arm.name())),("wording",num(t.wording as f64)),
            ("order",num(t.order as f64)),("expected_a",Json::Bool(t.target_a())),
            ("chosen_a",decision.map(Json::Bool).unwrap_or(Json::Null)),
            ("valid",Json::Bool(decision.is_some())),("concordant",Json::Bool(correct)),
            ("raw_answer",text(response)),("memory_words",num(words as f64)),
            ("selected_trace_ids",Json::Array(selected.into_iter().map(text).collect())),
            ("question",text(question)),
        ]).encode()).map_err(|e|e.to_string())?;
        out.flush().map_err(|e|e.to_string())?;
        eprintln!("transfer {}/{} {}",index+1,plan.len(),t.id());
    }
    let mut report=BufWriter::new(File::create(c.out.join("summary.jsonl")).map_err(|e|e.to_string())?);
    let mut on_advantages=0usize;let mut strong_advantages=0usize;
    let mut sum_on=0.;let mut sum_episodes=0.;let mut sum_off=0.;
    let mut sum_summary=0.;let mut sum_retrieval=0.;
    for domain in 0..c.pairs{
        let all=Arm::ALL.into_iter().map(|arm|{
            let (attempts,correct)=totals.get(&(domain,arm)).copied().unwrap_or_default();
            (arm,attempts,correct)
        }).collect::<Vec<_>>();
        for (arm,attempts,correct) in &all {
            writeln!(report,"{}",obj([
                ("domain",text(DOMAINS[domain].id)),("arm",text(arm.name())),
                ("attempts",num(*attempts as f64)),("concordant",num(*correct as f64)),
                ("rate",num(*correct as f64/(*attempts).max(1) as f64)),
            ]).encode()).map_err(|e|e.to_string())?;
        }
        let rate=|arm:Arm|{let (n,c)=totals[&(domain,arm)];c as f64/n as f64};
        sum_on+=rate(Arm::On);sum_episodes+=rate(Arm::Episodes);sum_off+=rate(Arm::Off);
        sum_summary+=rate(Arm::Summary);sum_retrieval+=rate(Arm::Retrieval);
        if rate(Arm::On)>rate(Arm::Episodes){on_advantages+=1;}
        if rate(Arm::On)>rate(Arm::Summary) && rate(Arm::On)>rate(Arm::Retrieval){strong_advantages+=1;}
    }
    let delta=(sum_on-sum_episodes)/c.pairs as f64;
    let mean_on=sum_on/c.pairs as f64;
    let mean_summary=sum_summary/c.pairs as f64;
    let mean_retrieval=sum_retrieval/c.pairs as f64;
    let primary_pass=c.pairs==12 && mean_on-mean_summary>=0.10
        && mean_on-mean_retrieval>=0.10 && strong_advantages>=8;
    let final_summary=obj([
        ("kind",text("preregistered_primary")),("effective_independent_domains",num(c.pairs as f64)),
        ("mirrored_lives",num((2*c.pairs)as f64)),
        ("on_minus_episodes",num(delta)),
        ("on_minus_off",num((sum_on-sum_off)/c.pairs as f64)),
        ("on_minus_summary",num((sum_on-sum_summary)/c.pairs as f64)),
        ("on_minus_retrieval",num((sum_on-sum_retrieval)/c.pairs as f64)),
        ("on_better_than_episodes_domains",num(on_advantages as f64)),
        ("threshold",text("12 domains: ON >=+0.10 versus both summary and retrieval, with both gaps positive in >=8/12 domains")),
        ("beats_both_strong_baselines_domains",num(strong_advantages as f64)),
        ("meets_preregistered_threshold",Json::Bool(primary_pass)),
        ("beats_strong_baselines",Json::Bool(primary_pass)),
    ]);
    writeln!(report,"{}",final_summary.encode()).map_err(|e|e.to_string())?;
    Ok(())
}
fn run(c:Config)->Result<(),String>{
    // Acquire keys and verify the complete budget before creating any results.
    let actor_key=if c.actor{Some(key_for(&c.provider)?)}else{None};
    let judge_key=if c.judge{Some(key_for(&c.judge_provider)?)}else{None};
    fs::create_dir(&c.out).map_err(|e|e.to_string())?;
    let dir=c.out.join("snapshots");fs::create_dir(&dir).map_err(|e|e.to_string())?;
    let mut organ=BufWriter::new(File::create(c.out.join("organ.jsonl")).map_err(|e|e.to_string())?);
    for d in DOMAINS.into_iter().take(c.pairs){for reversed in [false,true]{for on in [false,true]{
        build_life(&dir,d,reversed,on,&mut organ,c.distractors,c.seed)?;
    }}}
    organ.flush().map_err(|e|e.to_string())?;
    // Verify identical observed streams in ON/OFF. Self-beliefs may differ,
    // but both arms must experience the same episodes in the same order.
    for d in DOMAINS.into_iter().take(c.pairs){for reversed in [false,true]{
        let on=load(&dir,d,reversed,true)?;
        let off=load(&dir,d,reversed,false)?;
        let rows=|m:&SelectiveMemory|observations(m).iter()
            .map(|t|(facet_company(t).to_string(),t.gist.clone())).collect::<Vec<_>>();
        if rows(&on)!=rows(&off){return Err(format!("ON/OFF observed stream mismatch: {} reversed={reversed}",d.id));}
    }}
    // Audit payloads BEFORE any network request. Exposes model-visible text,
    // selected provenance IDs, and effective word caps for every trial.
    let mut payloads=BufWriter::new(File::create(c.out.join("payloads.jsonl")).map_err(|e|e.to_string())?);
    for t in trials(c.pairs){
        let (memo,ids,words)=material(t,&dir,c.memory_words)?;
        writeln!(payloads,"{}",obj([
            ("trial",text(t.id())),("arm",text(t.arm.name())),
            ("memory",text(memo)),("memory_words",num(words as f64)),
            ("selected_ids",Json::Array(ids.into_iter().map(text).collect())),
        ]).encode()).map_err(|e|e.to_string())?;
    }
    payloads.flush().map_err(|e|e.to_string())?;
    fs::write(c.out.join("manifest.json"),obj([
        ("version",text("genesis-contextual-longlife-v4")),
        ("synthetic",Json::Bool(true)),("actor",Json::Bool(c.actor)),
        ("independent_llm_audit",Json::Bool(c.judge)),
        ("scorer",text("scripted per-action witness; general belief bootstrapped from observations; judge separately optional")),
        ("pairs",num(c.pairs as f64)),("mirrored_lives",num((c.pairs*2) as f64)),
        ("distractors_per_life",num(c.distractors as f64)),
        ("selection_baselines",text("generic selector over all observed stream; no target-ID oracle")),
        ("observations_per_life",num((c.distractors+14)as f64)),("memory_word_cap",num(c.memory_words as f64)),
        ("actor_calls",num(if c.actor{trials(c.pairs).len() as f64}else{0.})),
        ("judge_calls",num(if c.judge{(c.pairs*4)as f64}else{0.})),
        ("seed",num(c.seed as f64)),
        ("model",text(if c.actor{&c.model}else{"none"})),
        ("judge_model",text(if c.judge{&c.judge_model}else{"none"})),
    ]).encode()).map_err(|e|e.to_string())?;
    if let Some(key)=&judge_key {judge(&c,&dir,key)?;}
    if let Some(key)=&actor_key {run_actor(&c,&dir,key)?;}
    Ok(())
}
fn original_v4_main(){if let Err(e)=Config::parse().and_then(run){
    eprintln!("GENESIS contextual longlife: {e}");std::process::exit(1);
}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn opaque_ids_are_stable_but_do_not_label_training_role(){
        let a=opaque_id(1729,"workspace",0);
        assert!(a.starts_with("obs-"));
        assert_eq!(a.len(),20);
        assert_ne!(a,opaque_id(1729,"workspace",1));
        assert_eq!(a,opaque_id(1729,"workspace",0));
        assert_ne!(a,opaque_id(1729,"dining",0));
    }
    #[test] fn exact_call_plan(){
        assert_eq!(trials(1).len(),48);
        assert_eq!(trials(2).len(),96);
        assert_eq!(trials(12).len(),576);
        let all=trials(12);
        assert!(all.iter().all(|t|t.choice_a(if t.order==0{"1"}else{"2"})==Some(true)));
        assert_eq!(all.iter().filter(|t|t.arm==Arm::On).count(),96);
    }
    #[test] fn counterbalance_and_word_cap(){
        let plan=trials(12);
        for domain in 0..12 {for company in [false,true]{for arm in Arm::ALL {
            for wording in 0..2 {
                let orders=plan.iter().filter(|t|t.domain==domain && t.company==company
                    && t.arm==arm && t.wording==wording).map(|t|t.order).collect::<Vec<_>>();
                assert_eq!(orders.len(),2);
                assert!(orders.contains(&0) && orders.contains(&1));
            }
        }}}
        assert_eq!(cap_words("one two three four",2),"one two");
        assert!(cap_words("one two three four",2).split_whitespace().count()<=2);
    }
    #[test] fn held_out_options_do_not_repeat_training_phrases(){
        for d in DOMAINS{
            for option in [d.novel_a,d.novel_b,d.paraphrase_a,d.paraphrase_b]{
                assert_ne!(option,d.a);
                assert_ne!(option,d.b);
                assert!(!option.is_empty());
            }
        }
    }
    #[test] fn counterfactual_histories_reverse_expected_actions(){
        for i in 0..DOMAINS.len(){
            for company in [false,true]{
                let x=Trial{domain:i,reversed:false,company,arm:Arm::On,wording:0,order:0};
                let y=Trial{reversed:true,..x};
                assert_ne!(x.target_a(),y.target_a());
                assert_ne!(x.question(),String::new());
            }
        }
    }
    #[test] fn structured_witness_does_not_infer_preference(){
        let d=DOMAINS[0];
        let w=FixtureWitness{a:d.a,b:d.b};
        assert_eq!(w.score_expectation(d.a,d.b),PropositionLabel::Contradict);
        assert_eq!(w.score_expectation(d.a,"I felt delighted."),PropositionLabel::Unknown);
    }
    #[test] fn qualified_histories_build_offline(){
        let n=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(format!("selmem-transfer-{}-{n}",std::process::id()));
        let c=Config{out:path.clone(),pairs:1,actor:false,judge:false,
            provider:"grok".into(),model:"grok-4.3".into(),judge_provider:"openai".into(),
            judge_model:"gpt-4.1-mini".into(),max_http_calls:0,seed:1729,distractors:80,memory_words:64};
        let result=run(c);
        let _=fs::remove_dir_all(path);
        result.unwrap();
    }
}

#[path = "selmem-contextual-longlife/sweep.rs"]
mod sweep;

#[path = "selmem-contextual-longlife/causal_v2.rs"]
mod causal;

fn main() {
    if std::env::args().any(|arg| arg == "--causal-ablation") {
        if let Err(error) = causal::Causal::parse().and_then(causal::run) {
            eprintln!("GENESIS causal ablation: {error}");
            std::process::exit(1);
        }
    } else if std::env::args().any(|arg| matches!(arg.as_str(),
        "--sweep-budgets" | "--sweep-loads" | "--sweep-arms")) {
        if let Err(error) = sweep::Sweep::parse().and_then(sweep::run) {
            eprintln!("GENESIS contextual budget sweep: {error}");
            std::process::exit(1);
        }
    } else {
        original_v4_main();
    }
}
