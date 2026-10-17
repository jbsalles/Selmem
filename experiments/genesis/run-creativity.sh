#!/usr/bin/env bash
set -euo pipefail
out_dir=${1:?Usage: bash experiments/genesis/run-creativity.sh OUT_DIR}
runtime_config=${SELMEM_CONFIG:-.selmem}
dataset=${LOCOMO_DATASET:-experiments/genesis/results/locomo10.json}
[[ -f run.sh && -f "$runtime_config" && -f "$dataset" ]] || { printf 'Run from repository root with runtime config and official locomo10.json available.\n' >&2; exit 2; }
[[ ! -e "$out_dir" && ! -e "$out_dir.tar.gz" && ! -e "$out_dir.log" && ! -e "$out_dir.locomo.json" ]] || { printf 'Choose fresh output paths.\n' >&2; exit 2; }
mkdir -p "$(dirname "$out_dir")"
prepare_args=("$dataset" "$out_dir.locomo.json")
if [[ ${GENESIS_CREATIVE_ALL:-0} == 1 ]]; then prepare_args+=(--all); fi
bash experiments/genesis/tools.sh prepare-creative "${prepare_args[@]}"
./run.sh build --release --bin selmem-genesis
export SELMEM_REASONING=${GENESIS_CREATIVE_REASONING:-none}
args=(creative --locomo "$out_dir.locomo.json" --out "$out_dir" --config "$runtime_config" --llm "${GENESIS_CREATIVE_PROVIDER:-grok}" --repeats 3 --context-words 600 --max-http-calls "${GENESIS_CREATIVE_HTTP_LIMIT:-3000}")
if [[ -n ${GENESIS_CREATIVE_MODEL:-} ]]; then args+=(--model "$GENESIS_CREATIVE_MODEL"); fi
if [[ ${GENESIS_CREATIVE_PREPARE_ONLY:-0} == 1 ]]; then args+=(--prepare-only); fi
printf 'CREATIVE: shared native life, four conditions, three tasks, three generation repeats; no sleep. HTTP cap includes native lifecycle/reconstruction/scorer calls and retries. Collection is unrated.\n'
./run.sh run --release --bin selmem-genesis -- "${args[@]}" 2>&1 | tee "$out_dir.log"
tar -czf "$out_dir.tar.gz" -C "$(dirname "$out_dir")" "$(basename "$out_dir")" "$(basename "$out_dir").log" "$(basename "$out_dir").locomo.json"
printf 'COLLECTION SAVED: %s.tar.gz\n' "$out_dir"
