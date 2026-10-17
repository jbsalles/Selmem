//! Strict, audited proposition scoring for experimental HTTP recall.
use super::json::{self, obj, text, Json};
use selmem::{
    EntityProfile, FailurePolicy, HttpNarrator, LlmCallLog, MemoryTrace, Mood, Narrator,
    PropositionLabel, PropositionScorer, SelectiveMemory,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(super) struct Audit {
    pub calls: Vec<Json>,
    pub error: Option<String>,
    pub narrator: selmem::recall::http::HttpExchangeLog,
    pub reconstructions: Vec<Json>,
    pub semantic_calls: Vec<Json>,
}

// Normalize transport formatting only; the organ validates all semantic values.
fn parse_event_annotations(raw: &str, event: &str) -> Result<selmem::EventSemantics, String> {
    const KEYS: &[&str] = &[
        "entities",
        "actions",
        "event_type",
        "bearer",
        "loss",
        "stake_mark",
        "absence",
        "valence",
        "arousal",
        "disgust",
        "self_relevance",
        "goal_relevance",
        "agency",
        "schema",
        "confidence",
    ];
    const NUMBERS: &[&str] = &[
        "valence",
        "arousal",
        "disgust",
        "self_relevance",
        "goal_relevance",
        "confidence",
    ];
    let raw = raw.trim();
    let mut fields = std::collections::BTreeMap::<String, String>::new();
    if raw.starts_with('{') {
        let value = json::parse(raw).map_err(|e| format!("semantic JSON format: {e}"))?;
        for (key, value) in value.object()? {
            let value = if NUMBERS.contains(&key.as_str()) {
                match value {
                    Json::Number(n) if n.is_finite() => n.to_string(),
                    Json::String(raw) => {
                        let n=raw.trim().parse::<f64>().map_err(|_|format!("semantic numeric field expected: {key}; received {}",value.encode()))?;
                        if !n.is_finite(){return Err(format!("non-finite semantic numeric field: {key}"));}
                        n.to_string()
                    },
                    _ => return Err(format!("semantic numeric field expected: {key}; received {}",value.encode())),
                }
            } else {
                value
                    .string()
                    .map_err(|_| format!("semantic string field expected: {key}"))?
                    .to_string()
            };
            fields.insert(key.clone(), value);
        }
    } else {
        // Compatibility with historical model outputs: newline or semicolon.
        for field in raw
            .split(['\n', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| format!("semantic key=value format: {field}"))?;
            let key = key.trim();
            let mut value = value.trim();
            // A sentence-final dot is not part of a numeric value (e.g. 1.0.).
            if NUMBERS.contains(&key) && value.parse::<f32>().is_err() {
                value = value.strip_suffix('.').unwrap_or(value);
            }
            if fields.insert(key.into(), value.into()).is_some() {
                return Err(format!("duplicate semantic field: {key}"));
            }
        }
    }
    for key in fields.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unexpected semantic field: {key}"));
        }
    }
    let mut normalized = String::new();
    for key in KEYS {
        let value = fields
            .get(*key)
            .ok_or_else(|| format!("semantic field absent after format decoding: {key}"))?;
        if value.contains(['\n', '\r']) {
            return Err(format!("multiline semantic value: {key}"));
        }
        normalized.push_str(&format!("{key}={value}\n"));
    }
    selmem::encode::semantic::parse_semantics(&normalized, event)
        .map_err(|e| format!("invalid semantic annotations: {e}"))
}

