//! Small JSON tree for the experiment files; strings use SelMem's tested decoder.
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
pub fn text(s: impl Into<String>) -> Json {
    Json::String(s.into())
}
pub fn num(n: impl Into<f64>) -> Json {
    Json::Number(n.into())
}
impl Json {
    pub fn get(&self, k: &str) -> Result<&Json, String> {
        if let Self::Object(o) = self {
            o.get(k).ok_or_else(|| format!("missing JSON field {k}"))
        } else {
            Err("expected object".into())
        }
    }
    pub fn string(&self) -> Result<&str, String> {
        if let Self::String(s) = self {
            Ok(s)
        } else {
            Err("expected string".into())
        }
    }
    pub fn number(&self) -> Result<f64, String> {
        if let Self::Number(n) = self {
            Ok(*n)
        } else {
            Err("expected number".into())
        }
    }
    pub fn array(&self) -> Result<&[Json], String> {
        if let Self::Array(a) = self {
            Ok(a)
        } else {
            Err("expected array".into())
        }
    }
    pub fn object(&self) -> Result<&BTreeMap<String, Json>, String> {
        if let Self::Object(o) = self {
            Ok(o)
        } else {
            Err("expected object".into())
        }
    }
    pub fn put(&mut self, k: &str, value: Json) {
        if let Self::Object(o) = self {
            o.insert(k.into(), value);
        }
    }
    pub fn encode(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => format!("\"{}\"", selmem::net::httpx::json_esc(s)),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::encode).collect::<Vec<_>>().join(",")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}:{}", text(k).encode(), v.encode()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}
pub fn parse(s: &str) -> Result<Json, String> {
    fn value(s: &str, depth: usize) -> Result<(Json, &str), String> {
        if depth > 64 {
            return Err("JSON nesting limit".into());
        }
        let s = s.trim_start();
        if s.starts_with('"') {
            let (v, n) = selmem::net::httpx::parse_json_string(s).ok_or("invalid JSON string")?;
            return Ok((text(v), &s[n..]));
        }
        for (token, v) in [
            ("null", Json::Null),
            ("true", Json::Bool(true)),
            ("false", Json::Bool(false)),
        ] {
            if let Some(rest) = s.strip_prefix(token) {
                return Ok((v, rest));
            }
        }
        if s.starts_with('[') || s.starts_with('{') {
            let object = s.starts_with('{');
            let end = if object { '}' } else { ']' };
            let mut rest = &s[1..];
            let mut a = Vec::new();
            let mut o = BTreeMap::new();
            if rest.trim_start().starts_with(end) {
                rest = rest.trim_start();
                return Ok((
                    if object {
                        Json::Object(o)
                    } else {
                        Json::Array(a)
                    },
                    &rest[1..],
                ));
            }
            loop {
                rest = rest.trim_start();
                let key = if object {
                    let (k, r) = value(rest, depth + 1)?;
                    rest = r.trim_start().strip_prefix(':').ok_or("expected colon")?;
                    Some(k.string()?.to_string())
                } else {
                    None
                };
                let (v, r) = value(rest, depth + 1)?;
                rest = r.trim_start();
                if let Some(k) = key {
                    if o.insert(k, v).is_some() {
                        return Err("duplicate JSON key".into());
                    }
                } else {
                    a.push(v);
                }
                if rest.starts_with(end) {
                    return Ok((
                        if object {
                            Json::Object(o)
                        } else {
                            Json::Array(a)
                        },
                        &rest[1..],
                    ));
                }
                rest = rest.strip_prefix(',').ok_or("expected comma")?;
            }
        }
        let n = s
            .find(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E')))
            .unwrap_or(s.len());
        let raw = &s[..n];
        if raw.is_empty() {
            return Err("invalid JSON value".into());
        }
        let v: f64 = raw.parse().map_err(|_| "invalid number")?;
        if !v.is_finite() {
            return Err("nonfinite number".into());
        }
        Ok((num(v), &s[n..]))
    }
    let (v, rest) = value(s, 0)?;
    if !rest.trim().is_empty() {
        return Err("trailing JSON".into());
    }
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_and_unicode() {
        let s = r#"{"a":[1,true,null,{"s":"\uD83D\uDE00"}],"b":"é"}"#;
        let j = parse(s).unwrap();
        assert_eq!(parse(&j.encode()).unwrap(), j);
        assert_eq!(j.get("b").unwrap().string().unwrap(), "é");
    }
    #[test]
    fn reject_corrupt() {
        for s in ["[]garbage", "{\"a\":1,\"a\":2}", "[1,]", "{\"a\" 1}"] {
            assert!(parse(s).is_err(), "{s}");
        }
    }
}
