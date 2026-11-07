//! Bounded observation support attached to living traces, never read from archives.
use std::collections::{BTreeMap, BTreeSet};
use super::{model::{Channel, TraceStatus, MemoryOperation, EvidenceOrigin, now_secs}, store::MemoryStore};
use crate::recall::{PropositionScorer, PropositionLabel};
const MAX_GROUPS: usize = 128;
const MAX_OBSERVATIONS: usize = 64;
const MAX_VARIANTS: usize = 8;
const MAX_TEXT: usize = 1024;
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObservationGroup {
    pub source: String,
    pub channel: String,
    pub primary_claim: String,
    /// Observation identities are caller provenance, not inferred independence.
    pub observations: BTreeMap<String, String>,
    pub updated_at: u64,
}
impl ObservationGroup {
    pub fn counts(&self) -> BTreeMap<String, usize> {
        let mut counts=BTreeMap::new();
        for claim in self.observations.values(){*counts.entry(claim.clone()).or_insert(0)+=1;}
        counts
    }
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObservationEvidence {
    /// Explicit cut for experiments. Old memories and new stores default off.
    pub enabled: bool,
    pub groups: BTreeMap<String, ObservationGroup>,
}
#[derive(Debug, PartialEq)]
pub enum ObservationUpdate { None, Duplicate, Conflict, Capacity, Updated(String, bool) }
fn normalized(text:&str)->String {text.split_whitespace().collect::<Vec<_>>().join(" ")}
fn eligible(store:&MemoryStore,id:&str)->bool {
    store.traces.get(id).is_some_and(|t|!t.suppressed && t.status==TraceStatus::Active && !t.channel.verbatim())
}
impl ObservationEvidence {
    pub fn seed(&mut self, trace_id:&str,source:&str,channel:Channel,observation:&str,claim:&str){
        if !self.enabled || channel.verbatim() || observation.is_empty() || observation.len()>MAX_TEXT || source.is_empty() || source.len()>MAX_TEXT || claim.is_empty() || claim.len()>MAX_TEXT || self.groups.len()>=MAX_GROUPS {return;}
        if self.groups.values().any(|g|g.source==source && g.observations.contains_key(observation)){return;}
        self.groups.entry(trace_id.into()).or_insert_with(||ObservationGroup {
            source:source.into(),channel:format!("{channel:?}"),primary_claim:normalized(claim),
            observations:BTreeMap::from([(observation.into(),normalized(claim))]),updated_at:now_secs(),
        });
    }
    /// Metadata accompanies the selected trace; it is not a reconstructed event.
    pub fn render(&self,store:&MemoryStore,trace_id:&str)->Option<String>{
        if !self.enabled || !eligible(store,trace_id){return None;}
        let group=self.groups.get(trace_id)?;
        if group.observations.len()<2{return None;}
        let claims=group.counts().into_iter().map(|(claim,n)|format!("{n} distinct observation IDs support this observed claim: {claim}")).collect::<Vec<_>>().join("\n");
        Some(format!("Observation support metadata (not additional recalled episodes). Counts include the initial carrier observation: do not add the narrative as another observation. Counts use caller-provided IDs; statistical independence is not verified. Different claims remain separate and do not erase earlier observations.\n{claims}"))
    }

    /// Validate before mutating either carrier. Reject lossy or invalid unions.
    pub fn can_transfer_on_merge(&self, keep:&str, other:&str)->bool {
        if keep==other {return false;}
        let (Some(dst),Some(src))=(self.groups.get(keep),self.groups.get(other)) else{return true;};
        if dst.source!=src.source || dst.channel!=src.channel{return false;}
        let mut union=dst.observations.clone();
        for (id,claim) in &src.observations {
            if union.get(id).is_some_and(|old|old!=claim){return false;}
            union.insert(id.clone(),claim.clone());
        }
        union.len()<=MAX_OBSERVATIONS && union.values().collect::<BTreeSet<_>>().len()<=MAX_VARIANTS
    }
    /// Metadata transfer is independent of the runtime signal switch.
    /// Incompatible groups remain intact; merge must check eligibility first.
    pub fn transfer_on_merge(&mut self, keep:&str, other:&str) {
        if !self.can_transfer_on_merge(keep,other){return;}
        let Some(src)=self.groups.remove(other) else{return;};
        match self.groups.get_mut(keep) {
            None=>{self.groups.insert(keep.into(),src);},
            Some(dst)=>{dst.observations.extend(src.observations);dst.updated_at=dst.updated_at.max(src.updated_at);}
        }
    }

