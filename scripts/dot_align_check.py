#!/usr/bin/env python3
"""Pixel-analyze the stats-line separator dots vs digit ink centers."""
from PIL import Image
import sys

shot = sys.argv[1] if len(sys.argv) > 1 else "/home/z/my-project/shots/session3/r8-scanned.png"
img = Image.open(shot).convert("RGB")
W, H = img.size

# Stats row lives around y 88-108, x 340..900 (main column). Scan for the
# small round dots (3px, tertiary gray ~#8a8f98-ish) and the digit ink.
# 1) Find the darkest-ink row profile in the stats band within main col.
x0, x1, y0, y1 = 340, 900, 86, 110

def ink(y, x0=x0, x1=x1, thresh=120):
    cnt = 0
    for x in range(x0, x1):
        r, g, b = img.getpixel((x, y))
        if r < thresh and g < thresh and b < thresh:
            cnt += 1
    return cnt

# Row profile: rows with digit ink (many dark pixels)
rows = [(y, ink(y)) for y in range(y0, y1)]
text_rows = [y for y, c in rows if c > 30]
print("text ink rows:", text_rows[0], "..", text_rows[-1], "center:", (text_rows[0] + text_rows[-1]) / 2)

# Find dots: isolated 3-4px gray blobs. Dots are ~#8a8f98 (lighter than text).
# Scan for pixels that are mid-gray (not near-white, not near-black).
def dot_candidates():
    found = []
    for y in range(y0, y1):
        for x in range(x0, x1):
            r, g, b = img.getpixel((x, y))
            if 100 < r < 190 and 100 < g < 190 and 100 < b < 190:
                # check neighborhood is small cluster
                found.append((x, y))
    # cluster by x
    clusters = {}
    for x, y in found:
        clusters.setdefault(x // 10, []).append((x, y))
    out = []
    for k, pts in clusters.items():
        if 3 <= len(pts) <= 30:
            xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
            if max(ys) - min(ys) <= 5 and max(xs) - min(xs) <= 5:
                out.append((sum(xs)/len(xs), sum(ys)/len(ys), len(pts)))
    return out

for cx, cy, n in dot_candidates():
    print(f"dot cluster at x={cx:.0f} y-center={cy:.1f} ({n}px)")
