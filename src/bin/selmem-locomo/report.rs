use super::json::{obj, text, Json};
use super::memory::save;
use std::{fs, path::Path};
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn csv(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub fn write(out: &Path, manifest: &Json, rows: &[Json]) -> Result<(), String> {
    save(&out.join("manifest.json"), manifest)?;
    save(&out.join("responses.json"), &Json::Array(rows.to_vec()))?;
    let mut review=String::from("response_id,probe,response,decision_or_action,originality_1_5,usefulness_1_5,unsupported_factual_claims,notes\n");
    let mut reveal = std::collections::BTreeMap::new();
    let mut cards = String::new();
    for row in rows {
        let id = row.get("id")?.string()?;
        let p = row.get("probe")?.string()?;
        let answer = row.get("response")?.string()?;
        let condition = row.get("condition")?.string()?;
        review += &format!("{},{},{},,,,,\n", csv(id), csv(p), csv(answer));
        reveal.insert(
            id.into(),
            obj([
                ("condition", text(condition)),
                ("repetition", row.get("repetition")?.clone()),
            ]),
        );
        cards += &format!(
            "<article><h3>{} · {}</h3><p class=label hidden>{}</p><pre>{}</pre></article>",
            esc(id),
            esc(p),
            esc(condition),
            esc(answer)
        );
    }
    fs::write(out.join("blind_review.csv"), review).map_err(|e| e.to_string())?;
    save(&out.join("reveal.json"), &Json::Object(reveal))?;
    let page = format!(
        r#"<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content="width=device-width"><title>Two Claires — LoCoMo</title><style>body{{margin:40px auto;padding:0 24px;max-width:1200px;background:#101421;color:#e9edfa;font:16px system-ui}}h1{{font-size:42px}}button{{padding:12px 18px;border:0;border-radius:12px;background:#b7bcff;cursor:pointer}}main{{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:18px}}article{{padding:22px;background:#1b2235;border:1px solid #35405e;border-radius:18px}}pre{{font:inherit;white-space:pre-wrap;overflow-wrap:anywhere}}.label{{color:#b7bcff}}</style><h1>Same past, two futures</h1><p>{} · {} responses · model: {}</p><p>Rate the responses before revealing the conditions. Differences alone do not establish causality.</p><button onclick="document.querySelectorAll('.label').forEach(x=>x.hidden=!x.hidden)">Reveal / hide conditions</button><main>{cards}</main></html>"#,
        esc(manifest.get("status")?.string()?),
        rows.len(),
        esc(manifest.get("model")?.string()?)
    );
    fs::write(out.join("report.html"), page).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_markup() {
        assert!(!esc("<script>&\"").contains('<'));
        assert_eq!(csv("a,\"b"), "\"a,\"\"b\"");
    }
}
