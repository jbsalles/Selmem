use crate::{fixture, Result};
use fixture::{Device, Episode, Observation};
use selmem::{EncodeInput, EntityProfile, RecallBias, RecallWrite, SelectiveMemory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tiktoken_rs::CoreBPE;

#[derive(Deserialize, Serialize)]
pub struct Readout {
    pub episode_id: String,
    pub trace_id: String,
    pub narrative: String,
    pub observation: Option<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained: Option<RetainedReadout>,
}

#[derive(Deserialize, Serialize)]
pub struct RetainedReadout {
    pub trace_id: String,
    pub narrative: String,
}

#[derive(Deserialize, Serialize)]
pub struct History {
    pub outcome: Device,
    pub episodes: Vec<Episode>,
    pub panel: Vec<Readout>,
}

pub fn replay(outcome: Device, keep_excerpt: bool) -> Result<History> {
    let episodes = fixture::history(outcome, keep_excerpt);
    let mut panel = replay_panel(&episodes, false)?;
    if keep_excerpt {
        for (base, retained) in panel.iter_mut().zip(replay_panel(&episodes, true)?) {
            base.retained = Some(RetainedReadout {
                trace_id: retained.trace_id,
                narrative: retained.narrative,
            });
        }
    }
    Ok(History {
        outcome,
        episodes,
        panel,
    })
}

fn replay_panel(episodes: &[Episode], keep_excerpt: bool) -> Result<Vec<Readout>> {
    let mut profile = EntityProfile::new("printmaker");
    profile.max_recall = 1;
    let mut organ = SelectiveMemory::new(profile).detach_clock();
    let mut intake = BTreeMap::new();
    for (i, event) in episodes.iter().enumerate() {
        let mut input = EncodeInput::observed(&event.raw, &event.id);
        input.source = &event.id;
        if event.excerpt.is_some() {
            // Trial utility is shared by both histories and both encoding paths.
            input.utility = 0.65;
        }
        let decision = match event.excerpt.as_deref().filter(|_| keep_excerpt) {
            Some(excerpt) => organ.live_with_observed_excerpt(input, excerpt)?,
            None => organ.live_with(input),
        };
        if event.panel {
            let id = decision.trace_id.ok_or_else(|| {
                format!(
                    "{} was not admitted by SelMem: {}",
                    event.id, decision.reason
                )
            })?;
            if decision.parts != 1 || decision.kept_n != 1 {
                return Err(format!("{} did not encode as a single trace", event.id).into());
            }
            intake.insert(&event.id, id);
        }
        if i % 2 == 1 {
            organ.advance_hours(24.);
            organ.sleep();
        }
    }

    // The fixed panel isolates delivery. Read the same surviving episode IDs
    // through the native narrator even when a brief would not retrieve them.
    let mut panel = Vec::new();
    for event in episodes.iter().filter(|e| e.panel) {
        let trace_id = intake.remove(&event.id).unwrap();
        let (recalled, _) = organ.remember_with(
            "printmaking workshop",
            RecallWrite::ReadOnly,
            RecallBias::ForceMarked,
            std::slice::from_ref(&trace_id),
        );
        let reading = recalled
            .into_iter()
            .find(|r| r.trace_id == trace_id && !r.narrative.trim().is_empty())
            .ok_or_else(|| format!("{} did not survive for both readouts", event.id))?;
        if keep_excerpt
            && event
                .excerpt
                .as_ref()
                .is_some_and(|text| text != &reading.narrative)
        {
            return Err(format!("{} lost its retained observation", event.id).into());
        }
        panel.push(Readout {
            episode_id: event.id.clone(),
            trace_id,
            narrative: reading.narrative,
            observation: event.observation.clone(),
            retained: None,
        });
    }
    Ok(panel)
}

pub fn tokens(bpe: &CoreBPE, text: &str) -> usize {
    bpe.encode_ordinary(text).len()
}

fn append_entry(base: &str, text: &str, padding: &str) -> String {
    format!("{base}<entry>\n{}\n{padding}</entry>\n", text.trim())
}

pub struct MatchedPanels {
    // Order within each history is native, then observed action and result.
    pub text: [[String; 2]; 2],
    pub tokens: usize,
    pub ends: Vec<usize>,
}

pub fn match_panels(
    bpe: &CoreBPE,
    histories: &[History; 2],
    keep_excerpt: bool,
) -> Result<MatchedPanels> {
    if histories[0]
        .panel
        .iter()
        .map(|entry| &entry.episode_id)
        .ne(histories[1].panel.iter().map(|entry| &entry.episode_id))
    {
        return Err("history panels must contain the same episodes in order".into());
    }
    let mut blocks: [String; 4] = Default::default();
    let mut ends = Vec::new();
    for (left, right) in histories[0].panel.iter().zip(&histories[1].panel) {
        let treatment = |entry: &Readout| -> Result<String> {
            if keep_excerpt {
                Ok(entry
                    .retained
                    .as_ref()
                    .ok_or_else(|| format!("Missing retained reading for {}", entry.episode_id))?
                    .narrative
                    .clone())
            } else {
                Ok(entry
                    .observation
                    .as_ref()
                    .map_or_else(|| entry.narrative.clone(), Observation::render))
            }
        };
        let payloads = [
            left.narrative.clone(),
            treatment(left)?,
            right.narrative.clone(),
            treatment(right)?,
        ];
        let plain: [String; 4] =
            std::array::from_fn(|i| append_entry(&blocks[i], &payloads[i], ""));
        let counts = plain.each_ref().map(|s| tokens(bpe, s));
        let target = counts.into_iter().max().unwrap();
        for (i, text) in plain.into_iter().enumerate() {
            blocks[i] = if counts[i] == target {
                text
            } else {
                // Match each entry boundary with whitespace. Keep whole native
                // passages and avoid adding words to either readout.
                (1..=(target - counts[i] + 12) * 4)
                    .map(|n| append_entry(&blocks[i], &payloads[i], &" \t".repeat(n)))
                    .find(|text| tokens(bpe, text) == target)
                    .ok_or("could not match readout token boundaries with whitespace")?
            };
        }
        ends.push(target);
    }
    let [a, b, c, d] = blocks;
    let budget = *ends.last().ok_or("empty fixed panel")?;
    Ok(MatchedPanels {
        text: [[a, b], [c, d]],
        tokens: budget,
        ends,
    })
}

pub fn text_id(text: &str) -> String {
    selmem::util::sha256::hex(text.as_bytes())
}

pub fn embedding_inputs<'a>(
    histories: &'a [History; 2],
    briefs: &'a [String],
) -> BTreeMap<String, &'a str> {
    histories
        .iter()
        .flat_map(|h| h.episodes.iter().map(|e| e.raw.as_str()))
        .chain(briefs.iter().map(String::as_str))
        .map(|text| (text_id(text), text))
        .collect()
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn bm25(query: &str, docs: &[Episode]) -> Vec<f32> {
    let terms = words(query).into_iter().collect::<BTreeSet<_>>();
    let documents = docs.iter().map(|e| words(&e.raw)).collect::<Vec<_>>();
    let mean_length = documents.iter().map(Vec::len).sum::<usize>() as f32 / docs.len() as f32;
    let mut scores = vec![0.; docs.len()];
    for term in terms {
        let df = documents.iter().filter(|doc| doc.contains(&term)).count() as f32;
        let idf = (1. + (docs.len() as f32 - df + 0.5) / (df + 0.5)).ln();
        for (doc, score) in documents.iter().zip(&mut scores) {
            let count = doc.iter().filter(|w| **w == term).count() as f32;
            let length_penalty = 0.25 + 0.75 * doc.len() as f32 / mean_length;
            *score += idf * count * 2.2 / (count + 1.2 * length_penalty);
        }
    }
    scores
}

