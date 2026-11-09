//! Caller-authorized normative anchors on living traces. No archive lookup,
//! natural-language extraction, or enforcement of the actor's final answer.
use crate::{Attribution, MemoryStore, TraceStatus};
use std::collections::{BTreeMap, BTreeSet};
pub type NormScope = BTreeMap<String, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modality {
    Permit,
    Forbid,
    Verify,
}
impl Modality {
    fn token(self) -> &'static str {
        match self {
            Self::Permit => "permit",
            Self::Forbid => "forbid",
            Self::Verify => "verify",
        }
    }
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "permit" => Ok(Self::Permit),
            "forbid" => Ok(Self::Forbid),
            "verify" => Ok(Self::Verify),
            _ => Err("invalid modality".into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormClause {
    pub when: NormScope,
    pub modality: Modality,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormRelation {
    Root,
    Exception(String),
    Revision(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormRecord {
    pub source: String,
    pub scope: NormScope,
    pub action: String,
    pub clauses: Vec<NormClause>,
    /// Used only when no explicit clause applies; None inherits the parent.
    pub otherwise: Option<Modality>,
    pub relation: NormRelation,
}
fn atom(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-./".contains(&b))
}
fn valid_scope(s: &NormScope) -> bool {
    !s.is_empty() && s.len() <= 8 && s.iter().all(|(k, v)| atom(k) && atom(v))
}
fn fields(s: &NormScope) -> String {
    s.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ")
}
impl NormRecord {
    pub fn parent(&self) -> Option<&str> {
        match &self.relation {
            NormRelation::Root => None,
            NormRelation::Exception(p) | NormRelation::Revision(p) => Some(p),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !atom(&self.source)
            || !atom(&self.action)
            || !valid_scope(&self.scope)
            || self.clauses.is_empty()
            || self.clauses.len() > 16
            || self.clauses.iter().any(|c| !valid_scope(&c.when))
        {
            return Err("invalid normative fields; bounded ASCII identifiers required".into());
        }
        if self.parent().is_some_and(|p| !atom(p)) {
            return Err("invalid parent".into());
        }
        if matches!(self.relation, NormRelation::Exception(_)) && self.otherwise.is_some() {
            return Err("exception must inherit the parent's fallback".into());
        }
        Ok(())
    }
    /// Render the protected semantics directly; do not paraphrase them via an LLM.
    pub fn render(&self, id: &str) -> String {
        let mut s = format!(
            "Rule {id} from {}; scope {}; action {}. ",
            self.source,
            fields(&self.scope),
            self.action
        );
        match &self.relation {
            NormRelation::Root => {}
            NormRelation::Exception(p) => s.push_str(&format!(
                "Exception to {p}; other parent clauses remain effective. "
            )),
            NormRelation::Revision(p) => s.push_str(&format!("Replaces {p} in this scope. ")),
        }
        for c in &self.clauses {
            s.push_str(&format!(
                "{} when {}. ",
                c.modality.token(),
                fields(&c.when)
            ));
        }
        if let Some(m) = self.otherwise {
            s.push_str(&format!("Otherwise {}.", m.token()));
        }
        s.trim().into()
    }
    /// Exact typed semantic comparison, not a classifier for arbitrary prose.
    pub fn grounds(&self, candidate: &Self) -> bool {
        self == candidate
    }
}
#[derive(Clone, Debug, Default)]
pub struct NormativeMemory {
    records: BTreeMap<String, NormRecord>,
    /// Runtime trust configuration is never restored from a vault.
    authorities: BTreeSet<String>,
}
#[derive(Clone, Debug, Default)]
pub struct NormativeContext {
    pub text: String,
    pub trace_ids: Vec<String>,
    pub issues: Vec<String>,
}
impl NormativeContext {
    pub(crate) fn token_overflow() -> Self {
        Self {
            text:"Normative context incomplete: token budget exceeded; request clarification before acting.".into(),
            trace_ids:vec![],issues:vec!["token_budget_exceeded".into()],
        }
    }
    pub fn words(&self) -> usize {
        self.text.split_whitespace().count()
    }
    pub fn bounded(&self, budget: usize) -> Self {
        if self.words() <= budget {
            return self.clone();
        }
        let message =
            "Normative context incomplete: budget exceeded; request clarification before acting.";
        Self {
            text: if message.split_whitespace().count() <= budget {
                message.into()
            } else {
                String::new()
            },
            trace_ids: vec![],
            issues: vec!["budget_exceeded".into()],
        }
    }
    pub fn to_json(&self) -> String {
        let q = |s: &str| format!("\"{}\"", crate::net::httpx::json_esc(s));
        format!(
            "{{\"text\":{},\"trace_ids\":[{}],\"issues\":[{}]}}",
            q(&self.text),
            self.trace_ids
                .iter()
                .map(|s| q(s))
                .collect::<Vec<_>>()
                .join(","),
            self.issues
                .iter()
                .map(|s| q(s))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
impl NormativeMemory {
    pub fn set_authorities(&mut self, sources: &[&str]) -> Result<(), String> {
        if sources.len() > 64 || sources.iter().any(|s| !atom(s)) {
            return Err("invalid authority list".into());
        }
        self.authorities = sources.iter().map(|s| s.to_string()).collect();
        Ok(())
    }
    pub fn get(&self, id: &str) -> Option<&NormRecord> {
        self.records.get(id)
    }
    pub(crate) fn remove(&mut self, id: &str) {
        self.records.remove(id);
    }
    pub(crate) fn register(&mut self, id: &str, record: NormRecord) -> Result<(), String> {
        record.validate()?;
        if !atom(id) || !self.authorities.contains(&record.source) {
            return Err("source is not authorized by the application".into());
        }
        if self.records.contains_key(id) || self.records.len() >= 256 {
            return Err("anchor already exists or normative capacity reached".into());
        }
        if let Some(parent) = record.parent() {
            let p = self.records.get(parent).ok_or("parent anchor missing")?;
            if parent == id
                || p.source != record.source
                || p.scope != record.scope
                || p.action != record.action
            {
                return Err("parent authority, scope or action mismatch".into());
            }
        }
        self.records.insert(id.into(), record);
        Ok(())
    }
    fn live(&self, store: &MemoryStore, id: &str, r: &NormRecord) -> bool {
        self.authorities.contains(&r.source)
            && store.traces.get(id).is_some_and(|t| {
                !t.suppressed
                    && t.status == TraceStatus::Active
                    && t.access >= 0.15
                    && t.attribution == Attribution::External
                    && t.source == r.source
            })
    }
    /// Explicit scope selection. Complete rule families are delivered atomically.
    /// No selection by category, intended answer, or desired modality.
    pub fn compose(
        &self,
        store: &MemoryStore,
        scope: &NormScope,
        action: &str,
        budget: usize,
    ) -> Result<NormativeContext, String> {
        if !valid_scope(scope) || !atom(action) || budget < 32 {
            return Err("valid scope, action and at least 32 words required".into());
        }
        let candidates = self
            .records
            .iter()
            .filter(|(_, r)| {
                r.action == action
                    && !r
                        .scope
                        .iter()
                        .any(|(k, v)| scope.get(k).is_some_and(|q| q != v))
            })
            .collect::<Vec<_>>();
        let mut issues = BTreeSet::new();
        let mut ordered = Vec::new();
        let mut visited = BTreeSet::new();
        if candidates.is_empty() {
            issues.insert("no_applicable_norms".to_string());
        }
        for (id, r) in &candidates {
            if r.scope.keys().any(|k| !scope.contains_key(k)) {
                issues.insert("scope_incomplete".into());
            }
            let mut chain = Vec::new();
            let mut seen = BTreeSet::new();
            let mut next = Some(id.as_str());
            while let Some(key) = next {
                if !seen.insert(key.to_string()) {
                    issues.insert("dependency_cycle".into());
                    break;
                }
                let Some(record) = self.records.get(key) else {
                    issues.insert("dependency_missing".into());
                    break;
                };
                if !self.live(store, key, record) {
                    issues.insert("source_or_trace_unavailable".into());
                }
                if record.source != r.source || record.scope != r.scope || record.action != r.action
                {
                    issues.insert("dependency_mismatch".into());
                }
                chain.push(key.to_string());
                next = record.parent();
            }
            for key in chain.into_iter().rev() {
                if visited.insert(key.clone()) {
                    ordered.push(key);
                }
            }
        }
        if candidates
            .iter()
            .filter(|(_, r)| r.parent().is_none())
            .count()
            > 1
        {
            issues.insert("multiple_roots_review_required".into());
        }
        let mut parents = BTreeSet::new();
        for (_, r) in &candidates {
            if let NormRelation::Revision(p) = &r.relation {
                if !parents.insert(p) {
                    issues.insert("competing_revisions".into());
                }
            }
        }
        let overlap =
            |a: &NormScope, b: &NormScope| !a.iter().any(|(k, v)| b.get(k).is_some_and(|w| w != v));
        for (i, (_, a)) in candidates.iter().enumerate() {
            for (j, c) in a.clauses.iter().enumerate() {
                if a.clauses
                    .iter()
                    .skip(j + 1)
                    .any(|d| c.modality != d.modality && overlap(&c.when, &d.when))
                {
                    issues.insert("conflicting_clauses".into());
                }
            }
            for (_, b) in candidates.iter().skip(i + 1) {
                if a.parent() == b.parent()
                    && a.parent().is_some()
                    && a.clauses.iter().any(|c| {
                        b.clauses
                            .iter()
                            .any(|d| c.modality != d.modality && overlap(&c.when, &d.when))
                    })
                {
                    issues.insert("conflicting_siblings".into());
                }
            }
        }
        let unavailable = issues.iter().any(|s| {
            matches!(
                s.as_str(),
                "scope_incomplete"
                    | "dependency_cycle"
                    | "dependency_missing"
                    | "source_or_trace_unavailable"
                    | "dependency_mismatch"
            )
        });
        let mut text = String::new();
        if !issues.is_empty() {
            text.push_str(&format!(
                "Normative review required: {}. Do not infer permission; request clarification.\n",
                issues.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if unavailable {
            ordered.clear();
        } else {
            for id in &ordered {
                text.push_str(&self.records[id].render(id));
                text.push('\n');
            }
        }
        Ok(NormativeContext {
            text: text.trim().into(),
            trace_ids: ordered,
            issues: issues.into_iter().collect(),
        }
        .bounded(budget))
    }
    pub(crate) fn pack(&self) -> String {
        let mut t = vec!["v1".into(), self.records.len().to_string()];
        fn scope(t: &mut Vec<String>, s: &NormScope) {
            t.push(s.len().to_string());
            for (k, v) in s {
                t.push(k.clone());
                t.push(v.clone());
            }
        }
        for (id, r) in &self.records {
            t.extend([id.clone(), r.source.clone(), r.action.clone()]);
            t.push(
                match r.relation {
                    NormRelation::Root => "root",
                    NormRelation::Exception(_) => "exception",
                    NormRelation::Revision(_) => "revision",
                }
                .into(),
            );
            t.push(r.parent().unwrap_or("-").into());
            scope(&mut t, &r.scope);
            t.push(r.clauses.len().to_string());
            for c in &r.clauses {
                t.push(c.modality.token().into());
                scope(&mut t, &c.when);
            }
            t.push(r.otherwise.map(Modality::token).unwrap_or("-").into());
        }
        t.join(" ")
    }
    pub(crate) fn unpack(s: &str) -> Result<Self, String> {
        let mut t = s.split_whitespace();
        if take(&mut t)? != "v1" {
            return Err("unknown normative version".into());
        }
        let mut out = Self::default();
        for _ in 0..count(&mut t, 256)? {
            let id = take(&mut t)?.to_string();
            let source = take(&mut t)?.into();
            let action = take(&mut t)?.into();
            let kind = take(&mut t)?;
            let parent = take(&mut t)?;
            let relation = match kind {
                "root" if parent == "-" => NormRelation::Root,
                "exception" => NormRelation::Exception(parent.into()),
                "revision" => NormRelation::Revision(parent.into()),
                _ => return Err("invalid relation".into()),
            };
            let scope = read_scope(&mut t)?;
            let mut clauses = Vec::new();
            for _ in 0..count(&mut t, 16)? {
                clauses.push(NormClause {
                    modality: Modality::parse(take(&mut t)?)?,
                    when: read_scope(&mut t)?,
                });
            }
            let otherwise = match take(&mut t)? {
                "-" => None,
                s => Some(Modality::parse(s)?),
            };
            let r = NormRecord {
                source,
                scope,
                action,
                clauses,
                otherwise,
                relation,
            };
            r.validate()?;
            if !atom(&id) || out.records.insert(id, r).is_some() {
                return Err("invalid or duplicate normative id".into());
            }
        }
        if t.next().is_some() {
            return Err("trailing normative data".into());
        }
        Ok(out)
    }
}
fn take<'a>(t: &mut impl Iterator<Item = &'a str>) -> Result<&'a str, String> {
    t.next().ok_or_else(|| "truncated normative data".into())
}
fn count<'a>(t: &mut impl Iterator<Item = &'a str>, max: usize) -> Result<usize, String> {
    let n: usize = take(t)?.parse().map_err(|_| "invalid count")?;
    if n > max {
        return Err("normative count limit".into());
    }
    Ok(n)
}
fn read_scope<'a>(t: &mut impl Iterator<Item = &'a str>) -> Result<NormScope, String> {
    let n = count(t, 8)?;
    let mut s = NormScope::new();
    for _ in 0..n {
        let k = take(t)?.to_string();
        if s.insert(k, take(t)?.into()).is_some() {
            return Err("duplicate scope key".into());
        }
    }
    Ok(s)
}