    /// Saturated reconstructive weight from distinct primary-claim support only.
    /// Contradictory observations remain available as separate evidence, but
    /// must not increase the positive recall signal of the carrier claim.
    /// Zero when disabled, ineligible, or fewer than two positive observations.
    /// Never rewrites core or gist. Used only to modulate recall and optional signal.
    pub fn support_weight(&self, store: &MemoryStore, trace_id: &str) -> f32 {
        if !self.enabled || !eligible(store, trace_id) {
            return 0.0;
        }
        let Some(group) = self.groups.get(trace_id) else {
            return 0.0;
        };
        let n = group.observations.values()
            .filter(|claim| *claim == &group.primary_claim)
            .count();
        if n < 2 {
            return 0.0;
        }
        // log1p saturates: ~0.16 at 2, ~0.28 at 4, ~0.42 at 8, ~0.55 at 16, ~0.75 at 64.
        (0.22 * (1.0 + n as f32).ln()).min(0.75)
    }
    pub fn encode(&self)->String {
        let mut words=vec!["1".into(),if self.enabled{"1"}else{"0"}.into(),self.groups.len().to_string()];
        for (id,g) in &self.groups {
            words.extend([hex(id),hex(&g.source),hex(&g.channel),hex(&g.primary_claim),g.updated_at.to_string(),g.observations.len().to_string()]);
            for (observation,claim) in &g.observations{words.extend([hex(observation),hex(claim)]);}
        }
        words.join(" ")
    }
    pub fn decode(data:&str)->Result<Self,String>{
        let mut p=data.split_whitespace();
        if p.next()!=Some("1"){return Err("unsupported observation evidence version".into());}
        let enabled=match p.next(){Some("0")=>false,Some("1")=>true,_=>return Err("invalid evidence flag".into())};
        let count=number(p.next())?;if count>MAX_GROUPS{return Err("evidence groups overflow".into());}
        let mut groups=BTreeMap::new();let mut identities=BTreeSet::new();
        for _ in 0..count {
            let id=unhex(p.next())?;let source=unhex(p.next())?;let channel=unhex(p.next())?;let primary_claim=unhex(p.next())?;
            let updated_at=p.next().ok_or("missing evidence timestamp")?.parse::<u64>().map_err(|_|"invalid evidence timestamp")?;
            let n=number(p.next())?;if n==0 || n>MAX_OBSERVATIONS{return Err("invalid observation count".into());}
            let mut observations=BTreeMap::new();
            for _ in 0..n{let o=unhex(p.next())?;let claim=unhex(p.next())?;
                if !identities.insert((source.clone(),o.clone())) || observations.insert(o,claim).is_some(){return Err("duplicate observation identity".into());}
            }
            let variants:BTreeSet<_>=observations.values().collect();if variants.len()>MAX_VARIANTS{return Err("evidence variants overflow".into());}
            if !observations.values().any(|c|c==&primary_claim){return Err("missing primary evidence claim".into());}
            if groups.insert(id,ObservationGroup{source,channel,primary_claim,observations,updated_at}).is_some(){return Err("duplicate evidence group".into());}
        }
        if p.next().is_some(){return Err("extra observation evidence data".into());}
        Ok(Self{enabled,groups})
    }
}
fn number(s:Option<&str>)->Result<usize,String>{s.ok_or("missing evidence count")?.parse().map_err(|_|"invalid evidence count".into())}
fn hex(s:&str)->String{s.as_bytes().iter().map(|b|format!("{b:02x}")).collect()}
fn unhex(s:Option<&str>)->Result<String,String>{
    let s=s.ok_or("missing evidence text")?;if s.is_empty() || s.len()>MAX_TEXT*2 || s.len()%2!=0{return Err("invalid evidence text length".into());}
    let mut bytes=Vec::new();for pair in s.as_bytes().chunks(2){let h=std::str::from_utf8(pair).map_err(|_|"invalid evidence hex")?;bytes.push(u8::from_str_radix(h,16).map_err(|_|"invalid evidence hex")?);}
    String::from_utf8(bytes).map_err(|_|"invalid evidence UTF-8".into())
}
/// Exact repeats need no model. Different claims are attached only when an
/// outside scorer judges a near candidate contradictory, never via embeddings.
/// Unknown and merely entailed paraphrases remain on the ordinary salience path.
pub fn update(store:&mut MemoryStore,source:&str,channel:Channel,observation:Option<&str>,claim:&str,scorer:Option<&dyn PropositionScorer>)->ObservationUpdate{
    if !store.observation_evidence.enabled || channel.verbatim(){return ObservationUpdate::None;}
    let Some(observation)=observation.filter(|s|!s.is_empty() && s.len()<=MAX_TEXT)else{return ObservationUpdate::None;};
    let claim=normalized(claim);
    if claim.is_empty() || claim.len()>MAX_TEXT || source.is_empty() || source.len()>MAX_TEXT{return ObservationUpdate::None;}
    let channel=format!("{channel:?}");
    let mut candidates=Vec::new();
    for (id,g) in &store.observation_evidence.groups{
        if g.source!=source {continue;}
        if let Some(old)=g.observations.get(observation){return if old==&claim{ObservationUpdate::Duplicate}else{ObservationUpdate::Conflict};}
        if !eligible(store,id){continue;}
        if g.channel!=channel{continue;}
        let counts=g.counts();
        if counts.contains_key(&claim){candidates.push((id.clone(),1.0,false));continue;}
        let similarity=counts.keys().map(|c|crate::encode::scoring::lexical_similarity(c,&claim)).fold(0.0_f32,f32::max);
        if similarity>=0.70{candidates.push((id.clone(),similarity,true));}
    }
    candidates.sort_by(|a,b|b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut selected=None;
    for (id,_,different) in candidates.into_iter().take(3){
        if !different{selected=Some((id,false));break;}
        let Some(scorer)=scorer else{continue;};
        let group=&store.observation_evidence.groups[&id];
        if group.counts().keys().any(|c|scorer.score(c,&claim)==PropositionLabel::Contradict){selected=Some((id,true));break;}
    }
    let Some((id,_))=selected else{return ObservationUpdate::None;};
    let group=store.observation_evidence.groups.get_mut(&id).unwrap();
    let counter=claim!=group.primary_claim;
    if group.observations.len()>=MAX_OBSERVATIONS || (!group.counts().contains_key(&claim) && group.counts().len()>=MAX_VARIANTS){return ObservationUpdate::Capacity;}
    group.observations.insert(observation.into(),claim.clone());group.updated_at=now_secs();
    let trace=store.traces.get_mut(&id).unwrap();
    // A contradiction is retained, not converted into positive support/confidence.
    if !counter{trace.anchor=(trace.anchor+0.02*(1.0-trace.anchor)).clamp(0.0,1.0);}
    trace.record_operation(MemoryOperation{kind:if counter{"counter-observation"}else{"observation-support"}.into(),at:now_secs(),source_trace_ids:vec![id.clone()],source_axiom_ids:Vec::new(),source_center:None,before:String::new(),after:format!("source={source}; observation={observation}; claim={claim}"),confidence:trace.confidence,origin:EvidenceOrigin::Event});
    ObservationUpdate::Updated(id,counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SelectiveMemory, EntityProfile, EncodeInput, RecallWrite, RecallBias};
    fn memory()->SelectiveMemory{let mut m=SelectiveMemory::new(EntityProfile::new("test"));m.set_observation_evidence(true);m}
    fn offer(m:&mut SelectiveMemory,id:&str,event:&str)->crate::EncodeDecision{
        let mut input=EncodeInput::new(event);input.source="sensor";input.observation_id=Some(id);input.permanence=0.9;
        m.live_with(input)
    }
    fn count(m:&SelectiveMemory)->usize{m.store.observation_evidence.groups.values().map(|g|g.observations.len()).sum()}
    struct Contrast;
    impl PropositionScorer for Contrast {
        fn score(&self,a:&str,b:&str)->PropositionLabel {
            if a.contains("six batteries") && b.contains("seven batteries"){PropositionLabel::Contradict}else{PropositionLabel::Unknown}
        }
        fn name(&self)->&str{"controlled-contrast"}
    }
    #[test]fn independent_ids_accumulate_without_trace_or_archive_copies(){
        let mut m=memory();let claim="Mira supplied six batteries to the workshop.";
        let first=offer(&mut m,"1",claim);assert!(first.kept);let id=first.trace_id.unwrap();let core=m.store.traces[&id].core.clone();
        for i in 2..=12{let d=offer(&mut m,&i.to_string(),claim);assert!(d.kept);assert_eq!(d.kept_n,0);assert_eq!(d.trace_id.as_deref(),Some(id.as_str()));}
        assert_eq!(m.store.traces.len(),1);assert_eq!(m.store.archives.len(),1);assert_eq!(count(&m),12);assert_eq!(m.store.traces[&id].core,core);
        let before=m.store.observation_evidence.clone();
        let (recall,_)=m.remember_with("Mira batteries",RecallWrite::ReadOnly,RecallBias::Observed,&[]);
        assert!(recall.iter().any(|r|r.narrative.contains("12 distinct observation IDs")));
        assert_eq!(m.store.observation_evidence,before);
        for _ in 0..5{m.remember("Mira batteries");}assert_eq!(count(&m),12);
        for t in m.store.traces.values(){assert!(!t.gist.contains("distinct observation IDs"));assert!(!t.core.contains("distinct observation IDs"));}
    }
    #[test]fn replay_conflicting_identity_and_unidentified_calls_do_not_add_support(){
        let mut m=memory();let claim="Mira supplied six batteries to the workshop.";offer(&mut m,"same",claim);
        assert!(!offer(&mut m,"same",claim).kept);
        assert!(!offer(&mut m,"same","Mira supplied seven batteries to the workshop.").kept);assert_eq!(count(&m),1);
        let mut input=EncodeInput::new(claim);input.source="sensor";m.live_with(input);assert_eq!(count(&m),1);
    }
    #[test]fn contradictions_remain_separate_and_unknown_or_other_sources_do_not_count(){
        let mut m=memory().with_scorer(Box::new(Contrast));let original="Mira supplied six batteries to the workshop.";
        let id=offer(&mut m,"1",original).trace_id.unwrap();let frozen=m.store.traces[&id].core.clone();
        let original_anchor=m.store.traces[&id].anchor;
        let counter=offer(&mut m,"2","Mira supplied seven batteries to the workshop.");assert!(counter.kept);assert!(counter.reason.contains("counter-observation"));assert_eq!(counter.kept_n,0);
        assert_eq!(m.store.observation_evidence.groups[&id].counts().len(),2);assert_eq!(m.store.traces[&id].core,frozen);
        assert!(offer(&mut m,"counter-repeat","Mira supplied seven batteries to the workshop.").reason.contains("counter-observation"));
        assert_eq!(m.store.traces[&id].anchor,original_anchor);
        let state=m.store.observation_evidence.clone();
        assert_eq!(update(&mut m.store,"different-source",Channel::Selfhood,Some("3"),original,None),ObservationUpdate::None);
        assert_eq!(update(&mut m.store,"sensor",Channel::Selfhood,Some("3"),"Luma supplied six batteries to the workshop.",Some(&Contrast)),ObservationUpdate::None);
        assert_eq!(m.store.observation_evidence,state);
    }
    #[test]fn bounded_capacity_refuses_untracked_support_instead_of_double_counting(){
        let mut m=memory();let claim="Mira supplied six batteries to the workshop.";offer(&mut m,"0",claim);
        for i in 1..MAX_OBSERVATIONS{assert!(offer(&mut m,&i.to_string(),claim).kept);}
        assert!(!offer(&mut m,"overflow",claim).kept);assert_eq!(count(&m),MAX_OBSERVATIONS);assert_eq!(m.store.traces.len(),1);
        assert!(!offer(&mut m,"0",claim).kept);assert_eq!(count(&m),MAX_OBSERVATIONS);
    }
    #[test]fn file_and_sqlite_roundtrip_keep_dedup_and_counts_without_archive_reads(){
        for extension in ["vault","sqlite"]{
            let mut m=memory();let first=offer(&mut m,"a","Mira supplied six batteries to the workshop.");offer(&mut m,"b","Mira supplied six batteries to the workshop.");
            let path=std::env::temp_dir().join(format!("selmem-evidence-{}-{}.{extension}",std::process::id(),first.trace_id.unwrap()));m.path=Some(path.clone());m.save().unwrap();
            let expected=m.store.observation_evidence.clone();let mut restored=SelectiveMemory::open(&path,EntityProfile::new("ignored")).unwrap();assert_eq!(restored.store.observation_evidence,expected);
            restored.store.archives.clear();assert!(!offer(&mut restored,"a","Mira supplied six batteries to the workshop.").kept);
            let (recalled,_)=restored.remember_with("Mira batteries",RecallWrite::ReadOnly,RecallBias::Observed,&[]);assert!(recalled.iter().any(|r|r.narrative.contains("2 distinct observation IDs")));
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]fn codec_rejects_corruption_and_support_disappears_with_trace_or_cut(){
        let mut m=memory();let id=offer(&mut m,"a","Mira supplied six batteries to the workshop.").trace_id.unwrap();offer(&mut m,"b","Mira supplied six batteries to the workshop.");
        let encoded=m.store.observation_evidence.encode();assert_eq!(ObservationEvidence::decode(&encoded).unwrap(),m.store.observation_evidence);assert!(ObservationEvidence::decode(&(encoded.clone()+" extra")).is_err());assert!(ObservationEvidence::decode("1 1 129").is_err());
        m.set_observation_evidence(false);assert!(m.store.observation_evidence.render(&m.store,&id).is_none());m.set_observation_evidence(true);
        m.store.traces.get_mut(&id).unwrap().suppressed=true;assert!(m.store.observation_evidence.render(&m.store,&id).is_none());
        assert!(!offer(&mut m,"a","Mira supplied six batteries to the workshop.").kept);
        assert_eq!(ObservationEvidence::decode(&m.store.observation_evidence.encode()).unwrap(),m.store.observation_evidence);
        m.store.traces.remove(&id);assert!(m.store.observation_evidence.render(&m.store,&id).is_none());
    }
    #[test]fn transfer_on_merge_unions_observations_onto_keeper(){
        let mut m=memory();
        let keep=offer(&mut m,"ka","Mira supplied six batteries to the workshop.").trace_id.unwrap();
        let other=offer(&mut m,"ob","Jules delivered three coils to the dock.").trace_id.unwrap();
        assert_ne!(keep,other);
        // Ensure the dropped trace carries an extra observation the keeper lacks.
        m.store.observation_evidence.seed(&other,"src-other",crate::Channel::Selfhood,"obs-base","Jules delivered three coils to the dock.");
        if let Some(g)=m.store.observation_evidence.groups.get_mut(&other){
            g.observations.insert("obs-extra".into(),"Jules delivered three coils to the dock.".into());
        } else {
            m.store.observation_evidence.seed(&other,"src-other",crate::Channel::Selfhood,"obs-extra","Jules delivered three coils to the dock.");
        }
        assert!(m.store.observation_evidence.groups.get(&other).is_some());
        let before=m.store.observation_evidence.groups.get(&keep).map(|g|g.observations.len()).unwrap_or(0);
        m.store.observation_evidence.transfer_on_merge(&keep,&other);
        assert!(m.store.observation_evidence.groups.get(&other).is_none());
        let after=m.store.observation_evidence.groups.get(&keep).unwrap().observations.len();
        assert!(after>before);
        assert!(m.store.observation_evidence.groups.get(&keep).unwrap().observations.contains_key("obs-extra"));
    }
    #[test]fn support_weight_saturates_and_is_zero_under_two_or_disabled(){
        let mut m=memory();
        let id=offer(&mut m,"a","Mira supplied six batteries to the workshop.").trace_id.unwrap();
        assert_eq!(m.store.observation_evidence.support_weight(&m.store,&id),0.0);
        offer(&mut m,"b","Mira supplied six batteries to the workshop.");
        let w2=m.store.observation_evidence.support_weight(&m.store,&id);
        assert!(w2>0.1 && w2<0.3);
        for i in 0..6 { offer(&mut m,&format!("c{i}"),"Mira supplied six batteries to the workshop."); }
        let w=m.store.observation_evidence.support_weight(&m.store,&id);
        assert!(w>w2 && w<=0.75);
        m.set_observation_evidence(false);
        assert_eq!(m.store.observation_evidence.support_weight(&m.store,&id),0.0);
    }
}
