#!/bin/bash
# Responsive audit: capture key views at multiple desktop widths (labels collapse <1500px)
cd /home/z/my-project
mkdir -p shots/responsive

capture_width() {
  local W=$1
  agent-browser set viewport $W 900
  agent-browser wait 900
  # Explore / Treemap (mode buttons keep title attr)
  agent-browser find first "nav[aria-label='Application sections'] button:nth-child(4)" click >/dev/null 2>&1 || true
  agent-browser find first "nav[aria-label='Application sections'] button:nth-child(1)" click >/dev/null 2>&1 || true
  agent-browser wait 600
  agent-browser find role button click --name "Treemap" >/dev/null 2>&1 || agent-browser find first "[title='Treemap']" click >/dev/null 2>&1 || true
  agent-browser wait 1300
  agent-browser screenshot shots/responsive/w${W}-treemap.png
  agent-browser find role button click --name "Folders" >/dev/null 2>&1 || agent-browser find first "[title='Folders']" click >/dev/null 2>&1 || true
  agent-browser wait 1300
  agent-browser screenshot shots/responsive/w${W}-folders.png
  # Monitor tab = 4th nav button
  agent-browser find first "nav[aria-label='Application sections'] button:nth-child(4)" click >/dev/null 2>&1 || true
  agent-browser wait 1600
  agent-browser screenshot shots/responsive/w${W}-monitor.png
  agent-browser find first "nav[aria-label='Application sections'] button:nth-child(1)" click >/dev/null 2>&1 || true
  agent-browser wait 500
}

for W in 1440 1680 1920; do
  capture_width $W
done
agent-browser set viewport 1680 1050
echo "DONE"
