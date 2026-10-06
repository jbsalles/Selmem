/// Instrument the content actually delivered to the mouth, independent of profile.
pub struct EchoNarrator;
impl selmem::Narrator for EchoNarrator {
    fn reconstruct(&self, t: &selmem::MemoryTrace, m: &selmem::Mood, q: &str) -> String {
        selmem::Narrator::reconstruct(&selmem::RuleNarrator, t, m, q)
    }
    fn distill_axiom(&self, t: &[&selmem::MemoryTrace]) -> Option<String> {
        selmem::Narrator::distill_axiom(&selmem::RuleNarrator, t)
    }
    fn reply_disposed(
        &self,
        _: &str,
        memories: &[String],
        axioms: &[String],
        _: &selmem::Mood,
        _: &selmem::WorkingTalk,
        _: &str,
    ) -> String {
        memories
            .iter()
            .chain(axioms.iter().take(1))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
