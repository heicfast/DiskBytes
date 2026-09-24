#!/usr/bin/env bash
# with_dev.sh — start vite + run a command while it stays alive, then tear down.
# The sandbox reaps background processes when a tool call ends, so the dev
# server and the test sequence must share one bash invocation.
# Usage: bash scripts/with_dev.sh <command...>
#        AGENT_BROWSER_SESSION=b5snap to reuse a browser session.
set -u
cd "$(dirname "$0")/.."

pkill -f "node_modules/.bin/vite" 2>/dev/null
sleep 0.5

./node_modules/.bin/vite --port 1420 --strictPort > /tmp/vite.log 2>&1 &
VITE_PID=$!
trap 'kill $VITE_PID 2>/dev/null; pkill -f "node_modules/.bin/vite" 2>/dev/null' EXIT

# wait for readiness (vite binds ::1 by default; curl resolves both)
ready=0
for i in $(seq 1 60); do
  if curl -s -o /dev/null --max-time 1 "http://localhost:1420/"; then
    ready=1
    break
  fi
  sleep 0.5
done
if [ "$ready" != "1" ]; then
  echo "VITE FAILED TO START:" >&2
  tail -20 /tmp/vite.log >&2
  exit 1
fi
echo "[with_dev] vite up (pid $VITE_PID)"

"$@"
rc=$?
echo "[with_dev] command exited $rc"
exit $rc
