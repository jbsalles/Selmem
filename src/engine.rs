//! The organ the rest of the crate talks to.
//!
//! Typical loop:
//! `live_with` (encode) → `remember` / `speak` (reconstruct) → `sleep` (consolidate).
//! Persistence (`save` / `open`) is a vault. The narrator never opens it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::model::{
    ClockGuard, IdentityAxiom, MemoryClock, Mood, OrganCut, RecallTally, RecalledMemory,
};
use crate::core::profile::EntityProfile;
use crate::core::store::MemoryStore;
use crate::core::talk::WorkingTalk;
use crate::dream::{self, DreamReport};
use crate::encode::embed::{Embedder, HashEmbedder};
use crate::encode::{self, EncodeDecision, EncodeInput};
use crate::persist;
use crate::recall;
use crate::recall::narrator::{Narrator, RuleNarrator};
use crate::recall::{RecallBias, RecallWrite, RetrievalDump};

#[path = "engine/ingest.rs"]
mod ingest;
#[path = "engine/mouth.rs"]
mod mouth;

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
    memory_context_options: crate::MemoryContextOptions,
    query_scope: Vec<crate::ContextFacet>,
    query_context: Option<String>,
    query_self_cues: bool,
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
    pub memory_context: crate::MemoryContext,
    pub memory_context_options: crate::MemoryContextOptions,
    pub reading: String,
    epoch: u64,
}

impl MouthDraft {
    pub fn memory_reply(&self) -> crate::MemoryReply<'_> {
        crate::MemoryReply {
            user: &self.user,
            memories: &self.memories,
            axioms: &self.axioms,
            mood: &self.mood,
            talk: &self.talk,
            reading: &self.reading,
            context: &self.memory_context,
            options: &self.memory_context_options,
        }
    }
    pub fn reply(&self, narrator: &dyn Narrator) -> String {
        narrator.reply_with_context(&self.memory_reply())
    }
    pub fn delivery_audit_json(&self, narrator: &dyn Narrator) -> String {
        self.memory_reply()
            .audit_json(narrator.supports_memory_context())
    }
}
impl SelectiveMemory {
    /// Host trust configuration, never inferred from text. Reapply after loading.
    pub fn set_normative_authorities(&mut self, sources: &[&str]) -> Result<(), String> {
        self.store.normative.set_authorities(sources)
    }
    /// Bind caller-validated semantics to an admitted external trace.
    /// The host must authenticate the source and validate its annotation.
    pub fn register_normative(
        &mut self,
        trace_id: &str,
        record: crate::NormRecord,
    ) -> Result<(), String> {
        let t = self
            .store
            .traces
            .get(trace_id)
            .ok_or("carrier trace missing")?;
        if t.source != record.source
            || t.attribution != crate::Attribution::External
            || t.suppressed
            || t.status != crate::TraceStatus::Active
        {
            return Err("normative carrier must be active, external and source-matched".into());
        }
        self.store.normative.register(trace_id, record)
    }
    /// Opt-in normative focus: complete living rule families under the context budget.
    /// Does not expose ordinary episodes, dispositions or working history in this draft.
    /// The user's current request remains available. Ordinary open_mouth is unchanged.
    pub fn open_mouth_normative(
        &mut self,
        user: &str,
        scope: &crate::NormScope,
        action: &str,
    ) -> Result<MouthDraft, String> {
        if self.memory_context_options.format != crate::MemoryContextFormat::Structured
            || !self.memory_context_options.episodes
        {
            return Err("normative focus requires structured episodic delivery".into());
        }
        if self.mouth_held {
            return Err("a mouth is already open".into());
        }
        let context = self.store.normative.compose(
            &self.store,
            scope,
            action,
            self.memory_context_options.words,
        )?;
        let memory_context = crate::MemoryContext {
            agent_name: Some(self.profile.name.clone()),
            normative: Some(context),
            ..Default::default()
        }
        .filtered(&self.memory_context_options);
        if self
            .memory_context_options
            .token_budget
            .is_some_and(|budget| {
                (self.memory_context_options.token_counter)(&memory_context.to_json()) > budget
            })
        {
            return Err("token budget cannot fit the normative incomplete-context marker".into());
        }
        let mut draft = self.open_mouth(user);
        draft.memory_context = memory_context;
        draft.memories = vec![draft
            .memory_context
            .normative
            .as_ref()
            .unwrap()
            .text
            .clone()];
        draft.axioms.clear();
        draft.reading.clear();
        draft.talk = WorkingTalk::default();
        Ok(draft)
    }

