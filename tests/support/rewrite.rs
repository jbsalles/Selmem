/// Known paraphrase proposal; acceptance still needs an explicit witness.
pub struct RewriteNarrator;
impl selmem::Narrator for RewriteNarrator {
    fn reconstruct(&self, t: &selmem::MemoryTrace, m: &selmem::Mood, q: &str) -> String {
        selmem::Narrator::reconstruct(&selmem::RuleNarrator, t, m, q)
    }
    fn distill_axiom(&self, t: &[&selmem::MemoryTrace]) -> Option<String> {
        selmem::Narrator::distill_axiom(&selmem::RuleNarrator, t)
    }
    fn rewrite(
        &self,
        t: &selmem::MemoryTrace,
        _: &[&selmem::MemoryTrace],
        _: &selmem::EntityProfile,
    ) -> Option<String> {
        Some(format!("I recall: {}", t.core))
    }
}
