//! The organ the rest of the crate talks to.
//!
//! Typical loop:
//! `live_with` (encode) → `remember` / `speak` (reconstruct) → `sleep` (consolidate).
//! Persistence (`save` / `open`) is a vault. The narrator never opens it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::dream::{self, DreamReport};
use crate::encode::embed::{Embedder, HashEmbedder};
use crate::encode::{self, EncodeDecision, EncodeInput};
use crate::core::model::{ClockGuard, IdentityAxiom, MemoryClock, Mood, OrganCut, RecallTally, RecalledMemory};
use crate::core::talk::WorkingTalk;
use crate::recall::narrator::{Narrator, RuleNarrator};
use crate::recall::{RecallBias, RecallWrite, RetrievalDump};
use crate::persist;
use crate::core::profile::EntityProfile;
use crate::recall;
use crate::core::store::MemoryStore;

pub struct SelectiveMemory {
    pub profile: EntityProfile,
    pub store: MemoryStore,
    pub mood: Mood,
    /// Live thread. Not a trace. Not persisted. Sleep commits then clears it.
    pub talk: WorkingTalk,
    pub path: Option<PathBuf>,
    pub cut: OrganCut,
    pub recall_tally: RecallTally,
    /// Live HTTP narrator bind. Not in the vault. Empty url = RuleNarrator.
    pub llm: LlmBind,
    narrator: Arc<dyn Narrator>,
    /// Experiment seed. Does not make the night stochastic; it seeds the id stream.
    pub seed: u32,
    pub clock: MemoryClock,
    embedder: Box<dyn Embedder>,
}

/// Runtime LLM endpoint. Key stays on the process; GET only reports a mask.
#[derive(Clone, Debug, Default)]
pub struct LlmBind {
    pub url: String,
    pub model: String,
    pub key: Option<String>,
}

pub struct MouthDraft {
    pub user: String,
    pub memories: Vec<String>,
    pub axioms: Vec<String>,
    pub mood: Mood,
    pub talk: WorkingTalk,
    pub hold: bool,
    pub dump: RetrievalDump,
}

impl SelectiveMemory {
    pub fn new(profile: EntityProfile) -> Self {
        Self {
            profile,
            store: MemoryStore::new(),
            mood: Mood::default(),
            talk: WorkingTalk::default(),
            path: None,
            cut: OrganCut::full(),
            recall_tally: RecallTally::default(),
            llm: LlmBind::default(),
            narrator: Arc::new(RuleNarrator),
            embedder: Box::new(HashEmbedder),
            seed: 0,
            clock: MemoryClock::default(),
        }
    }

