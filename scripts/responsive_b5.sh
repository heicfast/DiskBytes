#!/usr/bin/env bash
# Responsive audit: 1024/1280/1440 — programmatic overflow detection +
# captures at each width with a deep breadcrumb chain (5 levels).
set -u
cd "$(dirname "$0")/.."
OUT=ci-artifacts/round18-local/b5-responsive
mkdir -p "$OUT"
export AGENT_BROWSER_SESSION=b5resp

agent-browser open "http://localhost:1420/" >/dev/null 2>&1
sleep 5
agent-browser set viewport 1440 860 >/dev/null 2>&1
agent-browser eval "(() => { [...document.querySelectorAll('button')].find(x=>x.textContent.trim()==='Scan This PC'&&!x.closest('nav'))?.click(); return 1; })()" >/dev/null 2>&1
sleep 11

# drill 4 deep: C: -> Users -> me -> Documents -> Invoices 2026 (deepest known chain)
agent-browser eval "(() => {
  const drill=(name)=>{const c=[...document.querySelectorAll('.db-folder-card')].find(x=>x.textContent.includes(name)); if(c){c.dispatchEvent(new MouseEvent('dblclick',{bubbles:true})); return true;} return false;};
  let log=[];
  for(const n of ['Users','me','Documents','Invoices 2026']) log.push(n+':'+drill(n));
  return log.join(' ');
})()" 2>&1 | tail -1
sleep 3

for W in 1440 1280 1024; do
  agent-browser set viewport $W 860 >/dev/null 2>&1
  sleep 1.5
  echo "=== $W ==="
  agent-browser eval "(() => {
    const doc=document.documentElement;
    const over=[];
    // any element extending past viewport right edge (visible ones only)
    document.querySelectorAll('body *').forEach(e=>{
      const r=e.getBoundingClientRect();
      if(r.width>0&&r.right>innerWidth+1&&!e.closest('[hidden]')&&getComputedStyle(e).position!=='fixed'){
        if(over.length<6) over.push(e.className.toString().slice(0,30)+'@'+Math.round(r.right));
      }
    });
    const bc=document.querySelector('.db-breadcrumb');
    const bcr=bc?bc.getBoundingClientRect():null;
    const clipped=bc?bc.scrollWidth>bc.clientWidth:false;
    return JSON.stringify({vw:innerWidth, hscroll:doc.scrollWidth>doc.clientWidth+1, overflows:over, bcRight:Math.round(bcr?.right||0), bcClip:clipped});
  })()" 2>&1 | tail -1
  agent-browser screenshot "$OUT/w$W.png" >/dev/null 2>&1
done
echo "DONE"
