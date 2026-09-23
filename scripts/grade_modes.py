#!/usr/bin/env python3
"""Capture + VLM-grade all 5 canvas modes in both themes via the live app.
Requires the dev server running and a completed scan.
Usage: python3 scripts/grade_modes.py
"""
import json
import os
import subprocess
import time

OUT = "shots/wave5-grades"
os.makedirs(OUT, exist_ok=True)

MODES = ["Treemap", "Sunburst", "Flame", "Bubbles", "Mind Map"]

def ab(*args):
    subprocess.run(["agent-browser", *args], check=False, capture_output=True, timeout=30)

def eval_js(js):
    r = subprocess.run(["agent-browser", "eval", js], capture_output=True, text=True, timeout=30)
    return r.stdout.strip().strip('"')

def grade(img, mode, theme):
    prompt = (
        f"Senior designer grading: {mode} visualization in a disk analyzer ({theme} theme). "
        "Grade /10 for: algorithm quality (proportions, packing, no overlap/clipping), "
        "label coverage, visual polish. One line justification. "
        "Format: GRADE: n/10 — notes"
    )
    subprocess.run(["z-ai", "vision", "-p", prompt, "-o", f"{OUT}/g.json", "-i", img],
                   check=True, capture_output=True, timeout=300)
    with open(f"{OUT}/g.json") as f:
        return json.load(f)["choices"][0]["message"]["content"].strip()

def main():
    results = []
    for theme in ("light", "dark"):
        eval_js(f"(() => {{ document.documentElement.setAttribute('data-theme','{theme}'); return 'ok' }})()")
        time.sleep(0.8)
        for mode in MODES:
            eval_js(f"(() => {{ const b=[...document.querySelectorAll('button')].find(x=>x.getAttribute('title')==='{mode}'); b?.click(); return 'set' }})()")
            time.sleep(1.6)
            img = f"{OUT}/{mode.lower().replace(' ','-')}-{theme}.png"
            ab("screenshot", img)
            g = grade(img, mode, theme)
            line = f"{mode} [{theme}]: {g}"
            print(line)
            results.append(line)
            time.sleep(1.2)
    with open(f"{OUT}/GRADES.md", "w") as f:
        f.write("# Canvas mode grades (wave 5)\n\n" + "\n".join(results) + "\n")
    print(f"\n→ {OUT}/GRADES.md")

if __name__ == "__main__":
    main()