fn rank(scores: &[f32]) -> Vec<usize> {
    let mut ranked = (0..scores.len()).collect::<Vec<_>>();
    ranked.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
    ranked
}

pub fn rag_readout(
    bpe: &CoreBPE,
    docs: &[Episode],
    brief: &str,
    embeddings: Option<&BTreeMap<String, Vec<f32>>>,
    budget: usize,
) -> Result<(String, Vec<String>)> {
    let lexical = rank(&bm25(brief, docs));
    let ranked = if let Some(embeddings) = embeddings {
        let query = &embeddings[&text_id(brief)];
        let semantic = docs
            .iter()
            .map(|e| selmem::encode::embed::cosine(query, &embeddings[&text_id(&e.raw)]))
            .collect::<Vec<_>>();
        let mut fused = vec![0.; docs.len()];
        for ordering in [lexical, rank(&semantic)] {
            for (place, i) in ordering.into_iter().enumerate() {
                fused[i] += 1. / (61. + place as f32);
            }
        }
        rank(&fused)
    } else {
        lexical
    };
    let mut result = String::new();
    let mut ids = Vec::new();
    for i in ranked {
        let candidate = append_entry(&result, &docs[i].raw, "");
        if tokens(bpe, &candidate) <= budget {
            result = candidate;
            ids.push(docs[i].id.clone());
        }
    }
    if ids.is_empty() {
        return Err("no complete RAG passage fits the matched budget".into());
    }
    Ok((result, ids))
}