    pub fn open(path: impl AsRef<Path>, profile: EntityProfile) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let snap = if is_sqlite(&path) {
                crate::persist::sqlite::load(&path)?
            } else {
                persist::load(&path)?
            };
            Ok(Self {
                profile: snap.profile,
                store: snap.store,
                mood: snap.mood,
                talk: WorkingTalk::default(),
                path: Some(path),
                cut: OrganCut::full(),
                recall_tally: RecallTally::default(),
                llm: LlmBind::default(),
                narrator: Arc::new(RuleNarrator),
                embedder: Box::new(HashEmbedder),
                seed: 0,
                clock: MemoryClock::default(),
            })
        } else {
            Ok(Self {
                profile,
                store: MemoryStore::new(),
                mood: Mood::default(),
                talk: WorkingTalk::default(),
                path: Some(path),
                cut: OrganCut::full(),
                recall_tally: RecallTally::default(),
                llm: LlmBind::default(),
                narrator: Arc::new(RuleNarrator),
                embedder: Box::new(HashEmbedder),
                seed: 0,
                clock: MemoryClock::default(),
            })
        }
    }

    pub fn with_narrator(mut self, narrator: Box<dyn Narrator>) -> Self {
        self.narrator = Arc::from(narrator);
        self
    }

    pub fn narrator_arc(&self) -> Arc<dyn Narrator> {
        Arc::clone(&self.narrator)
    }

    pub fn embed_log(&self) -> crate::encode::embed::EmbedLog {
        self.embedder.embed_log()
    }

    /// Record the seed and offset the process id stream so two seeds are not the same run.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        let origin = (seed as u64).saturating_mul(0x1000).max(1);
        crate::core::model::set_next_id(origin);
        self
    }

    /// Attach or detach the HTTP narrator. Empty `url` falls back to rules.
    pub fn set_llm(&mut self, url: &str, model: &str, key: Option<String>) -> Result<(), String> {
        let url = url.trim();
        let model = model.trim();
        if url.is_empty() {
            self.narrator = Arc::new(RuleNarrator);
            self.llm = LlmBind::default();
            return Ok(());
        }
        let key = match key {
            Some(k) if k.trim().is_empty() => self.llm.key.clone(),
            Some(k) => Some(k),
            None => self.llm.key.clone(),
        };
        let n = crate::recall::HttpNarrator::parse(url, model, key.clone())
            .ok_or_else(|| "llm url must be http:// or https://".to_string())?;
        self.narrator = Arc::new(n);
        self.llm = LlmBind {
            url: url.to_string(),
            model: if model.is_empty() { "llama3".into() } else { model.into() },
            key,
        };
        Ok(())
    }

    pub fn with_cut(mut self, cut: OrganCut) -> Self {
        self.cut = cut;
        self
    }

    pub fn reset_recall_tally(&mut self) {
        self.recall_tally = RecallTally::default();
    }

    pub fn with_embedder(mut self, embedder: Box<dyn Embedder>) -> Self {
        self.embedder = embedder;
        self
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if is_sqlite(path) {
            crate::persist::sqlite::save(path, &self.profile, &self.mood, &self.store)
        } else {
            persist::save(path, &self.profile, &self.mood, &self.store)
        }
    }

    pub fn live(&mut self, event: &str) -> EncodeDecision {
        self.live_with(EncodeInput::new(event))
    }

    /// Tool / journal line. Always kept. Sleep does not rewrite it.
    pub fn live_log(&mut self, event: &str) -> EncodeDecision {
        let mut ev = EncodeInput::new(event);
        ev.channel = crate::core::model::Channel::Log;
        ev.source = "log";
        ev.utility = 0.85;
        ev.permanence = 0.70;
        ev.self_relevance = 0.15;
        ev.arousal = 0.15;
        self.live_with(ev)
    }

    pub fn detach_clock(mut self) -> Self {
        self.clock = self.clock.detach();
        self
    }

    pub fn advance_hours(&mut self, hours: f32) {
        self.clock.advance_hours(hours);
    }

    fn enter_clock(&self) -> ClockGuard {
        ClockGuard::push(self.clock.clone())
    }

    pub fn live_with(&mut self, input: EncodeInput<'_>) -> EncodeDecision {
        let _clock = self.enter_clock();
        self.ingest(input, true)
    }

    /// Same gate as `live_with`. `hold` writes the live thread; sleep commit does not.
    ///
    /// Order is the organ: interpret → paint → maybe hear → split → gate →
    /// maybe accept_core → blend mood. `commit_talk` calls this with `hold = false`.
    fn ingest(&mut self, mut input: EncodeInput<'_>, hold: bool) -> EncodeDecision {
        let axioms: Vec<String> = self
            .store
            .living_axioms()
            .into_iter()
            .map(|ax| ax.statement.clone())
            .collect();
        encode::interpret(&mut input, &self.mood, &axioms, self.narrator.as_ref());
        encode::paint(&mut self.store, &self.mood, &mut input);
        // How long the hour lasts follows how hard the text hit, unless the
        // caller already pinned permanence (shared protocol days).
        if input.permanence <= 0.0 {
            let shock = (input.valence.abs() * 0.55 + input.disgust * 0.70
                + (input.arousal - 0.18).max(0.0) * 0.25)
                .clamp(0.0, 1.0);
            input.permanence = (0.16 + 0.80 * shock).clamp(0.08, 0.97);
            input.self_relevance = input.self_relevance.max(0.32 + 0.65 * shock);
        }
        if hold {
            self.talk.hear(input.event, input.schema.as_deref());
        }
        let valence = input.valence;
        let arousal = input.arousal;
        let disgust = input.disgust;
        let input_source = input.source;
        let event_owned = input.event.to_string();
        let proposed = encode::propose_split(&event_owned, self.narrator.as_ref());
        let decision = encode::encode_with_parts(
            &mut self.store,
            &self.profile,
            input,
            self.embedder.as_ref(),
            proposed.as_deref(),
        );
        encode::maybe_set_core(
            &mut self.store,
            self.narrator.as_ref(),
            &decision,
            input_source,
            &event_owned,
        );
        if decision.kept {
            self.mood.blend(
                &Mood {
                    valence,
                    arousal,
                    disgust,
                },
                self.profile.mood_blend,
            );
        }
        decision
    }

    /// The sitting becomes hours, then the frame dies.
    ///
    /// Sleep right after a chat must not wipe the conversation: each recorded
    /// turn is one live event (affect → paint → gate). Topic-only is still
    /// not an episode — experiments live then sleep and must not grow extra traces.
    fn commit_talk(&mut self) {
        self.talk.refresh();
        let schema = self.talk.schema.clone();
        let turns = std::mem::take(&mut self.talk.turns);
        self.talk.clear();
        for t in turns {
            // The sitting is the other voice. Glueing the reply in made the
            // entity's words count as what happened to it.
            let event = t.user.trim();
            if event.is_empty() {
                continue;
            }
            let (v, a, d, guessed) = encode::affect::guess(event);
            let mut ev = EncodeInput::new(event);
            ev.source = "talk";
            ev.valence = v;
            ev.arousal = a;
            ev.disgust = d;
            ev.self_relevance = 0.35;
            ev.utility = 0.45;
            ev.permanence = 0.40;
            ev.schema = schema.clone().or(guessed);
            let _ = self.ingest(ev, false);
        }
    }

    pub fn remember(&mut self, query: &str) -> Vec<RecalledMemory> {
        self.remember_with(query, RecallWrite::Live, RecallBias::Observed, &[])
            .0
    }

    pub fn remember_with(
        &mut self,
        query: &str,
        write: RecallWrite,
        bias: RecallBias,
        marked: &[String],
    ) -> (Vec<RecalledMemory>, RetrievalDump) {
        let _clock = self.enter_clock();
        let out = recall::recall_with(
            &mut self.store,
            &self.profile,
            self.narrator.as_ref(),
            self.embedder.as_ref(),
            query,
            &self.mood,
            self.cut,
            write,
            bias,
            marked,
        );
        if write == RecallWrite::Live {
            self.recall_tally.n += out.memories.len() as u32;
            for r in &out.memories {
                if r.pulled_toward_core {
                    self.recall_tally.pulled += 1;
                }
                if r.reconsolidated {
                    self.recall_tally.reconsolidated += 1;
                }
            }
            if !out.memories.is_empty() {
                let mut v = 0.0;
                let mut a = 0.0;
                let mut d = 0.0;
                let n = out.memories.len() as f32;
                for r in &out.memories {
                    if let Some(t) = self.store.traces.get(&r.trace_id) {
                        v += t.valence;
                        a += t.arousal;
                        d += t.disgust;
                    }
                }
                self.mood.blend(
                    &Mood {
                        valence: v / n,
                        arousal: a / n,
                        disgust: d / n,
                    },
                    self.profile.mood_blend * 1.4,
                );
            }
        }
        (out.memories, out.dump)
    }

    pub fn sleep(&mut self) -> DreamReport {
        let _clock = self.enter_clock();
        self.commit_talk();
        crate::persist::prune_orphaned_archives(&mut self.store);
        dream::dream_budget(
            &mut self.store,
            &self.profile,
            self.narrator.as_ref(),
            self.embedder.as_ref(),
            self.cut,
        )
    }

    /// Full night regardless of budget. Lab path.
    pub fn sleep_deep(&mut self) -> DreamReport {
        let _clock = self.enter_clock();
        self.commit_talk();
        crate::persist::prune_orphaned_archives(&mut self.store);
        dream::dream_cut(
            &mut self.store,
            &self.profile,
            self.narrator.as_ref(),
            self.embedder.as_ref(),
            self.cut,
        )
    }

    pub fn lineage(&self, schema: &str) -> Vec<&IdentityAxiom> {
        let mut xs: Vec<_> = self
            .store
            .axioms
            .values()
            .filter(|a| a.schema.as_deref() == Some(schema))
            .collect();
        xs.sort_by_key(|a| a.created_at);
        xs
    }

    pub fn who_am_i(&self) -> Vec<&IdentityAxiom> {
        let mut axioms = self.store.living_axioms();
        axioms.sort_by(|a, b| {
            let rank = |l| match l {
                crate::core::model::AxiomLayer::Trait => 2,
                crate::core::model::AxiomLayer::Belief => 1,
                crate::core::model::AxiomLayer::Motif => 0,
            };
            rank(b.layer)
                .cmp(&rank(a.layer))
                .then(
                    b.strength
                        .partial_cmp(&a.strength)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        axioms
    }

    pub fn speak(&mut self, user: &str) -> String {
        self.speak_inner(user, true, RecallBias::Observed, &[], false).0
    }

    /// Probe path. Does not read or write the live thread, and does not write the book.
    pub fn speak_isolated(&mut self, user: &str) -> String {
        self.speak_isolated_with(user, RecallBias::Observed, &[]).0
    }

    pub fn speak_isolated_with(
        &mut self,
        user: &str,
        bias: RecallBias,
        marked: &[String],
    ) -> (String, RetrievalDump) {
        self.speak_inner(user, false, bias, marked, false)
    }

    /// Isolated probe whose mouth sees living axioms only — no retrieved gists.
    pub fn speak_isolated_axioms(
        &mut self,
        user: &str,
        bias: RecallBias,
        marked: &[String],
    ) -> (String, RetrievalDump) {
        self.speak_inner(user, false, bias, marked, true)
    }

    pub fn clear_talk(&mut self) {
        self.talk.clear();
    }

    /// Chat only. Pin the sitting's content lines so sleep can clear the
    /// frame without dropping what was just said. Does not change the gate.
    pub fn keep_sitting(&mut self) -> (usize, Option<String>) {
        self.talk.refresh();
        let lines: Vec<String> = self
            .talk
            .turns
            .iter()
            .map(|t| t.user.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let cue = self.talk.topic.clone();
        let mut chunks: Vec<String> = Vec::new();
        for line in lines {
            let short = line.split_whitespace().count() <= 2;
            if short {
                if let Some(prev) = chunks.last_mut() {
                    prev.push('\n');
                    prev.push_str(&line);
                    continue;
                }
            }
            chunks.push(line);
        }
        let mut kept = 0usize;
        for chunk in &chunks {
            if chunk.split_whitespace().count() <= 2 {
                continue;
            }
            if self
                .store
                .archives
                .values()
                .any(|a| a.source == "talk" && a.verbatim == *chunk)
            {
                continue;
            }
            let (v, a, d, schema) = encode::affect::guess(chunk);
            let mut ev = EncodeInput::new(chunk);
            ev.source = "talk";
            ev.channel = crate::core::model::Channel::World;
            ev.valence = v;
            ev.arousal = a;
            ev.disgust = d;
            ev.self_relevance = 0.55;
            ev.utility = 0.55;
            ev.permanence = 0.35;
            ev.schema = schema;
            if self.ingest(ev, false).kept {
                kept += 1;
            }
        }
        (kept, cue)
    }

    /// Sitting hours are not vows. Fidelity floors at 0.15 (husk).
    /// Access drops; at the floor the hour goes latent, then leaves.
    pub fn fade_sitting(&mut self) {
        let talk_ids: Vec<String> = self
            .store
            .traces
            .values()
            .filter(|t| {
                t.permanence < 0.80
                    && t.archive_id
                        .as_ref()
                        .and_then(|id| self.store.archives.get(id))
                        .map(|a| a.source == "talk")
                        .unwrap_or(false)
            })
            .map(|t| t.id.clone())
            .collect();
        let mut drop = Vec::new();
        for id in talk_ids {
            let Some(t) = self.store.traces.get_mut(&id) else {
                continue;
            };
            crate::encode::scoring::refresh_access(t, &self.profile);
            t.fidelity = (t.fidelity - 0.04).max(0.15);
            t.access = (t.access - 0.05).min(t.access).max(0.0);
            t.permanence = (t.permanence - 0.01).max(0.0);
            // Floor is the husk. Forgetting is access → latent → leave the book.
            if t.fidelity <= 0.151 && t.access < 0.12 {
                if t.status == crate::core::model::TraceStatus::Latent {
                    drop.push(id);
                } else {
                    t.status = crate::core::model::TraceStatus::Latent;
                }
            }
        }
        for id in drop {
            self.store.release_trace(&id);
        }
    }

    /// Empty the book, the seal, axioms, mood and the sitting. Profile stays.
    pub fn reset(&mut self) {
        self.store = MemoryStore::new();
        self.mood = Mood::default();
        self.talk.clear();
    }

    /// Drop the frame if the conversation went idle (10 min) or hit 2 h.
    pub fn refresh_talk(&mut self) {
        self.talk.refresh();
    }

    pub fn open_mouth(&mut self, user: &str) -> MouthDraft {
        self.open_mouth_inner(user, true, RecallBias::Observed, &[], false)
    }

    pub fn close_mouth(&mut self, draft: &MouthDraft, reply: &str) {
        if draft.hold {
            self.talk.record(&draft.user, reply);
        }
    }

    fn speak_inner(
        &mut self,
        user: &str,
        hold: bool,
        bias: RecallBias,
        marked: &[String],
        axioms_only: bool,
    ) -> (String, RetrievalDump) {
        let draft = self.open_mouth_inner(user, hold, bias, marked, axioms_only);
        let reply = self.narrator.reply(
            &draft.user,
            &draft.memories,
            &draft.axioms,
            &draft.mood,
            &draft.talk,
        );
        self.close_mouth(&draft, &reply);
        (reply, draft.dump)
    }

    fn open_mouth_inner(
        &mut self,
        user: &str,
        hold: bool,
        bias: RecallBias,
        marked: &[String],
        axioms_only: bool,
    ) -> MouthDraft {
        let _clock = self.enter_clock();
        if hold {
            self.talk.hear(user, None);
        }
        let query = if hold {
            self.talk.recall_query(user)
        } else {
            user.to_string()
        };
        let write = if hold {
            RecallWrite::Live
        } else {
            RecallWrite::ReadOnly
        };
        let (recalled, dump) = self.remember_with(&query, write, bias, marked);
        if hold {
            // Spoken utility: the hour that actually entered the mouth, not every neighbor.
            if let Some(id) = dump.selected.first() {
                self.note_spoken(id);
            }
        }
        let empty = WorkingTalk::default();
        let talk = if hold { &self.talk } else { &empty };
        // Isolated probes: retrieved scenes + living axioms.
        // Stance-without-scene lives in recall/stance.rs (ablation only).
        let (memories, axioms) = if hold {
            let memories: Vec<String> = recalled
                .into_iter()
                .take(2)
                .map(|r| {
                    let core = self
                        .store
                        .traces
                        .get(&r.trace_id)
                        .map(|t| t.core.as_str())
                        .unwrap_or("");
                    pin_happened(&r.narrative, core)
                })
                .collect();
            let mut axioms = vec![format!("I am {}.", self.profile.name)];
            axioms.extend(
                self.who_am_i()
                    .into_iter()
                    .take(4)
                    .map(|a| a.statement.clone()),
            );
            (memories, axioms)
        } else {
            let lineage = crate::recall::retrieve::lineage_of(&self.store, marked);
            let lineage_schemas: Vec<String> = lineage
                .iter()
                .filter_map(|id| self.store.traces.get(id).and_then(|t| t.schema.clone()))
                .collect();
            let drop_ax = matches!(bias, RecallBias::DropLineage);
            // A choice is not reading comprehension of the hour. The dump still
            // records what was selected. A motif is not a policy: only a belief
            // or a trait may color the sentence.
            let choice = crate::recall::retrieve::choice_ask(user);
            let axioms: Vec<String> = self
                .who_am_i()
                .into_iter()
                .filter(|a| {
                    if choice && matches!(a.layer, crate::core::model::AxiomLayer::Motif) {
                        return false;
                    }
                    if !drop_ax {
                        return true;
                    }
                    !crate::recall::retrieve::axiom_supported_by_lineage(
                        &a.support_trace_ids,
                        a.schema.as_deref(),
                        &lineage,
                        &lineage_schemas,
                    )
                })
                .map(|a| a.statement.clone())
                .collect();
            let memories: Vec<String> = if axioms_only || choice {
                Vec::new()
            } else {
                recalled.into_iter().map(|r| r.narrative).collect()
            };
            (memories, axioms)
        };
        MouthDraft {
            user: user.to_string(),
            memories,
            axioms,
            mood: self.mood.clone(),
            talk: talk.clone(),
            hold,
            dump,
        }
    }

    pub fn audit(&self, trace_id: &str) -> Option<&str> {
        self.store.archive_verbatim(trace_id)
    }

    /// Spoken utility stamp. Live `speak` calls this on the selected hour.
    /// Isolated probes do not. When `util_to_strength` is on, living axioms
    /// that list the hour gain a bounded step of strength.
    pub fn note_spoken(&mut self, trace_id: &str) -> bool {
        let verbatim = match self.store.traces.get(trace_id) {
            Some(t) => t.channel.verbatim(),
            None => return false,
        };
        if verbatim {
            return false;
        }
        if let Some(t) = self.store.traces.get_mut(trace_id) {
            t.rehearsals = t.rehearsals.saturating_add(1);
        }
        if self.cut.util_to_strength {
            const STEP: f32 = 0.08;
            let ids = self.store.living_axiom_ids_for(trace_id);
            for id in ids {
                if let Some(a) = self.store.axioms.get_mut(&id) {
                    if a.superseded_by.is_none() {
                        let cap = a.layer.strength_cap();
                        a.strength = (a.strength + STEP).min(cap);
                    }
                }
            }
        }
        true
    }

    /// Keep this hour. Does not open the archive. Next nights decay it more slowly.
    pub fn pin(&mut self, trace_id: &str) -> bool {
        let Some(t) = self.store.traces.get_mut(trace_id) else {
            return false;
        };
        t.permanence = t.permanence.max(0.92);
        t.self_relevance = t.self_relevance.max(0.9);
        t.anchor = t.anchor.max(0.85);
        t.status = crate::core::model::TraceStatus::Active;
        t.access = t.access.max(0.7);
        true
    }

    /// Directed forgetting. The hour stays on the book; default recall skips it.
    pub fn suppress(&mut self, trace_id: &str) -> bool {
        let Some(t) = self.store.traces.get_mut(trace_id) else {
            return false;
        };
        if t.suppressed {
            return true;
        }
        t.suppressed = true;
        t.drifts.push(crate::core::model::DriftEvent {
            kind: crate::core::model::DriftKind::Suppress,
            at: crate::core::model::now_secs(),
            note: "directed forgetting".into(),
            fidelity_delta: 0.0,
            valence_delta: 0.0,
            disgust_delta: 0.0,
        });
        true
    }

    /// Revise the interpretation. The archive and the reality claim stay.
    pub fn reinterpret(&mut self, trace_id: &str, statement: &str) -> bool {
        let statement = statement.trim();
        if statement.is_empty() {
            return false;
        }
        let Some(t) = self.store.traces.get_mut(trace_id) else {
            return false;
        };
        let before = t.interpretation.statement.clone();
        if before == statement {
            return true;
        }
        t.interpretation.statement = statement.to_string();
        t.interpretation.confidence = (t.interpretation.confidence * 0.85).clamp(0.2, 1.0);
        t.semantic.claim = statement.to_string();
        t.record_operation(crate::core::model::MemoryOperation {
            kind: "reinterpret".into(),
            at: crate::core::model::now_secs(),
            source_trace_ids: vec![trace_id.to_string()],
            source_axiom_ids: Vec::new(),
            source_center: t.schema.clone(),
            before,
            after: statement.to_string(),
            confidence: t.interpretation.confidence,
            origin: crate::core::model::EvidenceOrigin::Event,
        });
        true
    }

    pub fn unsuppress(&mut self, trace_id: &str) -> bool {
        let Some(t) = self.store.traces.get_mut(trace_id) else {
            return false;
        };
        t.suppressed = false;
        true
    }
}

/// Keep the frozen fact next to a drifted retelling. Live speak only.
fn pin_happened(narrative: &str, core: &str) -> String {
    let core = core.trim();
    if core.is_empty() {
        return narrative.to_string();
    }
    let n = narrative.to_lowercase();
    let tokens: Vec<&str> = core
        .split_whitespace()
        .filter(|w| w.chars().count() > 3)
        .collect();
    let hit = tokens
        .iter()
        .filter(|t| n.contains(&t.to_lowercase()))
        .count();
    if !tokens.is_empty() && hit * 2 >= tokens.len() {
        return narrative.to_string();
    }
    format!("{narrative}\n(what happened: {core})")
}

fn is_sqlite(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("db") | Some("sqlite") | Some("sqlite3")
    )
}
