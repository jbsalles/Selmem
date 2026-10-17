#!/usr/bin/env bash
set -euo pipefail
[[ -f run.sh && -f Cargo.toml ]] || { printf 'Run from the repository root.\n' >&2; exit 2; }
exec bash run.sh run --release --bin selmem-genesis -- tools "$@"
