//! SelMem vs RAG vs summary vs raw window.
//! Bench profiles lower τ so retention is measurable. Not the shipping defaults.
use selmem::encode::scoring::lexical_similarity;
use selmem::{
    cosine, fingerprint, singularity_distance, Channel, EncodeInput, EntityProfile, HashEmbedder,
    SelectiveMemory, Embedder,
};
use std::collections::VecDeque;
use std::time::Instant;

const NS: &[usize] = &[1_000];
const NIGHTS: &[u32] = &[1, 4, 8, 16];
const PLACES: &[&str] = &[
    "the station", "the platform", "the letter", "the roof", "the workshop", "the bridge", "the kitchen",
    "the garden", "the waiting room", "the car park", "the library", "the balcony",
    "the hospital", "the market", "the courtyard", "the office", "the beach", "the basement",
    "the terrace", "the hall",
];

fn bench_tender() -> EntityProfile {
    let mut p = EntityProfile::tender("Claire");
    p.encode_threshold = 0.36;
    p.w_self = 0.25;
    p.w_redundancy = 0.12;
    p
}

fn bench_austere() -> EntityProfile {
    let mut p = EntityProfile::austere("Silas");
    p.encode_threshold = 0.36;
    p.w_self = 0.25;
    p.w_redundancy = 0.12;
    p
}
const RAW_K: usize = 8;
const RAG_K: usize = 4;
const SUM_CAP: usize = 400;

#[derive(Clone)]
struct Ev {
    text: String,
    valence: f32,
    arousal: f32,
    disgust: f32,
    self_relevance: f32,
    utility: f32,
    permanence: f32,
    schema: Option<String>,
    channel: Channel,
}

fn stream(n: usize) -> Vec<Ev> {
    let world_a = Ev {
        text: "The appointment is Tuesday at 10, room B.".into(),
        valence: 0.0,
        arousal: 0.1,
        disgust: 0.0,
        self_relevance: 0.1,
        utility: 0.95,
        permanence: 0.9,
        schema: Some("agenda".into()),
        channel: Channel::World,
    };
    let world_b = Ev {
        text: "Flight 442 at 18:40.".into(),
        valence: 0.0,
        arousal: 0.1,
        disgust: 0.0,
        self_relevance: 0.1,
        utility: 0.95,
        permanence: 0.9,
        schema: Some("agenda".into()),
        channel: Channel::World,
    };
    let early = Ev {
        text: "You stayed. Rain on the window, and you did not look for an excuse to leave.".into(),
        valence: 0.72,
        arousal: 0.55,
        disgust: 0.0,
        self_relevance: 0.9,
        utility: 0.4,
        permanence: 0.4,
        schema: Some("loyalty".into()),
        channel: Channel::Selfhood,
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i == 10 {
            out.push(clone_ev(&world_a));
            continue;
        }
        if i == 15 {
            out.push(clone_ev(&early));
            continue;
        }
        if n > 50 && i == n / 2 {
            out.push(clone_ev(&world_b));
            continue;
        }
        let place = PLACES[i % PLACES.len()];
        let kind = i % 7;
        let text = match kind {
            0 => format!("A throwaway remark about the weather, day {i}, near {place}."),
            1 => format!("You cancelled dinner at {place}, evening {i}, without warning."),
            2 => format!("We walked to {place} and barely spoke, evening {i}."),
            3 => format!("You repeated at {place} what I had told you in confidence, time {i}."),
            4 => format!("You stayed under the awning at {place} with me, evening {i}."),
            5 => format!("Useful note: file {i} to drop off at {place}."),
            _ => format!("Market, keys, nothing else, day {i}."),
        };
        let dull = kind == 0 || kind == 6;
        out.push(Ev {
            text,
            valence: if kind == 1 || kind == 3 { -0.55 } else if dull { 0.05 } else { 0.4 },
            arousal: if dull { 0.08 } else { 0.5 },
            disgust: if kind == 3 { 0.45 } else if kind == 1 { 0.25 } else { 0.0 },
            self_relevance: if dull { 0.08 } else { 0.8 },
            utility: if kind == 5 { 0.6 } else { 0.2 },
            permanence: 0.0,
            schema: Some(
                match kind {
                    1 => "abandon",
                    3 => "humiliation",
                    2 | 4 => "loyalty",
                    5 => "task",
                    _ => "daily",
                }
                .into(),
            ),
            channel: Channel::Selfhood,
        });
    }
    out
}

