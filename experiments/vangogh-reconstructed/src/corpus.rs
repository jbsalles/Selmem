use crate::Result;
use roxmltree::{Document, Node};
use selmem::{EncodeInput, EntityProfile, Narrator, RuleNarrator, SelectiveMemory, TraceStatus};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub const PERIODS: [&str; 2] = ["Nuenen", "Arles"];
pub const PASSAGES: usize = 768;

#[derive(Clone, Serialize, Deserialize)]
pub struct Passage {
    pub id: String,
    pub date: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: String,
    pub sources: BTreeSet<String>,
    pub text: String,
    pub gist: String,
    pub core: String,
    pub drift_count: usize,
    pub revisions: Vec<Revision>,
    pub fidelity: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Revision {
    kind: String,
    before: String,
    after: String,
    source_traces: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct History {
    pub passages: Vec<Passage>,
    pub traces: Vec<Trace>,
    pub admitted: usize,
    pub surviving_sources: usize,
    pub admitted_without_surviving_lineage: usize,
    pub missing_provenance: usize,
}

fn append_text(node: Node<'_, '_>, out: &mut String) {
    if node.is_text() {
        out.push_str(node.text().unwrap_or(""));
        return;
    }
    match node.tag_name().name() {
        "note" | "figure" | "figDesc" | "ptr" | "del" | "fw" => return,
        "lb" | "pb" => out.push(' '),
        "choice" => {
            if let Some(child) = node
                .children()
                .find(|n| n.has_tag_name("corr") || n.has_tag_name("reg"))
                .or_else(|| node.children().find(Node::is_element))
            {
                append_text(child, out);
            }
            return;
        }
        _ => {}
    }
    for child in node.children() {
        append_text(child, out);
    }
}

pub fn load(dir: &Path) -> Result<[Vec<Passage>; 2]> {
    let mut periods = [Vec::new(), Vec::new()];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !stem.starts_with("let") || path.extension().and_then(|s| s.to_str()) != Some("xml") {
            continue;
        }
        let xml = fs::read_to_string(&path)?;
        let doc = Document::parse(&xml)?;
        let Some(sent) = doc
            .descendants()
            .find(|n| n.has_tag_name("correspAction") && n.attribute("type") == Some("sent"))
        else {
            continue;
        };
        let field = |name| sent.children().find(|n| n.has_tag_name(name));
        if field("rs").and_then(|n| n.text()) != Some("Vincent van Gogh") {
            continue;
        }
        let Some(date) = field("date")
            .and_then(|n| n.attribute("when"))
            .filter(|s| s.len() == 10)
        else {
            continue;
        };
        let place = field("placeName").and_then(|n| n.text());
        let period = match place {
            Some("Nuenen") if ("1883-12-01"..="1885-11-30").contains(&date) => 0,
            Some("Arles") if ("1888-02-01"..="1889-05-31").contains(&date) => 1,
            _ => continue,
        };
        let Some(body) = doc.descendants().find(|n| {
            n.has_tag_name("div")
                && n.attribute("type") == Some("translation")
                && n.attribute(("http://www.w3.org/XML/1998/namespace", "lang")) == Some("en")
        }) else {
            continue;
        };
        for (index, paragraph) in body
            .descendants()
            .filter(|n| {
                n.has_tag_name("p")
                    && !n
                        .ancestors()
                        .skip(1)
                        .any(|a| ["p", "note", "figure"].contains(&a.tag_name().name()))
            })
            .enumerate()
        {
            let mut text = String::new();
            append_text(paragraph, &mut text);
            let text = text.split_whitespace().collect::<Vec<_>>();
            if !(12..=76).contains(&text.len()) {
                continue;
            }
            let text = text
                .join(" ")
                .replace("Vincent van Gogh", "the painter")
                .replace("Nuenen", "this town")
                .replace("Arles", "this town");
            periods[period].push(Passage {
                id: format!("{stem}-{index:04}"),
                date: date.into(),
                text,
            });
        }
    }
    for (index, passages) in periods.iter_mut().enumerate() {
        passages.sort_by(|a, b| (&a.date, &a.id).cmp(&(&b.date, &b.id)));
        let len = passages.len();
        if len < PASSAGES {
            return Err(format!(
                "{}: found {len} of {PASSAGES} required passages",
                PERIODS[index]
            )
            .into());
        }
        *passages = std::mem::take(passages)
            .into_iter()
            .enumerate()
            .filter_map(|(i, p)| (((i + 1) * PASSAGES / len) > (i * PASSAGES / len)).then_some(p))
            .collect();
    }
    Ok(periods)
}

pub fn replay(passages: Vec<Passage>, period: &str) -> History {
    let mut memory = SelectiveMemory::new(EntityProfile::new("artist")).detach_clock();
    let mut sources: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut operations = BTreeMap::<String, BTreeSet<String>>::new();
    let mut admitted = BTreeSet::new();
    for (i, passage) in passages.iter().enumerate() {
        let mut input = EncodeInput::new(&passage.text);
        input.source = &passage.id;
        if memory.live_with(input).kept {
            admitted.insert(passage.id.clone());
        }
        if i % 5 == 4 || i + 1 == passages.len() {
            // Snapshot origins before any merge can erase a trace.
            for trace in memory.store.traces.values() {
                operations.entry(trace.id.clone()).or_default().extend(
                    trace
                        .operations
                        .iter()
                        .flat_map(|op| op.source_trace_ids.iter().cloned()),
                );
                if trace.source.starts_with("let") {
                    sources
                        .entry(trace.id.clone())
                        .or_default()
                        .insert(trace.source.clone());
                }
            }
            memory.advance_hours(24.);
            if (i + 1) % 40 == 0 || i + 1 == passages.len() {
                memory.sleep_deep();
            } else {
                memory.sleep();
            }
        }
        if (i + 1) % 128 == 0 {
            eprintln!("{period}: replayed {}/{} passages", i + 1, passages.len());
        }
    }
    let known: BTreeSet<_> = passages.iter().map(|p| p.id.clone()).collect();
    let mut eligible: Vec<_> = memory
        .store
        .traces
        .values()
        .filter(|t| {
            !t.suppressed
                && !t.channel.verbatim()
                && t.status != TraceStatus::Latent
                && t.access > 0.
        })
        .collect();
    // Each surviving trace keeps the lineage it declares through recorded operations.
    for trace in &eligible {
        operations.entry(trace.id.clone()).or_default().extend(
            trace
                .operations
                .iter()
                .flat_map(|op| op.source_trace_ids.iter().cloned()),
        );
    }
    let order: BTreeMap<_, _> = passages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    eligible.sort_by(|a, b| {
        order
            .get(a.source.as_str())
            .cmp(&order.get(b.source.as_str()))
            .then(a.core.cmp(&b.core))
            .then(a.gist.cmp(&b.gist))
    });
    let mut traces = Vec::new();
    let mut missing_provenance = 0;
    for trace in eligible {
        let mut lineage = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending = vec![trace.id.as_str()];
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(origins) = sources.get(id) {
                lineage.extend(origins.intersection(&known).cloned());
            }
            if let Some(parents) = operations.get(id) {
                pending.extend(parents.iter().map(String::as_str));
            }
        }
        if lineage.is_empty() {
            missing_provenance += 1;
            continue;
        }
        let text = RuleNarrator.reconstruct(trace, &Default::default(), "");
        if text.trim().is_empty() {
            continue;
        }
        traces.push(Trace {
            id: trace.id.clone(),
            sources: lineage,
            text,
            gist: trace.gist.clone(),
            core: trace.core.clone(),
            drift_count: trace.drifts.len(),
            revisions: trace
                .operations
                .iter()
                .map(|op| Revision {
                    kind: op.kind.clone(),
                    before: op.before.clone(),
                    after: op.after.clone(),
                    source_traces: op.source_trace_ids.clone(),
                })
                .collect(),
            fidelity: trace.fidelity,
        });
    }
    let survivors: BTreeSet<_> = traces
        .iter()
        .flat_map(|t| t.sources.iter().cloned())
        .collect();
    let admitted_without_surviving_lineage = admitted.difference(&survivors).count();
    History {
        passages,
        traces,
        admitted: admitted.len(),
        surviving_sources: survivors.len(),
        admitted_without_surviving_lineage,
        missing_provenance,
    }
}
