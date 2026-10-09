//! Learned co-recall links. They never supply facts or change retrieval scores.
use super::model::Attribution;
use std::collections::BTreeMap;

const RATE: f32 = 0.15;
const HALF_LIFE_SECS: f64 = 30.0 * 86_400.0;
const MAX_PAIRS: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct Association {
    pub weight: f32,
    pub co_recall_episodes: u32,
    pub last_episode: u64,
    pub updated_at: u64,
}
impl Association {
    pub fn weight_at(&self, now: u64) -> f32 {
        (self.weight as f64
            * 2.0_f64.powf(-(now.saturating_sub(self.updated_at) as f64) / HALF_LIFE_SECS))
            as f32
    }
}

#[derive(Clone, Debug)]
pub struct AssociativeCue {
    pub trace_id: String,
    pub gist: String,
    pub attribution: Attribution,
    pub weight: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AssociationChange {
    pub a: String,
    pub b: String,
    pub before: f32,
    pub after: f32,
    pub episode: u64,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssociationGraph {
    /// Independent cuts; disabled by default to preserve existing experiments.
    pub learn: bool,
    pub reconstruct: bool,
    pub pairs: BTreeMap<(String, String), Association>,
    pub episode: u64,
    pub episode_started_at: Option<u64>,
    pub last_live_at: Option<u64>,
}

impl AssociationGraph {
    pub fn key(a: &str, b: &str) -> (String, String) {
        if a < b {
            (a.into(), b.into())
        } else {
            (b.into(), a.into())
        }
    }
    /// Explicit boundary for non-conversational training; repeated calls within
    /// one episode cannot strengthen a pair twice. The counter is persisted.
    pub fn begin_episode(&mut self, now: u64) {
        self.episode = self.episode.saturating_add(1);
        self.episode_started_at = Some(now);
        self.last_live_at = Some(now);
    }
    pub fn weight(&self, a: &str, b: &str, now: u64) -> f32 {
        self.pairs
            .get(&Self::key(a, b))
            .map_or(0.0, |p| p.weight_at(now))
    }
    /// Activation tuples: trace id, independent observation id, direct relevance
    /// times accessibility. Caller supplies only accepted, actually recalled traces.
    pub fn reinforce(
        &mut self,
        active: &[(String, String, f32)],
        now: u64,
    ) -> Vec<AssociationChange> {
        if !self.learn {
            return Vec::new();
        }
        let start = self.episode_started_at;
        let last = self.last_live_at;
        if start.is_none()
            || last.is_none()
            || now < last.unwrap_or(now)
            || now.saturating_sub(last.unwrap_or(now)) > 600
            || now.saturating_sub(start.unwrap_or(now)) > 7200
        {
            self.begin_episode(now);
        }
        self.last_live_at = Some(now);
        // Decayed links should not permanently occupy the finite graph budget.
        self.pairs.retain(|_, p| p.weight_at(now) >= 0.001);
        if active.len() < 2 {
            return Vec::new();
        }
        let mut active = active.to_vec();
        active.sort_by(|a, b| a.0.cmp(&b.0));
        active.dedup_by(|a, b| a.0 == b.0);
        let mut changes = Vec::new();
        // Fixed total reinforcement budget per call, rather than O(k^2) gain.
        let count = active.len().saturating_mul(active.len().saturating_sub(1)) / 2;
        let rate = RATE / count.max(1) as f32;
        for i in 0..active.len() {
            for j in i + 1..active.len() {
                let (a, oa, aa) = &active[i];
                let (b, ob, ab) = &active[j];
                if a == b
                    || oa == ob
                    || !aa.is_finite()
                    || !ab.is_finite()
                    || *aa <= 0.0
                    || *ab <= 0.0
                {
                    continue;
                }
                let key = Self::key(a, b);
                if !self.pairs.contains_key(&key) && self.pairs.len() >= MAX_PAIRS {
                    continue;
                }
                let pair = self.pairs.entry(key).or_insert(Association {
                    weight: 0.0,
                    co_recall_episodes: 0,
                    last_episode: 0,
                    updated_at: now,
                });
                if pair.last_episode == self.episode {
                    continue;
                }
                let before = pair.weight_at(now);
                pair.weight = (before
                    + rate * aa.clamp(0.0, 1.0) * ab.clamp(0.0, 1.0) * (1.0 - before))
                    .clamp(0.0, 1.0);
                pair.updated_at = now;
                pair.last_episode = self.episode;
                pair.co_recall_episodes = pair.co_recall_episodes.saturating_add(1);
                changes.push(AssociationChange {
                    a: a.clone(),
                    b: b.clone(),
                    before,
                    after: pair.weight,
                    episode: self.episode,
                });
            }
        }
        changes
    }
    pub fn remove_trace(&mut self, id: &str) {
        self.pairs.retain(|(a, b), _| a != id && b != id);
    }
    /// Shared codec for both snapshot containers. IDs are UTF-8 hex, not tokens.
    pub fn encode(&self) -> String {
        let mut out = format!(
            "1 {} {} {} {} {} {}",
            self.learn as u8,
            self.reconstruct as u8,
            self.episode,
            optional(self.episode_started_at),
            optional(self.last_live_at),
            self.pairs.len()
        );
        for ((a, b), p) in &self.pairs {
            out.push_str(&format!(
                " {} {} {} {} {} {}",
                hex(a),
                hex(b),
                p.weight,
                p.co_recall_episodes,
                p.last_episode,
                p.updated_at
            ));
        }
        out
    }
    pub fn decode(raw: &str) -> Result<Self, String> {
        let mut fields = raw.split_whitespace();
        let mut next = || {
            fields
                .next()
                .ok_or_else(|| "truncated association snapshot".to_string())
        };
        if next()? != "1" {
            return Err("unknown association snapshot version".into());
        }
        let learn = flag(next()?)?;
        let reconstruct = flag(next()?)?;
        let episode = integer(next()?)?;
        let episode_started_at = parse_optional(next()?)?;
        let last_live_at = parse_optional(next()?)?;
        let count = integer(next()?)?;
        if count > MAX_PAIRS as u64 {
            return Err("association snapshot exceeds pair budget".into());
        }
        let mut pairs = BTreeMap::new();
        for _ in 0..count {
            let a = unhex(next()?)?;
            let b = unhex(next()?)?;
            let weight: f32 = next()?.parse().map_err(|_| "invalid association weight")?;
            let episodes = integer(next()?)?;
            let last_episode = integer(next()?)?;
            let updated_at = integer(next()?)?;
            if a >= b
                || !weight.is_finite()
                || !(0.0..=1.0).contains(&weight)
                || episodes == 0
                || episodes > u32::MAX as u64
                || last_episode > episode
            {
                return Err("invalid association pair".into());
            }
            if pairs
                .insert(
                    (a, b),
                    Association {
                        weight,
                        co_recall_episodes: episodes as u32,
                        last_episode,
                        updated_at,
                    },
                )
                .is_some()
            {
                return Err("duplicate association pair".into());
            }
        }
        if fields.next().is_some() {
            return Err("trailing association snapshot fields".into());
        }
        Ok(Self {
            learn,
            reconstruct,
            pairs,
            episode,
            episode_started_at,
            last_live_at,
        })
    }
}
fn optional(v: Option<u64>) -> String {
    v.map_or_else(|| "-".into(), |v| v.to_string())
}
fn parse_optional(s: &str) -> Result<Option<u64>, String> {
    if s == "-" {
        Ok(None)
    } else {
        integer(s).map(Some)
    }
}
fn integer(s: &str) -> Result<u64, String> {
    s.parse().map_err(|_| "invalid association integer".into())
}
fn flag(s: &str) -> Result<bool, String> {
    match s {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err("invalid association flag".into()),
    }
}
fn hex(s: &str) -> String {
    s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str) -> Result<String, String> {
    if s.is_empty() || s.len() % 2 != 0 || !s.is_ascii() {
        return Err("invalid association id".into());
    }
    let bytes = (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid association id".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| "invalid association UTF-8".into())
}
