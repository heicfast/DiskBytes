#!/usr/bin/env python3
"""Crop + upscale regions of a screenshot for close VLM inspection."""
import sys
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
# Optional boxes: x y w h [scale] — multiple crops → out_0.png, out_1.png …
boxes = []
for arg in sys.argv[3:]:
    parts = [int(v) for v in arg.split(",")]
    boxes.append(parts)

img = Image.open(src)
print(f"source size: {img.size}")
if not boxes:
    # default: center-left quadrant where big treemap cells live
    w, h = img.size
    boxes = [[int(w*0.28), int(h*0.30), int(w*0.42), int(h*0.45), 2]]

for i, b in enumerate(boxes):
    x, y, cw, ch = b[0], b[1], b[2], b[3]
    scale = b[4] if len(b) > 4 else 2
    crop = img.crop((x, y, x + cw, y + ch))
    crop = crop.resize((cw * scale, ch * scale), Image.LANCZOS)
    path = out.replace(".png", f"_{i}.png")
    crop.save(path)
    print(f"saved {path} ({crop.size})")
