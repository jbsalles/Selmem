use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::SystemTime,
};

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_atelier-transfer"))
        .args(args)
        .output()
        .unwrap()
}

fn call(args: &[&str]) {
    let result = invoke(args);
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn assert_actor_error(out: &Path, expected: &str) {
    let result = invoke(&["actor", out.to_str().unwrap(), "--max-http-calls", "0"]);
    let message = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success());
    assert!(message.contains(expected), "{message}");
}

fn read(dir: &Path, file: &str) -> Value {
    serde_json::from_slice(&fs::read(dir.join(file)).unwrap()).unwrap()
}

#[test]
fn matched_and_blind_pilot_through_normal_entry_points() {
    exercise_pilot(["--max-http-calls", "0"], "hybrid", "hybrid_rag");
    exercise_pilot(["--rag", "bm25"], "bm25", "bm25_rag");
}

fn exercise_pilot(options: [&str; 2], retrieval: &str, rag_arm: &str) {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("atelier-{suffix}"));
    let out = dir.to_str().unwrap();
    assert_actor_error(&dir, "Use the directory created by `prepare`");
    call(&["prepare", out, "--legacy", "--mock"]);
    let empty = dir.join("unprepared");
    fs::create_dir(&empty).unwrap();
    assert_actor_error(
        &empty,
        &format!("Missing {}", empty.join("run.json").display()),
    );
    assert_actor_error(
        &dir,
        &format!("Missing {}", dir.join("plan.json").display()),
    );
    call(&["plan", out, options[0], options[1]]);
    let mut saved_plan = read(&dir, "plan.json");
    assert_eq!(saved_plan["retrieval"], retrieval);
    if retrieval == "hybrid" {
        // Older plans have no retrieval field and represent hybrid runs.
        saved_plan.as_object_mut().unwrap().remove("retrieval");
        fs::write(dir.join("plan.json"), saved_plan.to_string()).unwrap();
    } else {
        assert!(!dir.join("embeddings").exists());
        let mismatch = invoke(&["plan", out, "--max-http-calls", "0"]);
        assert!(!mismatch.status.success());
        assert!(String::from_utf8_lossy(&mismatch.stderr).contains("The saved plan uses BM25 RAG"));
        assert_eq!(read(&dir, "plan.json"), saved_plan);
    }
    call(&["actor", out, "--max-http-calls", "0"]);
    call(&["actor", out, "--max-http-calls", "0"]);

    let study = read(&dir, "run.json");
    let first = study["histories"][0]["episodes"].as_array().unwrap();
    let second = study["histories"][1]["episodes"].as_array().unwrap();
    let mut exchanged = Vec::new();
    for (a, b) in first.iter().zip(second) {
        assert_eq!(a["id"], b["id"]);
        let mut expected = a.clone();
        if !a["observation"].is_null() {
            let count = a["observation"]["first_choices"].as_u64().unwrap();
            let complement = 6 - count;
            expected["observation"]["first_choices"] = complement.into();
            expected["raw"] = a["raw"]
                .as_str()
                .unwrap()
                .replace(
                    &format!("{count} of the six"),
                    &format!("{complement} of the six"),
                )
                .into();
            exchanged.push((count, complement));
        }
        assert_eq!(&expected, b);
    }
    assert_eq!(exchanged, [(5, 1), (1, 5)]);

    let plan = read(&dir, "plan.json");
    let budget = plan["budget"].as_u64().unwrap() as usize;
    let jobs = plan["jobs"].as_array().unwrap();
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    assert_eq!(jobs.len(), 48);
    for brief in 0..6 {
        let for_cell = |history: usize, arm: &str| {
            jobs.iter()
                .find(|j| j["history"] == history && j["brief"] == brief && j["arm"] == arm)
                .unwrap()
        };
        assert_eq!(for_cell(0, "empty")["user"], for_cell(1, "empty")["user"]);
        let matched_len = bpe
            .encode_ordinary(for_cell(0, "selmem")["user"].as_str().unwrap())
            .len();
        for history in 0..2 {
            let native = for_cell(history, "selmem");
            let relation = for_cell(history, "selmem_relation");
            assert_eq!(native["episodes"], relation["episodes"]);
            let relation_text = relation["memory"].as_str().unwrap();
            let five = relation_text.find("Jo counted 5 of 6").unwrap();
            assert_eq!(
                five < relation_text.find("Jo counted 1 of 6").unwrap(),
                history == 0
            );
            for row in [native, relation] {
                assert_eq!(
                    bpe.encode_ordinary(row["user"].as_str().unwrap()).len(),
                    matched_len
                );
                let text = row["memory"].as_str().unwrap();
                assert_eq!(bpe.encode_ordinary(text).len(), budget);
                let ends = text
                    .match_indices("</entry>\n")
                    .map(|(i, s)| bpe.encode_ordinary(&text[..i + s.len()]).len())
                    .collect::<Vec<_>>();
                assert_eq!(json!(ends), plan["matched_entry_ends"]);
            }
            let rag = for_cell(history, rag_arm);
            assert!(bpe.encode_ordinary(rag["memory"].as_str().unwrap()).len() <= budget);
            assert_eq!(for_cell(history, "empty")["memory"], "");
        }
    }

    let blind = read(&dir, "blind.json");
    let blind = blind.as_array().unwrap();
    assert_eq!(blind.len(), 48);
    let mut grades = Vec::new();
    for row in blind {
        assert_eq!(row.as_object().unwrap().len(), 6);
        assert!(row.get("influence").is_none());
        let job = jobs.iter().find(|j| j["id"] == row["id"]).unwrap();
        let (originality, device) = match (
            job["arm"].as_str().unwrap(),
            job["history"].as_u64().unwrap(),
        ) {
            ("selmem_relation", 0) => (5, "interruption"),
            ("selmem_relation", 1) => (5, "concentration"),
            _ => (3, "other"),
        };
        grades.push(json!({"id":row["id"],"originality":originality,"fit":4,"device":device}));
    }
    let a = dir.join("ratings-a.json");
    let b = dir.join("ratings-b.json");
    let bytes = serde_json::to_vec(&grades).unwrap();
    fs::write(&a, &bytes).unwrap();
    fs::write(&b, &bytes).unwrap();
    call(&["score", out, a.to_str().unwrap(), b.to_str().unwrap()]);
    let score = read(&dir, "scores.json");
    assert_eq!(score["mock"], true);
    assert_eq!(score["retrieval"], retrieval);
    assert_eq!(score["raters"][0]["relation_vs"][1]["baseline"], rag_arm);
    assert_eq!(score["device_agreements"], 48);
    assert_eq!(
        score["raters"][0]["arms"][1]["switches_following_outcome"],
        6
    );
    assert_eq!(
        score["raters"][1]["relation_vs"][0]["mean_originality_delta"],
        2.0
    );
    assert_eq!(
        read(&dir, "attribution-review.json")
            .as_array()
            .unwrap()
            .len(),
        48
    );
    assert!(!dir.join("actor-attempts.json").exists());
    assert!(!dir.join("embedding-attempts.json").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn retained_outcomes_reach_grok_prompts_and_score_separately_from_declared_choices() {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("atelier-v2-{suffix}"));
    let out = dir.to_str().unwrap();
    call(&["prepare", out, "--mock"]);
    call(&["plan", out, "--rag", "bm25"]);

    let run = read(&dir, "run.json");
    assert_eq!(run["protocol"], "atelier-transfer-v2");
    assert_eq!(run["repeats"], 2);
    let plan = read(&dir, "plan.json");
    let jobs = plan["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 96);
    let for_cell = |history: usize, brief: usize, repeat: usize, arm: &str| {
        jobs.iter()
            .find(|job| {
                job["history"] == history
                    && job["brief"] == brief
                    && job["repeat"] == repeat
                    && job["arm"] == arm
            })
            .unwrap()
    };
    for job in jobs {
        let history = job["history"].as_u64().unwrap() as usize;
        let brief = job["brief"].as_u64().unwrap() as usize;
        let arm = job["arm"].as_str().unwrap();
        let memory = job["memory"].as_str().unwrap();
        assert_eq!(job["user"], for_cell(history, brief, 0, arm)["user"]);
        match arm {
            "selmem" => {
                let panel = run["histories"][history]["panel"].as_array().unwrap();
                for reading in panel {
                    let narrative = reading["retained"]["narrative"].as_str().unwrap();
                    assert!(memory.contains(narrative));
                    assert_ne!(reading["trace_id"], reading["retained"]["trace_id"]);
                }
                let prefix = for_cell(history, brief, 0, "selmem_prefix");
                assert_eq!(job["memory_tokens"], prefix["memory_tokens"]);
                assert_eq!(job["episodes"], prefix["episodes"]);
                let other = for_cell(1 - history, brief, 0, arm)["memory"]
                    .as_str()
                    .unwrap();
                let swapped = memory
                    .replace("recorded that 5", "recorded that swapped")
                    .replace("recorded that 1", "recorded that 5")
                    .replace("recorded that swapped", "recorded that 1");
                assert_eq!(swapped, other);
                assert_ne!(memory, other);
            }
            "selmem_prefix" | "empty" => {
                assert_eq!(job["user"], for_cell(1 - history, brief, 0, arm)["user"]);
                assert!(!memory.contains("Jo recorded"));
            }
            "bm25_rag" => {
                let ids = job["episodes"].as_array().unwrap();
                assert!(ids.contains(&json!("band-proof")));
                assert!(ids.contains(&json!("cluster-proof")));
            }
            _ => panic!("unexpected arm {arm}"),
        }
    }

    call(&["calibrate", out, "--max-http-calls", "0"]);
    assert_eq!(fs::read_dir(dir.join("responses")).unwrap().count(), 24);
    assert!(!dir.join("blind.json").exists());
    let calibration = read(&dir, "calibration.json");
    for brief in calibration["briefs"].as_array().unwrap() {
        assert_eq!(brief["samples"], 4);
        assert_eq!(brief["declared"]["other"], 4);
    }
    let control = for_cell(0, 0, 0, "empty");
    let control_file = dir
        .join("responses")
        .join(format!("{}.json", control["id"].as_str().unwrap()));
    let before = fs::read(&control_file).unwrap();
    call(&["actor", out, "--max-http-calls", "0"]);
    assert_eq!(fs::read(control_file).unwrap(), before);

    let mut grades = Vec::new();
    let blind = read(&dir, "blind.json");
    let blind = blind.as_array().unwrap();
    assert_eq!(blind.len(), 96);
    for row in blind {
        assert_eq!(row.as_object().unwrap().len(), 6);
        assert!(row["device"].is_null() && row.get("influence").is_none());
        let job = jobs.iter().find(|job| job["id"] == row["id"]).unwrap();
        let device = if job["arm"] == "selmem" {
            if job["history"] == 0 {
                "interruption"
            } else {
                "concentration"
            }
        } else {
            "other"
        };
        grades.push(json!({"id":row["id"],"originality":3,"fit":4,"device":device}));
    }
    let a = dir.join("ratings-a.json");
    let b = dir.join("ratings-b.json");
    let bytes = serde_json::to_vec(&grades).unwrap();
    fs::write(&a, &bytes).unwrap();
    fs::write(&b, &bytes).unwrap();
    call(&["score", out, a.to_str().unwrap(), b.to_str().unwrap()]);
    let score = read(&dir, "scores.json");
    assert_eq!(score["raters"][0]["arms"][1]["arm"], "selmem");
    assert_eq!(
        score["raters"][0]["arms"][1]["switches_following_outcome"],
        12
    );
    assert_eq!(
        score["raters"][0]["arms"][1]["concentration_rate_shift"],
        1.0
    );
    assert_eq!(
        score["declared_devices"][1]["switches_following_outcome"],
        0
    );
    assert_eq!(score["declared_blind_agreement"], json!([72, 72]));
    assert!(!dir.join("actor-attempts.json").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn embedding_provider_can_change_until_a_vector_is_saved() {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("atelier-provider-{suffix}"));
    let out = dir.to_str().unwrap();
    call(&["prepare", out]);

    let cache = dir.join("embeddings");
    fs::create_dir(&cache).unwrap();
    let chat = "https://example.invalid/v1/chat/completions";
    let embeddings = "http://127.0.0.1:11434/v1/embeddings";
    let attempts = json!(["first", "second", "third", "fourth"]);
    fs::write(
        cache.join("provider.json"),
        json!({"endpoint": chat, "model": "chat-model"}).to_string(),
    )
    .unwrap();
    fs::write(dir.join("embedding-attempts.json"), attempts.to_string()).unwrap();

    // All four attempts are already spent. Each case must stop locally.
    let plan_error = |endpoint: &str, model: &str| {
        let result = Command::new(env!("CARGO_BIN_EXE_atelier-transfer"))
            .args(["plan", out, "--max-http-calls", "4"])
            .env("SELMEM_EMBED_URL", endpoint)
            .env("SELMEM_EMBED_MODEL", model)
            .env_remove("SELMEM_EMBED_API_KEY")
            .output()
            .unwrap();
        assert!(!result.status.success());
        String::from_utf8(result.stderr).unwrap()
    };
    let wrong_route = plan_error(chat, "chat-model");
    assert!(
        wrong_route.contains("points to a chat route"),
        "{wrong_route}"
    );
    let resumed = plan_error(embeddings, "embed-v1");
    assert!(resumed.contains("paused at 4/4"), "{resumed}");
    let provider = read(&cache, "provider.json");
    assert_eq!(
        provider,
        json!({"endpoint": embeddings, "model": "embed-v1"})
    );

    let run = read(&dir, "run.json");
    let text = run["histories"][0]["episodes"][0]["raw"].as_str().unwrap();
    let id = selmem::util::sha256::hex(text.as_bytes());
    fs::write(cache.join(format!("{id}.json")), "[1.0, 0.0]").unwrap();
    let incompatible = plan_error(embeddings, "embed-v2");
    assert!(
        incompatible.contains("1 saved embeddings"),
        "{incompatible}"
    );
    assert_eq!(read(&cache, "provider.json"), provider);
    assert_eq!(read(&dir, "embedding-attempts.json"), attempts);
    fs::remove_dir_all(dir).unwrap();
}
