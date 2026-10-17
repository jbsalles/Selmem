#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
cargo build --release --bin selmem-creative-replay
exec "${CARGO_TARGET_DIR:-target}/release/selmem-creative-replay" "$@"
