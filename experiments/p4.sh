#!/bin/bash
# P4 locked grid. Fill .selmem first. Does not invent a model.
# Usage:
#   ./experiments/p4.sh grok
#   ./experiments/p4.sh luna
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TAG="${1:-grok}"
PAIRS="${PAIRS:-5}"
SEED="${SEED:-1}"
K="${K:-8}"

echo "commit $(git rev-parse HEAD 2>/dev/null || echo no-git)"
uname -a
test -f .selmem || { echo "missing .selmem"; exit 1; }

run_one() {
  local bias="$1"
  local out="$2"
  local extra=()
  extra+=(--p4 --pairs "$PAIRS" --seed "$SEED" --last-k "$K" --out "$out")
  if [ "$bias" != "observed" ]; then
    extra+=(--bias "$bias")
  fi
  echo "=== $TAG $bias -> $out ==="
  ./run.sh run --release --example persist -- "${extra[@]}"
}

mkdir -p experiments
run_one observed "experiments/selmem-persist-p4-${TAG}-n${PAIRS}.json"
run_one drop "experiments/selmem-persist-p4-drop-${TAG}-n${PAIRS}.json"
run_one lineage "experiments/selmem-persist-p4-lineage-${TAG}-n${PAIRS}.json"
echo "done $TAG"
