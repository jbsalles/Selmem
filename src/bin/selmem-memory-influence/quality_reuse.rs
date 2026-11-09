//! Validate original paid quality requests before reusing their scores unchanged.
use super::*;
fn bounded_file(path: &Path, max: u64) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ensure(
        meta.file_type().is_file() && meta.len() <= max,
        "quality reuse requires bounded regular source files",
    )
}
fn expected_wire(design: &Json, payload: &Json, repeat: u32) -> Result<Json> {
    let cap = selmem::recall::http::token_cap_field(s(design, "endpoint")?, s(design, "model")?);
    Ok(obj([
        ("model", design.get("model")?.clone()),
        (
            "messages",
            arr([
                obj([("role", text("system")), ("content", text(choice::QUALITY))]),
                obj([("role", text("user")), ("content", text(payload.encode()))]),
            ]),
        ),
        ("temperature", num(0)),
        ("seed", num(100 + repeat)),
        (cap, num(1600)),
        ("reasoning_effort", design.get("reasoning_effort")?.clone()),
        ("response_format", obj([("type", text("json_object"))])),
    ]))
}
pub fn prepare(
    source: &Path,
    proposals: &Json,
    pools: &[Option<(Json, choice::Candidates)>],
) -> Result<Json> {
    let design = load(&source.join("design.json"))?;
    ensure(
        s(&design, "mode")? == "frozen-pool-reevaluation-021"
            && s(&design, "selection_protocol")? == "selmem-response-choice-4",
        "quality reuse supports complete 021 reevaluation sources only",
    )?;
    ensure(
        design.get("quality_system")? == &text(choice::QUALITY)
            && design.get("evaluation_temperature")?.number()? == 0.0,
        "source quality prompt or sampling changed",
    )?;
    for file in ["responses.jsonl", "http.jsonl"] {
        bounded_file(&source.join(file), 50_000_000)?;
    }
    let mut records = BTreeMap::<String, Json>::new();
    let reader = std::io::BufReader::new(
        fs::File::open(source.join("responses.jsonl")).map_err(|e| e.to_string())?,
    );
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        ensure(line.len() <= 500_000, "oversized source response record")?;
        let row = json::parse(&line)?;
        records.insert(s(&row, "signature")?.into(), row);
    }
    let mut starts = BTreeSet::new();
    let reader = std::io::BufReader::new(
        fs::File::open(source.join("http.jsonl")).map_err(|e| e.to_string())?,
    );
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        ensure(line.len() <= 500_000, "oversized source HTTP record")?;
        let row = json::parse(&line)?;
        if s(&row, "event")? == "start" && s(&row, "stage")? == "quality" {
            starts.insert((
                s(&row, "job")?.to_string(),
                s(&row, "signature")?.to_string(),
            ));
        }
    }
    let mut reuse = vec![];
    for (p, pool) in proposals.array()?.iter().zip(pools) {
        let Some((input, c)) = pool else { continue };
        let audit = p.get("audit")?;
        ensure(
            s(audit, "assessment_protocol")? == "task-only-quality-021",
            "unrecognized source quality audit",
        )?;
        let options = ChoiceOptions {
            mode: ChoiceMode::Always,
            influence: if s(p, "condition")? == "candidates_empty" {
                Influence::None
            } else {
                Influence::Moderate
            },
            max_words: 180,
        };
        ensure(
            audit.get("options")? == &options.to_json(),
            "source selection settings differ",
        )?;
        let quality = audit.get("quality_audit")?;
        choice::validate_quality_audit(input, c, &options, quality)?;
        let repeat = p.get("repeat")?.number()? as u32;
        let job = format!(
            "{}-{}-{repeat}-{}",
            s(p, "history")?,
            s(p.get("task")?, "id")?,
            s(p, "condition")?
        );
        let body = expected_wire(&design, quality.get("request")?, repeat)?;
        let mock = design.get("mock")? == &Json::Bool(true);
        let signature = sha256::hex(
            format!(
                "{job}|quality|{}|{mock}|{}",
                s(&design, "endpoint")?,
                body.encode()
            )
            .as_bytes(),
        );
        ensure(
            starts.contains(&(job.clone(), signature.clone())),
            "source quality HTTP start/signature missing",
        )?;
        let cache_path = source.join("cache").join(format!("{signature}.json"));
        bounded_file(&cache_path, 100_000)?;
        let cache = load(&cache_path)?;
        ensure(
            s(&cache, "signature")? == signature,
            "source quality cache signature mismatch",
        )?;
        let content = s(&cache, "content")?;
        ensure(
            json::parse(content)? == *quality.get("response")?,
            "source cached quality differs from audit",
        )?;
        let recorded = records
            .get(&signature)
            .ok_or("source quality provider record missing")?;
        ensure(
            recorded.get("request")? == &body,
            "source quality provider request differs from expected request",
        )?;
        let response = recorded.get("response")?;
        let choices = response.get("choices")?.array()?;
        ensure(
            choices.len() == 1 && s(&choices[0], "finish_reason")? == "stop",
            "source quality provider response incomplete",
        )?;
        ensure(
            s(choices[0].get("message")?, "content")? == content,
            "source provider response differs from quality cache",
        )?;
        reuse.push(obj([
            ("job", text(job)),
            ("signature", text(signature)),
            ("source_cache_sha256", text(hash_file(&cache_path)?)),
            ("request", body),
            ("response", response.clone()),
        ]));
    }
    ensure(
        reuse.len() == 48,
        "expected 48 validated reusable quality results",
    )?;
    Ok(obj([
        ("kind", text("validated-existing-quality-022")),
        (
            "source_design_sha256",
            text(hash_file(&source.join("design.json"))?),
        ),
        (
            "source_responses_sha256",
            text(hash_file(&source.join("responses.jsonl"))?),
        ),
        (
            "source_http_sha256",
            text(hash_file(&source.join("http.jsonl"))?),
        ),
        ("new_quality_calls", num(0)),
        ("records", arr(reuse)),
    ]))
}
