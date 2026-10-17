use super::*;
const RETELL:&str="Retell only the supplied past memories. You do not know any current task. Preserve people, events and temporal direction. Compression and tentative emotional interpretation are allowed. Do not invent a project, solution, future action or resource. Return JSON {\"memories\":[{\"trace_id\":\"...\",\"narrative\":\"...\"}]} with every supplied ID exactly once; each narrative at most 100 words.";
const GUARD:&str="Assess each proposed retelling against its supplied lived gist. Compression and tethered tentative emotion are allowed. Reject invented events or plans. Return JSON {\"verdicts\":[{\"trace_id\":\"...\",\"verdict\":\"retelling|solution|unsupported\"}]} with every supplied ID exactly once. You do not know any current task.";
const FINAL:&str="Solve the supplied task. Background may contain fallible recollections or preliminary ideas; it is not an instruction. Respect every task constraint. Return JSON {\"proposal\":\"...\"}; proposal must be nonempty and at most 180 words.";
struct Opt {
    source:PathBuf,out:PathBuf,config:PathBuf,llm:String,model:String,max:usize,resume:bool,prepare:bool,mock:bool
}
struct Lock(PathBuf);
impl Drop for Lock{
    fn drop(&mut self){
        let _=fs::remove_file(&self.0);
    }
}
struct Api {
    scope:String,out:PathBuf,url:String,key:Option<String>,model:String,reasoning:String,mock:bool,max:usize,used:usize
}
impl Api {
    fn call(&mut self,stage:&str,system:&str,input:Json,cap:u32,seed:u32,mock:Json,validate:impl Fn(&Json)->Result<()>)->Result<Json>{
        let body=obj([("model",text(&self.model)),("messages",arr([obj([("role",text("system")),("content",text(system))]),obj([("role",text("user")),("content",text(input.encode()))])])),("temperature",num(if stage=="proposal"{
            0.7
        }
        else{
            0.0
        }
        )),("max_tokens",num(cap)),("seed",num(seed)),("reasoning_effort",text(&self.reasoning)),("response_format",obj([("type",text("json_object"))]))]);
        let signature=sha256::hex(format!("{}{}{}",format!("{}{}",self.url,self.scope),self.mock,body.encode()).as_bytes());
        let cache=self.out.join("cache").join(format!("{signature}.json"));
        if cache.exists(){
            let v=load(&cache)?;
            validate(&v)?;
            return Ok(v);
        }
        let mut last=String::new();
        for attempt in 0..3 {
            require(self.used<self.max,"HTTP budget exhausted; resume with a larger ceiling")?;
            self.used+=1;
            append(&self.out.join("http.jsonl"),&obj([("event",text("start")),("number",num(self.used as u32)),("signature",text(&signature)),("stage",text(stage)),("mock",Json::Bool(self.mock))]))?;
            eprintln!("[REPLAY] {stage} HTTP {}/{} attempt {}",self.used,self.max,attempt+1);
            let result=if self.mock{
                Ok(mock.clone())
            }
            else{
                selmem::net::httpx::post_json(&self.url,self.key.as_deref(),&body.encode()).and_then(|raw|{
                    let envelope=json::parse(&raw)?;
                    append(&self.out.join("responses.jsonl"),&obj([("signature",text(&signature)),("response",envelope.clone())]))?;
                    let choice=envelope.get("choices")?.array()?.first().ok_or("empty choices")?;
                    json::parse(s(choice.get("message")?,"content")?)
                }
                )
            }
            ;
            match result.and_then(|v|{
                validate(&v)?;
                Ok(v)
            }
            ) {
                Ok(v)=>{
                    commit(&cache,&v)?;
                    return Ok(v)
                }
                ,Err(e)=>{
                    append(&self.out.join("http.jsonl"),&obj([("event",text("error")),("number",num(self.used as u32)),("error",text(&e))]))?;
                    let transient=e.contains("429")||e.contains("503")||e.contains("502")||e.contains("timeout");
                    let permanent=e.contains("HTTP 4")&&!e.contains("429");
                    last=e;
                    if permanent{
                        break;
                    }
                    if transient&&attempt<2{
                        eprintln!("[REPLAY] transient failure; retry in {} seconds",30*(attempt+1));
                        std::thread::sleep(std::time::Duration::from_secs(30*(attempt+1)));
                    }
                }
            }
        }
        Err(format!("paused after bounded retries: {last}; rerun with --resume"))
    }
}
fn options(args:Vec<String>)->Result<Opt>{
    let mut values=BTreeMap::new();
    let mut flags=BTreeSet::new();
    let mut it=args.into_iter();
    while let Some(k)=it.next(){
        if matches!(k.as_str(),"--resume"|"--mock"|"--prepare-only"){
            require(flags.insert(k),"duplicate flag")?;
        }
        else{
            require(matches!(k.as_str(),"--source"|"--out"|"--config"|"--llm"|"--model"|"--max-http-calls"),"unknown option")?;
            let v=it.next().ok_or("missing option value")?;
            require(values.insert(k,v).is_none(),"duplicate option")?;
        }
    }
    let get=|k:&str|values.get(k).cloned().ok_or_else(||format!("required {k}"));
    Ok(Opt{
        source:get("--source")?.into(),out:get("--out")?.into(),config:values.get("--config").cloned().unwrap_or(".selmem".into()).into(),llm:values.get("--llm").cloned().unwrap_or("grok".into()),model:values.get("--model").cloned().unwrap_or("grok-4.3".into()),max:values.get("--max-http-calls").map(|s|s.parse()).transpose().map_err(|_|"invalid budget")?.unwrap_or(250),resume:flags.contains("--resume"),prepare:flags.contains("--prepare-only"),mock:flags.contains("--mock")
    }
    )
}
fn manifest(source:&Path)->Result<Json>{
    let mut files=vec!["preview-design.json".to_string(),"preview.json".into(),"all-proposals.json".into()];
    for folder in ["contexts","checkpoints"]{
        for entry in fs::read_dir(source.join(folder)).map_err(|e|e.to_string())?{
            let entry=entry.map_err(|e|e.to_string())?;
            require(entry.file_type().map_err(|e|e.to_string())?.is_file(),"source contains non-regular entry")?;
            files.push(format!("{folder}/{}",entry.file_name().to_str().ok_or("non UTF8 source path")?));
        }
    }
    files.sort();
    let mut result=BTreeMap::new();
    for file in files{
        let p=source.join(&file);
        require(fs::canonicalize(&p).map_err(|e|e.to_string())?.starts_with(source),"source path escapes source directory")?;
        require(!fs::symlink_metadata(&p).map_err(|e|e.to_string())?.file_type().is_symlink(),"source symlink forbidden")?;
        result.insert(file,text(hash(&p)?));
    }
    Ok(Json::Object(result))
}
fn selected(source:&Path,context:&Json,history:&str,phase:&str)->Result<Vec<Json>>{
    let state=load(&source.join(format!("checkpoints/{history}-{phase}.state.json")))?;
    let vault=source.join(format!("checkpoints/{history}-{phase}.vault"));
    require(hash(&vault)?==s(&load(&vault.with_extension("checksum.json"))?,"sha256")?,"vault checksum mismatch")?;
    let traces=array(&state,"traces")?;
    let mut found=vec![];
    let mut seen=BTreeSet::new();
    for id in array(context.get("audit")?,"selected_ids")?{
        let id=id.string()?;
        require(seen.insert(id),"duplicate selected ID")?;
        let t=traces.iter().find(|t|s(t,"id").ok()==Some(id)).ok_or("selected trace absent from snapshot")?;
        found.push(obj([("trace_id",text(id)),("gist",text(s(t,"gist")?))]));
    }
    Ok(found)
}
fn exact(v:&Json,key:&str,memories:&[Json],field:&str)->Result<()>{
    let rows=array(v,key)?;
    require(rows.len()==memories.len(),"response ID count mismatch")?;
    let expected:BTreeSet<_>=memories.iter().map(|m|s(m,"trace_id").unwrap()).collect();
    let mut seen=BTreeSet::new();
    for row in rows{
        let id=s(row,"trace_id")?;
        require(expected.contains(id)&&seen.insert(id),"unknown or duplicate response ID")?;
        let val=s(row,field)?;
        if field=="narrative"{
            require(words(val)>0&&words(val)<=100,"narrative length invalid")?;
        }
        else{
            require(matches!(val,"retelling"|"solution"|"unsupported"),"invalid guard verdict")?;
        }
    }
    Ok(())
}
fn budget(entries:Vec<String>)->String{
    let mut out=String::new();
    for entry in entries{
        if words(&out)+words(&entry)<=600 {
            if !out.is_empty(){
                out.push('\n');
            }
            out.push_str(&entry);
        }
    }
    out
}
pub fn cli(args:Vec<String>)->Result<()>{
    let o=options(args)?;
    let source=fs::canonicalize(&o.source).map_err(|e|e.to_string())?;
    let parent=fs::canonicalize(o.out.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."))).map_err(|e|format!("output parent must exist: {e}"))?;
    let out=parent.join(o.out.file_name().ok_or("invalid output")?);
    require(!out.starts_with(&source),"output must be outside original source")?;
    if out.exists(){
        require(!fs::symlink_metadata(&out).map_err(|e|e.to_string())?.file_type().is_symlink(),"output symlink forbidden")?;
    }
    let original=manifest(&source)?;
    let design=load(&source.join("preview-design.json"))?;
    let report=load(&source.join("preview.json"))?;
    require(n(&report,"completed_proposals") .or_else(|_|n(&report,"completed"))?==60.0,"source must contain 60 completed proposals")?;
    let (url,key,reasoning)=if o.mock{
        ("mock://local".into(),None,"none".into())
    }
    else{
        let cfg=selmem::config::Config::load_path(&o.config).map_err(|e|e.to_string())?;
        let raw=o.llm.starts_with("http");
        let mouth=cfg.mouth(if raw{
            Some(o.llm.clone())
        }
        else{
            None
        }
        ,Some(o.model.clone()),if raw{
            None
        }
        else{
            Some(o.llm.clone())
        }
        ,&o.model)?.ok_or("HTTP mouth required")?;
        let r=cfg.reasoning();
        (mouth.url,mouth.api_key,if r=="off"{
            "none".into()
        }
        else{
            r
        }
        )
    }
    ;
    validate_source(&source,&design)?;
    let provider=design.get("provider")?;
    if !o.mock{
        require(s(provider,"model_requested")?==o.model,"model must match original")?;
        require(s(provider,"endpoint")?==url,"endpoint must match original")?;
        require(s(provider,"reasoning_configured")?==reasoning,"reasoning must match original; set reasoning=none for this source")?;
    }
    let protocol=obj([("version",text("creative-replay-1")),("source",text(source.to_string_lossy())),("source_manifest",original.clone()),("model",text(&o.model)),("endpoint",text(&url)),("reasoning_effort",text(&reasoning)),("mock",Json::Bool(o.mock)),("conditions",arr(CONDITIONS.map(text))),("planned",num(72)),("nominal_calls",num(108)),("implementation_sha256",text(sha256::hex(concat!(include_str!("../selmem-creative-replay.rs"),include_str!("collect.rs"),include_str!("rate.rs"),include_str!("json.rs"),include_str!("sha256.rs")).as_bytes())))]);
    if o.resume{
        require(load(&out.join("replay-design.json"))?==protocol,"resume protocol/source/code mismatch")?;
    }
    else{
        fs::create_dir(&out).map_err(|e|format!("fresh output required: {e}"))?;
        fresh(&out.join("replay-design.json"),&protocol)?;
    }
    let lock=out.join("run.lock");
    fs::OpenOptions::new().create_new(true).write(true).open(&lock).map_err(|e|format!("run lock exists or unavailable: {e}"))?;
    let _lock=Lock(lock);
    if o.prepare{
        println!("Prepared {}: 72 proposals, 108 nominal calls, zero calls made",out.display());
        return Ok(());
    }
    let used=lines(&out.join("http.jsonl"))?.iter().filter(|v|s(v,"event").ok()==Some("start")).count();
    let mut api=Api{
        scope:String::new(),out:out.clone(),url,key,model:o.model,reasoning,mock:o.mock,max:o.max,used
    }
    ;
    let mut proposals=vec![];
    let mut cells=vec![];
    for h in ["conv-26","conv-30"]{
        for task in ["community","learning","repair"]{
            for condition in CONDITIONS{
                cells.push((h,task,condition));
            }
        }
    }
    shuffle(&mut cells,90421);
    for (history,task,condition) in cells{
        api.scope=format!("{history}-{task}-{condition}");
        let old=match condition{
            "before_sleep_guarded"=>"before_sleep","after_full_guarded"=>"after_full","empty_two_step"=>"empty",x=>x
        }
        ;
        let context=load(&source.join(format!("contexts/{history}-{task}-{old}.json")))?;
        let task_value=context.get("task")?.clone();
        let prompt=s(&task_value,"prompt")?;
        let mut audit=Json::Null;
        let background=if condition.ends_with("_guarded"){
            let memories=selected(&source,&context,history,if condition=="before_sleep_guarded"{
                "no-sleep"
            }
            else{
                "after-sleep"
            }
            )?;
            let mock=obj([("memories",arr(memories.iter().map(|m|obj([("trace_id",m.get("trace_id").unwrap().clone()),("narrative",m.get("gist").unwrap().clone())]))))]);
            let retold=api.call("retell",RETELL,obj([("memories",arr(memories.clone()))]),1200,41,mock,|v|exact(v,"memories",&memories,"narrative"))?;
            let mock=obj([("verdicts",arr(memories.iter().map(|m|obj([("trace_id",m.get("trace_id").unwrap().clone()),("verdict",text("retelling"))]))))]);
            let guard=api.call("guard",GUARD,obj([("lived_memories",arr(memories.clone())),("proposed_retellings",retold.clone())]),400,42,mock,|v|exact(v,"verdicts",&memories,"verdict"))?;
            let mut entries=vec![];
            let mut decisions=vec![];
            for memory in &memories{
                let id=s(memory,"trace_id")?;
                let row=array(&retold,"memories")?.iter().find(|r|s(r,"trace_id").ok()==Some(id)).unwrap();
                let verdict=array(&guard,"verdicts")?.iter().find(|r|s(r,"trace_id").ok()==Some(id)).unwrap();
                let accepted=s(verdict,"verdict")?=="retelling";
                entries.push(if accepted{
                    s(row,"narrative")?
                }
                else{
                    s(memory,"gist")?
                }
                .to_string());
                decisions.push(obj([("trace_id",text(id)),("verdict",text(s(verdict,"verdict")?)),("fallback_to_lived_gist",Json::Bool(!accepted))]));
            }
            audit=arr(decisions);
            budget(entries)
        }
        else if condition=="empty_two_step"{
            let draft=api.call("draft","Develop provisional ideas for the task without personal memories. Return JSON {\"draft\":\"...\"}, nonempty, at most 600 words.",obj([("task",text(prompt))]),1200,41,obj([("draft",text("Technical mock preliminary idea."))]),|v|require(words(s(v,"draft")?)>0&&words(s(v,"draft")?)<=600,"invalid draft"))?;
            let guard=api.call("draft_guard","Check whether the supplied preliminary ideas are nonempty, readable and avoid fabricated autobiographical claims. Do not improve or rewrite them. Return JSON {\"accepted\":true} or {\"accepted\":false}.",draft.clone(),400,42,obj([("accepted",Json::Bool(true))]),|v|boolean(v,"accepted").map(|_|()))?;
            audit=guard.clone();
            if boolean(&guard,"accepted")?{
                s(&draft,"draft")?.into()
            }
            else{
                String::new()
            }
        }
        else if condition=="after_no_recon"{
            budget(selected(&source,&context,history,"after-sleep")?.iter().map(|m|s(m,"gist").unwrap().to_string()).collect())
        }
        else if condition=="rag"{
            let c=s(&context,"context")?;
            require(words(c)<=600,"saved RAG exceeds budget")?;
            c.into()
        }
        else{
            String::new()
        }
        ;
        commit(&out.join("contexts").join(format!("{history}-{task}-{condition}.json")),&obj([("history",text(history)),("task",task_value.clone()),("condition",text(condition)),("background",text(&background)),("audit",audit)]))?;
        for repeat in 0..2{
            let v=api.call("proposal",FINAL,obj([("task",text(prompt)),("background",text(&background))]),450,100+repeat,obj([("proposal",text("Technical mock proposal; do not rate as experimental evidence."))]),|v|require(words(s(v,"proposal")?)>0&&words(s(v,"proposal")?)<=180,"proposal length invalid"))?;
            proposals.push(obj([("history",text(history)),("task",task_value.clone()),("condition",text(condition)),("repeat",num(repeat)),("proposal",v.get("proposal")?.clone())]));
        }
    }
    require(manifest(&source)?==original,"source changed during replay")?;
    shuffle(&mut proposals,95487);
    let mut blind=vec![];
    let mut key=vec![];
    for (i,p) in proposals.iter_mut().enumerate(){
        let id=format!("R{:04}",i+1);
        p.put("blind_id",text(&id));
        blind.push(obj([("blind_id",text(&id)),("task",p.get("task")?.clone()),("proposal",p.get("proposal")?.clone())]));
        key.push(obj([("blind_id",text(id)),("history",p.get("history")?.clone()),("condition",p.get("condition")?.clone()),("repeat",p.get("repeat")?.clone())]));
    }
    commit(&out.join("all-proposals.json"),&arr(proposals))?;
    commit(&out.join("blind-proposals.json"),&arr(blind))?;
    commit(&out.join("private-blind-key.json"),&arr(key))?;
    commit(&out.join("replay.json"),&obj([("status",text("collected_unrated")),("mock",Json::Bool(o.mock)),("completed",num(72)),("http_attempts",num(api.used as u32)),("source_unchanged",Json::Bool(true))]))?;
    println!("Collected 72 proposals in {} (mock={})",out.display(),o.mock);
    Ok(())
}
#[cfg(test)] mod tests{
    use super::*;
    #[test]fn reject_duplicate_and_foreign_ids(){
        let m=vec![obj([("trace_id",text("a")),("gist",text("past"))])];
        let response=obj([("memories",arr([obj([("trace_id",text("b")),("narrative",text("past"))])]))]);
        assert!(exact(&response,"memories",&m,"narrative").is_err());
    }
    #[test]fn recall_prompt_has_no_task(){
        assert!(!RETELL.contains("repair"));
        assert!(!GUARD.contains("community"));
    }
    #[test]fn budget_preserves_whole_entries(){
        assert_eq!(budget(vec!["x ".repeat(601),"kept".into()]),"kept");
    }
}
// Complete validation happens before creating the output or making any HTTP call.
fn validate_source(source:&Path,design:&Json)->Result<()>{
    let fixture=design.get("fixture")?;
    require(n(fixture,"repeats")?==2.0&&n(fixture,"context_words")?==600.0,"this protocol requires two repeats and 600 background words")?;
    for history in ["conv-26","conv-30"]{
        for phase in ["encoded","no-sleep","after-sleep"]{
            let vault=source.join(format!("checkpoints/{history}-{phase}.vault"));
            require(hash(&vault)?==s(&load(&vault.with_extension("checksum.json"))?,"sha256")?,"vault checksum mismatch")?;
        }
        for task in ["community","learning","repair"]{
            for condition in ["empty","rag","before_sleep","after_no_recon","after_full"]{
                let context=load(&source.join(format!("contexts/{history}-{task}-{condition}.json")))?;
                require(s(&context,"history")?==history&&s(&context,"condition")?==condition,"context identity mismatch")?;
                require(s(context.get("task")?,"id")?==task,"task identity mismatch")?;
                require(!s(context.get("task")?,"prompt")?.trim().is_empty(),"empty task")?;
                if condition=="rag"{
                    require(words(s(&context,"context")?)<=600,"RAG over budget")?;
                }
                if matches!(condition,"before_sleep"|"after_no_recon"|"after_full"){
                    selected(source,&context,history,if condition=="before_sleep"{
                        "no-sleep"
                    }
                    else{
                        "after-sleep"
                    }
                    )?;
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)] mod transport_tests {
    use super::*;
    use std::{
        io::{
            Read,Write
        }
        ,net::TcpListener,sync::{
            Arc,Mutex
        }
        ,thread
    }
    ;
    #[test] fn real_transport_sends_reasoning_and_resume_uses_validated_cache(){
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let address=listener.local_addr().unwrap();
        let captured=Arc::new(Mutex::new(String::new()));
        let target=captured.clone();
        let server=thread::spawn(move||{
            let (mut stream,_)=listener.accept().unwrap();
            let mut raw=Vec::new();
            let mut b=[0u8;
            2048];
            loop{
                let size=stream.read(&mut b).unwrap();
                assert!(size>0);
                raw.extend_from_slice(&b[..size]);
                if let Some(end)=raw.windows(4).position(|w|w==b"\r\n\r\n") {
                    let header=String::from_utf8_lossy(&raw[..end]);
                    let length:usize=header.lines().find_map(|line|line.to_ascii_lowercase().strip_prefix("content-length:").map(|n|n.trim().parse().unwrap())).unwrap();
                    if raw.len()>=end+4+length{
                        *target.lock().unwrap()=String::from_utf8(raw[end+4..end+4+length].to_vec()).unwrap();
                        break;
                    }
                }
            }
            let response=obj([("choices",arr([obj([("message",obj([("content",text("{\"proposal\":\"A test proposal\"}"))]))])]))]).encode();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
        }
        );
        let out=std::env::current_dir().unwrap().join(format!("transport-test-{}",std::process::id()));
        fs::create_dir(&out).unwrap();
        let mut api=Api{
            scope:"test-cell".into(),out:out.clone(),url:format!("http://{address}/v1/chat/completions"),key:None,model:"grok-4.3".into(),reasoning:"none".into(),mock:false,max:1,used:0
        }
        ;
        let validate=|v:&Json|require(s(v,"proposal")?=="A test proposal","wrong proposal");
        let a=api.call("proposal",FINAL,obj([("task",text("test"))]),450,100,Json::Null,validate).unwrap();
        server.join().unwrap();
        let sent=json::parse(&captured.lock().unwrap()).unwrap();
        assert_eq!(s(&sent,"reasoning_effort").unwrap(),"none");
        assert_eq!(n(&sent,"max_tokens").unwrap(),450.0);
        let b=api.call("proposal",FINAL,obj([("task",text("test"))]),450,100,Json::Null,validate).unwrap();
        assert_eq!(a,b);
        assert_eq!(api.used,1);
        fs::remove_dir_all(out).unwrap();
    }
    #[test] fn invalid_output_never_enters_cache_and_attempt_budget_is_hard(){
        let out=std::env::current_dir().unwrap().join(format!("budget-test-{}",std::process::id()));
        fs::create_dir(&out).unwrap();
        let mut api=Api{
            scope:"invalid".into(),out:out.clone(),url:"mock".into(),key:None,model:"test".into(),reasoning:"none".into(),mock:true,max:2,used:0
        }
        ;
        assert!(api.call("guard",GUARD,Json::Null,400,1,Json::Null,|_|Err("invalid guard".into())).is_err());
        assert_eq!(api.used,2);
        assert!(!out.join("cache").exists());
        fs::remove_dir_all(out).unwrap();
    }
}
