/// Format constraint for the mouth. It does not name an act and it does not map a valence.
pub const READING_CONSTRAINT: &str = "Speak from the reading profile provided. Do not recite it. Do not name it. Do not contradict it without a marked reason.";

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

pub fn post_json(url: &str, api_key: Option<&str>, body: &str) -> Result<String, String> {
    if url.starts_with("https://") || which_curl() {
        return curl_post(url, api_key, body);
    }
    raw_http_post(url, api_key, body)
}

fn which_curl() -> bool {
    Command::new("curl").arg("--version").output().is_ok()
}

fn curl_post(url: &str, api_key: Option<&str>, body: &str) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    let timeout = crate::config::Config::get().http_timeout();
    // Secret stays on stdin (--config -), never in argv.
    cmd.args([
        "-sS",
        "--max-time",
        &timeout,
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "-w",
        "\n__SELMEM_HTTP__:%{http_code}",
        "--config",
        "-",
        url,
    ]);
    cmd.arg("--data-binary").arg(body);
    // wait_with_output only reads handles piped before spawn. Inherited stdout
    // is why the body showed on the terminal and the parser saw 0 bytes.
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let Some(stdin) = child.stdin.as_mut() {
        if let Some(k) = api_key {
            let line = format!("header = \"Authorization: Bearer {}\"\n", k.replace('"', ""));
            stdin.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if !out.status.success() && !stdout.contains("__SELMEM_HTTP__:") {
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    let raw = if stdout.contains("__SELMEM_HTTP__:") {
        stdout
    } else {
        format!("{stdout}{stderr}")
    };
    let mut body = split_http_status(&raw)?;
    if !body.contains("choices") && stderr.contains("choices") {
        body.push_str(&stderr);
    }
    Ok(body)
}

fn raw_http_post(url: &str, api_key: Option<&str>, body: &str) -> Result<String, String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "URL http(s) attendue".to_string())?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = if let Some((h, p)) = hostport.split_once(':') {
        (h, p.parse::<u16>().unwrap_or(80))
    } else {
        (hostport, 80)
    };
    let path = if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(k) = api_key {
        req.push_str(&format!("Authorization: Bearer {k}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    let mut stream = TcpStream::connect(format!("{host}:{port}")).map_err(|e| e.to_string())?;
    let timeout = crate::config::Config::get().http_timeout().parse().unwrap_or(60);
    stream
        .set_read_timeout(Some(Duration::from_secs(timeout)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(timeout)))
        .ok();
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).map_err(|e| e.to_string())?;
    let (status, body) = split_raw_status(&raw);
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}: {}", body.chars().take(180).collect::<String>()));
    }
    Ok(body)
}

fn split_http_status(raw: &str) -> Result<String, String> {
    const MARK: &str = "__SELMEM_HTTP__:";
    if let Some(i) = raw.rfind(MARK) {
        let body = raw[..i].trim_end_matches(['\n', '\r']).to_string();
        let code: u16 = raw[i + MARK.len()..].trim().parse().unwrap_or(0);
        if code != 0 && !(200..300).contains(&code) {
            return Err(format!("HTTP {code}: {}", body.chars().take(180).collect::<String>()));
        }
        return Ok(body);
    }
    Ok(raw.to_string())
}

fn split_raw_status(raw: &str) -> (u16, String) {
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or(raw).to_string();
    (status, body)
}

pub fn json_esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// First `"key": "…"` whose key is a real JSON key, not a substring.
/// UTF-8 safe. Prefers `choices[0].message.content` when `key == "content"`.
pub fn extract_json_string(body: &str, key: &str) -> Option<String> {
    if key == "content" {
        if let Some(s) = chat_message_content(body) {
            return Some(s);
        }
    }
    first_string_field(body, key)
}

pub fn first_string_field(json: &str, key: &str) -> Option<String> {
    let mut i = 0;
    let bytes = json.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let (s, n) = parse_json_string(&json[i..])?;
            let after = json[i + n..].trim_start();
            if s == key && after.starts_with(':') {
                let val = after[1..].trim_start();
                if val.starts_with('"') {
                    return parse_json_string(val).map(|(v, _)| v);
                }
            }
            i += n;
            continue;
        }
        i += 1;
    }
    None
}

pub fn first_string_array(json: &str, key: &str) -> Option<Vec<String>> {
    let mut i = 0;
    let bytes = json.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let (s, n) = parse_json_string(&json[i..])?;
            let after = json[i + n..].trim_start();
            if s == key && after.starts_with(':') {
                let val = after[1..].trim_start();
                if val.starts_with('[') {
                    return Some(parse_string_array(val));
                }
            }
            i += n;
            continue;
        }
        i += 1;
    }
    None
}

fn chat_message_content(body: &str) -> Option<String> {
    // choices -> first object -> message -> content
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let (s, n) = parse_json_string(&body[i..])?;
            let after = body[i + n..].trim_start();
            if s == "choices" && after.starts_with(':') {
                let val = after[1..].trim_start();
                return content_in_first_choice(val);
            }
            i += n;
            continue;
        }
        i += 1;
    }
    None
}

