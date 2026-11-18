use serde_json::{json, Value};
use std::{fs, process::Command};
#[test]
fn strict_shortfall_and_deterministic_pilot() {
    let root = std::env::temp_dir().join(format!("vg-locomo-test-{}", std::process::id()));
    fs::create_dir_all(root.join("xml")).unwrap();
    for (id, n) in [(1, 2), (2, 5000)] {
        let text = "A quiet walk beside the river. ".repeat(n);
        let xml = format!(
            r#"<TEI><correspAction type="sent"><rs>Vincent van Gogh</rs></correspAction><correspAction type="received"><rs>Theo</rs></correspAction><div type="translation" xml:lang="en"><p>{text}</p></div><note>EDITORIAL SECRET</note></TEI>"#
        );
        fs::write(root.join(format!("xml/let00{id}.xml")), xml).unwrap();
    }
    let data = json!([{"qa":[{"answer":"ANSWER SECRET"}],"conversation":{"speaker_a":"A","speaker_b":"B","session_1":[{"dia_id":"D1:1","speaker":"A","text":"I went to the market for bread and fruit. ".repeat(100)}]}}]);
    fs::write(root.join("noise.json"), data.to_string()).unwrap();
    let run = |name: &str, mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_selmem-vangogh-realistic"))
            .arg(root.join("xml"))
            .arg(root.join("noise.json"))
            .arg(root.join(name))
            .args(["--mode", mode, "--noise-percent", "90"])
            .output()
            .unwrap()
    };
    let full = run("full", "full");
    assert!(!full.status.success());
    assert!(String::from_utf8_lossy(&full.stderr).contains("insufficient unique noise"));
    assert!(!root.join("full").exists());
    for name in ["pilot-a", "pilot-b"] {
        let r = run(name, "pilot");
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    }
    for file in [
        "manifest.json",
        "stream-mixed.jsonl",
        "stream-clean.jsonl",
        "audit-mixed.jsonl",
    ] {
        assert_eq!(
            fs::read(root.join("pilot-a").join(file)).unwrap(),
            fs::read(root.join("pilot-b").join(file)).unwrap()
        );
    }
    let read = |file: &str| fs::read_to_string(root.join("pilot-a").join(file)).unwrap();
    let m: Value = serde_json::from_str(&read("manifest.json")).unwrap();
    assert_eq!(m["selected_letters"], 1);
    assert!((m["noise_fraction"].as_f64().unwrap() - 0.9).abs() < 0.001);
    let mixed = read("stream-mixed.jsonl");
    assert!(!mixed.contains("SECRET"));
    assert!(!mixed.contains("source_kind"));
    let audit: Vec<Value> = read("audit-mixed.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let rows: Vec<Value> = mixed
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let clean: Vec<Value> = read("stream-clean.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        rows.iter()
            .zip(&audit)
            .filter(|(_, a)| a["source_kind"] == "letter")
            .map(|(r, _)| &r["text"])
            .collect::<Vec<_>>(),
        clean.iter().map(|r| &r["text"]).collect::<Vec<_>>()
    );
    let p = root.join("xml/let002.xml");
    let shortened = fs::read_to_string(&p).unwrap().replace(
        &"A quiet walk beside the river. ".repeat(5000),
        &"A quiet walk beside the river. ".repeat(50),
    );
    fs::write(p, shortened).unwrap();
    let default_run = Command::new(env!("CARGO_BIN_EXE_selmem-vangogh-realistic"))
        .arg(root.join("xml"))
        .arg(root.join("noise.json"))
        .arg(root.join("default-ten"))
        .output()
        .unwrap();
    assert!(
        default_run.status.success(),
        "{}",
        String::from_utf8_lossy(&default_run.stderr)
    );
    let m: Value =
        serde_json::from_slice(&fs::read(root.join("default-ten/manifest.json")).unwrap()).unwrap();
    assert_eq!(m["selected_letters"], 2);
    assert_eq!(m["requested_noise_percent"], 10);
    assert_eq!(
        m["target_noise_tokens"].as_u64().unwrap(),
        m["source_tokens"].as_u64().unwrap() / 9
    );
    assert!((m["noise_fraction"].as_f64().unwrap() - 0.1).abs() < 0.001);
    fs::remove_dir_all(root).unwrap();
}
