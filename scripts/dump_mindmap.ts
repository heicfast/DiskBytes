/** Deterministic dump of the mock mind-map layout (debug N12 parity). */
import { buildLayout } from "../src/mock/layouts";
import { MockTree } from "../src/mock/tree";

const tree = new MockTree();
const res = buildLayout(tree, 0, "mind-map", 900, 700, 7, "by-folder");
const meta = res.meta as Record<string, unknown>;
console.log("cells:", res.cells.length, "meta:", JSON.stringify({ ...meta, groups: (meta.groups as unknown[]).length }));
const byDepth = new Map<number, number>();
for (const c of res.cells) byDepth.set(c.depth, (byDepth.get(c.depth) ?? 0) + 1);
console.log("by depth:", JSON.stringify([...byDepth.entries()]));
// Level structure: root dot + first 12 cells with geometry
for (const c of res.cells.slice(0, 13)) {
  console.log(
    `id=${c.id} d=${c.depth} dot=(${c.g[0].toFixed(0)},${c.g[1].toFixed(0)}) r=${c.g[2].toFixed(1)} parent=(${c.g[3].toFixed(0)},${c.g[4].toFixed(0)}) rgba=${(c.rgba >>> 0).toString(16)}`,
  );
}
