//! Controlled, fixed-selection replay of a completed GENESIS preview.
//! This binary adds an experimental guard; it does not modify the organ.
#[path="selmem-creative-replay/json.rs"] mod json;
#[path="selmem-creative-replay/sha256.rs"] mod sha256;
#[path="selmem-creative-replay/collect.rs"] mod collect;
#[path="selmem-creative-replay/rate.rs"] mod rate;
use json::{
    Json,obj,num,text
}
;
use std::{
    collections::{
        BTreeMap,BTreeSet
    }
    ,fs,io::Write,path::{
        Path,PathBuf
    }
}
;
type Result<T>=std::result::Result<T,String>;
const CONDITIONS:[&str;
6]=["empty","empty_two_step","rag","before_sleep_guarded","after_no_recon","after_full_guarded"];
fn s<'a>(v:&'a Json,k:&str)->Result<&'a str>{
    v.get(k)?.string()
}
fn n(v:&Json,k:&str)->Result<f64>{
    v.get(k)?.number()
}
fn array<'a>(v:&'a Json,k:&str)->Result<&'a [Json]>{
    v.get(k)?.array().map(|a| &a[..])
}
fn boolean(v:&Json,k:&str)->Result<bool>{
    match v.get(k)?{
        Json::Bool(b)=>Ok(*b),_=>Err(format!("boolean required: {k}"))
    }
}
fn arr(xs:impl IntoIterator<Item=Json>)->Json{
    Json::Array(xs.into_iter().collect())
}
fn require(ok:bool,msg:&str)->Result<()>{
    if ok{
        Ok(())
    }
    else{
        Err(msg.into())
    }
}
fn load(p:&Path)->Result<Json>{
    json::parse(&fs::read_to_string(p).map_err(|e|format!("{}: {e}",p.display()))?)
}
fn hash(p:&Path)->Result<String>{
    Ok(sha256::hex(&fs::read(p).map_err(|e|format!("{}: {e}",p.display()))?))
}
fn words(s:&str)->usize{
    s.split_whitespace().count()
}
fn commit(p:&Path,v:&Json)->Result<()>{
    if let Some(parent)=p.parent(){
        fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    }
    let tmp=p.with_extension("replay.tmp");
    let mut f=fs::File::create(&tmp).map_err(|e|e.to_string())?;
    f.write_all(format!("{}\n",v.encode()).as_bytes()).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;
    fs::rename(tmp,p).map_err(|e|e.to_string())
}
fn fresh(p:&Path,v:&Json)->Result<()>{
    let mut f=fs::OpenOptions::new().create_new(true).write(true).open(p).map_err(|e|format!("fresh output {}: {e}",p.display()))?;
    f.write_all(format!("{}\n",v.encode()).as_bytes()).and_then(|_|f.sync_all()).map_err(|e|e.to_string())
}
fn append(p:&Path,v:&Json)->Result<()>{
    let mut f=fs::OpenOptions::new().append(true).create(true).open(p).map_err(|e|e.to_string())?;
    f.write_all(format!("{}\n",v.encode()).as_bytes()).and_then(|_|f.sync_all()).map_err(|e|e.to_string())
}
fn lines(p:&Path)->Result<Vec<Json>>{
    if !p.exists(){
        return Ok(vec![]);
    }
    fs::read_to_string(p).map_err(|e|e.to_string())?.lines().filter(|s|!s.trim().is_empty()).map(json::parse).collect()
}
fn shuffle<T>(xs:&mut [T],mut seed:u64){
    for i in (1..xs.len()).rev(){
        seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        xs.swap(i,((seed>>32) as usize)%(i+1));
    }
}
fn main(){
    if let Err(e)=cli(std::env::args().skip(1).collect()){
        eprintln!("CREATIVE REPLAY: {e}");
        std::process::exit(1);
    }
}
fn cli(mut args:Vec<String>)->Result<()>{
    if args.is_empty()||args.iter().any(|a|a=="--help"){
        println!("selmem-creative-replay collect --source COMPLETED_PREVIEW --out FRESH_DIR --config .selmem --llm grok --model grok-4.3 [--max-http-calls 250] [--resume] [--prepare-only] [--mock]\nselmem-creative-replay template RUN FRESH_RATINGS.json\nselmem-creative-replay combine FRESH_COMBINED.json JUDGE_A.json JUDGE_B.json\nselmem-creative-replay analyze RUN COMBINED_RATINGS.json");
        return Ok(());
    }
    let command=args.remove(0);
    match command.as_str(){
        "collect"=>collect::cli(args),"template"=>{
            require(args.len()==2,"template RUN FRESH_OUTPUT")?;
            rate::template(Path::new(&args[0]),Path::new(&args[1]))
        }
        ,"combine"=>{
            require(args.len()>=3,"combine FRESH_OUTPUT INPUT_A INPUT_B [INPUT...]")?;
            let mut all=vec![];
            for p in &args[1..]{
                all.extend(load(Path::new(p))?.array()?.to_vec());
            }
            fresh(Path::new(&args[0]),&arr(all))
        }
        ,"analyze"=>{
            require(args.len()==2,"analyze RUN RATINGS")?;
            let dir=Path::new(&args[0]);
            let report=rate::analyze(dir,&load(Path::new(&args[1]))?)?;
            let p=dir.join("replay-ratings.json");
            fresh(&p,&report)?;
            println!("{}",p.display());
            Ok(())
        }
        ,_=>Err(format!("unknown command: {command}"))
    }
}
