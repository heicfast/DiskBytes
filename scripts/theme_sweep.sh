#!/usr/bin/env bash
# b5-11/b5-12 theme sweep: capture every tab + key overlays, both themes,
# at the app-default 1440x860 viewport. Usage: bash scripts/theme_sweep.sh [dark|light|both]
set -u
cd "$(dirname "$0")/.."
THEME="${1:-both}"
OUT=ci-artifacts/round18-local/b5-themes
mkdir -p "$OUT"

export AGENT_BROWSER_SESSION=b5theme
agent-browser open "http://localhost:1420/" >/dev/null 2>&1
sleep 5
agent-browser set viewport 1440 860 >/dev/null 2>&1
sleep 1
# fresh scan
agent-browser eval "(() => { [...document.querySelectorAll('button')].find(x=>x.textContent.trim()==='Scan This PC'&&!x.closest('nav'))?.click(); return 1; })()" >/dev/null 2>&1
sleep 11

switch_theme() {
  agent-browser eval "(() => { const cur=document.documentElement.getAttribute('data-theme'); if(cur==='$1') return 'already'; const b=[...document.querySelectorAll('button')].find(x=>x.title==='Use dark theme'||x.title==='Use light theme'); b?.click(); return 'switched'; })()" >/dev/null 2>&1
  sleep 2.5
}

cap() { agent-browser screenshot "$OUT/$1.png" >/dev/null 2>&1; echo "shot $1"; }

tour_theme() {
  switch_theme "$1"
  cap "$1-01-explore"
  # context menu on the canvas (right-click)
  agent-browser eval "(() => { const c=document.querySelector('canvas'); if(!c) return 'no-canvas'; const r=c.getBoundingClientRect(); const ev=new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:r.x+300,clientY:r.y+300}); c.dispatchEvent(ev); return 'menu'; })()" >/dev/null 2>&1
  sleep 1
  cap "$1-02-ctxmenu"
  agent-browser press Escape >/dev/null 2>&1; sleep 0.5
  # queue popover (Cleanup button)
  agent-browser find role button click --name "Cleanup" >/dev/null 2>&1
  sleep 1
  cap "$1-03-queue"
  agent-browser press Escape >/dev/null 2>&1; sleep 0.5
  # Duplicates
  agent-browser find role button click --name "Duplicates" >/dev/null 2>&1; sleep 2
  cap "$1-04-duplicates"
  # Applications
  agent-browser find role button click --name "Applications" >/dev/null 2>&1; sleep 2
  cap "$1-05-applications"
  # Monitor
  agent-browser find role button click --name "Monitor" >/dev/null 2>&1; sleep 3
  cap "$1-06-monitor"
  # Snapshots (take one first so a row renders)
  agent-browser find role button click --name "Snapshots" >/dev/null 2>&1; sleep 2
  agent-browser find role button click --name "Take Snapshot Now" >/dev/null 2>&1; sleep 2.5
  cap "$1-07-snapshots"
  # back to Explore for the license dialog (Help -> License)
  agent-browser find role button click --name "Explore" >/dev/null 2>&1; sleep 2
  agent-browser eval "(() => { const b=[...document.querySelectorAll('button')].find(x=>(x.title||'').toLowerCase().includes('license')); if(b){b.click();return 'license-open'} return 'no-license-btn'; })()" 2>&1 | tail -1
  sleep 1.5
  cap "$1-08-license"
  agent-browser press Escape >/dev/null 2>&1; sleep 0.5
  # preview overlay via TopSizes row context? simpler: search filter active
  agent-browser find role textbox fill --name "Filter by name" "temp" >/dev/null 2>&1
  sleep 1.5
  cap "$1-09-filtered"
  agent-browser find role textbox fill --name "Filter by name" "" >/dev/null 2>&1
  sleep 1
}

if [ "$THEME" = "dark" ] || [ "$THEME" = "both" ]; then tour_theme dark; fi
if [ "$THEME" = "light" ] || [ "$THEME" = "both" ]; then tour_theme light; fi
echo "DONE → $OUT"
