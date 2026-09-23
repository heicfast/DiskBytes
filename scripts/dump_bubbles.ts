/** Deterministic dump of the mock bubbles layout (fill-fit audit). */
import { buildLayout } from "../src/mock/layouts";
import { MockTree } from "../src/mock/tree";

const tree = new MockTree();
const res = buildLayout(tree, 0, "bubbles", 818, 567, 7, "by-folder");
const meta = res.meta as Record<string, unknown>;
const circles = res.cells.filter((c) => (c.flags & 0b111) === 2); // KIND_CIRCLE
console.log("circles:", circles.length);
for (const c of circles.slice(0, 14)) {
  const [x, y, r] = c.g;
  console.log(
    `id=${c.id} d=${c.depth} c=(${x.toFixed(0)},${y.toFixed(0)}) r=${r.toFixed(1)}`,
  );
}
// fill-fit check: for each parent-child pair (child g[3],g[4] unused for circles — use containment)
// find the max child extent within each container
const byDepth = new Map<number, typeof circles>();
for (const c of circles) {
  const arr = byDepth.get(c.depth) ?? [];
  arr.push(c);
  byDepth.set(c.depth, arr);
}
const roots = byDepth.get(1) ?? [];
for (const p of roots) {
  // children = depth-2 circles whose center is inside p
  const kids = (byDepth.get(2) ?? []).filter(
    (k) => Math.hypot(k.g[0] - p.g[0], k.g[1] - p.g[1]) < p.g[2],
  );
  if (!kids.length) continue;
  const maxKidR = Math.max(...kids.map((k) => Math.hypot(k.g[0] - p.g[0], k.g[1] - p.g[1]) + k.g[2]));
  console.log(
    `container id=${p.id} r=${p.g[2].toFixed(1)} -> max child extent ${maxKidR.toFixed(1)} (fill ${(maxKidR / p.g[2] * 100).toFixed(1)}%)`,
  );
}
