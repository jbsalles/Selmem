use crate::fixtures::{Episode, NIGHTS};
use selmem::encode::semantic::{LexicalInterpreter, SemanticInterpreter};
use selmem::recall::retrieve::{RecallBias, RecallWrite};
use selmem::{EncodeInput, EntityProfile, SelectiveMemory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tiktoken_rs::CoreBPE;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    Selmem,
    Bm25Snippets,
}

pub const ARMS: [Arm; 2] = [Arm::Selmem, Arm::Bm25Snippets];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Packet {
    pub memory: String,
    pub tokens: usize,
    pub sources: Vec<String>,
    pub intact_before_packing: Vec<String>,
    pub intact_trials: Vec<String>,
}

#[derive(Clone)]
struct Passage {
    source: String,
    text: String,
}

fn compact(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn count(bpe: &CoreBPE, text: &str) -> usize {
    bpe.encode_ordinary(text).len()
}

fn framed(text: &str) -> String {
    format!("<entry>\n{}\n</entry>\n", text.trim())
}

fn evidence_ids<'a>(events: &'a [Episode], text: &str) -> impl Iterator<Item = &'a str> {
    let text = compact(text);
    events.iter().filter_map(move |event| {
        event
            .action_and_result
            .as_ref()
            .filter(|evidence| text.contains(*evidence))
            .map(|_| event.id.as_str())
    })
}

pub fn trial_report_tokens(bpe: &CoreBPE, events: &[Episode]) -> usize {
    let reports: String = events
        .iter()
        .filter(|event| event.action_and_result.is_some())
        .map(|event| framed(&event.report))
        .collect();
    count(bpe, &reports)
}

fn pack(bpe: &CoreBPE, ranked: &[Passage], events: &[Episode], capacity: usize) -> Packet {
    let mut memory = String::new();
    let mut sources = Vec::new();
    let mut intact = BTreeSet::new();
    for passage in ranked {
        if sources.contains(&passage.source) || passage.text.trim().is_empty() {
            continue;
        }
        let candidate = format!("{memory}{}", framed(&passage.text));
        if count(bpe, &candidate) > capacity {
            continue;
        }
        memory = candidate;
        sources.push(passage.source.clone());
        intact.extend(evidence_ids(events, &passage.text).map(str::to_owned));
    }
    Packet {
        tokens: count(bpe, &memory),
        memory,
        sources,
        intact_before_packing: ranked
            .iter()
            .flat_map(|passage| evidence_ids(events, &passage.text))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        intact_trials: intact.into_iter().collect(),
    }
}

fn terms(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|w| {
            w.len() > 2
                && !matches!(
                    w.as_str(),
                    "the"
                        | "and"
                        | "with"
                        | "from"
                        | "for"
                        | "into"
                        | "each"
                        | "use"
                        | "said"
                        | "was"
                        | "were"
                        | "had"
                        | "that"
                        | "this"
                        | "which"
                )
        })
}

struct Snippets {
    passages: Vec<Passage>,
    lengths: Vec<usize>,
    postings: BTreeMap<String, Vec<(usize, usize)>>,
    average_length: f64,
}

