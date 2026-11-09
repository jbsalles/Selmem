//! Task-blind readout intervention, never a replacement for native memory.
use super::{
    creative_life as life,
    json::{num, obj, text, Json},
    runner::*,
    sha256,
};
use std::{collections::BTreeSet, path::Path};
pub(super) const VERSION: &str = "task-blind-guarded-preview-026";
const RETELL:&str="Retell each supplied current lived gist as one short past observation, at most 100 words each. Preserve attribution and factual limits. Compression and tentative interpretation are allowed. Do not invent people, events, motives, actions, quantities, instructions or solution plans. You do not know any current task. Return only JSON {\"memories\":[{\"trace_id\":\"supplied ID\",\"narrative\":\"...\"}]} with every ID exactly once.";
const GUARD:&str="Compare each proposed retelling only with its supplied current lived gist. Return only JSON {\"verdicts\":[{\"trace_id\":\"supplied ID\",\"verdict\":\"retelling|solution|unsupported\"}]} for every supplied ID exactly once. Accept compression and tethered tentative interpretation as retelling. Reject invented events, solution plans and instructions. You do not know the creative task. This is a fallible provenance check, not a creativity rating.";
fn valid(v: &Json, key: &str, memories: &[Json]) -> Result<()> {
    if v.object()?.len() != 1 {
        return Err("one response field required".into());
    }
    let rows = array(v, key)?;
    if rows.len() != memories.len() {
        return Err("exact ID coverage required".into());
    }
    let expected = memories
        .iter()
        .map(|m| s(m, "trace_id"))
        .collect::<Result<BTreeSet<_>>>()?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let id = s(row, "trace_id")?;
        if row.object()?.len() != 2 || !expected.contains(id) || !seen.insert(id) {
            return Err("unknown or duplicate ID".into());
        }
        let value = s(
            row,
            if key == "memories" {
                "narrative"
            } else {
                "verdict"
            },
        )?;
        if key == "memories" {
            if value.split_whitespace().count() == 0 || value.split_whitespace().count() > 100 {
                return Err("retelling must contain 1..100 words".into());
            }
        } else if !["retelling", "solution", "unsupported"].contains(&value) {
            return Err("invalid guard verdict".into());
        }
    }
    Ok(())
}
fn selected(m: &Memory, audit: &Json) -> Result<Vec<Json>> {
    let state = m.call("inspect", obj([]))?;
    let mut seen = BTreeSet::new();
    let mut memories = vec![];
    for id in array(audit, "selected_ids")? {
        let id = id.string()?;
        if !seen.insert(id) {
            return Err("duplicate selected ID".into());
        }
        let trace = array(&state, "traces")?
            .iter()
            .find(|t| s(t, "id").ok() == Some(id))
            .ok_or("selected trace absent from current state")?;
        memories.push(obj([
            ("trace_id", text(id)),
            ("gist", text(s(trace, "gist")?)),
        ]));
    }
    Ok(memories)
}
fn combine(memories: &[Json], retold: &Json, guard: &Json) -> Result<(Vec<String>, Json)> {
    let retell_ok = valid(retold, "memories", memories).is_ok();
    let guard_ok = retell_ok && valid(guard, "verdicts", memories).is_ok();
    let mut entries = vec![];
    let mut decisions = vec![];
    for memory in memories {
        let id = s(memory, "trace_id")?;
        let verdict = if !retell_ok {
            "invalid_retelling"
        } else if !guard_ok {
            "invalid_guard"
        } else {
            s(
                array(guard, "verdicts")?
                    .iter()
                    .find(|r| s(r, "trace_id").ok() == Some(id))
                    .unwrap(),
                "verdict",
            )?
        };
        let accepted = verdict == "retelling";
        entries.push(
            if accepted {
                s(
                    array(retold, "memories")?
                        .iter()
                        .find(|r| s(r, "trace_id").ok() == Some(id))
                        .unwrap(),
                    "narrative",
                )?
            } else {
                s(memory, "gist")?
            }
            .to_string(),
        );
        decisions.push(obj([
            ("trace_id", text(id)),
            ("verdict", text(verdict)),
            ("fallback_to_lived_gist", Json::Bool(!accepted)),
        ]));
    }
    Ok((entries, arr(decisions)))
}
fn cached_ask(
    p: &mut Provider,
    prompt: Json,
    mode: &str,
    system: &str,
    seed: u64,
    path: &Path,
) -> Result<Json> {
    let request = obj([
        ("mode", text(mode)),
        ("system", text(system)),
        ("prompt", prompt.clone()),
        ("seed", num(seed as f64)),
        ("provider", p.metadata()),
    ]);
    if path.exists() {
        let old = load(path)?;
        if old.get("request")? != &request {
            return Err("task-blind stage inputs changed".into());
        }
        return Ok(old.get("reply")?.clone());
    }
    let reply = p.ask_with(prompt, mode, system, 0., Some(seed))?;
    life::commit(path, &obj([("request", request), ("reply", reply.clone())]))?;
    Ok(reply)
}
pub(super) fn context(
    m: &Memory,
    audit: &Json,
    reconstruct: bool,
    p: &mut Provider,
    out: &Path,
) -> Result<(String, Json)> {
    let before = file_hash(&m.path)?;
    let memories = selected(m, audit)?;
    let request = obj([("memories", arr(memories.clone()))]);
    let signature = sha256::hex(
        obj([
            ("version", text(VERSION)),
            ("snapshot_sha256", text(&before)),
            ("request", request.clone()),
            ("retell_system", text(RETELL)),
            ("guard_system", text(GUARD)),
            ("reconstruct", Json::Bool(reconstruct)),
        ])
        .encode()
        .as_bytes(),
    );
    let path = out
        .join("guarded-readouts")
        .join(format!("{signature}.json"));
    let result = if path.exists() {
        let cached = load(&path)?;
        if s(&cached, "signature")? != signature
            || cached.get("memories")? != &arr(memories.clone())
        {
            return Err("guarded cache inputs changed".into());
        }
        (
            s(&cached, "context")?.to_string(),
            cached.get("audit")?.clone(),
        )
    } else {
        let (entries, decisions, retold, guard) = if reconstruct && !memories.is_empty() {
            super::progress::line("PREVIEW task-blind retelling and guard");
            let retold = cached_ask(
                p,
                request,
                "preview_retell",
                RETELL,
                2026102601,
                &out.join("guarded-stages")
                    .join(format!("{signature}-retell.json")),
            )?;
            let guard = if valid(&retold, "memories", &memories).is_ok() {
                cached_ask(
                    p,
                    obj([
                        ("lived_memories", arr(memories.clone())),
                        ("proposed_retellings", retold.clone()),
                    ]),
                    "preview_guard",
                    GUARD,
                    2026102602,
                    &out.join("guarded-stages")
                        .join(format!("{signature}-guard.json")),
                )?
            } else {
                Json::Null
            };
            let (entries, decisions) = combine(&memories, &retold, &guard)?;
            (entries, decisions, retold, guard)
        } else {
            (
                memories
                    .iter()
                    .map(|m| s(m, "gist").map(str::to_string))
                    .collect::<Result<Vec<_>>>()?,
                arr([]),
                Json::Null,
                Json::Null,
            )
        };
        let context = super::creativity::trim_context(&entries, 600);
        let a = obj([
            ("protocol", text(VERSION)),
            ("reconstruction_requested", Json::Bool(reconstruct)),
            ("memories", arr(memories.clone())),
            ("decisions", decisions),
            ("retelling", retold),
            ("guard", guard),
            ("entries", arr(entries.iter().map(text))),
            (
                "context_words",
                num(context.split_whitespace().count() as f64),
            ),
        ]);
        life::commit(
            &path,
            &obj([
                ("signature", text(&signature)),
                ("memories", arr(memories.clone())),
                ("context", text(&context)),
                ("audit", a.clone()),
            ]),
        )?;
        (context, a)
    };
    if file_hash(&m.path)? != before {
        return Err("guarded readout changed source snapshot".into());
    }
    let mut a = audit.clone();
    a.put("guarded_readout", result.1);
    a.put("delivery",text("Selected current lived gists only; no reading profile, axioms, cores or sealed archive. Guarded reconstruction is a readout intervention."));
    Ok((result.0, a))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guards_reject_inventions_bad_shapes_and_untethered_ids() {
        let memories = vec![obj([
            ("trace_id", text("t1")),
            ("gist", text("A past room meeting.")),
        ])];
        let retold = obj([(
            "memories",
            arr([obj([
                ("trace_id", text("t1")),
                ("narrative", text("An earlier room meeting.")),
            ])]),
        )]);
        for verdict in ["retelling", "solution", "unsupported"] {
            let guard = obj([(
                "verdicts",
                arr([obj([("trace_id", text("t1")), ("verdict", text(verdict))])]),
            )]);
            let (entries, decisions) = combine(&memories, &retold, &guard).unwrap();
            assert_eq!(
                entries[0],
                if verdict == "retelling" {
                    "An earlier room meeting."
                } else {
                    "A past room meeting."
                }
            );
            assert_eq!(
                decisions.array().unwrap()[0]
                    .get("fallback_to_lived_gist")
                    .unwrap(),
                &Json::Bool(verdict != "retelling")
            );
        }
        for bad in [
            Json::Null,
            obj([("memories", arr([]))]),
            obj([(
                "memories",
                arr([obj([
                    ("trace_id", text("foreign")),
                    ("narrative", text("Invented plan.")),
                ])]),
            )]),
        ] {
            assert!(valid(&bad, "memories", &memories).is_err());
            assert_eq!(
                combine(&memories, &bad, &Json::Null).unwrap().0[0],
                "A past room meeting."
            );
        }
        assert_eq!(
            combine(&memories, &retold, &Json::Null).unwrap().0[0],
            "A past room meeting."
        );
    }
    #[test]
    fn reconstruction_payload_has_only_current_selected_gists() {
        let out = super::super::lifecycle_tests::root();
        std::fs::create_dir_all(&out).unwrap();
        let cfg =
            super::super::json::parse(include_str!("../../../experiments/genesis/config.json"))
                .unwrap();
        let m = Memory::new("selmem_full", out.join("state.vault"), &cfg);
        m.init().unwrap();
        m.call(
            "live",
            obj([(
                "event",
                obj([
                    ("id", text("e1")),
                    (
                        "text",
                        text("We shared useful books during a room meeting."),
                    ),
                ]),
            )]),
        )
        .unwrap();
        let state = m.call("inspect", obj([])).unwrap();
        let id = array(&state, "traces").unwrap()[0]
            .get("id")
            .unwrap()
            .clone();
        let audit = obj([
            ("selected_ids", arr([id])),
            ("query", text("SECRET_TASK_600_EUROS")),
        ]);
        let mut p = Provider::new(true, "mock".into(), "".into(), None, &out);
        let hash = file_hash(&m.path).unwrap();
        let first = context(&m, &audit, true, &mut p, &out).unwrap();
        let count = std::fs::read_to_string(out.join("responses.jsonl"))
            .unwrap()
            .lines()
            .count();
        assert_eq!(count, 2);
        for line in std::fs::read_to_string(out.join("responses.jsonl"))
            .unwrap()
            .lines()
        {
            let row = super::super::json::parse(line).unwrap();
            assert!(!row.get("prompt").unwrap().encode().contains("SECRET_TASK"));
            assert!(!row.get("prompt").unwrap().encode().contains("core"));
        }
        assert_eq!(context(&m, &audit, true, &mut p, &out).unwrap(), first);
        assert_eq!(
            std::fs::read_to_string(out.join("responses.jsonl"))
                .unwrap()
                .lines()
                .count(),
            count
        );
        // Simulate interruption after a committed retelling but before its guard.
        for file in std::fs::read_dir(out.join("guarded-readouts")).unwrap() {
            std::fs::remove_file(file.unwrap().path()).unwrap();
        }
        for file in std::fs::read_dir(out.join("guarded-stages")).unwrap() {
            let file = file.unwrap().path();
            if file.to_string_lossy().ends_with("-guard.json") {
                std::fs::remove_file(file).unwrap();
            }
        }
        assert_eq!(context(&m, &audit, true, &mut p, &out).unwrap(), first);
        let records = std::fs::read_to_string(out.join("responses.jsonl")).unwrap();
        assert_eq!(records.lines().count(), count + 1);
        assert_eq!(
            s(
                &super::super::json::parse(records.lines().last().unwrap()).unwrap(),
                "mode"
            )
            .unwrap(),
            "preview_guard"
        );
        assert_eq!(file_hash(&m.path).unwrap(), hash);
        std::fs::remove_dir_all(out).unwrap();
    }
}
