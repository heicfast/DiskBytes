#!/usr/bin/env python3
"""Detect dark text-label boxes on the pastel canvas and report true overlaps.

Labels are drawn in ON_PASTEL ink (#0F172A) with a white halo — dark
clusters are text. Dot borders are rgba(29,29,31,0.28) (much lighter).
"""
from PIL import Image
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "ci-artifacts/round18-local/mindmap-fixed-v6.png"
# canvas crop offsets used for v6 (canvas CSS rect in the page shot)
X, Y, W, H = 297, 260, 818, 567
img = Image.open(path).convert("RGB")
crop = img.crop((X, Y, X + W, Y + H))
a = crop.load()

# dark-ink mask: sum of channels low (ink #0F172A ≈ 15+23+42=80; dot
# borders at 28% alpha over pastel ≈ much lighter)
w, h = crop.size
mask = [[(sum(a[x, y]) < 200) for x in range(w)] for y in range(h)]

# connected components (8-neigh, iterative BFS) on the mask
seen = [[False] * w for _ in range(h)]
boxes = []
for y0 in range(h):
    for x0 in range(w):
        if not mask[y0][x0] or seen[y0][x0]:
            continue
        stack = [(x0, y0)]
        seen[y0][x0] = True
        minx = maxx = x0
        miny = maxy = y0
        n = 0
        while stack:
            x, y = stack.pop()
            n += 1
            minx, maxx = min(minx, x), max(maxx, x)
            miny, maxy = min(miny, y), max(maxy, y)
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < w and 0 <= ny < h and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = True
                        stack.append((nx, ny))
        # text boxes: small clusters, wider than tall typically; filter
        # noise (tiny) and big blobs (link lines are light, not dark)
        if 4 <= n <= 4000 and (maxx - minx) >= 4 and (maxy - miny) >= 4:
            boxes.append((minx, miny, maxx, maxy, n))

# merge boxes on the same text line that are close horizontally (letter
# groups): two boxes belong to one label if vertical ranges overlap >50%
# and horizontal gap < 6px
boxes.sort(key=lambda b: (b[1], b[0]))
merged = []
for b in boxes:
    placed = False
    for m in merged:
        v_overlap = min(b[3], m[3]) - max(b[1], m[1])
        h_gap = max(b[0], m[0]) - min(b[2], m[2])
        if v_overlap > 0.5 * min(b[3] - b[1], m[3] - m[1]) and h_gap < 6:
            m[0] = min(m[0], b[0]); m[1] = min(m[1], b[1])
            m[2] = max(m[2], b[2]); m[3] = max(m[3], b[3])
            placed = True
            break
    if not placed:
        merged.append(list(b))

print(f"text boxes: {len(merged)}")
overlaps = []
for i in range(len(merged)):
    for j in range(i + 1, len(merged)):
        A, B = merged[i], merged[j]
        ix = min(A[2], B[2]) - max(A[0], B[0])
        iy = min(A[3], B[3]) - max(A[1], B[1])
        if ix > 0 and iy > 0:
            overlaps.append((A, B, ix, iy))
if not overlaps:
    print("NO glyph-box overlaps")
else:
    print(f"{len(overlaps)} OVERLAPPING PAIRS:")
    for A, B, ix, iy in overlaps:
        print(f"  box{A[:4]} x box{B[:4]} intersect {ix}x{iy}px")
# also report min gaps < 2px (touching)
close = []
for i in range(len(merged)):
    for j in range(i + 1, len(merged)):
        A, B = merged[i], merged[j]
        gx = max(A[0], B[0]) - min(A[2], B[2])
        gy = max(A[1], B[1]) - min(A[3], B[3])
        if gx <= 0 and gy <= 0:
            continue
        gap = max(gx, gy)
        if 0 <= gap < 2:
            close.append((A, B, gap))
print(f"pairs closer than 2px: {len(close)}")
for A, B, g in close[:8]:
    print(f"  box{A[:4]} ~ box{B[:4]} gap {g}px")