fn content_in_first_choice(after_colon: &str) -> Option<String> {
    let arr = after_colon.trim_start();
    if !arr.starts_with('[') {
        return None;
    }
    let inner = arr[1..].trim_start();
    if !inner.starts_with('{') {
        return None;
    }
    // message.content — not a sibling "content" on a tool/reasoning blob.
    if let Some(msg) = object_after_key(inner, "message") {
        for key in ["content", "reasoning_content", "reasoning", "text", "output"] {
            if let Some(s) = first_string_field(msg, key) {
                if !s.trim().is_empty() {
                    return Some(s);
                }
            }
        }
    }
    for key in ["content", "reasoning_content", "reasoning", "text"] {
        if let Some(s) = first_string_field(inner, key) {
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn object_after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let mut i = 0;
    let bytes = json.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let (s, n) = parse_json_string(&json[i..])?;
            let after = json[i + n..].trim_start();
            if s == key && after.starts_with(':') {
                let val = after[1..].trim_start();
                if val.starts_with('{') {
                    return Some(val);
                }
            }
            i += n;
            continue;
        }
        i += 1;
    }
    None
}

pub fn first_number_field(json: &str, key: &str) -> Option<f32> {
    let mut i = 0;
    let bytes = json.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let (s, n) = parse_json_string(&json[i..])?;
            let after = json[i + n..].trim_start();
            if s == key && after.starts_with(':') {
                let val = after[1..].trim_start();
                let num: String = val
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                return num.parse().ok();
            }
            i += n;
            continue;
        }
        i += 1;
    }
    None
}

/// `s` starts at the opening quote. Returns (decoded, bytes consumed).
pub fn parse_json_string(s: &str) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((out, i + 1)),
            b'\\' if i + 1 < bytes.len() => {
                match bytes[i + 1] {
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'u' if i + 5 < bytes.len() => {
                        let hex = s.get(i + 2..i + 6)?;
                        if let Ok(cp) = u32::from_str_radix(hex, 16) {
                            if (0xD800..=0xDBFF).contains(&cp) {
                                let tail = s.get(i + 6..i + 12).unwrap_or("");
                                if let Some(low) = tail.strip_prefix("\\u").and_then(|h| u32::from_str_radix(&h[..4.min(h.len())], 16).ok()) {
                                    if (0xDC00..=0xDFFF).contains(&low) && tail.len() >= 6 {
                                        let u = 0x10000 + (((cp - 0xD800) << 10) | (low - 0xDC00));
                                        if let Some(ch) = char::from_u32(u) {
                                            out.push(ch);
                                        }
                                        i += 12;
                                        continue;
                                    }
                                }
                            } else if let Some(ch) = char::from_u32(cp) {
                                out.push(ch);
                            }
                        }
                        i += 6;
                        continue;
                    }
                    _ => {}
                }
                i += 2;
            }
            _ => {
                let ch = s[i..].chars().next()?;
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    None
}

fn parse_string_array(s: &str) -> Vec<String> {
    let mut body = s.trim_start();
    if !body.starts_with('[') {
        return Vec::new();
    }
    body = body[1..].trim_start();
    let mut out = Vec::new();
    loop {
        body = body.trim_start();
        if body.is_empty() || body.starts_with(']') {
            break;
        }
        if body.starts_with(',') {
            body = body[1..].trim_start();
            continue;
        }
        if body.starts_with('"') {
            if let Some((v, n)) = parse_json_string(body) {
                out.push(v);
                body = &body[n..];
                continue;
            }
        }
        break;
    }
    out
}

pub fn extract_json_array_f32(body: &str, key: &str) -> Option<Vec<f32>> {
    let pat = format!("\"{key}\"");
    let mut search = body;
    let mut best: Option<Vec<f32>> = None;
    while let Some(i) = search.find(&pat) {
        let abs = body.len() - search.len() + i;
        let after = body[abs + pat.len()..].trim_start();
        if let Some(rest) = after.strip_prefix(':') {
            if let Some(vals) = parse_f32_array(rest.trim_start()) {
                if best.as_ref().map(|b| vals.len() > b.len()).unwrap_or(true) {
                    best = Some(vals);
                }
            }
        }
        search = &body[abs + pat.len()..];
    }
    best
}

fn parse_f32_array(after: &str) -> Option<Vec<f32>> {
    let start = after.find('[')?;
    let rest = &after[start + 1..];
    let end = rest.find(']')?;
    let vals: Vec<f32> = rest[..end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_json_string;

    #[test]
    fn surrogate_pair_becomes_one_scalar() {
        let (s, n) = parse_json_string(r#""\uD83D\uDE00""#).unwrap();
        assert_eq!(s, "😀");
        assert!(n > 4);
    }
}
