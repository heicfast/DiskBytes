#!/usr/bin/env python3
"""Find CSS classes defined in stylesheets but never referenced in
src/**/*.tsx (heuristic dead-CSS detector)."""
import re
import pathlib

root = pathlib.Path("/home/z/my-project/src")
css_files = list((root / "styles").glob("*.css")) + [(root / "theme/tokens.css")]
tsx = "\n".join(p.read_text() for p in root.rglob("*.tsx"))
ts = "\n".join(p.read_text() for p in root.rglob("*.ts"))

defined = {}
for f in css_files:
    for m in re.finditer(r"\.([a-z][a-z0-9-]{3,})", f.read_text()):
        cls = m.group(1)
        # skip pseudo/state fragments like is-staged used via template strings
        defined.setdefault(cls, f.name)

used_tsx = set(re.findall(r"['\"`]([a-z][a-z0-9- ]{3,})['\"`]", tsx + ts))
dead = []
for cls, fname in sorted(defined.items()):
    # a class is used if it appears in any template/class attribute string
    found = any(cls in u.split() for u in used_tsx)
    if not found:
        dead.append((cls, fname))
print(f"defined: {len(defined)}, likely-dead: {len(dead)}")
for cls, fname in dead:
    print(f"  .{cls}  ({fname})")