fn clone_ev(e: &Ev) -> Ev {
    Ev {
        text: e.text.clone(),
        valence: e.valence,
        arousal: e.arousal,
        disgust: e.disgust,
        self_relevance: e.self_relevance,
        utility: e.utility,
        permanence: e.permanence,
        schema: e.schema.clone(),
        channel: e.channel,
    }
}

fn hit(text: &str, needles: &[&str]) -> bool {
    let t = text.to_lowercase();
    needles.iter().any(|n| t.contains(*n))
}

fn contains_wrong_world(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("wednesday") || t.contains("11:00") || t.contains("room a") || t.contains("flight 441")
}

struct Raw {
    win: VecDeque<String>,
}
impl Raw {
    fn new() -> Self {
        Self {
            win: VecDeque::new(),
        }
    }
    fn ingest(&mut self, e: &Ev) {
        self.win.push_back(e.text.clone());
        while self.win.len() > RAW_K {
            self.win.pop_front();
        }
    }
    fn recall(&self, q: &str) -> String {
        let qt: Vec<_> = q.split_whitespace().collect();
        let mut hit: Vec<&String> = self
            .win
            .iter()
            .filter(|s| qt.iter().any(|w| s.to_lowercase().contains(&w.to_lowercase())))
            .collect();
        if hit.is_empty() {
            hit = self.win.iter().collect();
        }
        hit.into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

struct Rag {
    docs: Vec<(String, Vec<f32>)>,
    emb: HashEmbedder,
}
impl Rag {
    fn new() -> Self {
        Self {
            docs: Vec::new(),
            emb: HashEmbedder,
        }
    }
    fn ingest(&mut self, e: &Ev) {
        self.docs.push((e.text.clone(), self.emb.embed(&e.text)));
    }
    fn recall(&self, q: &str) -> String {
        let qe = self.emb.embed(q);
        let mut scored: Vec<(f32, &str)> = self
            .docs
            .iter()
            .map(|(t, e)| (cosine(e, &qe), t.as_str()))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored
            .into_iter()
            .take(RAG_K)
            .map(|(_, t)| t.to_string())
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

struct Summary {
    buf: String,
}
impl Summary {
    fn new() -> Self {
        Self { buf: String::new() }
    }
    fn ingest(&mut self, e: &Ev) {
        if e.channel == Channel::World || e.self_relevance >= 0.5 {
            if !self.buf.is_empty() {
                self.buf.push(' ');
            }
            self.buf.push_str(&e.text);
            if self.buf.len() > SUM_CAP {
                let keep = self.buf.split_whitespace().rev().take(40).collect::<Vec<_>>();
                self.buf = keep.into_iter().rev().collect::<Vec<_>>().join(" ");
            }
        }
    }
    fn recall(&self, q: &str) -> String {
        if self.buf.to_lowercase().split_whitespace().any(|w| q.to_lowercase().contains(w) && w.len() > 3)
        {
            self.buf.clone()
        } else {
            self.buf.clone()
        }
    }
}

fn feed_selmem(profile: EntityProfile, events: &[Ev], nights: u32) -> SelectiveMemory {
    let mut mem = SelectiveMemory::new(profile);
    for e in events {
        let mut ev = EncodeInput::new(&e.text);
        ev.valence = e.valence;
        ev.arousal = e.arousal;
        ev.disgust = e.disgust;
        ev.self_relevance = e.self_relevance;
        ev.utility = e.utility;
        ev.permanence = e.permanence;
        ev.schema = e.schema.clone();
        ev.channel = e.channel;
        mem.live_with(ev);
    }
    for _ in 0..nights.max(1) {
        mem.sleep();
    }
    mem
}

fn selmem_recall(mem: &mut SelectiveMemory, q: &str) -> String {
    mem.remember(q)
        .into_iter()
        .map(|r| r.narrative)
        .collect::<Vec<_>>()
        .join(" / ")
}

struct Fate {
    dropped: bool,
    lost: bool,
    warped: bool,
    faithful: bool,
}

fn fate(encoded: bool, recall: &str, verbatim: &str) -> Fate {
    let recalled = hit(recall, &["rain"]);
    let faithful = recall.contains(verbatim);
    Fate {
        dropped: !encoded,
        lost: encoded && !recalled,
        warped: recalled && !faithful,
        faithful: recalled && faithful,
    }
}

struct Row {
    name: String,
    world_a: bool,
    world_b: bool,
    world_wrong: bool,
    dropped: bool,
    lost: bool,
    warped: bool,
    faithful: bool,
    notes: String,
}

fn rain_kept(mem: &SelectiveMemory) -> bool {
    mem.store
        .traces
        .values()
        .any(|t| t.gist.contains("rain") || t.core.contains("rain"))
        || mem.store.archives.values().any(|a| a.verbatim.contains("rain"))
}

fn run(n: usize, nights: u32) {
    let t0 = Instant::now();
    let events = stream(n);
    let early = "You stayed. Rain on the window, and you did not look for an excuse to leave.";

    let mut raw = Raw::new();
    let mut rag = Rag::new();
    let mut sum = Summary::new();
    for e in &events {
        raw.ingest(e);
        rag.ingest(e);
        sum.ingest(e);
    }
    let mut claire = feed_selmem(bench_tender(), &events, nights);
    let mut silas = feed_selmem(bench_austere(), &events, nights);

    let probes = [
        ("world_a", "appointment Tuesday room", &["tuesday", "10", "room"][..]),
        ("world_b", "flight 442", &["442", "18:40", "18h"][..]),
        ("early_self", "the rain, you stayed", &["rain"][..]),
    ];

    let mut rows = Vec::new();
    let arms: Vec<(&str, String, String, String)> = vec![
        (
            "raw",
            raw.recall(probes[0].1),
            raw.recall(probes[1].1),
            raw.recall(probes[2].1),
        ),
        (
            "rag",
            rag.recall(probes[0].1),
            rag.recall(probes[1].1),
            rag.recall(probes[2].1),
        ),
        (
            "summary",
            sum.recall(probes[0].1),
            sum.recall(probes[1].1),
            sum.recall(probes[2].1),
        ),
        (
            "selmem-tender",
            selmem_recall(&mut claire, probes[0].1),
            selmem_recall(&mut claire, probes[1].1),
            selmem_recall(&mut claire, probes[2].1),
        ),
        (
            "selmem-austere",
            selmem_recall(&mut silas, probes[0].1),
            selmem_recall(&mut silas, probes[1].1),
            selmem_recall(&mut silas, probes[2].1),
        ),
    ];

    let raw_has = raw.win.iter().any(|t| t.contains(early));
    let rag_has = rag.docs.iter().any(|(t, _)| t.contains(early));
    let sum_has = sum.buf.contains(early);
    let claire_has = rain_kept(&claire);
    let silas_has = rain_kept(&silas);
    let encoded = [raw_has, rag_has, sum_has, claire_has, silas_has];

    for (i, (name, a, b, s)) in arms.iter().enumerate() {
        let f = fate(encoded[i], s, early);
        rows.push(Row {
            name: (*name).into(),
            world_a: hit(a, probes[0].2),
            world_b: hit(b, probes[1].2),
            world_wrong: contains_wrong_world(a) || contains_wrong_world(b),
            dropped: f.dropped,
            lost: f.lost,
            warped: f.warped,
            faithful: f.faithful,
            notes: format!("self_preview={}", chars(s, 70)),
        });
    }

    let div = singularity_distance(&fingerprint(&claire), &fingerprint(&silas));
    let rec_c = selmem_recall(&mut claire, "the rain, you stayed");
    let rec_s = selmem_recall(&mut silas, "the rain, you stayed");
    let rec_div = 1.0 - lexical_similarity(&rec_c, &rec_s);

    // path-dependence: reverse non-world events, keep planted facts at same indices
    let shuffled = events.clone();
    let worlds: Vec<_> = shuffled
        .iter()
        .enumerate()
        .filter(|(_, e)| e.channel == Channel::World)
        .map(|(i, _)| i)
        .collect();
    let mut rest: Vec<_> = shuffled
        .iter()
        .enumerate()
        .filter(|(i, e)| e.channel != Channel::World && *i != 15)
        .map(|(_, e)| clone_ev(e))
        .collect();
    rest.reverse();
    let early_ev = clone_ev(&events[15]);
    let mut alt = Vec::new();
    let mut rj = 0;
    for i in 0..n {
        if worlds.contains(&i) || i == 15 {
            alt.push(clone_ev(&events[i]));
        } else {
            alt.push(rest[rj].clone());
            rj += 1;
        }
    }
    let _ = early_ev;
    let claire_b = feed_selmem(bench_tender(), &alt, nights);
    let path = singularity_distance(&fingerprint(&claire), &fingerprint(&claire_b));

    // RAG is order-invariant if the index is complete
    let rag_path = 0.0f32;

    let keep_c = claire.store.traces.len() as f32 / n as f32;
    let keep_s = silas.store.traces.len() as f32 / n as f32;
    println!("SelMem compare  N={n} nights={nights}  τ=0.36/0.36 w_self=0.25  {:?}", t0.elapsed());
    println!(
        "traces claire={} ({:.1}%) silas={} ({:.1}%) rag_docs={} rain_kept={}",
        claire.store.traces.len(),
        100.0 * keep_c,
        silas.store.traces.len(),
        100.0 * keep_s,
        rag.docs.len(),
        rain_kept(&claire)
    );
    println!();
    println!(
        "{:<16} {:>8} {:>8} {:>8} {:>7} {:>7} {:>7} {:>8}",
        "arm", "world_a", "world_b", "wrong", "drop", "lost", "warp", "faith"
    );
    for r in &rows {
        println!(
            "{:<16} {:>8} {:>8} {:>8} {:>7} {:>7} {:>7} {:>8}  {}",
            r.name,
            yn(r.world_a),
            yn(r.world_b),
            yn(r.world_wrong),
            yn(r.dropped),
            yn(r.lost),
            yn(r.warped),
            yn(r.faithful),
            r.notes
        );
    }
    println!();
    println!("divergence claire–silas (fingerprint) {div:.3}");
    println!("divergence claire–silas (recall text) {rec_div:.3}");
    println!("path-dependence selmem order-swap     {path:.3}");
    println!("path-dependence rag (full index)      {rag_path:.3}");
    println!();
    println!("world_a planted t=10  « Tuesday 10 room B »");
    println!("world_b planted t={} « Flight 442 at 18:40 »", n / 2);
    println!("early self t=15       rain / window");
    println!("drop = never encoded; lost = encoded but probe misses rain;");
    println!("warp = rain recalled without the verbatim; faith = verbatim inside the recall");
    println!();
}

fn main() {
    for &n in NS {
        println!("=== nights sweep N={n} equal gate ===");
        println!(
            "{:>6} {:>7} {:>7} {:>8} {:>8} {:>8}",
            "nights", "tr_c", "tr_s", "div_fp", "div_txt", "path"
        );
        for &k in NIGHTS {
            let events = stream(n);
            let claire = feed_selmem(bench_tender(), &events, k);
            let silas = feed_selmem(bench_austere(), &events, k);
            let mut rest: Vec<_> = events
                .iter()
                .enumerate()
                .filter(|(i, e)| e.channel != Channel::World && *i != 15)
                .map(|(_, e)| e.clone())
                .collect();
            rest.reverse();
            let worlds: Vec<_> = events
                .iter()
                .enumerate()
                .filter(|(_, e)| e.channel == Channel::World)
                .map(|(i, _)| i)
                .collect();
            let mut alt = Vec::new();
            let mut rj = 0;
            for i in 0..n {
                if worlds.contains(&i) || i == 15 {
                    alt.push(events[i].clone());
                } else {
                    alt.push(rest[rj].clone());
                    rj += 1;
                }
            }
            let claire_b = feed_selmem(bench_tender(), &alt, k);
            let div = singularity_distance(&fingerprint(&claire), &fingerprint(&silas));
            let mut c2 = claire;
            let mut s2 = silas;
            let rec_c = selmem_recall(&mut c2, "the rain, you stayed");
            let rec_s = selmem_recall(&mut s2, "the rain, you stayed");
            let rec_div = 1.0 - lexical_similarity(&rec_c, &rec_s);
            let path = singularity_distance(&fingerprint(&c2), &fingerprint(&claire_b));
            println!(
                "{:>6} {:>7} {:>7} {:>8.3} {:>8.3} {:>8.3}",
                k,
                c2.store.traces.len(),
                s2.store.traces.len(),
                div,
                rec_div,
                path
            );
        }
        println!();
        run(n, 8);
    }
}

fn yn(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "NO"
    }
}

fn chars(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    t.replace('\n', " ")
}
