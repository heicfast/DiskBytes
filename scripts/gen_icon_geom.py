#!/usr/bin/env python3
"""Generate premium view-pictogram geometry (24-grid) for Icon.tsx.

Sunburst: segmented arcs (2 outer + 1 inner) + filled hub, round caps.
Bubbles: tangent-packed circles (Pythagoras-verified).
"""
import math

def pt(cx, cy, r, deg):
    a = math.radians(deg)
    return (round(cx + r * math.cos(a), 2), round(cy + r * math.sin(a), 2))

def arc(cx, cy, r, a0, a1, sweep=1):
    """SVG arc path from angle a0 to a1 (deg, 0=east, y-down SVG coords)."""
    x0, y0 = pt(cx, cy, r, a0)
    x1, y1 = pt(cx, cy, r, a1)
    large = 1 if abs(a1 - a0) > 180 else 0
    return f"M {x0} {y0} A {r} {r} 0 {large} {sweep} {x1} {y1}"

C = 12.0
print("SUNBURST (outer r=8.4, inner r=4.9):")
# Outer ring: two segments — top-major (185°→355° through 270 = top) and right-lower (25°→115°)
# SVG y-down: angle 270° = up. Top-major arc: from 170° to 370° (=10°) sweeping through 270.
print(" outer A (top, big):", arc(C, C, 8.4, 170, 370))
print(" outer B (lower-right):", arc(C, C, 8.4, 30, 115))
# Inner ring: one offset segment lower-left: 120°→210°
print(" inner C (lower-left):", arc(C, C, 4.9, 120, 215))

print()
print("BUBBLES tangency check:")
c1 = (9.0, 13.8, 6.1)
c2 = (17.2, 7.6, 4.15)
c3 = (6.2, 4.2, 2.6)
for name, (a, b) in {"c1-c2": (c1, c2), "c1-c3": (c1, c3), "c2-c3": (c2, c3)}.items():
    d = math.dist(a[:2], b[:2])
    print(f" {name}: dist={d:.2f} rsum={a[2]+b[2]:.2f} gap={d-(a[2]+b[2]):+.2f}")

print()
print("TREEMAP cells (outer 3..21):")
print(" vertical divide x=10.75; left split y=14.75; right split y=10")
print("FLAMEGRAPH rows (icicle, bottom widest):")
for w, y in [(15, 19.75), (9.5, 15.25), (5, 10.75), (2, 6.25)]:
    x0 = 12 - w / 2
    print(f" y={y}: from {x0} to {x0+w}")
