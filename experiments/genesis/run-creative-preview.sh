#!/usr/bin/env bash
set -euo pipefail
fixture=${1:?Usage: bash experiments/genesis/run-creative-preview.sh FIXTURE OUT_DIR [--resume|--prepare-only]}
out=${2:?Output directory required}
extra=()
case ${3:-} in '') ;; --resume|--prepare-only) extra=("$3");; *) printf 'Unknown third argument\n' >&2; exit 2;; esac
[[ $# -le 3 ]] || { printf 'Too many arguments\n' >&2; exit 2; }
config=${SELMEM_CONFIG:-.selmem}
[[ -f run.sh && -f "$fixture" && -f "$config" ]] || { printf 'Run from repository root with config and fixture.\n' >&2; exit 2; }
export SELMEM_TEMP=0
export SELMEM_REASONING=${GENESIS_PREVIEW_REASONING:-none}
args=(creative-preview --fixture "$fixture" --out "$out" --config "$config" --llm "${GENESIS_PREVIEW_PROVIDER:-grok}" --model "${GENESIS_PREVIEW_MODEL:-grok-4.3}" --max-http-calls "${GENESIS_PREVIEW_HTTP_LIMIT:-3000}")
printf 'GENESIS preview: 788 source turns encoded once; 20 nights; 60 creative proposals.\nHTTP ceiling includes retries, memory interpretation, reconstruction and scoring. It is not a price estimate or completion guarantee.\n'
mkdir -p "$(dirname "$out")"
bash run.sh run --release --bin selmem-genesis -- "${args[@]}" "${extra[@]}" 2>&1 | tee -a "$out.log"
tar -czf "$out.tar.gz" -C "$(dirname "$out")" "$(basename "$out")"
printf 'SAVED: %s.tar.gz\n' "$out"
