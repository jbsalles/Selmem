#!/usr/bin/env bash
set -euo pipefail
root=${1:-experiments/vangogh-realistic/source}
fetch() {
  local repo=$1 pin=$2 dest=$3 subset=$4
  if [[ -e "$dest" ]]; then
    [[ $(git -C "$dest" rev-parse HEAD) == "$pin" ]] || { echo 'Unexpected source revision' >&2; exit 1; }
    [[ -z $(git -C "$dest" status --porcelain) ]] || { echo 'Modified source checkout' >&2; exit 1; }
  else
    git init "$dest"
    git -C "$dest" remote add origin "$repo"
    git -C "$dest" sparse-checkout init --cone
    git -C "$dest" sparse-checkout set "$subset"
    git -C "$dest" fetch --depth 1 --filter=blob:none origin "$pin"
    git -C "$dest" checkout --detach FETCH_HEAD
  fi
}
fetch https://github.com/HuygensING/vangogh.git ab158199731faf965b4312ff98815f687b8a6811 "$root/vangogh" tei/2025-03-12/letters
fetch https://github.com/snap-research/locomo.git 3eb6f2c585f5e1699204e3c3bdf7adc5c28cb376 "$root/locomo" data
printf 'Sources ready at %s. LoCoMo: CC BY-NC 4.0; letters: CC BY-NC-SA 4.0.\n' "$root"
