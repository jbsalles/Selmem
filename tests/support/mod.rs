//! Explicit witness fixtures for testing the consumer of external labels.
//! These do not implement a lexical scorer or evaluate model correctness.
use selmem::{PropositionLabel, PropositionScorer};
pub struct FixedScorer(pub PropositionLabel);
impl PropositionScorer for FixedScorer {
    fn score(&self, _: &str, _: &str) -> PropositionLabel {
        self.0
    }
    fn name(&self) -> &str {
        "test-fixed-witness"
    }
}
