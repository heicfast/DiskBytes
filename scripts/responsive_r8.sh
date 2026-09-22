#!/usr/bin/env bash
# Responsive audit: 4 widths × overflow probe + measurements (mock backend).
set -u
cd /home/z/my-project
OUT=shots/session3/responsive
mkdir -p "$OUT"
for W in 1280 1440 1680 1920; do
  agent-browser set viewport "$W" 900 >/dev/null 2>&1
  sleep 1.2
  agent-browser screenshot "$OUT/w${W}.png" >/dev/null 2>&1
  echo "== ${W}x900 =="
  agent-browser eval "(() => {
    const of = [];
    const probe = (sel) => {
      document.querySelectorAll(sel).forEach((el) => {
        if (el.scrollWidth > el.clientWidth + 1 && !el.classList.contains('db-folders-scroll')) of.push(sel + ':' + (el.scrollWidth - el.clientWidth));
      });
    };
    ['.db-topbar', '.db-sidebar', '.db-main', '.db-inspector', '.db-content-head', '.db-toolbar-row'].forEach(probe);
    const r = (s) => { const el = document.querySelector(s); return el ? Math.round(el.getBoundingClientRect().width) : null; };
    return JSON.stringify({ overflow: of, sidebar: r('.db-sidebar'), main: r('.db-main'), inspector: r('.db-inspector'), bodyScrollX: document.documentElement.scrollWidth > window.innerWidth });
  })()" 2>/dev/null | tail -1
done
agent-browser set viewport 1600 900 >/dev/null 2>&1
