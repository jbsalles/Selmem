#!/bin/sh
set -eu
# Render supplies PORT. All other options use SelMem's existing SELMEM_* variables.
exec selmemd --bind "0.0.0.0:${PORT:-10000}" "$@"
