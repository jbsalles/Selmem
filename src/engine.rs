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

/// The organ. Encode, reconstruct, sleep. The vault is a dump, not the memory.
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
    semantic_interpreter: Option<Arc<dyn crate::encode::semantic::SemanticInterpreter>>,
    /// Experiment seed. Does not make the night stochastic; it seeds the id stream.
    pub seed: u32,
    pub clock: MemoryClock,
    embedder: Box<dyn Embedder>,
    /// Latest mouth open. A stale close does not record the turn.
    mouth_epoch: u64,
    /// A mouth has opened and not yet closed. A second open is refused.
    mouth_held: bool,
    /// Outside labeler. None means the organ does not invent a proposition.
    scorer: Option<Box<dyn crate::recall::PropositionScorer>>,
    /// Ablation. The mouth does not receive the reading profile.
    drop_stake: bool,
}

/// Runtime LLM endpoint. Key stays on the process; GET only reports a mask.
#[derive(Clone, Debug, Default)]
pub struct LlmBind {
    pub url: String,
    pub model: String,
    pub key: Option<String>,
    /// Which named plug is attached. Empty when rules, or when a raw URL was set.
    pub plug: String,
}

pub struct MouthDraft {
    pub user: String,
    pub memories: Vec<String>,
    pub axioms: Vec<String>,
    pub mood: Mood,
    pub talk: WorkingTalk,
    pub hold: bool,
    pub dump: RetrievalDump,
    epoch: u64,
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
            semantic_interpreter: None,
            embedder: Box::new(HashEmbedder),
            seed: 0,
            clock: MemoryClock::default(),
            mouth_epoch: 0,
            mouth_held: false,
            scorer: None,
            drop_stake: false,
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
                cut: snap.cut,
                recall_tally: RecallTally::default(),
                llm: LlmBind::default(),
                narrator: Arc::new(RuleNarrator),
            semantic_interpreter: None,
                embedder: Box::new(HashEmbedder),
                seed: 0,
                clock: MemoryClock {
                    origin_real: if snap.clock_origin == 0 {
                        crate::core::model::now_secs()
                    } else {
                        snap.clock_origin
                    },
                    jump: snap.clock_jump,
                    scale: snap.clock_scale.max(1),
                    detached: snap.clock_detached,
                },
                mouth_epoch: 0,
                mouth_held: false,
                scorer: None,
            drop_stake: false,
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
            semantic_interpreter: None,
                embedder: Box::new(HashEmbedder),
                seed: 0,
                clock: MemoryClock::default(),
                mouth_epoch: 0,
                mouth_held: false,
                scorer: None,
            drop_stake: false,
            })
        }
    }

    /// Select semantic interpretation independently of behavioral expression.
    pub fn with_semantic_interpreter(
        mut self, interpreter: Box<dyn crate::encode::semantic::SemanticInterpreter>,
    ) -> Self {
        self.semantic_interpreter = Some(Arc::from(interpreter));
        self
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

    /// Ablation. The mouth is not given the reading profile.
    pub fn with_drop_stake(mut self) -> Self {
        self.drop_stake = true;
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
            plug: String::new(),
        };
        Ok(())
    }

    /// Attach a named plug. The key comes from that plug's config, not the vault.
    pub fn set_plug(
        &mut self,
        plug: &str,
        url: &str,
        model: &str,
        key: Option<String>,
    ) -> Result<(), String> {
        self.set_llm(url, model, key)?;
        self.llm.plug = plug.to_string();
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
            crate::persist::sqlite::save(
                path,
                &self.profile,
                &self.mood,
                &self.store,
                self.clock.jump,
                self.clock.scale,
                self.clock.detached,
                self.clock.origin_real,
                &self.cut,
            )
        } else {
            persist::save(
                path,
                &self.profile,
                &self.mood,
                &self.store,
                self.clock.jump,
                self.clock.scale,
                self.clock.detached,
                self.clock.origin_real,
                &self.cut,
            )
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

    /// Independent co-recall cuts. Existing experiments remain off by default.
    pub fn set_associations(&mut self, learn: bool, reconstruct: bool) {
        self.store.associations.learn = learn;
        self.store.associations.reconstruct = reconstruct;
    }

    /// Start a new training episode explicitly instead of waiting for a
    /// conversational gap (10 minutes) or the two-hour episode limit.
    pub fn begin_association_episode(&mut self) {
        let _clock = self.enter_clock();
        self.store
            .associations
            .begin_episode(crate::core::model::now_secs());
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
        if input.semantics.is_none() {
            if let Some(interpreter) = &self.semantic_interpreter {
                match interpreter.interpret_event(input.event) {
                    Ok(semantics) => input.semantics = Some(semantics),
                    Err(reason) => return EncodeDecision {
                        kept: false, score: 0.0, reason: format!("semantic interpretation failed: {reason}"),
                        trace_id: None, archive_id: String::new(), parts: 0, kept_n: 0,
                    },
                }
            }
        }
        if let Some(semantics) = &input.semantics {
            if let Err(reason) = semantics.validate() {
                return EncodeDecision {
                    kept: false, score: 0.0, reason, trace_id: None,
                    archive_id: String::new(), parts: 0, kept_n: 0,
                };
            }
            input.valence = semantics.valence;
            input.arousal = semantics.arousal;
            input.disgust = semantics.disgust;
            input.self_relevance = semantics.self_relevance;
            input.goal_align = semantics.goal_relevance;
            input.attribution = semantics.attribution;
            input.schema = semantics.schema.clone();
        } else {
            // Compatibility path for historical protocols / old narrator implementations.
            encode::interpret(&mut input, &self.mood, &axioms, self.narrator.as_ref());
        }
        let event_owned = input.event.to_string();
        let proposed = encode::propose_split(&event_owned, self.narrator.as_ref());
        let parts = encode::split_event(&event_owned, proposed.as_deref());
        if parts.len() > 1 && input.semantics.is_some() && input.part_semantics.is_none() {
            if let Some(interpreter) = &self.semantic_interpreter {
                let annotations: Result<Vec<_>, String> = parts.iter().map(|part| {
                    let s = interpreter.interpret_event(part)?;
                    s.validate()?;
                    Ok(s)
                }).collect();
                match annotations {
                    Ok(s) => input.part_semantics = Some(s),
                    Err(reason) => return EncodeDecision {
                        kept: false, score: 0.0, reason: format!("semantic interpretation failed: {reason}"),
                        trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
                    },
                }
            } else {
                return EncodeDecision {
                    kept: false, score: 0.0, reason: "multipart annotations require per-slice semantics".into(),
                    trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
                };
            }
        }
        if let Some(annotations) = &input.part_semantics {
            if annotations.len() != parts.len() || annotations.iter().any(|s| s.validate().is_err()) {
                return EncodeDecision {
                    kept: false, score: 0.0, reason: "invalid per-slice annotations".into(),
                    trace_id: None, archive_id: String::new(), parts: parts.len(), kept_n: 0,
                };
            }
        }
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
        let annotated_core = input.semantics.is_some();
        let decision = encode::gate::encode_with_interpreter(
            &mut self.store,
            &self.profile,
            input,
            self.embedder.as_ref(),
            proposed.as_deref(),
            self.semantic_interpreter.as_deref(),
        );
        if !annotated_core {
            encode::maybe_set_core(
                &mut self.store,
                self.narrator.as_ref(),
                &decision,
                input_source,
                &event_owned,
            );
        }
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
        let scorer = self.scorer.take();
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
            scorer.as_deref().unwrap_or(&crate::recall::NullScorer),
        );
        self.scorer = scorer;
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
        let scorer = self.scorer.take();
        let kind = crate::dream::evaluate_budget(&self.store, &self.profile).kind;
        let report = dream::dream_kind(
            &mut self.store,
            &self.profile,
            self.narrator.as_ref(),
            self.embedder.as_ref(),
            self.cut,
            kind,
            scorer.as_deref().unwrap_or(&crate::recall::NullScorer),
        );
        self.scorer = scorer;
        report
    }

    /// Full night regardless of budget. Lab path.
    pub fn sleep_deep(&mut self) -> DreamReport {
        let _clock = self.enter_clock();
        self.commit_talk();
        crate::persist::prune_orphaned_archives(&mut self.store);
        let scorer = self.scorer.take();
        let report = dream::dream_kind(
            &mut self.store,
            &self.profile,
            self.narrator.as_ref(),
            self.embedder.as_ref(),
            self.cut,
            crate::dream::NightKind::Deep,
            scorer.as_deref().unwrap_or(&crate::recall::NullScorer),
        );
        self.scorer = scorer;
        report
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
        let _clock = self.enter_clock();
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
        let _clock = self.enter_clock();
        let talk_ids: Vec<String> = self
            .store
            .traces
            .values()
            .filter(|t| {
                t.permanence < 0.80
                    && t.source == "talk"
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
        let _clock = self.enter_clock();
        self.talk.refresh();
    }

    pub fn mouth_held(&self) -> bool {
        self.mouth_held
    }

    pub fn open_mouth(&mut self, user: &str) -> MouthDraft {
        self.open_mouth_inner(user, true, RecallBias::Observed, &[], false)
    }

    pub fn close_mouth(&mut self, draft: &MouthDraft, reply: &str) {
        let _clock = self.enter_clock();
        if draft.epoch != self.mouth_epoch {
            return;
        }
        self.mouth_held = false;
        if draft.hold {
            self.talk.record(&draft.user, reply);
        }
    }

    /// Release a failed mouth call without recording its fallback or error as
    /// an assistant turn. Recall write-backs already performed are retained.
    pub fn cancel_mouth(&mut self, draft: &MouthDraft) {
        if draft.epoch == self.mouth_epoch {
            self.mouth_held = false;
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
        let profile = if self.drop_stake {
            String::new()
        } else {
            crate::recall::reading::ReadingProfile::for_query(&self.store, &draft.mood, &draft.user, &draft.dump.selected).render()
        };
        let reply = self.narrator.reply_disposed(
            &draft.user,
            &draft.memories,
            &draft.axioms,
            &draft.mood,
            &draft.talk,
            &profile,
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
        self.mouth_held = true;
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
        let scoped_reading = crate::recall::reading::ReadingProfile::for_query(
            &self.store, &self.mood, user, &dump.selected);
        let external_axiom_allowed = |a: &crate::core::model::IdentityAxiom| {
            let external = a.support_trace_ids.iter().filter_map(|id| self.store.traces.get(id))
                .any(|t| t.attribution == crate::core::model::Attribution::External);
            !external || scoped_reading.axioms.contains(&a.statement)
        };
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
                    .filter(|a| external_axiom_allowed(a))
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
            let axioms: Vec<String> = self
                .who_am_i()
                .into_iter()
                .filter(|a| {
                    if !external_axiom_allowed(a) { return false; }
                    if drop_ax
                        && crate::recall::retrieve::axiom_supported_by_lineage(
                            &a.support_trace_ids,
                            a.schema.as_deref(),
                            &lineage,
                            &lineage_schemas,
                        )
                    {
                        return false;
                    }
                    // Only the strongest belief may color a probe that does not name the hour.
                    // Every other belief stays behind the probe's own tokens, or an unrelated stake colors the offer.
                    let strongest_belief = self
                        .who_am_i()
                        .into_iter()
                        .find(|a| a.layer == crate::core::model::AxiomLayer::Belief)
                        .map(|a| a.id.clone());
                    if a.layer == crate::core::model::AxiomLayer::Belief
                        && strongest_belief.as_deref() == Some(a.id.as_str())
                    {
                        return true;
                    }
                    let mut probe_cloud = std::collections::HashMap::new();
                    for token in crate::encode::scoring::token_set(user) {
                        probe_cloud.insert(token, 1.0);
                    }
                    crate::recall::retrieve::statement_anchored(&probe_cloud, &a.statement)
                })
                .map(|a| a.statement.clone())
                .collect();
            let memories: Vec<String> = if axioms_only {
                Vec::new()
            } else {
                recalled.into_iter().map(|r| r.narrative).collect()
            };
            (memories, axioms)
        };
        let mut axioms = axioms;
        for interpretation in scoped_reading.axioms.iter().filter(|s| s.starts_with("Tentative observed interpretation;")) {
            if !axioms.contains(interpretation) { axioms.push(interpretation.clone()); }
        }
        MouthDraft {
            user: user.to_string(),
            memories,
            axioms,
            mood: self.mood.clone(),
            talk: talk.clone(),
            hold,
            dump,
            epoch: {
                self.mouth_epoch = self.mouth_epoch.saturating_add(1);
                self.mouth_epoch
            },
        }
    }

    pub fn scorer_name(&self) -> String {
        self.scorer.as_ref().map(|s| s.name().to_string()).unwrap_or_else(|| "null".into())
    }

    pub fn with_scorer(mut self, scorer: Box<dyn crate::recall::PropositionScorer>) -> Self {
        self.scorer = Some(scorer);
        self
    }

    /// Ask the outside scorer. Stores the label. Does not change gist, core, or claim.
    pub fn score_against_claim(&mut self, trace_id: &str, sentence: &str) -> crate::recall::PropositionLabel {
        let claim = self
            .store
            .traces
            .get(trace_id)
            .map(|t| t.reality.claim.clone())
            .unwrap_or_default();
        let label = match self.scorer.as_ref() {
            Some(s) => s.score(&claim, sentence),
            None => crate::recall::PropositionLabel::Unknown,
        };
        if let Some(t) = self.store.traces.get(trace_id) {
            let _ = t;
            self.store.measures.push((trace_id.to_string(), label.token().into()));
        }
        label
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

