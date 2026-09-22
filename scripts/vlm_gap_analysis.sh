#!/bin/bash
# VLM gap analysis: current UI vs reference screenshots
cd /home/z/my-project
mkdir -p /tmp/vlm-gaps

declare -A pairs=(
  ["treemap"]="Refrences-Screenshots/tree.png:shots/before/102-mode-Treemap.png"
  ["sunburst"]="Refrences-Screenshots/sunburst.png:shots/before/103-mode-Sunburst.png"
  ["flame"]="Refrences-Screenshots/flames.png:shots/before/104-mode-Flame.png"
  ["bubbles"]="Refrences-Screenshots/bubbles.png:shots/before/105-mode-Bubbles.png"
  ["mindmap"]="Refrences-Screenshots/minmap.png:shots/before/106-mode-MindMap.png"
  ["folders"]="Refrences-Screenshots/folders.png:shots/before/101-mode-Folders.png"
  ["topsizes"]="Refrences-Screenshots/topSize.png:shots/before/107-mode-TopSizes.png"
  ["agemap"]="Refrences-Screenshots/agemap.png:shots/before/108-mode-AgeMap.png"
  ["list"]="Refrences-Screenshots/lists.png:shots/before/109-mode-List.png"
)

PROMPT='You are comparing two UI screenshots of a disk space analyzer app. IMAGE 1 is the TARGET reference design (what we want to achieve). IMAGE 2 is the CURRENT implementation. Identify every visual gap between them with precision: 1) Layout differences (panel widths, proportions, spacing) 2) Color differences (backgrounds, accents, chart colors - give hex) 3) Typography differences 4) Component styling differences (buttons, cards, chips, badges, radii, borders, shadows, gradients) 5) Visualization rendering differences (how items are drawn, labels, gaps, strokes) 6) Alignment/symmetry/centering issues in IMAGE 2 7) Missing elements in IMAGE 2 present in IMAGE 1. Be brutally specific and exhaustive - this is a production-grade polish audit. Format: numbered list of concrete actionable fixes.'

for key in "${!pairs[@]}"; do
  IFS=':' read -r ref cur <<< "${pairs[$key]}"
  echo "=== Analyzing $key ==="
  z-ai vision -p "$PROMPT" -i "./$ref" -i "./$cur" -o "/tmp/vlm-gaps/$key.json" 2>&1 | grep -c "saved" || true
done
echo "ALL DONE"