impl Snippets {
    fn new(events: &[Episode]) -> Self {
        let mut passages = Vec::new();
        let mut lengths = Vec::new();
        let mut postings = BTreeMap::<String, Vec<(usize, usize)>>::new();
        for event in events {
            let notes: Vec<_> = event
                .notes
                .split_inclusive(['.', '?', '!', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            // The short report is visible to both methods as a single unit.
            let snippets =
                std::iter::once(event.report.clone()).chain((0..notes.len()).flat_map(|start| {
                    let notes = &notes;
                    (start + 1..=(start + 3).min(notes.len()))
                        .map(move |end| notes[start..end].join(" "))
                }));
            for text in snippets {
                let mut frequencies = BTreeMap::new();
                for term in terms(&text) {
                    *frequencies.entry(term).or_insert(0) += 1;
                }
                let index = passages.len();
                lengths.push(frequencies.values().sum());
                for (term, frequency) in frequencies {
                    postings.entry(term).or_default().push((index, frequency));
                }
                passages.push(Passage {
                    source: event.id.clone(),
                    text,
                });
            }
        }
        let average_length = lengths.iter().sum::<usize>() as f64 / passages.len().max(1) as f64;
        Self {
            passages,
            lengths,
            postings,
            average_length: average_length.max(1.0),
        }
    }

    fn ranked(&self, query: &str) -> Vec<Passage> {
        let mut scores = vec![0.0; self.passages.len()];
        for term in terms(query).collect::<BTreeSet<_>>() {
            if let Some(hits) = self.postings.get(&term) {
                let df = hits.len() as f64;
                let idf = (1.0 + (scores.len() as f64 - df + 0.5) / (df + 0.5)).ln();
                for &(index, frequency) in hits {
                    let tf = frequency as f64;
                    let norm =
                        1.2 * (0.25 + 0.75 * self.lengths[index] as f64 / self.average_length);
                    scores[index] += idf * tf * 2.2 / (tf + norm);
                }
            }
        }
        let mut indices: Vec<_> = (0..scores.len())
            .filter(|&index| scores[index] > 0.0)
            .collect();
        indices.sort_by(|&left, &right| {
            scores[right]
                .total_cmp(&scores[left])
                .then(self.lengths[left].cmp(&self.lengths[right]))
                .then(left.cmp(&right))
        });
        indices
            .into_iter()
            .map(|i| self.passages[i].clone())
            .collect()
    }
}

#[derive(Serialize)]
pub struct TrialAdmission {
    pub id: String,
    pub kept: bool,
    pub score: f32,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ReplayAudit {
    pub episodes: usize,
    pub admitted: usize,
    pub trial_admissions: Vec<TrialAdmission>,
    pub retained_trial_cores: Vec<String>,
}

pub struct Replay<'a> {
    book: SelectiveMemory,
    snippets: Snippets,
    events: &'a [Episode],
    pub audit: ReplayAudit,
}

impl<'a> Replay<'a> {
    pub fn new(events: &'a [Episode]) -> crate::Result<Self> {
        let mut profile = EntityProfile::new("atelier");
        profile.max_recall = 16;
        let mut book = SelectiveMemory::new(profile).detach_clock();
        let mut admitted = 0;
        let mut trial_admissions = Vec::new();
        for day in 0..=NIGHTS {
            if day > 0 {
                book.advance_hours(24.0);
            }
            for event in events.iter().filter(|event| event.day == day) {
                let text = event.text();
                let mut semantics = LexicalInterpreter.interpret_event(&text)?;
                semantics.core.claim = event.report.clone();
                let mut input = EncodeInput::observed(&text, &event.id);
                input.source = &event.id;
                input.utility = 0.65;
                input.semantics = Some(semantics);
                let decision = book.live_with(input);
                admitted += usize::from(decision.kept);
                if event.action_and_result.is_some() {
                    trial_admissions.push(TrialAdmission {
                        id: event.id.clone(),
                        kept: decision.kept,
                        score: decision.score,
                        reason: decision.reason,
                    });
                }
            }
            if day > 0 {
                book.sleep();
            }
        }
        let retained_trial_cores = book
            .store
            .traces
            .values()
            .flat_map(|trace| evidence_ids(events, &trace.core))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(Self {
            book,
            events,
            snippets: Snippets::new(events),
            audit: ReplayAudit {
                episodes: events.len(),
                admitted,
                trial_admissions,
                retained_trial_cores,
            },
        })
    }

    pub fn packets(
        &mut self,
        bpe: &CoreBPE,
        commission: &str,
        budgets: &[usize],
    ) -> Vec<(usize, Arm, Packet)> {
        let query =
            format!("First glance at a target in an otherwise regular display. {commission}");
        let (recalls, _) =
            self.book
                .remember_with(&query, RecallWrite::ReadOnly, RecallBias::Observed, &[]);
        let selmem: Vec<_> = recalls
            .into_iter()
            .map(|recall| Passage {
                source: self.book.store.traces[&recall.trace_id].source.clone(),
                text: recall.narrative,
            })
            .collect();
        let bm25 = self.snippets.ranked(&query);
        let mut packets = Vec::new();
        for &budget in budgets {
            for (arm, ranked) in [(Arm::Selmem, &selmem), (Arm::Bm25Snippets, &bm25)] {
                packets.push((budget, arm, pack(bpe, ranked, self.events, budget)));
            }
        }
        packets
    }
}
