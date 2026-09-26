/// Lexicon for `retell`. Empty = no textual drift from the table.
/// Strings live here, not in the drift function.
#[derive(Clone, Debug, Default)]
pub struct Voice {
    pub marker: String,
    pub replacements: Vec<(String, String)>,
    pub suffix_warm: String,
    pub suffix_cold: String,
}

impl Voice {
    pub fn parse(raw: &str) -> Self {
        let mut v = Self::default();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, val)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let val = val.trim().to_string();
            match k {
                "marker" => v.marker = val,
                "suffix_warm" => v.suffix_warm = val,
                "suffix_cold" => v.suffix_cold = val,
                _ => v.replacements.push((k.to_string(), val)),
            }
        }
        v
    }

    pub fn tender() -> Self {
        Self::parse(include_str!("../../voices/tender.txt"))
    }

    pub fn austere() -> Self {
        Self::parse(include_str!("../../voices/austere.txt"))
    }

    pub fn from_gains(embellish: f32, disgust: f32) -> Self {
        let g = embellish - disgust;
        if g >= 0.04 {
            Self::tender()
        } else if g <= -0.04 {
            Self::austere()
        } else {
            Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.marker.is_empty() && self.replacements.is_empty()
    }
}

/// Sensitivity knobs. Digits are exploratory — see PARAMETERS.md.
/// Literature warrants mechanism shape, not 0.40 or 0.18.
#[derive(Clone, Debug)]
pub struct EntityProfile {
    pub name: String,
    pub encode_threshold: f32,
    pub w_arousal: f32,
    pub w_novelty: f32,
    pub w_self: f32,
    pub w_utility: f32,
    pub w_goal: f32,
    pub w_redundancy: f32,
    pub decay_lambda: f32,
    pub rehearsal_boost: f32,
    pub embellish_gain: f32,
    pub disgust_gain: f32,
    pub disgust_cap: f32,
    pub fidelity_loss_on_recall: f32,
    pub reconsolidation_eta: f32,
    pub mood_blend: f32,
    pub cold_access: f32,
    pub myth_access: f32,
    pub max_recall: usize,
    pub extinction_rate: f32,
    pub merge_similarity: f32,
    /// Identity-gate cut for `DetachKind::Hold`. Causal / frame / contradiction misses ignore it.
    pub ground_min_overlap: f32,
    /// Misses allowed before the organ rewrites its gist toward the core.
    pub ground_strikes: usize,
    /// 0 = let even important traces warp. 1 = pull hard toward the core.
    pub narrator_firmness: f32,
    /// Deep night if at least this many new Selfhood hours since the last deep night.
    pub deep_min_hours: u32,
    /// Or if those new hours carry at least this much arousal+disgust.
    pub deep_min_charge: f32,
    pub voice: Voice,
}

impl EntityProfile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            encode_threshold: 0.32,
            w_arousal: 0.25,
            w_novelty: 0.15,
            w_self: 0.25,
            w_utility: 0.15,
            w_goal: 0.10,
            w_redundancy: 0.20,
            decay_lambda: 0.08,
            rehearsal_boost: 0.18,
            embellish_gain: 0.12,
            disgust_gain: 0.10,
            disgust_cap: 0.92,
            fidelity_loss_on_recall: 0.04,
            reconsolidation_eta: 0.25,
            mood_blend: 0.08,
            cold_access: 0.12,
            myth_access: 0.04,
            max_recall: 4,
            extinction_rate: 0.06,
            merge_similarity: 0.32,
            ground_min_overlap: 0.18,
            ground_strikes: 3,
            narrator_firmness: 0.55,
            // 1 new hour → deep. Charge cut is high so default nights match today's benches.
            // Live setting: raise hours (3) and lower charge (1.2).
            deep_min_hours: 1,
            deep_min_charge: 9.0,
            voice: Voice::default(),
        }
    }

    pub fn tender(name: impl Into<String>) -> Self {
        Self {
            encode_threshold: 0.28,
            embellish_gain: 0.18,
            disgust_gain: 0.05,
            decay_lambda: 0.10,
            narrator_firmness: 0.42,
            voice: Voice::tender(),
            ..Self::new(name)
        }
    }

    pub fn austere(name: impl Into<String>) -> Self {
        Self {
            encode_threshold: 0.40,
            narrator_firmness: 0.72,
            embellish_gain: 0.05,
            disgust_gain: 0.16,
            decay_lambda: 0.06,
            w_self: 0.30,
            voice: Voice::austere(),
            ..Self::new(name)
        }
    }

    pub fn voice_kind(&self) -> &'static str {
        let g = self.embellish_gain - self.disgust_gain;
        if g >= 0.04 {
            "tender"
        } else if g <= -0.04 {
            "austere"
        } else {
            "neutral"
        }
    }

    pub fn set_voice(&mut self, kind: &str) {
        match kind.trim().to_ascii_lowercase().as_str() {
            "tender" => {
                self.embellish_gain = 0.18;
                self.disgust_gain = 0.05;
                self.voice = Voice::tender();
            }
            "austere" => {
                self.embellish_gain = 0.05;
                self.disgust_gain = 0.16;
                self.voice = Voice::austere();
            }
            _ => {
                self.embellish_gain = 0.10;
                self.disgust_gain = 0.10;
                self.voice = Voice::default();
            }
        }
    }
}
