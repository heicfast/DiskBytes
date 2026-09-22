# DiskBytes Worklog

---
Task ID: 0
Agent: main (Super Z)
Task: Setup — unzip codebase, read all docs, VLM-analyze 9 reference screenshots, capture before-state, gap analysis, GitHub repo + CI baseline

Work Log:
- Unzipped DiskBytes_code.zip to /home/z/my-project (repo root), kept Refrences-Screenshots/ as local reference (gitignored)
- Read README.md, docs/ACCEPTANCE.md, docs/DISTRIBUTION.md, roadmap.md, package.json, tauri.conf.json, all 4 workflows
- Stack: Tauri 2 (Rust core in src-tauri/core, platform win.rs/mac.rs) + React 18 + TS + Vite 6 + zustand + framer-motion + plain CSS (tokens.css + 7 style files)
- VLM-analyzed all 9 reference screenshots → docs/DESIGN-REFERENCE-VLM.md (DaisyDisk-style macOS-native aesthetic: #F5F5F7 canvas, white cards, coral #FF6B4A accent, pastel chart families, 8px grid, pill controls, tabular nums)
- Started vite dev (port 1420, mock backend DEV-only), captured 17 before-state screenshots → shots/before/
- VLM gap analysis current-vs-reference for all 9 modes → docs/GAP-ANALYSIS.md
- Key gaps found: (1) treemap renders monochrome in browser — mock assigns tone family ONLY at depth 0 and inherits; single-child root (This PC → C:) makes everything blue; Rust core assigns family per sibling index at every level (too rainbow, no branch cohesion). Neither matches reference's "distinct family per top branch, inherited by descendants". (2) sunburst center is empty — reference shows folder name + size in center. (3) age map colors in AgeMapMode drift from tokens (darker). (4) mind-map links gray — reference uses branch colors. (5) warning banner not tinted. (6) H1 26px vs reference 28-32px. (7) donut ring thinner than reference.
- npm install, typecheck PASS, vitest 30/30 PASS
- Installed Rust toolchain; cargo check -p diskbytes-core PASSES on Linux
- Created GitHub repo heicfast/DiskBytes, pushed baseline commit 5208561
- CI + UI Screenshots + macOS Build workflows all triggered and running on baseline

Stage Summary:
- Baseline green locally (typecheck + vitest + cargo check core); CI running on GitHub
- Deliverable repo: https://github.com/heicfast/DiskBytes
- Next: parallel polish tracks (A: tokens/CSS/components by main; B: canvas rendering + mock parity; C: Rust color families) then local gates → push → CI → screenshots → VLM verify loop

---
Task ID: C
Agent: rust-core-colors
Task: Fix by-folder pastel family assignment in all 5 Rust layout engines — one distinct family per effective top-level branch, inherited by descendants (single-child "This PC" → "C:" roots no longer monochrome / per-level rainbow)
Work Log:
- Read worklog + all layout engines + scan/node.rs (children_sorted returns &[u32], CSR slice, size-descending after rollup::finalize)
- Key finding (differs from brief's problem statement): treemap/sunburst/flame/mindmap colored ByFolder cells with the SIBLING INDEX i as family at EVERY level (rainbow; their threaded top_index was vestigial for cell colors), while bubbles truly inherited top_index (monochrome for single-child roots). Both behaviors violate the spec "one pastel family per top-level branch, inherited"; the branch-root fix corrects all five uniformly.
- mod.rs: added pub effective_branch_root(tree, root) — descends single-sizeable-child (on_disk > 0, is_dir, child_count > 0) chains, ≤ 64 iterations, returns first node whose sizeable children branch; added pub(crate) depth_below(tree, node, root) parent-walk helper (shared; treemap's private depth_from_root deleted in favor of it)
- treemap.rs: treemap() computes branch_root once; layout_children threads branch_root + family; fam = if parent == branch_root { i } else { family } feeds node_color and the recursion
- sunburst.rs / flame.rs / mindmap.rs: same pattern — layout_ring/layout_row/layout_branches thread branch_root; fam = if node == branch_root { i } else { top_index } now feeds BOTH the cell color and the recursion (the old depth_here == 1 condition is gone)
- bubbles.rs: emit threads branch_root + branch_level (= depth_below(branch_root, root) + 1); child family condition if b.id == branch_root { i } else { top_index } (was depth == 0); alpha tiers ALPHA_PRIMARY 0xB4 (root container + chain + effective top-level branches) / ALPHA_NESTED 0xD9 (descendants) via (rgb << 8) | alpha
- mindmap.rs: same tiering — ALPHA_TOP 0xFF for dots at or above the branch level, ALPHA_NESTED 0xCC deeper (matches the DEV mock's 0xff/0xcc tiers); gray root dot unchanged
- sunburst.rs: center disc gray 0x8E8E93 → brand coral 0xFF6B4A, CENTER_R_FRACTION 0.12 → 0.16 (label drawn JS-side)
- treemap_groups (by-type/by-age regroup path) and groups.rs untouched; ALL public fn signatures unchanged (app crate calls unaffected)
- 5 new tests (single-child "This PC" → "C:" → 6 folders tree): treemap branch families + depth-3 inheritance, effective_branch_root descent, sunburst branch arcs + coral center/radius, bubbles branch families + 0xB4/0xD9 tiers, mindmap branch families + 0xFF/0xCC tiers
- Verified regression value: temporarily restoring each engine's OLD family logic makes its new test FAIL (bubbles: 1 family at branch level; treemap/sunburst/mindmap: depth-3 descendants collapse to family 0) — then restored the fix
- Validation: cargo fmt clean; cargo clippy -p diskbytes-core --all-targets -- -D warnings clean; cargo test -p diskbytes-core 132/132 pass (127 baseline + 5 new); cargo check -p diskbytes-core clean; workspace check fails only on pre-existing Linux gdk-sys system-dep (app crate targets Windows CI, unchanged public API)
- Incident + recovery: an accidental `git checkout -- treemap.rs` during diagnostics wiped that file's uncommitted changes; reapplied the identical edits and re-validated green (no other files affected; parallel agents' TS changes untouched)

Stage Summary:
- All 5 engines now assign one distinct pastel family per effective top-level branch (descending-size sibling index) and inherit it below; shade still varies by depth + sibling index via folder_family_color
- Geometry/cell counts unchanged (colors + alpha + sunburst center radius fraction only); no existing test needed updating (none asserted rgba values)
- 132/132 core tests pass, clippy pedantic clean, fmt clean; no TODO/FIXME; no new deps; portable Rust (is_some_and is stable ≥ 1.70, MSRV 1.80)
- Next: task B should port the same effective-branch-root family rule + alpha tiers into the DEV mock (src/mock/layouts.ts) so browser parity matches the Rust core

---
Task ID: B
Agent: canvas-mock-parity
Task: Canvas rendering polish (sunburst center label, colored mind-map links, double selection ring, white treemap separators) + unified effective-branch-root family coloring across all 5 mock modes

Work Log:
- src/mock/layouts.ts: added `effectiveBranchRoot` (descend while exactly ONE sizeable dir-with-children child; return first branching node — same semantics as Task C's Rust helper) + `buildFamilies` (families assign at branch-root's sizeable children, size-desc index; descendants inherit; chain above branch root gets family 0)
- Replaced per-mode tone threading in all 5 mock modes (squarify/sunburst ring/flame rows/bubbles/mind-map dots) with the family map; shade now varies by depth + sibling parity (mirrors core `folder_family_color`); TONE_BASE updated to the core FAMILIES palette (blue/teal/violet/amber/rose/green/sky/slate pastels) for color parity; ALL geometry math untouched
- Mock sunburst: r0 = rMax*0.16, added KIND_CIRCLE center cell (id=rootId, depth 0, rgba 0xFF6B4ABB, no DIR_BIT — matches Rust `Cell::circle`), rings start at r0+ringGap; bubbles 0xb4/0xd9 and mind-map 0xff/0xcc alphas kept
- meta.groups now lists the effective-branch-root children so footer legend chips match the canvas families (was: layout-root children)
- src/viz/CanvasViz.tsx: sunburst center label (root name 600 11px + `bytes(totalBytes)` 700 15px, white, soft dark shadow rgba(0,0,0,0.25)/blur 4, clip r*1.7, disc radius from min arc g[2] fallback min(w,h)*0.07); root id added to both names fetches (component warm-up + paint repaint) for sunburst, double-paint pattern kept; generic circle label skipped for the center disc
- Mind-map links now stroke the child DOT cell's own rgb at alpha 0.45 (lineWidth 1.8, same quadratic curve); treemap rects stroke rgba(255,255,255,0.55) 1.5px (flame keeps old hairline per spec); selection = coral 2.5px ring + new white rgba(255,255,255,0.85) 1px inner ring at 3px inset (ringPath gained `inset` param + degenerate-geometry guards; hover ring untouched)
- Browser verification loop (agent-browser): scan → Treemap/Sunburst/Flame/Bubbles/Mind Map + By type / By age screenshots; VLM-verified each; pixel-sampled mind-map link tints; selection double-ring verified on treemap rect + sunburst arc; drill-down into C: verified (families reassign, center label "Local Disk (C:) 129 GB")

Stage Summary:
- Deliverable screenshots: shots/trackB/{treemap,sunburst,flame,bubbles,mindmap,treemap-bytype,treemap-byage}.png — all multi-family (monochrome treemap bug fixed), sunburst shows coral center disc with white "This PC 129 GB", mind-map links tinted per child family, white treemap separators, labels readable, no console errors (agent-browser errors clean)
- npm run typecheck PASS; npm test 30/30 PASS (decode contract untouched)
- Mock semantics verified against Task C's landed Rust `effective_branch_root` (same descent rule; chain nodes get family 0 on both sides)
- Deviations (all color-only, within scope): TONE_BASE swapped to core FAMILIES palette; shade formula now mirrors core (depth+parity) per "shade varies by depth + sibling index"; legend groups re-based to branch-root children; center-label clip interpreted as r*1.7 (consistent with existing CIRCLE label r*1.6 pattern); chose shadowBlur halo over 2px dark halo for the center label
