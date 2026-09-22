#!/bin/bash
# Dark-theme full-surface capture tour. Usage: bash dark_tour.sh
set -x
OUT=/home/z/my-project/shots/session4/dark
mkdir -p $OUT

snap() { agent-browser screenshot "$OUT/$1.png" >/dev/null 2>&1; echo "shot $1"; }

# ensure scan is done (mock has no auto-scan in browser — click it)
agent-browser reload >/dev/null 2>&1
sleep 2
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.textContent.trim()==="Scan This PC"); if (b) { b.click(); return "scan-started"; } return "already-active"; })()' 2>/dev/null | tail -1
sleep 8

# set dark theme (title is the ACTION, not the state — check data-theme first)
agent-browser eval '(() => { const cur = document.documentElement.getAttribute("data-theme"); if (cur === "dark") return "already-dark"; const b=[...document.querySelectorAll("button")].find(x=>x.title==="Use dark theme"); if(b){b.click();return "switched-to-dark";} return "no theme btn"; })()' 2>/dev/null | tail -1
sleep 1

# 1. Folders view
snap 01-folders
# 2-6. canvas modes
for m in Treemap Sunburst Flame Bubbles; do
  agent-browser eval "(() => { const b=[...document.querySelectorAll('button')].find(x=>x.getAttribute('aria-label')==='$m'); if(b){b.click();return '$m';} return 'no $m'; })()" >/dev/null 2>&1
  sleep 1.6
  snap "0$((2 + 0))-$(echo $m | tr 'A-Z' 'a-z')" 2>/dev/null || snap "mode-$m"
done
snap 02-treemap
# 7. Top Sizes
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.getAttribute("aria-label")==="Top Sizes"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.6; snap 07-topsizes
# 8. Age Map
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.getAttribute("aria-label")==="Age Map"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.6; snap 08-agemap
# 9. List
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.getAttribute("aria-label")==="List"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.6; snap 09-list
# 10. Duplicates
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.textContent.trim()==="Duplicates"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.8; snap 10-duplicates
# 11. Applications
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.textContent.trim()==="Applications"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.8; snap 11-applications
# 12. Monitor
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.textContent.trim()==="Monitor"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 3; snap 12-monitor
# 13. Snapshots
agent-browser eval '(() => { const b=[...document.querySelectorAll("button")].find(x=>x.textContent.trim()==="Snapshots"); b.click(); return "ok"; })()' >/dev/null 2>&1; sleep 1.8; snap 13-snapshots
echo "TOUR DONE"
