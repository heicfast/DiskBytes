#!/usr/bin/env python3
"""Round-16 full-window tour audit — senior UI/UX designer critique per frame.

First CI tour ever captured at 1920x1080 (complete window incl. inspector).
Audits every frame with a detailed designer prompt; writes REPORT.md.
Usage: python3 scripts/audit_round16.py
"""
import json
import os
import subprocess

OUT = "ci-artifacts/round17"
TOKEN_ENV = "GH_TOKEN"

PROMPT = (
    "You are a senior UI/UX design auditor reviewing a real Windows screenshot "
    "(1920x1080) of DiskBytes, a DaisyDisk-style disk analyzer (Tauri app; "
    "macOS-native design language: light gray canvas, white cards, coral accent, "
    "pastel chart families, 3-pane layout: sidebar / viz canvas / inspector). "
    "The window is windowed (taskbar visible at bottom is expected and correct). "
    "Answer in EXACTLY 3 lines:\n"
    "(1) STATE: which view/tab/mode/dialog is shown\n"
    "(2) VERDICT: PASS or FAIL\n"
    "(3) NOTES: max 25 words — ONLY real defects (clipped/truncated text, "
    "invisible/low-contrast controls, overlapping elements, misalignment, "
    "blank/broken areas, washed-out colors). Ignore taskbar/desktop background. "
    "If perfect, write 'clean'."
)

def vlm(img):
    out = f"{OUT}/vlm-step-audit.json"
    subprocess.run(
        ["z-ai", "vision", "-p", PROMPT, "-o", out, "-i", img],
        check=True, capture_output=True, timeout=300,
    )
    with open(out) as f:
        return json.load(f)["choices"][0]["message"]["content"]

def main():
    frames = sorted(f for f in os.listdir(OUT) if f.startswith("step-") and f.endswith(".png"))
    # trailing repeated frames: audit unique sizes fully, but keep 17..25 anyway
    rows = []
    for fr in frames:
        p = os.path.join(OUT, fr)
        try:
            content = vlm(p)
        except subprocess.CalledProcessError as e:
            content = f"AUDIT ERROR: {e}"
        lines = [l.strip() for l in content.strip().split("\n") if l.strip()]
        state = lines[0][:130] if lines else "?"
        verdict = lines[1][:60] if len(lines) > 1 else "?"
        notes = lines[2][:220] if len(lines) > 2 else "?"
        rows.append((fr, verdict, state, notes))
        print(f"{fr} | {verdict[:20]:20} | {notes[:90]}")
    with open(f"{OUT}/REPORT.md", "w") as f:
        f.write(f"# Round-16 full-window tour audit (1920x1080)\n\n")
        f.write("| frame | verdict | state | notes |\n|---|---|---|---|\n")
        for fr, v, s, n in rows:
            f.write(f"| {fr} | {v} | {s} | {n} |\n")
    fails = sum(1 for _, v, _, _ in rows if "FAIL" in v.upper())
    print(f"\nSUMMARY: {len(rows)-fails}/{len(rows)} PASS, {fails} FAIL")

if __name__ == "__main__":
    main()
