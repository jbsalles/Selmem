//! Preparation-only recovery. Never advance or encode the legacy pending recall twice.
use super::{
    json::{num, obj, text, Json},
    runner::*,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn lines(path: &Path) -> Result<Vec<Json>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    fs::read_to_string(path)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(super::json::parse)
        .collect()
}
pub(super) fn validate(
    root: &Path,
    fixture: &Json,
    config: &Json,
    hashes: &[(PathBuf, String)],
    mock: bool,
    model: &str,
    expected: Vec<(String, String, Json, String)>,
) -> Result<Vec<Json>> {
    let report = load(&root.join("sleep.json"))?;
    if n(&report, "completed_decisions")? != 0. || n(&report, "completed_nights")? != 0. {
        return Err(
            "resume supports prefix preparation only; intervention results must not be replayed"
                .into(),
        );
    }
    if flag(&report, "mock") != mock {
        return Err("resume mock/real mismatch".into());
    }
    let design = load(&root.join("sleep-design.json"))?;
    if design.get("fixture")? != fixture
        || design.get("config")? != config
        || s(design.get("provider")?, "model_requested")? != model
    {
        return Err("resume fixture, memory config or model differs".into());
    }
    let original = array(&design, "source_hashes")?;
    if original.len() != hashes.len()
        || original
            .iter()
            .zip(hashes)
            .any(|(a, (_, h))| s(a, "sha256").ok() != Some(h))
    {
        return Err("resume original sources differ".into());
    }
    let rows = lines(&root.join("qualification.jsonl"))?;
    if rows.len() != expected.len() {
        return Err("resume requires complete faithful qualification".into());
    }
    for ((history, id, prompt, answer), row) in expected.iter().zip(&rows) {
        if s(row, "history")? != history
            || s(row, "id")? != id
            || row.get("prompt")? != prompt
            || (!mock && (!flag(row, "correct") || s(row.get("answer")?, "choice")? != answer))
        {
            return Err("resume qualification identity or answer mismatch".into());
        }
    }
    Ok(rows)
}
pub(super) fn copy_logs(root: &Path, out: &Path) -> Result<()> {
    for name in [
        "qualification.jsonl",
        "prefix-encoding.jsonl",
        "encoding-receipts.jsonl",
    ] {
        let p = root.join(name);
        if p.exists() {
            fs::copy(&p, out.join(name)).map_err(|e| e.to_string())?;
        }
    }
    write(&out.join("recovery.json"),&obj([("parent",text(root.to_string_lossy())),("parent_design_sha256",text(file_hash(&root.join("sleep-design.json"))?)),("parent_http_attempts",load(&root.join("sleep.json"))?.get("http_attempts")?.clone()),("method",text("reuse qualified prompts and completed prefixes; legacy pending recall retried without tick/live; new prefix events commit immutable snapshots only after full success"))]))
}
fn checkpoint(out: &Path, id: &str, m: &Memory, completed: usize, pending: Json) -> Result<()> {
    let name = format!(
        "{id}-step-{completed:03}{}.vault",
        if pending == Json::Null {
            ""
        } else {
            "-pending-recall"
        }
    );
    let frozen = m.copy(out.join("checkpoints").join(&name))?;
    let path = out.join("preparation.json");
    let mut ledger = if path.exists() { load(&path)? } else { obj([]) };
    ledger.put(
        id,
        obj([
            ("completed", num(completed as f64)),
            ("snapshot", text(format!("checkpoints/{name}"))),
            ("sha256", text(file_hash(&frozen.path)?)),
            ("pending_receipt", pending),
        ]),
    );
    write(&path, &ledger)
}
pub(super) fn prepare(
    root: Option<&Path>,
    out: &Path,
    id: &str,
    events: &[Json],
    config: &Json,
) -> Result<Memory> {
    let destination = out.join(format!("{id}-prefix.vault"));
    let mut completed = 0usize;
    let mut pending = Json::Null;
    let mut imported = None;
    if let Some(root) = root {
        let ledger = root.join("preparation.json");
        if ledger.exists() {
            if let Ok(entry) = load(&ledger)?.get(id) {
                completed = n(entry, "completed")? as usize;
                pending = entry.get("pending_receipt")?.clone();
                let relative = Path::new(s(entry, "snapshot")?);
                if relative.is_absolute()
                    || relative
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err("unsafe checkpoint path".into());
                }
                let path = root.join(relative);
                if file_hash(&path)? != s(entry, "sha256")? {
                    return Err("checkpoint hash differs".into());
                }
                imported = Some(path);
            }
        } else {
            let path = root.join(format!("{id}-prefix.vault"));
            if path.exists() {
                let done = lines(&root.join("prefix-encoding.jsonl"))?
                    .into_iter()
                    .filter(|r| s(r, "history").ok() == Some(id))
                    .collect::<Vec<_>>();
                completed = done.len();
                if completed > events.len() {
                    return Err("legacy prefix longer than fixture".into());
                }
                for (r, e) in done.iter().zip(events) {
                    if r.get("event")? != e {
                        return Err("legacy completed event differs".into());
                    }
                }
                let receipts = lines(&root.join("encoding-receipts.jsonl"))?
                    .into_iter()
                    .filter(|r| {
                        s(r, "snapshot")
                            .ok()
                            .is_some_and(|p| Path::new(p).file_name() == path.file_name())
                    })
                    .collect::<Vec<_>>();
                if receipts.len() == completed + 1 && completed < events.len() {
                    let r = receipts.last().unwrap();
                    if n(r, "step")? != (completed + 1) as f64
                        || r.get("event")? != &events[completed]
                    {
                        return Err("legacy pending event differs".into());
                    }
                    pending = r.get("receipt")?.clone();
                } else if receipts.len() != completed {
                    return Err("legacy recovery has ambiguous encoding boundary".into());
                }
                let memory = Memory::new("selmem_full", path.clone(), config);
                let state = memory.call("inspect", obj([]))?;
                if pending != Json::Null && &state != pending.get("state")? {
                    return Err("legacy pending snapshot does not match saved live receipt".into());
                }
                imported = Some(path);
            }
        }
    }
    if completed > events.len() {
        return Err("invalid checkpoint event count".into());
    }
    let memory = if let Some(path) = imported {
        Memory::new("selmem_full", path, config).copy(destination)?
    } else {
        let m = Memory::new("selmem_full", destination, config);
        m.init()?;
        m
    };
    checkpoint(out, id, &memory, completed, pending.clone())?;
    let mut rehearsal = Provider::new(true, "mock".into(), String::new(), None, out);
    if pending != Json::Null {
        if completed >= events.len() {
            return Err("pending recall past prefix".into());
        }
        super::progress::line(format!(
            "RESUME history={id} event={} stage=recall (tick/live already saved)",
            completed + 1
        ));
        let recall = memory.call(
            "recall",
            obj([("query", events[completed].get("recall_query")?.clone())]),
        )?;
        pending.put("live_recall", recall);
        log(
            &out.join("prefix-encoding.jsonl"),
            &obj([
                ("history", text(id)),
                ("event", events[completed].clone()),
                ("receipt", pending),
            ]),
        )?;
        completed += 1;
        checkpoint(out, id, &memory, completed, Json::Null)?;
    }
    for (i, e) in events.iter().enumerate().skip(completed) {
        super::progress::line(format!(
            "PREFIX history={id} event={}/{} (committed={i})",
            i + 1,
            events.len()
        ));
        let transaction = memory.copy(out.join("event-transaction.vault"))?;
        let receipt = transaction.ingest(e, i + 1, &mut rehearsal)?;
        // Publish ledger pointer only to a fully successful immutable snapshot.
        checkpoint(out, id, &transaction, i + 1, Json::Null)?;
        transaction.copy(memory.path.clone())?;
        log(
            &out.join("prefix-encoding.jsonl"),
            &obj([
                ("history", text(id)),
                ("event", e.clone()),
                ("receipt", receipt),
            ]),
        )?;
    }
    Ok(memory)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immutable_checkpoint_resume_skips_encoded_events() {
        let root = super::super::lifecycle_tests::root();
        let out = super::super::lifecycle_tests::root();
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&out).unwrap();
        let cfg = obj([
            ("memory_backend", text("rules")),
            ("associations_learn", Json::Bool(true)),
            ("associations_reconstruct", Json::Bool(true)),
            ("observation_evidence", Json::Bool(true)),
            ("hours_per_interaction", num(1.)),
            ("sleep_every", num(100000.)),
        ]);
        let event = obj([
            ("id", text("one")),
            ("text", text("Mira supplied six batteries to the workshop.")),
            ("recall_query", text("Mira batteries")),
            ("critical", Json::Bool(true)),
        ]);
        let m = prepare(None, &root, "test", &[event.clone()], &cfg).unwrap();
        let hash = file_hash(&m.path).unwrap();
        let recovered = prepare(Some(&root), &out, "test", &[event], &cfg).unwrap();
        assert_eq!(file_hash(&recovered.path).unwrap(), hash);
        assert!(!out.join("encoding-receipts.jsonl").exists());
        let ledger = load(&root.join("preparation.json")).unwrap();
        let snapshot = root.join(s(ledger.get("test").unwrap(), "snapshot").unwrap());
        fs::write(snapshot, "corrupt").unwrap();
        let bad = super::super::lifecycle_tests::root();
        fs::create_dir_all(&bad).unwrap();
        assert!(prepare(Some(&root), &bad, "test", &[], &cfg)
            .err()
            .unwrap()
            .contains("checkpoint hash"));
        for p in [root, out, bad] {
            fs::remove_dir_all(p).unwrap();
        }
    }
    #[test]
    fn legacy_pending_recall_does_not_advance_or_encode_again() {
        let root = super::super::lifecycle_tests::root();
        let out = super::super::lifecycle_tests::root();
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&out).unwrap();
        let cfg = obj([
            ("memory_backend", text("rules")),
            ("associations_learn", Json::Bool(true)),
            ("associations_reconstruct", Json::Bool(true)),
            ("observation_evidence", Json::Bool(true)),
            ("hours_per_interaction", num(1.)),
            ("sleep_every", num(100000.)),
        ]);
        let e = obj([
            ("id", text("x")),
            ("text", text("Mira supplied six batteries to the workshop.")),
            ("recall_query", text("Mira batteries")),
            ("critical", Json::Bool(true)),
        ]);
        let m = Memory::new("selmem_full", root.join("test-prefix.vault"), &cfg);
        m.init().unwrap();
        m.call("tick", obj([("hours", num(1.))])).unwrap();
        let receipt = m.call("live", obj([("event", e.clone())])).unwrap();
        log(
            &root.join("encoding-receipts.jsonl"),
            &obj([
                ("snapshot", text(m.path.to_string_lossy())),
                ("step", num(1.)),
                ("event", e.clone()),
                ("receipt", receipt.clone()),
            ]),
        )
        .unwrap();
        let hash = file_hash(&m.path).unwrap();
        let recovered = prepare(Some(&root), &out, "test", &[e], &cfg).unwrap();
        let state = recovered.call("inspect", obj([])).unwrap();
        assert_eq!(
            n(&state, "clock").unwrap(),
            n(receipt.get("state").unwrap(), "clock").unwrap()
        );
        assert_eq!(
            n(
                &array(&state, "observation_evidence").unwrap()[0],
                "observations"
            )
            .unwrap(),
            1.
        );
        assert_eq!(file_hash(&m.path).unwrap(), hash);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(out).unwrap();
    }
}
