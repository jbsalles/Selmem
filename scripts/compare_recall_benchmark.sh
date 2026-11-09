#!/usr/bin/env bash
# Run the exact same read-only fixture against baseline and modified trees.
# Requires a checked out base commit and the current refactor checkout.
set -euo pipefail
BASE='bdf77149f95759db5282deaa0f623a79406b2c20'
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
[[ -f src/bin/selmem-bench-recall.rs ]] || { echo 'Run the refactor first' >&2; exit 1; }
TMP="$(mktemp -d)"
cleanup() { git worktree remove --force "$TMP/baseline" 2>/dev/null || :; rm -rf "$TMP"; }
trap cleanup EXIT
git worktree add --detach "$TMP/baseline" "$BASE" >/dev/null
cp src/bin/selmem-bench-recall.rs "$TMP/baseline/src/bin/"
# Build the two versions independently to avoid target-dir interference.
(cd "$TMP/baseline" && ./run.sh build --release --bin selmem-bench-recall &&
 ./run.sh run --release --bin selmem-bench-recall -- "$@" > "$TMP/baseline.csv")
(./run.sh build --release --bin selmem-bench-recall &&
 ./run.sh run --release --bin selmem-bench-recall -- "$@" > "$TMP/refactored.csv")
# Fail if the fixture produces different stored counts or selected contents.
cut -d, -f1-6 "$TMP/baseline.csv" > "$TMP/base-correctness.csv"
cut -d, -f1-6 "$TMP/refactored.csv" > "$TMP/ref-correctness.csv"
diff -u "$TMP/base-correctness.csv" "$TMP/ref-correctness.csv"
echo 'Fixture counts and selected trace contents match.'
echo 'Baseline:'; cat "$TMP/baseline.csv"
echo 'Refactor:'; cat "$TMP/refactored.csv"
