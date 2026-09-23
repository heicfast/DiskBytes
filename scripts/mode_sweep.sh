#!/usr/bin/env bash
# Mode sweep: capture every canvas mode at the app-default viewport,
# both themes, for VLM grading. Usage: bash scripts/mode_sweep.sh
set -u
cd "$(dirname "$0")/.."
OUT=ci-artifacts/round18-local/sweep
mkdir -p "$OUT"

ab() { agent-browser "$@" >/dev/null 2>&1; }

ab set viewport 1440 860
sleep 1
ab reload
sleep 5
# start scan via the welcome CTA
agent-browser find role button click --name "Scan This PC" >/dev/null 2>&1
sleep 7

MODES=("Treemap" "Sunburst" "Flame" "Bubbles" "Mind Map" "Age Map")
for theme in light dark; do
  if [ "$theme" = "dark" ]; then
    agent-browser find role button click --name "Use dark theme" >/dev/null 2>&1
    sleep 2
  fi
  for m in "${MODES[@]}"; do
    agent-browser find role button click --name "$m" >/dev/null 2>&1
    sleep 2.2
    agent-browser screenshot "$OUT/${m// /}-$theme.png" >/dev/null 2>&1
    echo "captured $m-$theme"
  done
done
# restore light for subsequent work
agent-browser find role button click --name "Use light theme" >/dev/null 2>&1
echo SWEEP_DONE
