#!/usr/bin/env bash
set -euo pipefail
fixture=${1:?Usage: bash experiments/genesis/run-creative-life.sh FIXTURE OUT_DIR [--resume]}
out=${2:?Output directory required}
resume=()
if [[ ${3:-} == --resume ]]; then resume=(--resume); elif [[ -n ${3:-} ]]; then printf 'Unknown third argument\n' >&2; exit 2; fi
config=${SELMEM_CONFIG:-.selmem}
[[ -f run.sh && -f "$fixture" && -f "$config" ]] || { printf 'Run from repository root with config and fixture.\n' >&2; exit 2; }
plan=$(bash experiments/genesis/tools.sh life-profile "$fixture")
if [[ $plan == economy ]]; then default_limit=12000; printf 'GENESIS economy: 16 lives, two complete source histories, 90+30 days, 600 proposals; shared validated annotation cache.\n'; else default_limit=200000; printf 'GENESIS full: 80 lives, 90+30 days, 6480 proposals.\n'; fi
export SELMEM_TEMP=0
export SELMEM_REASONING=${GENESIS_LIFE_REASONING:-none}
args=(creative-life --fixture "$fixture" --out "$out" --config "$config" --llm "${GENESIS_LIFE_PROVIDER:-grok}" --model "${GENESIS_LIFE_MODEL:-grok-4.3}" --max-http-calls "${GENESIS_LIFE_HTTP_LIMIT:-$default_limit}")
printf 'Cumulative HTTP ceiling is not a dollar cap or a guarantee of completion.\n'
mkdir -p "$(dirname "$out")"
bash run.sh run --release --bin selmem-genesis -- "${args[@]}" "${resume[@]}" 2>&1 | tee -a "$out.log"
tar -czf "$out.tar.gz" -C "$(dirname "$out")" "$(basename "$out")"
printf 'SAVED: %s.tar.gz\n' "$out"
