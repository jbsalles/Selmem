use selmem::dream::{NIGHT_PASSES, SHALLOW_PASSES};

#[test]
fn night_passes_stay_in_scientific_order() {
    assert_eq!(
        NIGHT_PASSES,
        &["weather", "ladder", "rewrite", "merge", "release"]
    );
    assert_eq!(SHALLOW_PASSES, &["weather", "release"]);
}
