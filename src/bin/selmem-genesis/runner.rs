use super::{json::{self,num,obj,text,Json},native,sha256};
use std::{collections::BTreeMap,fs::{self,OpenOptions},io::{Seek,SeekFrom,Write},path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering}};
pub(super) type Result<T> = std::result::Result<T,String>;
const SYSTEM: &str = "You make decisions in a fictional community. Use supplied memory as fallible past experience. Return only the requested JSON. No agent name or experimental condition is part of your role.";
static SERIAL: AtomicU64 = AtomicU64::new(1);
pub(super) fn s<'a>(v: &'a Json, key: &str) -> Result<&'a str> {
    v.get(key)?.string()
}
pub(super) fn n(v: &Json, key: &str) -> Result<f64> {
    v.get(key)?.number()
}
pub(super) fn array<'a>(v: &'a Json, key: &str) -> Result<&'a [Json]> {
    v.get(key)?.array()
}
pub(super) fn flag(v: &Json, key: &str) -> bool {
    matches!(v.get(key), Ok(Json::Bool(true)))
}
pub(super) fn arr(xs: impl IntoIterator<Item = Json>) -> Json {
    Json::Array(xs.into_iter().collect())
}
pub(super) fn merge(a: &Json, b: &Json) -> Json {
    let mut out = a.clone();
    if let Json::Object(fields) = b {
        for (key, value) in fields {
            out.put(key, value.clone());
        }
    }
    out
}
pub(super) fn load(path: &Path) -> Result<Json> {
    json::parse(&fs::read_to_string(path).map_err(|e| e.to_string())?)
}
pub(super) fn file_hash(path: &Path) -> Result<String> {
    Ok(sha256::hex(&fs::read(path).map_err(|e| e.to_string())?))
}
pub(super) fn write(path: &Path, value: &Json) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("genesis.tmp");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(format!("{}\n", value.encode()).as_bytes())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub(super) fn log(path: &Path, value: &Json) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
    file.write_all(format!("{}\n", value.encode()).as_bytes())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}pub(super) struct Rng(u64);
impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub(super) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub(super) fn index(&mut self, len: usize) -> usize {
        (self.next() % len as u64) as usize
    }
    pub(super) fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.index(i + 1);
            xs.swap(i, j);
        }
    }
}pub(super) struct Provider {
    pub(super) mock: bool,
    pub(super) model: String,
    endpoint: String,
    seed: Option<u64>,
    records: PathBuf,
    api_key: Option<String>,
}
impl Provider {
    pub(super) fn new(
        mock: bool,
        model: String,
        endpoint: String,
        seed: Option<u64>,
        out: &Path,
    ) -> Self {
        Self {
            mock,
            model,
            endpoint,
            seed,
            records: out.join("responses.jsonl"),
            api_key: None,
        }
    }
    pub(super) fn with_api_key(mut self, key: Option<String>) -> Self {
        self.api_key = key;
        self
    }
    pub(super) fn metadata(&self) -> Json {
        obj([
            ("model_requested", text(&self.model)),
            ("endpoint", text(&self.endpoint)),
            ("mock", Json::Bool(self.mock)),
            ("temperature", num(0.)),
            (
                "reasoning_configured",
                text(selmem::Config::get().reasoning()),
            ),
            (
                "generation_seed_requested",
                self.seed.map(|v| num(v as f64)).unwrap_or(Json::Null),
            ),
        ])
    }
    pub(super) fn ask(&mut self, prompt: Json, mode: &str) -> Result<Json> {
        self.ask_with(prompt, mode, SYSTEM, 0., self.seed)
    }
    pub(super) fn ask_with(&mut self, prompt: Json, mode: &str, system: &str, temperature: f64, seed: Option<u64>) -> Result<Json> {
        let timer = super::progress::Call::start(format!(
            "{mode} model={}{}",
            self.model,
            if self.mock { " (mock)" } else { "" }
        ));
        let result: Result<(Json, Json)> = (|| {
            if self.mock {
                let reply = match mode {
                    "decision" | "life_choice" => obj([(
                        "choice",
                        prompt
                            .get("options")?
                            .array()?
                            .first()
                            .ok_or("empty options")?
                            .get("id")?
                            .clone(),
                    )]),
                    "creative" | "life_creative" => obj([("proposal", text("Technical rehearsal: build a rotating workshop where neighbors exchange small practical skills within the stated resources."))]),
                    _ => return Err("unknown provider mode".into()),
                };
                return Ok((
                    reply.clone(),
                    obj([
                        ("mock", Json::Bool(true)),
                        ("model", text("mock")),
                        ("reply", reply),
                    ]),
                ));
            }
            let key = std::env::var("GENESIS_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .or_else(|| self.api_key.clone());
            let mut body = obj([
                ("model", text(&self.model)),
                ("temperature", num(temperature)),
                (
                    "messages",
                    arr([
                        obj([("role", text("system")), ("content", text(system))]),
                        obj([("role", text("user")), ("content", text(prompt.encode()))]),
                    ]),
                ),
            ]);
            if let Some(seed) = seed {
                body.put("seed", num(seed as f64));
            }
            if mode=="life_creative" {body.put(selmem::recall::http::token_cap_field(&self.endpoint,&self.model),num(400.));}
            if mode=="life_choice" {body.put(selmem::recall::http::token_cap_field(&self.endpoint,&self.model),num(64.));}
            let reasoning = selmem::Config::get().reasoning();
            body.put("reasoning_effort", text(reasoning));
            let raw = json::parse(&super::transport::post(
                mode,
                &self.endpoint,
                key.as_deref(),
                &body.encode(),
            )?)?;
            let choice = raw
                .get("choices")?
                .array()?
                .first()
                .ok_or("missing response choice")?;
            let content=s(choice.get("message")?, "content")?;
            let reply=match json::parse(content) {Ok(v)=>v,Err(_) if mode=="life_creative"=>obj([("invalid_output",text(content))]),Err(e)=>return Err(e)};
            Ok((reply, raw))
        })();
        timer.finish(result.is_ok());
        match result {
            Ok((reply, raw)) => {
                log(
                    &self.records,
                    &obj([
                        ("prompt", prompt),
                        ("response", raw),
                        ("parsed", reply.clone()),
                        ("mode", text(mode)),
                    ]),
                )?;
                Ok(reply)
            }
            Err(error) => {
                log(
                    &self.records,
                    &obj([
                        ("prompt", prompt),
                        ("error", text(&error)),
                        ("mode", text(mode)),
                    ]),
                )?;
                Err(error)
            }
        }
    }
}
#[derive(Clone)]
pub(super) struct Memory { pub(super) arch:String, pub(super) path:PathBuf, config:Json }
impl Memory {
    pub(super) fn configuration(&self)->Json{self.config.clone()}
    pub(super) fn new(arch:&str,path:PathBuf,config:&Json)->Self{Self{arch:arch.into(),path,config:config.clone()}}
    pub(super) fn call(&self, op: &str, extra: Json) -> Result<Json> {
        let request = merge(
            &obj([
                ("op", text(op)),
                ("path", text(self.path.to_string_lossy())),
                ("memory_config", self.config.clone()),
                (
                    "id_origin",
                    num((SERIAL.fetch_add(1, Ordering::Relaxed) * 1_000_000) as f64),
                ),
            ]),
            &extra,
        );
        let timer = if matches!(op, "probe" | "recall" | "sleep") {
            Some(super::progress::Call::start(format!(
                "memory {op} backend={} architecture={}",
                s(&self.config, "memory_backend")?,
                self.arch
            )))
        } else {
            None
        };
        let result = native::execute(&request);
        if let Some(timer) = timer {
            timer.finish(result.is_ok());
        }
        result
    }

