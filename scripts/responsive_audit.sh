#!/bin/bash
# Responsive overflow audit: 1280/1440/1680/1920 × key surfaces.
cd /home/z/my-project
OUT=shots/session4/responsive
mkdir -p $OUT
for W in 1280 1440 1680 1920; do
  agent-browser set viewport $W 900 >/dev/null 2>&1
  sleep 0.9
  for MODE in Folders Treemap "Top Sizes" List; do
    LBL=$(echo $MODE | tr 'A-Z ' 'a-z' | tr -d ' -')
    agent-browser eval "(() => { const b=[...document.querySelectorAll('button')].find(x=>x.getAttribute('aria-label')==='$MODE'); if(b){b.click();return 'ok';} return 'no'; })()" >/dev/null 2>&1
    sleep 1.2
    agent-browser eval "(() => { const doc = document.documentElement; const overflowX = doc.scrollWidth - doc.clientWidth; const bad = []; document.querySelectorAll('.db-sidebar, .db-main-col, .db-inspector-col, .db-toolbar-row, .db-topbar, .db-title-row').forEach(el => { if (el.scrollWidth > el.clientWidth + 1) bad.push(el.className.split(' ')[0] + ':' + (el.scrollWidth - el.clientWidth)); }); return JSON.stringify({w: window.innerWidth, overflowX, bad}); })()" 2>/dev/null | tail -1
    agent-browser screenshot "$OUT/${W}-${LBL}.png" >/dev/null 2>&1
  done
  # monitor + duplicates tabs
  for TAB in Monitor Duplicates; do
    agent-browser eval "(() => { const b=[...document.querySelectorAll('button')].find(x=>x.textContent.trim()==='$TAB'); if(b){b.click();return 'ok';} return 'no'; })()" >/dev/null 2>&1
    sleep 1.4
    agent-browser eval "(() => { const doc = document.documentElement; return JSON.stringify({w: window.innerWidth, overflowX: doc.scrollWidth - doc.clientWidth}); })()" 2>/dev/null | tail -1
    agent-browser screenshot "$OUT/${W}-$(echo $TAB | tr 'A-Z' 'a-z').png" >/dev/null 2>&1
    agent-browser eval "(() => { const b=[...document.querySelectorAll('button')].find(x=>x.textContent.trim()==='Explore'); b?.click(); return 'back'; })()" >/dev/null 2>&1
    sleep 0.8
  done
done
echo "RESPONSIVE AUDIT DONE"
