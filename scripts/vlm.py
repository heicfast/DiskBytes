#!/usr/bin/env python3
"""VLM helper: call z-ai vision with an image + prompt, print content only."""
import json
import subprocess
import sys
import tempfile

img = sys.argv[1]
prompt = sys.argv[2]
out = tempfile.NamedTemporaryFile(suffix=".json", delete=False).name
r = subprocess.run(
    ["z-ai", "vision", "-i", img, "-p", prompt, "-o", out],
    capture_output=True, text=True, timeout=180,
)
try:
    d = json.load(open(out))
    print(d["choices"][0]["message"]["content"])
except Exception as e:
    print("RAW_FALLBACK", r.stdout[-3000:] if r.stdout else "", r.stderr[-1000:] if r.stderr else e, file=sys.stderr)
    sys.exit(1)