    pub(super) fn init(&self)->Result<()> {
        if self.path.exists(){return Err("refusing existing memory".into());}
        if !["selmem_full","selmem_no_sleep","selmem_no_recon"].contains(&self.arch.as_str()){return Err("unsupported native architecture".into());}
        self.call("init",obj([("architecture",text(&self.arch)),("associations_learn",self.config.get("associations_learn")?.clone()),("associations_reconstruct",self.config.get("associations_reconstruct")?.clone())]))?;Ok(())
    }
    pub(super) fn copy(&self, path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // Materialize independent bytes; no platform copy/reflink shortcuts.
        let bytes = fs::read(&self.path).map_err(|e| e.to_string())?;
        let mut target = fs::File::create(&path).map_err(|e| e.to_string())?;
        target.write_all(&bytes).map_err(|e| e.to_string())?;
        target.sync_all().map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            ..self.clone()
        })
    }

    pub(super) fn ingest(&self,event:&Json,step:usize,_provider:&mut Provider)->Result<Json>{
        super::progress::line(format!("EVENT step={step} id={} architecture={}",s(event,"id")?,self.arch));
        self.call("tick",obj([("hours",self.config.get("hours_per_interaction")?.clone())]))?;
        let mut receipt=self.call("live",obj([("event",event.clone())]))?;
        log(&self.path.parent().ok_or("snapshot has no parent")?.join("encoding-receipts.jsonl"),&obj([("snapshot",text(self.path.to_string_lossy())),("step",num(step as f64)),("event",event.clone()),("receipt",receipt.clone())]))?;
        let recall=self.call("recall",obj([("query",event.get("recall_query")?.clone())]))?;
        receipt.put("live_recall",recall);
        if step % (n(&self.config,"sleep_every")? as usize)==0 && self.arch!="selmem_no_sleep"{self.call("sleep",obj([]))?;}
        Ok(receipt)
    }
    pub(super) fn context(&self,query:&str)->Result<(Json,Json)>{
        let result=self.call("probe",obj([("query",text(query))]))?;
        let context=obj([("memories",result.get("memories")?.clone()),("axioms",result.get("axioms")?.clone()),("reading",result.get("reading")?.clone())]);
        Ok((context,result))
    }
    #[cfg(test)]
    pub(super) fn lesion(&self,kind:&str,roots:&[Json])->Result<Json>{self.call("lesion",obj([("kind",text(kind)),("roots",arr(roots.iter().cloned()))]))}
}
pub(super) fn configured_mouth(
    cfg: &selmem::Config,
    flags: &BTreeMap<String, String>,
) -> Result<Option<selmem::config::Mouth>> {
    if flags.contains_key("--llm") && flags.contains_key("--endpoint") {
        return Err("use --llm or --endpoint, not both".into());
    }
    let llm = flags
        .get("--llm")
        .or_else(|| flags.get("--endpoint"))
        .cloned();
    let named = llm
        .as_ref()
        .filter(|s| !s.starts_with("http://") && !s.starts_with("https://"));
    let plug = named.cloned().or_else(|| flags.get("--plug").cloned());
    let url = if named.is_some() { None } else { llm };
    cfg.mouth(url, flags.get("--model").cloned(), plug, "gpt-4o-mini")
}
pub(super) fn campaign_config(cfg:&selmem::Config,flags:&BTreeMap<String,String>,_quick:bool,mock:bool)->Result<Json>{
    let mut config=json::parse(include_str!("../../../experiments/genesis/config.json"))?;
    for key in ["associations_learn","associations_reconstruct","observation_evidence"] {
        if let Some(v)=cfg.resolve(None,key){config.put(key,Json::Bool(match v.as_str(){"true"=>true,"false"=>false,_=>return Err(format!("{key} must be true or false"))}));}
    }
    let backend=flags.get("--memory-backend").map(String::as_str).unwrap_or(if mock{"rules"}else{"http"});
    if !["rules","http"].contains(&backend){return Err("memory backend must be rules|http".into());}
    config.put("memory_backend",text(backend));
    if backend=="http" {
        let mouth=configured_mouth(cfg,flags)?.ok_or("configure a provider in .selmem")?;
        for (key,value) in [("memory_endpoint",&mouth.url),("scorer_endpoint",&mouth.url),("memory_model",&mouth.model),("scorer_model",&mouth.model),("memory_plug",&mouth.plug)]{config.put(key,text(value));}
    }
    for (flag,key,default,allowed) in [
        ("--lifecycle-backend","lifecycle_backend",if mock{"rules"}else{"http"},&["rules","http"][..]),
        ("--reconstruction-policy","reconstruction_policy","native",&["native","strict"][..]),
    ]{let v=flags.get(flag).map(String::as_str).unwrap_or(default);if !allowed.contains(&v){return Err(format!("invalid {flag}"));}config.put(key,text(v));}
    Ok(config)
}