#[cfg(test)]
mod annotation_format_tests {
    use super::*;
    const EVENT: &str = "Workshop rule: ulen requires machine kavi.";
    const OBSERVED: &str = "entities=ulen|kavi; actions=require; event_type=rule; bearer=world; loss=none; stake_mark=machine kavi; absence=none; valence=0; arousal=0; disgust=0; self_relevance=0; goal_relevance=0; agency=none; schema=workshop rule; confidence=1.0.";
    #[test]
    fn quoted_numbers_preserve_values_without_defaults_or_clamping() {
        let raw=r#"{"entities":"ulen|kavi","actions":"require","event_type":"rule","bearer":"world","loss":"none","stake_mark":"machine kavi","absence":"none","valence":"-0.4","arousal":"0.25","disgust":0,"self_relevance":"0.9","goal_relevance":0,"agency":"none","schema":"workshop rule","confidence":"1"}"#;
        let parsed=parse_event_annotations(raw,EVENT).unwrap();
        assert_eq!(parsed.arousal,0.25);assert_eq!(parsed.valence,-0.4);
        for invalid in ["null","true","{}","[]","\"high\"","\"NaN\"","\"inf\"","\"1.1\"","\"-0.1\"","\"\""]{
            assert!(parse_event_annotations(&raw.replace("\"arousal\":\"0.25\"",&format!("\"arousal\":{invalid}")),EVENT).is_err(),"{invalid}");
        }
        let f=annotation_format();let schema=f.get("json_schema").unwrap().get("schema").unwrap();
        assert_eq!(schema.get("required").unwrap().array().unwrap().len(),15);
        assert_eq!(schema.get("additionalProperties").unwrap(),&Json::Bool(false));
        assert_eq!(schema.get("properties").unwrap().get("arousal").unwrap().get("type").unwrap().string().unwrap(),"number");
    }
    #[test]
    fn exact_gpt_response_and_json_preserve_same_annotations() {
        let legacy = parse_event_annotations(OBSERVED, EVENT).unwrap();
        let value = json::parse(r#"{"entities":"ulen|kavi","actions":"require","event_type":"rule","bearer":"world","loss":"none","stake_mark":"machine kavi","absence":"none","valence":0,"arousal":0,"disgust":0,"self_relevance":0,"goal_relevance":0,"agency":"none","schema":"workshop rule","confidence":1}"#).unwrap();
        let structured = parse_event_annotations(&value.encode(), EVENT).unwrap();
        assert_eq!(legacy.core.entities, structured.core.entities);
        assert_eq!(legacy.schema, structured.schema);
        assert_eq!(legacy.core.confidence, 1.);
        assert_eq!(structured.valence, 0.);
        assert_eq!(structured.self_relevance, 0.);
        parse_event_annotations(&OBSERVED.replace("; ", "\n").trim_end_matches('.'), EVENT)
            .unwrap();
    }
    #[test]
    fn invalid_values_duplicates_missing_and_unknown_fields_fail() {
        for raw in [
            OBSERVED.replace("confidence=1.0.", "confidence=nan"),
            OBSERVED.replace("confidence=1.0.", "confidence=2"),
            OBSERVED.replace("event_type=rule", "event_type=bogus"),
            format!("{OBSERVED}; valence=0"),
            format!("{OBSERVED}; invented=x"),
        ] {
            assert!(parse_event_annotations(&raw, EVENT).is_err(), "{raw}");
        }
        assert!(parse_event_annotations("entities=ulen", EVENT)
            .unwrap_err()
            .contains("absent after format decoding"));
        assert!(parse_event_annotations("not annotations", EVENT)
            .unwrap_err()
            .contains("format"));
    }
}

// Constrain transport types; the organ still validates ranges and meaning.
fn annotation_format()->Json {
    let names=["entities","actions","event_type","bearer","loss","stake_mark","absence","valence","arousal","disgust","self_relevance","goal_relevance","agency","schema","confidence"];
    let numbers=["valence","arousal","disgust","self_relevance","goal_relevance","confidence"];
    let properties=Json::Object(names.iter().map(|k|(k.to_string(),obj([("type",text(if numbers.contains(k){"number"}else{"string"}))]))).collect());
    obj([("type",text("json_schema")),("json_schema",obj([("name",text("event_annotations")),("strict",Json::Bool(true)),("schema",obj([("type",text("object")),("properties",properties),("required",Json::Array(names.map(text).to_vec())),("additionalProperties",Json::Bool(false))]))]))])
}
fn decode_annotation_response(raw:&str,event:&str)->Result<selmem::EventSemantics,String>{
    let response=json::parse(raw)?;
    let message=response.get("choices")?.array()?.first().ok_or("missing semantic response")?.get("message")?;
    let mut semantics=parse_event_annotations(message.get("content")?.string()?,event)?;
    semantics.core.claim=event.into();semantics.validate()?;Ok(semantics)
}

// The existing organ interface remains authoritative. No annotations from an
// experimental condition or sealed archive are supplied to the interpreter.
struct EventInterpreter {
    endpoint: String,
    model: String,
    key: Option<String>,
    audit: Arc<Mutex<Audit>>,
}
impl selmem::SemanticInterpreter for EventInterpreter {
    fn interpret_event(&self, event: &str) -> Result<selmem::EventSemantics, String> {
        if let Some(error)=self.audit.lock().unwrap().error.clone(){return Err(error);}
        let timer =
            super::progress::Call::start(format!("semantic interpretation model={}", self.model));
        let mut body = obj([
            ("model", text(&self.model)), ("temperature", Json::Number(0.)),
            ("max_tokens", Json::Number(700.)),
            ("messages", Json::Array(vec![
                obj([("role", text("system")), ("content", text("Describe only the supplied event, without deciding any future action. Return only a JSON object with exactly these required keys. Use strings for categorical/list fields (lists separated by |) and JSON numbers for numeric fields. No markdown or prose. Required keys: entities (names separated by |, or -); actions (verbs separated by |, or -); event_type (none|decision|presence|rule|promise|limit|mood|absence); bearer (self|other|world); loss (none|status|access|coherence|time); stake_mark (brief event subject, or -); absence (none); valence (-1..1); arousal (0..1); disgust (0..1); self_relevance (0..1); goal_relevance (0..1); agency (none|internal|external); schema (brief situation category, or -); confidence (0..1). Schema classifies the situation, never an actor's personality or a behavioral instruction. Do not infer deception, intention or emotion without evidence. Do not output a core_claim: the experiment retains the supplied event verbatim as its factual core."))]),
                obj([("role", text("user")), ("content", text(event))]),
            ])),
        ]);
        let effort = selmem::Config::get().reasoning();
        body.put("reasoning_effort", text(effort));
        body.put("response_format",annotation_format());
        let request = body.encode();
        let cached=super::transport::semantic_lookup(&self.endpoint,&request).map_err(|error|{self.audit.lock().unwrap().error=Some(error.clone());error})?;
        let cache_hit=cached.is_some();
        let mut result=Err("semantic annotation attempts exhausted".to_string());
        for semantic_attempt in 1..=3 {
            let raw=if let Some(raw)=&cached{Ok(raw.clone())}else{super::transport::post("semantic_interpretation",&self.endpoint,self.key.as_deref(),&request)};
            result=raw.as_ref().map_err(Clone::clone).and_then(|raw|decode_annotation_response(raw,event));
            let retry_format=raw.is_ok()&&result.is_err()&&!cache_hit&&semantic_attempt<3;
            self.audit.lock().unwrap().semantic_calls.push(obj([
                ("cache_hit",Json::Bool(cache_hit)),("semantic_attempt",Json::Number(semantic_attempt as f64)),
                ("event",text(event)),("request_body",text(&request)),
                ("raw_response",raw.as_ref().map(text).unwrap_or(Json::Null)),
                ("error",result.as_ref().err().map(text).unwrap_or(Json::Null)),
            ]));
            if result.is_ok(){
                if !cache_hit{if let Err(error)=super::transport::semantic_store_validated(&self.endpoint,&request,raw.as_ref().map_err(Clone::clone)?){result=Err(error);}}
                break;
            }
            if !retry_format{break;}
            super::progress::line(format!("SEMANTIC FORMAT RETRY {}/3: {}",semantic_attempt+1,result.as_ref().unwrap_err()));
        }
        if let Err(error)=&result{self.audit.lock().unwrap().error=Some(error.clone());}
        timer.finish(result.is_ok());
        result
    }
}
#[derive(Clone)]
struct Scorer {
    endpoint: String,
    model: String,
    key: Option<String>,
    audit: Arc<Mutex<Audit>>,
}
struct CheckedNarrator {
    strict: bool,
    inner: HttpNarrator,
    scorer: Scorer,
    audit: Arc<Mutex<Audit>>,
}
impl CheckedNarrator {
    fn checked(&self, trace: &MemoryTrace, proposed: String, associated: bool) -> String {
        if self.inner.failure_log().failures > 0 {
            return proposed;
        }
        if trace.core.trim().is_empty() {
            self.audit.lock().unwrap().error =
                Some("empty sealed core; cannot license reconstruction".into());
            return trace.core.clone();
        }
        let exact = proposed.trim() == trace.core.trim();
        let label = if exact {
            PropositionLabel::Entail
        } else {
            self.scorer.score(&trace.core, &proposed)
        };
        let accepted = label == PropositionLabel::Entail;
        self.audit.lock().unwrap().reconstructions.push(obj([
            ("target", text(&trace.id)),
            ("associated", Json::Bool(associated)),
            ("proposed", text(&proposed)),
            ("core", text(&trace.core)),
            ("accepted", Json::Bool(accepted)),
            ("enforced", Json::Bool(self.strict)),
            ("label", text(label.token())),
            (
                "method",
                text(if exact { "exact_core" } else { "http_scorer" }),
            ),
            (
                "source",
                text(if accepted || !self.strict {
                    "http_reconstruction"
                } else {
                    "core_fallback"
                }),
            ),
        ]));
        if accepted || !self.strict {
            proposed
        } else {
            trace.core.clone()
        }
    }
}
impl Narrator for CheckedNarrator {
    fn failure_log(&self) -> LlmCallLog {
        self.inner.failure_log()
    }
    fn supports_associations(&self) -> bool {
        self.inner.supports_associations()
    }
    fn reconstruct(&self, trace: &MemoryTrace, mood: &Mood, query: &str) -> String {
        self.checked(trace, self.inner.reconstruct(trace, mood, query), false)
    }
    fn reconstruct_associated(
        &self,
        trace: &MemoryTrace,
        mood: &Mood,
        query: &str,
        cues: &[selmem::core::association::AssociativeCue],
    ) -> String {
        self.checked(
            trace,
            self.inner.reconstruct_associated(trace, mood, query, cues),
            !cues.is_empty(),
        )
    }
    fn distill_axiom(&self, traces: &[&MemoryTrace]) -> Option<String> {
        self.inner.distill_axiom(traces)
    }
    fn interpret(
        &self,
        event: &str,
        mood: &Mood,
        axioms: &[String],
    ) -> Option<selmem::recall::narrator::Interpretation> {
        self.inner.interpret(event, mood, axioms)
    }
    fn rewrite(
        &self,
        trace: &MemoryTrace,
        neighbors: &[&MemoryTrace],
        profile: &EntityProfile,
    ) -> Option<String> {
        self.inner
            .rewrite(trace, neighbors, profile)
            .map(|proposed| self.checked(trace, proposed, false))
    }
    fn recontextualize(&self, trace: &MemoryTrace, core: &str, profile: &EntityProfile) -> String {
        self.checked(
            trace,
            self.inner.recontextualize(trace, core, profile),
            false,
        )
    }
}
impl PropositionScorer for Scorer {
    fn name(&self) -> &str {
        &self.model
    }
    fn score(&self, claim: &str, sentence: &str) -> PropositionLabel {
        // The wrapper rejects the entire operation; do not spend more calls after failure.
        if self.audit.lock().unwrap().error.is_some(){return PropositionLabel::Unknown;}
        let timer =
            super::progress::Call::start(format!("proposition scorer model={}", self.model));
        let mut body = obj([
            ("model", text(&self.model)),
            ("temperature", Json::Number(0.)),
            ("max_tokens", Json::Number(32.)),
            ("messages", Json::Array(vec![
                obj([("role", text("system")), ("content", text("Reply with exactly one lowercase word: entail, contradict, or unknown. This is a historical-fact preservation audit. The labels are operational categories for the historical portion of SENTENCE, not logical entailment of its entire text. CLAIM is the only historical evidence. Shared names or quantities do not justify joining separate episodes. Adding before/after, a causal link, a confirmation by a named actor, or a motive is unknown unless that relation is stated in CLAIM. Do not borrow facts from another episode to complete CLAIM. First exclude clearly hypothetical future possibilities and explicitly tentative present interpretations from the historical comparison. Their lack of support in CLAIM does not by itself make the label unknown. Never exclude definite assertions about past events, motives, causes, quantities, people or actions. Then classify the historical portion: contradict if any historical assertion conflicts with CLAIM; otherwise unknown if it adds an unsupported historical assertion or omits a fact, actor or temporal relation in CLAIM; otherwise entail if it preserves all facts and temporal relations, allowing equivalent wording. A text with no preserved historical claim is unknown. A definite current or past choice is not a hypothetical future possibility. Do not use outside knowledge, add facts, or rewrite either text."))]),
                obj([("role", text("user")), ("content", text(format!("CLAIM:\n{claim}\nSENTENCE:\n{sentence}")))]),
            ])),
        ]);
        let reasoning = selmem::Config::get().reasoning();
        body.put("reasoning_effort", text(reasoning));
        let response = super::transport::post(
            "scorer",
            &self.endpoint,
            self.key.as_deref(),
            &body.encode(),
        );
        let label = response.as_ref().map_err(Clone::clone).and_then(|raw| {
            let parsed = json::parse(raw)?;
            let choices = parsed.get("choices")?.array()?;
            let content = choices
                .first()
                .ok_or("empty scorer choices")?
                .get("message")?
                .get("content")?
                .string()?
                .trim();
            match content {
                "entail" => Ok(PropositionLabel::Entail),
                "contradict" => Ok(PropositionLabel::Contradict),
                "unknown" => Ok(PropositionLabel::Unknown),
                _ => Err("invalid scorer label".into()),
            }
        });
        timer.finish(label.is_ok());
        let mut audit = self.audit.lock().unwrap();
        audit.calls.push(obj([
            ("claim", text(claim)),
            ("sentence", text(sentence)),
            (
                "raw_response",
                response.as_ref().map(text).unwrap_or(Json::Null),
            ),
            (
                "label",
                label
                    .as_ref()
                    .map(|l| text(l.token()))
                    .unwrap_or(Json::Null),
            ),
            (
                "error",
                label.as_ref().err().map(text).unwrap_or(Json::Null),
            ),
        ]));
        if let Err(error) = &label {
            audit.error = Some(error.clone());
        }
        label.unwrap_or(PropositionLabel::Unknown)
    }
}
pub(super) fn bind(
    m: SelectiveMemory,
    config: &Json,
) -> Result<(SelectiveMemory, Arc<Mutex<Audit>>), String> {
    if selmem::Config::get().temp().parse::<f64>().ok() != Some(0.) {
        return Err("GENESIS HTTP recall requires SELMEM temp=0".into());
    }
    let field = |k| config.get(k)?.string();
    let audit = Arc::new(Mutex::new(Audit::default()));
    let reconstruction_audit=audit.clone();
    let narrator = HttpNarrator::parse(
        field("memory_endpoint")?,
        field("memory_model")?,
        key_for(
            field("memory_endpoint")?,
            config.get("memory_plug").ok().and_then(|p| p.string().ok()),
        )?,
    )
    .ok_or("invalid memory endpoint")?
    .with_transport(Arc::new(move |url, key, body| {
        if let Some(error)=reconstruction_audit.lock().unwrap().error.clone(){return Err(error);}
        super::transport::post("reconstruction", url, key, body)
    }))
    .with_reconstruction_prompt("Reconstruct the supplied memory briefly. One sentence is enough. Preserve its factual gist without inventing past people, motives, causes, quantities or actions. The recall query is a topic cue, never evidence of a past event. Associative cues are fallible context, not new historical facts. You may express a clearly tentative present interpretation or future inclination. Do not turn a current question or option into a remembered choice.".into())
    .with_policy(FailurePolicy::Error)
    .with_exchange_log(audit.lock().unwrap().narrator.clone());
    let scorer = Scorer {
        endpoint: field("scorer_endpoint")?.into(),
        model: field("scorer_model")?.into(),
        key: key_for(
            field("scorer_endpoint")?,
            config.get("memory_plug").ok().and_then(|p| p.string().ok()),
        )?,
        audit: audit.clone(),
    };
    let mut bound = m
        .with_narrator(Box::new(CheckedNarrator {
            strict: config
                .get("reconstruction_policy")
                .and_then(Json::string)
                .unwrap_or("strict")
                == "strict",
            inner: narrator,
            scorer: scorer.clone(),
            audit: audit.clone(),
        }))
        .with_scorer(Box::new(scorer));
    if config.get("lifecycle_backend").and_then(Json::string) == Ok("http") {
        bound = bound.with_semantic_interpreter(Box::new(EventInterpreter {
            endpoint: field("memory_endpoint")?.into(),
            model: field("memory_model")?.into(),
            key: key_for(
                field("memory_endpoint")?,
                config.get("memory_plug").ok().and_then(|p| p.string().ok()),
            )?,
            audit: audit.clone(),
        }));
    }
    Ok((bound, audit))
}
pub(super) fn key_for(endpoint: &str, plug: Option<&str>) -> Result<Option<String>, String> {
    for name in ["GENESIS_MEMORY_API_KEY", "GENESIS_API_KEY"] {
        if let Ok(key) = std::env::var(name) {
            if !key.is_empty() {
                return Ok(Some(key));
            }
        }
    }
    let cfg = selmem::Config::get();
    if let Some(mouth) = cfg.mouth(None, None, plug.map(str::to_owned), "gpt-4o-mini")? {
        if mouth.url == endpoint {
            return Ok(mouth.api_key);
        }
    }
    Ok(cfg
        .plugs()
        .into_iter()
        .find(|m| m.url == endpoint)
        .and_then(|m| m.api_key)
        .or_else(|| cfg.api_key()))
}


#[cfg(test)] mod failure_latch_tests {
 use super::*;
 #[test]fn scorer_stops_http_after_first_failure(){
  let audit=Arc::new(Mutex::new(Audit::default()));audit.lock().unwrap().error=Some("original timeout".into());
  let scorer=Scorer{endpoint:"http://127.0.0.1:1".into(),model:"unused".into(),key:None,audit:audit.clone()};
  let _budget=crate::transport::begin(std::path::Path::new("unused"),1);
  assert_eq!(scorer.score("claim","sentence"),PropositionLabel::Unknown);assert_eq!(crate::transport::used(),0);assert_eq!(audit.lock().unwrap().error.as_deref(),Some("original timeout"));
 }
}
