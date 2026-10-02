use selmem::net::httpx::{
    extract_json_string, first_number_field, first_string_array, first_string_field, json_esc,
};

#[test]
fn utf8_french_is_not_split_into_bytes() {
    let raw = r#"{"content":"unjust decision"}"#;
    assert_eq!(
        first_string_field(raw, "content").as_deref(),
        Some("unjust decision")
    );
}

#[test]
fn content_inside_a_string_is_not_a_key() {
    let raw = r#"{"text":"say \"content\" please","event":"ok"}"#;
    assert_eq!(first_string_field(raw, "event").as_deref(), Some("ok"));
}

#[test]
fn chat_completion_uses_message_content_not_the_longest_string() {
    let raw = r#"{
      "id":"chatcmpl-1",
      "model":"gpt-4o-mini",
      "choices":[{
        "index":0,
        "message":{"role":"assistant","content":"I pull away."},
        "finish_reason":"stop"
      }],
      "usage":{"prompt_tokens":12}
    }"#;
    assert_eq!(
        extract_json_string(raw, "content").as_deref(),
        Some("I pull away.")
    );
}

#[test]
fn bifurcation_script_still_loads() {
    let s = selmem::script();
    assert!(s.sync.len() >= 8);
    assert!(s.salient.contains("unjust"));
    assert_eq!(s.probes.len(), 5);
}

#[test]
fn reasoning_longer_than_the_reply_does_not_win() {
    let raw = r#"{
      "choices":[{
        "message":{"role":"assistant","content":"I pull away."},
        "reasoning":{"content":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
      }]
    }"#;
    assert_eq!(
        extract_json_string(raw, "content").as_deref(),
        Some("I pull away.")
    );
}

#[test]
fn text_inside_a_string_does_not_steal_the_field() {
    let raw = r#"{"note":"the word \"text\" appears here","text":"the event"}"#;
    assert_eq!(first_string_field(raw, "text").as_deref(), Some("the event"));
}

#[test]
fn json_esc_escapes_control_chars() {
    let s = json_esc("ok\u{0001}fin");
    assert!(s.contains("\\u0001"), "got {s}");
    assert!(!s.contains('\u{0001}'));
}

#[test]
fn number_field_ignores_the_word_inside_a_string() {
    let raw = r#"{"note":"valence=-9","valence":0.25}"#;
    assert_eq!(first_number_field(raw, "valence"), Some(0.25));
}

#[test]
fn string_array_keeps_order() {
    let raw = r#"{"sync":["a","b","c"]}"#;
    assert_eq!(
        first_string_array(raw, "sync").unwrap(),
        ["a", "b", "c"]
    );
}

#[test]
fn grok_accept_content_is_readable() {
    let raw = r#"{"id":"8b1bbcbb-841b-9c21-8e5f-b3e5d22061ab","object":"chat.completion","created":1790980232,"model":"grok-4.3","choices":[{"index":0,"message":{"role":"assistant","content":"I accept.","refusal":null},"finish_reason":"stop"}],"usage":{"prompt_tokens":597,"completion_tokens":3,"total_tokens":600,"prompt_tokens_details":{"text_tokens":597,"audio_tokens":0,"image_tokens":0,"cached_tokens":576},"completion_tokens_details":{"reasoning_tokens":0,"audio_tokens":0,"accepted_prediction_tokens":0,"rejected_prediction_tokens":0},"num_sources_used":0,"cost_in_usd_ticks":1489500},"system_fingerprint":"fp_eb3c003fc66c14ed","service_tier":"default"}"#;
    assert_eq!(extract_json_string(raw, "content").as_deref(), Some("I accept."));
}
