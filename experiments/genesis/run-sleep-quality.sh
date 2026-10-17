#!/usr/bin/env bash
set -euo pipefail
source_dir=${1:?Usage: bash experiments/genesis/run-sleep-quality.sh EVIDENCE_ON_DIR OUT_DIR}
out_dir=${2:?Choose a fresh output directory}
runtime_config=${SELMEM_CONFIG:-.selmem}
[[ -f run.sh && -f "$runtime_config" ]] || { printf 'Run from repository root with .selmem configured.\n' >&2; exit 2; }
[[ -f "$source_dir/learning.json" && -f "$source_dir/reliable.vault" && -f "$source_dir/unreliable.vault" ]] || { printf 'Missing completed observation-support ON source.\n' >&2; exit 2; }
[[ ! -e "$out_dir" && ! -e "$out_dir.tar.gz" && ! -e "$out_dir.log" && ! -e "$out_dir.locomo.json" ]] || { printf 'Choose fresh output paths.\n' >&2; exit 2; }
dataset=${LOCOMO_DATASET:-experiments/genesis/results/locomo10.json}
mkdir -p "$(dirname "$dataset")" "$(dirname "$out_dir")"
if [[ ! -f "$dataset" ]]; then
  tmp="$dataset.download"
  trap 'rm -f "$tmp"' EXIT
  curl --fail --location --max-time 90 https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json -o "$tmp"
  mv "$tmp" "$dataset"
fi
bash experiments/genesis/tools.sh prepare-sleep "$dataset" "$out_dir.locomo.json"
./run.sh build --release --bin selmem-genesis
export SELMEM_REASONING=off
printf 'SLEEP QUALITY: 4 histories; 3 arms; checkpoints 0/1/5/20; 20 faithful qualification choices + 240 measured choices; 168 isolated final readouts; 153 new LoCoMo encodings with live recall; 160 native nights. HTTP count varies with reconstruction/scoring/retries; cap=%s attempts. Provider must match source.\n' "${GENESIS_SLEEP_HTTP_LIMIT:-3000}"
args=(sleep --source "$source_dir" --locomo "$out_dir.locomo.json" --out "$out_dir" --config "$runtime_config" --llm "${GENESIS_SLEEP_PROVIDER:-grok}" --max-http-calls "${GENESIS_SLEEP_HTTP_LIMIT:-3000}")
if [[ -n ${GENESIS_SLEEP_RESUME_FROM:-} ]]; then args+=(--resume-from "$GENESIS_SLEEP_RESUME_FROM"); fi
if [[ ${GENESIS_SLEEP_PREPARE_ONLY:-0} == 1 ]]; then args+=(--prepare-only); fi
if [[ -n ${GENESIS_SLEEP_MODEL:-} ]]; then args+=(--model "$GENESIS_SLEEP_MODEL"); fi
./run.sh run --release --bin selmem-genesis -- "${args[@]}" 2>&1 | tee "$out_dir.log"
tar -czf "$out_dir.tar.gz" -C "$(dirname "$out_dir")" "$(basename "$out_dir")" "$(basename "$out_dir").log" "$(basename "$out_dir").locomo.json"
printf 'COMPLETE: %s.tar.gz\n' "$out_dir"
