//! Working conversation frame. Not the lived book.
//!
//! The LLM call is stateless. This frame is what lets the next turn
//! still know what the last turns were about. Persist does not write it:
//! a session, not a sculpture. Sleep commits the sitting through the
//! encode gate into the book, then drops the frame. Continuity after
//! night is recall, not this buffer.
//!
//! Lifetime is the active conversation, not a fixed turn count.
//! Active = last hear or reply within [`ACTIVE_GAP_SECS`] (10 min).
//! Hard cap [`MAX_SESSION_SECS`] (2 h) from the first pulse.

use crate::core::model::now_secs;
use crate::encode::scoring::{lexical_similarity, token_set};

/// Silence longer than this ends the conversation.
pub const ACTIVE_GAP_SECS: u64 = 10 * 60;
/// A single thread never outlives this, even if still talking.
pub const MAX_SESSION_SECS: u64 = 2 * 60 * 60;
/// Flood cap inside one session. Not the lifetime.
const MAX_TURNS: usize = 80;
const MAX_LINE: usize = 220;

const LIGHT: &[&str] = &[
    "alors", "quoi", "donc", "mais", "comme", "cette", "cette", "tres", "très", "plus", "dans",
    "pour", "avec", "sans", "comment", "pourquoi", "quand", "quoi", "oui", "non", "bien", "tout",
    "rien", "cela", "cette", "what", "about", "think", "that", "this", "when", "just", "very",
    "well", "how", "why", "and", "the", "you", "toi", "tu", "te", "ton", "tes", "mon", "mes",
    "une", "des", "les", "est", "suis", "pas", "plus", "pense", "penses", "dis", "dit", "vois",
    "voir", "encore", "toujours",
];

#[derive(Clone, Debug)]
pub struct TalkTurn {
    pub user: String,
    pub reply: String,
}

#[derive(Clone, Debug, Default)]
pub struct WorkingTalk {
    pub topic: Option<String>,
    pub schema: Option<String>,
    pub turns: Vec<TalkTurn>,
    /// First pulse of this thread.
    pub opened_at: Option<u64>,
    /// Last hear or reply. Gap vs now decides activity.
    pub last_active_at: Option<u64>,
}

impl WorkingTalk {
    pub fn is_empty(&self) -> bool {
        self.topic.is_none() && self.schema.is_none() && self.turns.is_empty()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn refresh(&mut self) {
        self.refresh_at(now_secs());
    }

    pub fn refresh_at(&mut self, now: u64) {
        if self.is_empty() {
            return;
        }
        if !self.active_at(now) {
            self.clear();
        }
    }

    pub fn active(&self) -> bool {
        self.active_at(now_secs())
    }

    /// Live iff last answer/reply (or hear) is within 10 min
    /// and the thread is younger than 2 h.
    pub fn active_at(&self, now: u64) -> bool {
        if self.is_empty() {
            return false;
        }
        let last = match self.last_active_at.or(self.opened_at) {
            Some(t) => t,
            None => return false,
        };
        let open = self.opened_at.unwrap_or(last);
        now.saturating_sub(last) <= ACTIVE_GAP_SECS && now.saturating_sub(open) <= MAX_SESSION_SECS
    }

    fn pulse_at(&mut self, now: u64) {
        self.refresh_at(now);
        if self.opened_at.is_none() {
            self.opened_at = Some(now);
        }
        self.last_active_at = Some(now);
    }

    /// Cue mixed into recall so "et alors ?" can still find the thread.
    pub fn recall_query(&self, user: &str) -> String {
        match self.topic.as_deref() {
            Some(t) if !t.is_empty() => format!("{t} {user}"),
            _ => user.to_string(),
        }
    }

    pub fn hear(&mut self, user: &str, schema: Option<&str>) {
        self.hear_at(now_secs(), user, schema);
    }

    pub fn hear_at(&mut self, now: u64, user: &str, schema: Option<&str>) {
        // Expire a dead thread first. A hear opens the session;
        // only a reply (record) counts as activity for the 10 min window.
        self.refresh_at(now);
        if self.opened_at.is_none() {
            self.opened_at = Some(now);
        }
        if self.last_active_at.is_none() {
            self.last_active_at = Some(now);
        }
        if let Some(s) = schema.map(str::trim).filter(|s| !s.is_empty() && *s != "-") {
            self.schema = Some(s.to_string());
            if self.topic.is_none() {
                self.topic = Some(s.to_string());
            }
        }
        let content = content_tokens(user);
        if content.len() < 2 {
            return;
        }
        let candidate = content.into_iter().take(8).collect::<Vec<_>>().join(" ");
        match self.topic.as_deref() {
            None => self.topic = Some(candidate),
            Some(t) if lexical_similarity(user, t) < 0.10 => {
                self.topic = Some(candidate);
            }
            Some(_) => {}
        }
    }

    pub fn record(&mut self, user: &str, reply: &str) {
        self.record_at(now_secs(), user, reply);
    }

    pub fn record_at(&mut self, now: u64, user: &str, reply: &str) {
        self.pulse_at(now);
        self.turns.push(TalkTurn {
            user: clip(user, MAX_LINE),
            reply: clip(reply, MAX_LINE),
        });
        if self.turns.len() > MAX_TURNS {
            let drop = self.turns.len() - MAX_TURNS;
            self.turns.drain(0..drop);
        }
    }

    pub fn render(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        if let Some(t) = self.topic.as_deref() {
            out.push_str("fil: ");
            out.push_str(t);
            out.push('\n');
        }
        if let Some(s) = self.schema.as_deref() {
            out.push_str("schema du fil: ");
            out.push_str(s);
            out.push('\n');
        }
        if !self.turns.is_empty() {
            out.push_str("current conversation:\n");
            for t in &self.turns {
                out.push_str("- human: ");
                out.push_str(&t.user);
                out.push('\n');
                out.push_str("- you: ");
                out.push_str(&t.reply);
                out.push('\n');
            }
        }
        out
    }
}

fn clip(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= n {
        t.to_string()
    } else {
        t.chars().take(n).collect()
    }
}

fn content_tokens(text: &str) -> Vec<String> {
    token_set(text)
        .into_iter()
        .filter(|w| !LIGHT.contains(&w.as_str()))
        .collect()
}