    pub fn set_memory_context_options(
        &mut self,
        options: crate::MemoryContextOptions,
    ) -> Result<(), String> {
        options.validate()?;
        self.memory_context_options = options;
        Ok(())
    }
    pub fn memory_context_options(&self) -> &crate::MemoryContextOptions {
        &self.memory_context_options
    }
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
            memory_context_options: crate::MemoryContextOptions::default(),
            query_scope: Vec::new(),
            query_context: None,
            query_self_cues: false,
        }
    }

    pub fn open(path: impl AsRef<Path>, profile: EntityProfile) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let mut snap = if is_sqlite(&path) {
                crate::persist::sqlite::load(&path)?
            } else {
                persist::load(&path)?
            };
            // Runtime defaults to hash. Never compare saved remote vectors with hash queries.
            let hash = HashEmbedder;
            if snap.store.embedding_space.as_deref() != Some(hash.space_id().as_str()) {
                for t in snap.store.traces.values_mut() {
                    t.embedding = hash.embed(&t.gist);
                }
                snap.store.embedding_space = Some(hash.space_id());
            }
            let mut clock = MemoryClock {
                origin_real: if snap.clock_origin == 0 {
                    MemoryClock::default().origin_real
                } else {
                    snap.clock_origin
                },
                jump: snap.clock_jump,
                scale: snap.clock_scale.clamp(1, 200),
                detached: true,
            };
            // Legacy attached saves lost the process jump. Preserve at least their recorded timeline.
            if snap.clock_origin == 0 || !snap.clock_detached {
                let latest = snap
                    .store
                    .traces
                    .values()
                    .flat_map(|t| {
                        std::iter::once(t.created_at)
                            .chain(t.last_recalled_at)
                            .chain(t.last_consolidated_at)
                            .chain(t.drifts.iter().map(|d| d.at))
                            .chain(t.operations.iter().map(|o| o.at))
                    })
                    .chain(snap.store.axioms.values().map(|a| a.created_at))
                    .chain(snap.store.last_deep_at)
                    .max()
                    .unwrap_or(0);
                clock.jump = clock
                    .jump
                    .saturating_add(latest.saturating_sub(clock.now()));
            }
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
                clock,
                mouth_epoch: 0,
                mouth_held: false,
                scorer: None,
                drop_stake: false,
                memory_context_options: crate::MemoryContextOptions::default(),
                query_scope: Vec::new(),
                query_context: None,
                query_self_cues: false,
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
                memory_context_options: crate::MemoryContextOptions::default(),
                query_scope: Vec::new(),
                query_context: None,
                query_self_cues: false,
            })
        }
    }

    /// Select semantic interpretation independently of behavioral expression.
    pub fn with_semantic_interpreter(
        mut self,
        interpreter: Box<dyn crate::encode::semantic::SemanticInterpreter>,
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
            .ok_or_else(|| "llm url must be http:// or https://".to_string())?
            .with_choice_options(crate::ChoiceOptions::from_config(crate::Config::get())?);
        let options = crate::MemoryContextOptions::from_config(crate::Config::get())?;
        self.narrator = Arc::new(n);
        self.memory_context_options = options;
        self.llm = LlmBind {
            url: url.to_string(),
            model: if model.is_empty() {
                "llama3".into()
            } else {
                model.into()
            },
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
        // A backend switch is an explicit reindex, including equal-dimensional spaces.
        self.store.reindex(embedder.as_ref());
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

    /// Learn bounded support from caller-identified observations, not recalls.
    pub fn set_observation_evidence(&mut self, enabled: bool) {
        self.store.observation_evidence.enabled = enabled;
    }

    fn enter_clock(&self) -> ClockGuard {
        ClockGuard::push(self.clock.clone())
    }

    pub fn live_with(&mut self, input: EncodeInput<'_>) -> EncodeDecision {
        let _clock = self.enter_clock();
        self.ingest(input, true)
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
            rank(b.layer).cmp(&rank(a.layer)).then(
                b.strength
                    .partial_cmp(&a.strength)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        axioms
    }

    pub fn speak(&mut self, user: &str) -> String {
        self.speak_inner(user, true, RecallBias::Observed, &[], false)
            .0
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
            // A declared interlocutor name is a fallible external relationship trace.
            let named = declared_human_name(chunk)
                .map(|name| format!("The human speaking with me said their name is {name}."));
            let event = named.as_deref().unwrap_or(chunk);
            if self
                .store
                .archives
                .values()
                .any(|a| a.source == "talk" && a.verbatim == event)
            {
                continue;
            }
            let (v, a, d, schema) = encode::affect::guess(event);
            let mut ev = EncodeInput::new(event);
            ev.source = "talk";
            if named.is_some() {
                ev.channel = crate::core::model::Channel::Selfhood;
                ev.attribution = crate::core::model::Attribution::External;
                ev.schema = Some("interlocutor-name".into());
                ev.permanence = 0.58;
            } else {
                ev.channel = crate::core::model::Channel::World;
            }
            ev.valence = v;
            ev.arousal = a;
            ev.disgust = d;
            ev.self_relevance = if named.is_some() { 0.75 } else { 0.55 };
            ev.utility = if named.is_some() { 0.70 } else { 0.55 };
            if named.is_none() {
                ev.permanence = 0.35;
                ev.schema = schema;
            }
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
            .filter(|t| t.permanence < 0.80 && t.source == "talk")
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

    /// The caller attests these contextual facts for this ONE response.
    /// Untrusted/free-form prose is never silently interpreted as an observed fact.
    pub fn open_mouth_scoped(
        &mut self,
        user: &str,
        facets: &[crate::ContextFacet],
    ) -> Result<MouthDraft, String> {
        crate::core::self_model::validate_context_facets(facets)?;
        self.query_scope = facets.to_vec();
        Ok(self.open_mouth(user))
    }
    /// Caller-attested application context, separate from user prose and scope facts.
    /// Matching native self-view statements can cue episodic recall; expectations,
    /// support IDs and sealed archives are never used as retrieval instructions.
    pub fn open_mouth_in_context(
        &mut self,
        user: &str,
        context: &str,
        facets: &[crate::ContextFacet],
    ) -> Result<MouthDraft, String> {
        self.open_mouth_context_inner(user, context, facets, false)
    }
    /// Opt-in top-down retrieval: native applicable self-view gists cue episodes.
    /// This is a distinct retrieval route, not the original lexical baseline.
    pub fn open_mouth_in_context_cued(
        &mut self,
        user: &str,
        context: &str,
        facets: &[crate::ContextFacet],
    ) -> Result<MouthDraft, String> {
        self.open_mouth_context_inner(user, context, facets, true)
    }
    fn open_mouth_context_inner(
        &mut self,
        user: &str,
        context: &str,
        facets: &[crate::ContextFacet],
        cued: bool,
    ) -> Result<MouthDraft, String> {
        crate::core::self_model::validate_context_facets(facets)?;
        if context.trim().is_empty() || context.len() > 128 || context.chars().any(char::is_control)
        {
            return Err("invalid recall context".into());
        }
        self.query_context = Some(context.trim().to_string());
        self.query_self_cues = cued;
        self.query_scope = facets.to_vec();
        Ok(self.open_mouth(user))
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
        let failures_before = self.narrator.failure_log().failures;
        let reply = draft.reply(self.narrator.as_ref());
        if self.narrator.failure_log().failures > failures_before
            || reply.starts_with("[llm-error]")
        {
            self.cancel_mouth(&draft);
        } else {
            self.close_mouth(&draft, &reply);
        }
        (reply, draft.dump)
    }

    pub fn scorer_name(&self) -> String {
        self.scorer
            .as_ref()
            .map(|s| s.name().to_string())
            .unwrap_or_else(|| "null".into())
    }

    pub fn with_scorer(mut self, scorer: Box<dyn crate::recall::PropositionScorer>) -> Self {
        self.scorer = Some(scorer);
        self
    }

    /// Ask the outside scorer. Stores the label. Does not change gist, core, or claim.
    pub fn score_against_claim(
        &mut self,
        trace_id: &str,
        sentence: &str,
    ) -> crate::recall::PropositionLabel {
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
            self.store
                .measures
                .push((trace_id.to_string(), label.token().into()));
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
        let _clock = self.enter_clock();
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
        let _clock = self.enter_clock();
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

    /// Each UI sleep creates a fallible autobiographical episode, not an exact counter.
    pub fn record_sleep_memory(&mut self) -> Option<String> {
        let event = format!(
            "I slept through a night and woke again (virtual time {}).",
            self.clock.now()
        );
        let mut ev = EncodeInput::new(&event);
        ev.source = "sleep";
        ev.personal.evidence = crate::SelfEvidence::Observation;
        ev.channel = crate::core::model::Channel::Selfhood;
        ev.attribution = crate::core::model::Attribution::Internal;
        ev.schema = Some("night-recollection".into());
        ev.self_relevance = 0.78;
        ev.utility = 0.72;
        ev.arousal = 0.48;
        ev.permanence = 0.48;
        self.live_with(ev).trace_id
    }

    pub fn remembered_nights(&self) -> usize {
        self.store
            .traces
            .values()
            .filter(|t| {
                t.source == "sleep"
                    && !t.suppressed
                    && t.status == crate::core::model::TraceStatus::Active
                    && t.access >= 0.15
            })
            .count()
    }
}

fn asks_about_human_name(query: &str) -> bool {
    let q = query.to_lowercase();
    [
        "comment je m'appelle",
        "comment je m’appelle",
        "quel est mon nom",
        "quel est mon prenom",
        "quel est mon prénom",
        "what is my name",
        "what's my name",
        "do you know my name",
    ]
    .iter()
    .any(|phrase| q.contains(phrase))
}

fn asks_about_nights(query: &str) -> bool {
    let q = query.to_lowercase();
    [
        "nuit",
        "dormi",
        "dormis",
        "dormir",
        "sommeil",
        "sleep",
        "night",
        "how long",
        "combien de jour",
        "how many day",
    ]
    .iter()
    .any(|word| q.contains(word))
}

fn declared_human_name(message: &str) -> Option<String> {
    let raw = message.trim();
    let lower = raw.to_lowercase();
    for prefix in [
        "je m'appelle ",
        "je m’appelle ",
        "mon prénom est ",
        "mon prenom est ",
        "my name is ",
        "call me ",
    ] {
        if lower.starts_with(prefix) {
            let candidate = raw.get(prefix.len()..)?.split_whitespace().next()?;
            let candidate =
                candidate.trim_matches(|c: char| !c.is_alphabetic() && c != '-' && c != '\'');
            if (2..=32).contains(&candidate.chars().count())
                && candidate.chars().any(char::is_alphabetic)
            {
                return Some(candidate.to_string());
            }
        }
    }
    None
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
