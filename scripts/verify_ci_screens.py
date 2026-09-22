#!/usr/bin/env python3
"""Download the latest UI-screenshots artifact and VLM-audit every frame.

Usage: python3 scripts/verify_ci_screens.py <run-id> [max-frames]
Writes a per-frame report to ci-artifacts/<run-id>/REPORT.md
"""
import json
import os
import subprocess
import sys
import urllib.request

TOKEN = os.environ.get("GH_TOKEN", "")
REPO = "heicfast/DiskBytes"

def api(url):
    req = urllib.request.Request(url, headers={"Authorization": f"token {TOKEN}"})
    with urllib.request.urlopen(req) as r:
        return json.load(r)

def dl(url, dest):
    # NB: artifact URLs redirect to Azure blob storage, which rejects a
    # forwarded GitHub Authorization header — use curl -L (it strips auth on
    # cross-host redirects) instead of urllib.
    subprocess.run(
        ["curl", "-sL", "-H", f"Authorization: token {TOKEN}", "-o", dest, url],
        check=True, timeout=600,
    )

def vlm(prompt, images, out):
    cmd = ["z-ai", "vision", "-p", prompt, "-o", out]
    for i in images:
        cmd += ["-i", i]
    subprocess.run(cmd, check=True, capture_output=True, timeout=300)
    with open(out) as f:
        return json.load(f)["choices"][0]["message"]["content"]

def main():
    run_id = sys.argv[1]
    max_frames = int(sys.argv[2]) if len(sys.argv) > 2 else 30
    outdir = f"ci-artifacts/{run_id}"
    os.makedirs(outdir, exist_ok=True)

    arts = api(f"https://api.github.com/repos/{REPO}/actions/runs/{run_id}/artifacts")["artifacts"]
    shots = next((a for a in arts if a["name"] == "diskbytes-ui-screenshots"), None)
    logs = next((a for a in arts if a["name"] == "diskbytes-app-logs"), None)
    if not shots:
        print("NO SCREENSHOT ARTIFACT"); return 1
    dl(f"https://api.github.com/repos/{REPO}/actions/artifacts/{shots['id']}/zip", f"{outdir}/shots.zip")
    subprocess.run(["unzip", "-q", "-o", f"{outdir}/shots.zip", "-d", outdir], check=True)
    if logs:
        dl(f"https://api.github.com/repos/{REPO}/actions/artifacts/{logs['id']}/zip", f"{outdir}/logs.zip")
        subprocess.run(["unzip", "-q", "-o", f"{outdir}/logs.zip", "-d", outdir], check=True)
        stderr = os.path.join(outdir, "app-stderr.log")
        if os.path.exists(stderr):
            content = open(stderr).read()
            if "panicked" in content:
                print("!!! APP PANIC DETECTED:", content[-400:])
            else:
                print("app-stderr clean (no panics)")

    frames = sorted(f for f in os.listdir(outdir) if f.startswith("step-") and f.endswith(".png"))
    frames = frames[:max_frames]
    print(f"verifying {len(frames)} frames…")

    report = []
    for fr in frames:
        content = vlm(
            "Disk analyzer app screenshot from CI. Answer in 2 short lines: "
            "(1) MODE: which view/mode/state is shown; "
            "(2) QUALITY: PASS or FAIL followed by max 12 words of justification "
            "(multi-color visualization rendering correctly, no broken/blank/misaligned UI).",
            [os.path.join(outdir, fr)], f"{outdir}/vlm-{fr}.json")
        first = content.strip().split("\n")
        mode = first[0][:110] if first else "?"
        quality = first[1][:160] if len(first) > 1 else "?"
        report.append((fr, mode, quality))
        print(f"{fr}: {mode[:60]} | {quality[:80]}")

    with open(f"{outdir}/REPORT.md", "w") as f:
        f.write(f"# CI screenshot verification — run {run_id}\n\n")
        f.write("| frame | mode | verdict |\n|---|---|---|\n")
        for fr, mode, q in report:
            f.write(f"| {fr} | {mode.replace('|','/')} | {q.replace('|','/')} |\n")
    fails = sum(1 for _, _, q in report if "FAIL" in q.upper())
    print(f"\nSUMMARY: {len(report)-fails}/{len(report)} PASS, {fails} FAIL → {outdir}/REPORT.md")
    return 0 if fails == 0 else 2

if __name__ == "__main__":
    sys.exit(main())
