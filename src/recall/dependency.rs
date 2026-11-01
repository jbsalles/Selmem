//! Conservative dependencies in explicit, scoped record syntax.
//! This is not a natural-language rule/exception classifier.
use crate::core::model::MemoryTrace;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Record {
    pub issuer: String,
    pub version: String,
    pub scope: Vec<(String, String)>,
    pub parent: Option<String>,
}

pub(super) fn parse(text: &str) -> Option<Record> {
    let (header, body) = text.split_once(':')?;
    let words: Vec<_> = header.split_whitespace().collect();
    let marker = words.iter().position(|w| w.eq_ignore_ascii_case("record"))?;
    if marker == 0 { return None; }
    let version = *words.get(marker + 1)?;
    if !version.starts_with('v') || version.len() < 2
        || !version[1..].chars().all(|c| c.is_ascii_digit()) { return None; }
    let fields = &words[marker + 2..];
    if fields.is_empty() || fields.len() % 2 != 0 { return None; }
    let mut scope: Vec<_> = fields.chunks(2)
        .map(|p| (p[0].to_lowercase(), p[1].to_lowercase())).collect();
    scope.sort();
    if scope.windows(2).any(|w| w[0].0 == w[1].0) { return None; }
    let mut parent = None;
    for sentence in body.split('.') {
        let terms: Vec<_> = sentence.split_whitespace().map(str::to_lowercase).collect();
        let reference = terms.len() >= 3 && terms[0] == "all" && terms[1] == "other"
            && (terms[3..] == ["rules", "remain", "in", "effect"]
                || terms[3..] == ["rules", "remain", "effective"]);
        if reference && parent.replace(terms[2].clone()).is_some() { return None; }
    }
    Some(Record { issuer: words[..marker].join(" ").to_lowercase(),
        version: version.into(), scope, parent })
}

/// Only comma/semicolon-delimited `key value` query fields constrain scope.
/// Free prose and multi-valued fields remain unknown, not inferred mismatches.
pub(super) fn conflicts(record: &Record, query: &str) -> bool {
    record.scope.iter().any(|(key, value)| {
        let values: Vec<_> = query.split([',', ';', '\n']).filter_map(|clause| {
            let clause = clause.rsplit(':').next()?.trim();
            let terms: Vec<_> = clause.split_whitespace().collect();
            (terms.len() == 2 && terms[0].eq_ignore_ascii_case(key))
                .then(|| terms[1].trim_end_matches('.').to_lowercase())
        }).collect();
        values.len() == 1 && values[0] != *value
    })
}

pub(super) fn same_provenance(a: &MemoryTrace, b: &MemoryTrace) -> bool {
    !a.source.is_empty() && a.source == b.source
        && a.attribution == b.attribution && a.channel == b.channel
}

/// The short core anchors the header; the lived gist retains the explicit
/// version reference. This is a retrieval cue, not a new authoritative fact.
pub(super) fn record(trace: &MemoryTrace) -> Option<Record> {
    let mut core = parse(&trace.core)?;
    if let Some(gist) = parse(&trace.gist) {
        if core.issuer == gist.issuer && core.version == gist.version && core.scope == gist.scope {
            core.parent = gist.parent.or(core.parent);
        }
    }
    Some(core)
}

pub(super) fn is_parent(child: &Record, parent: &Record) -> bool {
    child.parent.as_ref() == Some(&parent.version) && parent.parent.is_none()
        && child.issuer == parent.issuer && child.scope == parent.scope
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_reference_and_scope_are_preserved() {
        let r = parse("Maintainer record v4 workspace Cedar audience Orion: allow exports for audit only. All other v2 rules remain effective.").unwrap();
        assert_eq!(r.parent.as_deref(), Some("v2"));
        assert!(conflicts(&r, "Request: workspace Birch, audience Orion, purpose audit"));
        assert!(!conflicts(&r, "Request: workspace Cedar, audience Orion, purpose audit"));
        assert!(!conflicts(&r, "Discuss workspace Birch and Cedar"));
        assert!(!conflicts(&r, "workspace Birch, workspace Cedar"));
    }
    #[test]
    fn incidental_mentions_are_not_dependencies() {
        assert_eq!(parse("Maintainer record v4 workspace Cedar: someone mentioned v2.").unwrap().parent, None);
        assert!(parse("A vaguely related exception to a rule").is_none());
        assert!(parse("Maintainer record v4 workspace Cedar workspace Birch: rule").is_none());
    }
}
