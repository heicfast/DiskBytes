/**
 * Mock layout engine (DEV/TEST ONLY): JS implementations of the 5 canvas
 * layouts emitting the SAME framed binary buffer as Rust `get_layout`
 * with the EXACT cell geometry contract (core/src/layout/mod.rs):
 *   RECT   g=[x, y, w, h, 0]
 *   ARC    g=[a0, a1, r0, r1, 0]  (center from meta.center)
 *   CIRCLE g=[cx, cy, r, 0, 0]
 *   DOT    g=[x, y, r, px, py]    (px/py = parent link position)
 *   HEADER g=[x, y, w, h, 0]
 */
import type { MockNode, MockTree } from "./tree";
import { CATEGORY_COLORS } from "./tree";

export interface MockCell {
  id: number;
  depth: number;
  flags: number;
  rgba: number;
  g: [number, number, number, number, number];
}

/** Pastel families (parity with core `folder_family_color`): blue, teal,
 *  violet, amber, rose, green, sky, slate. */
const TONE_BASE = [0x93c5fd, 0x99f6e4, 0xc4b5fd, 0xfde69a, 0xfdcdd3, 0xbbf7d0, 0xbae6fd, 0xcbd5e1];
const AGE_COLORS = [0x34d399, 0x60a5fa, 0x818cf8, 0xa78bfa, 0xf472b6, 0xf87171];

const DIR_BIT = 1 << 3;
const KIND_RECT = 0;
const KIND_ARC = 1;
const KIND_CIRCLE = 2;
const KIND_DOT = 3;
const MAX_CELLS = 20000;

function shade(hex: number, f: number): number {
  const r = Math.round(((hex >>> 16) & 0xff) * f);
  const g = Math.round(((hex >>> 8) & 0xff) * f);
  const b = Math.round((hex & 0xff) * f);
  return (Math.min(255, r) << 16) | (Math.min(255, g) << 8) | Math.min(255, b);
}

function ageBucket(modified: number, now: number): number {
  const days = (now - modified) / 86400;
  if (days <= 7) return 0;
  if (days <= 30) return 1;
  if (days <= 91) return 2;
  if (days <= 365) return 3;
  if (days <= 730) return 4;
  return 5;
}

function colorFor(
  node: MockNode,
  family: number,
  depth: number,
  mode: string,
  now: number,
  sibling = 0,
): number {
  if (mode === "by-type") {
    return shade(CATEGORY_COLORS[node.isDir ? 8 : node.category], 1);
  }
  if (mode === "by-age") {
    return AGE_COLORS[ageBucket(node.modified, now)];
  }
  const base = TONE_BASE[family % TONE_BASE.length];
  // Shade varies by depth + sibling parity (mirrors core
  // `folder_family_color`: neighbors separate, staying pastel).
  const f = Math.max(0.7, 1 - Math.min(depth, 4) * 0.06 - (sibling % 2 === 1 ? 0.05 : 0));
  return shade(base, f);
}

interface Item {
  node: number;
  v: number;
}

function childrenSorted(tree: MockTree, id: number): Item[] {
  return tree.nodes[id].children
    .map((c) => ({ node: c, v: tree.nodes[c].onDisk || tree.nodes[c].logical || 1 }))
    .filter((k) => k.v > 0)
    .sort((a, b) => b.v - a.v);
}

/** Strictly sizeable children (onDisk||logical > 0), size-desc. */
function sizeableSorted(tree: MockTree, id: number): Item[] {
  return tree.nodes[id].children
    .map((c) => ({ node: c, v: tree.nodes[c].onDisk || tree.nodes[c].logical }))
    .filter((k) => k.v > 0)
    .sort((a, b) => b.v - a.v);
}

/**
 * The node where by-folder families assign (mirrors the Rust engines'
 * effective branch root): descend while the node has exactly ONE
 * sizeable child that is a dir with children (This PC → C:), so families
 * start at the first real branching level.
 */
function effectiveBranchRoot(tree: MockTree, rootId: number): number {
  let cur = rootId;
  for (let guard = 0; guard < 64; guard++) {
    const n = tree.nodes[cur];
    if (!n || !n.isDir) return cur;
    const sizeable = n.children.filter((c) => {
      const k = tree.nodes[c];
      return k !== undefined && (k.onDisk > 0 || k.logical > 0);
    });
    if (sizeable.length !== 1) return cur;
    const only = tree.nodes[sizeable[0]];
    if (!only.isDir || only.children.length === 0) return cur;
    cur = sizeable[0];
  }
  return cur;
}

/**
 * node id → by-folder family index: families assign at the sizeable
 * children of the effective branch root (index among them, size desc);
 * every descendant inherits its branch's family. The single-child chain
 * above the branch root carries the dominant family so container strips
 * blend with their content.
 */
function buildFamilies(tree: MockTree, rootId: number, branchRoot: number): Map<number, number> {
  const fam = new Map<number, number>();
  const kids = sizeableSorted(tree, branchRoot);
  kids.forEach((k, i) => {
    const f = i % TONE_BASE.length;
    const stack = [k.node];
    while (stack.length > 0) {
      const id = stack.pop() as number;
      fam.set(id, f);
      const ch = tree.nodes[id].children;
      for (let j = 0; j < ch.length; j++) stack.push(ch[j]);
    }
  });
  const leadFam = kids.length > 0 ? (fam.get(kids[0].node) ?? 0) : 0;
  let cur = rootId;
  while (cur >= 0 && cur !== branchRoot) {
    fam.set(cur, leadFam);
    const p = tree.nodes[cur].parent;
    if (p === undefined) break;
    cur = p;
  }
  fam.set(branchRoot, leadFam);
  return fam;
}

/** Squarified treemap (Bruls et al.) — recursive on folders. */
function squarify(
  values: Item[],
  rect: [number, number, number, number],
  out: MockCell[],
  tree: MockTree,
  depth: number,
  maxDepth: number,
  fam: Map<number, number>,
  colorMode: string,
  now: number,
): void {
  const [x, y, w, h] = rect;
  const total = values.reduce((a, b) => a + b.v, 0);
  if (total <= 0 || w <= 1 || h <= 1) return;
  const scale = (w * h) / total;
  let cx = x;
  let cy = y;
  let items = values.slice();
  while (items.length > 0 && out.length < MAX_CELLS - 6) {
    const horizontal = w - (cx - x) >= h - (cy - y);
    const side = horizontal ? h - (cy - y) : w - (cx - x);
    if (side <= 0.5) break;
    // grow a row while the worst aspect ratio improves
    let row: Item[] = [items[0]];
    let rowSum = items[0].v;
    let best = Infinity;
    for (let i = 1; i < items.length; i++) {
      const candSum = rowSum + items[i].v;
      const candLen = (candSum * scale) / side;
      let worst = 0;
      for (const c of [...row, items[i]]) {
        const cw = horizontal ? (c.v * scale) / candLen : candLen;
        const chh = horizontal ? candLen : (c.v * scale) / candLen;
        worst = Math.max(worst, Math.max(cw / chh, chh / cw));
      }
      if (worst > best) break;
      best = worst;
      row = [...row, items[i]];
      rowSum = candSum;
    }
    items = items.slice(row.length);
    const rowLen = Math.max(0.5, (rowSum * scale) / side);
    let off = 0;
    for (let ri = 0; ri < row.length; ri++) {
      const c = row[ri];
      const n = tree.nodes[c.node];
      const cLen = (c.v * scale) / rowLen;
      const r: [number, number, number, number] = horizontal
        ? [cx, cy + off, rowLen, cLen]
        : [cx + off, cy, cLen, rowLen];
      // By-folder family: assigned at the effective branch root's
      // children, inherited by descendants; shade varies by depth +
      // sibling index (spec: one pastel family per top-level branch).
      if (r[2] >= 6 && r[3] >= 6) {
        out.push({
          id: c.node,
          depth,
          flags: (n.isDir ? DIR_BIT : 0) | KIND_RECT,
          rgba:
            (colorFor(n, fam.get(c.node) ?? 0, depth, colorMode, now, values.indexOf(c)) << 8) | 0xff,
          g: [r[0], r[1], r[2], r[3], 0],
        });
      }
      if (n.isDir && depth < maxDepth) {
        const hdr = depth === 0 ? 15 : 12;
        squarify(
          childrenSorted(tree, c.node),
          [r[0] + 1, r[1] + hdr, Math.max(0, r[2] - 2), Math.max(0, r[3] - hdr - 1)],
          out,
          tree,
          depth + 1,
          maxDepth,
          fam,
          colorMode,
          now,
        );
      }
      off += cLen;
    }
    if (horizontal) cx += rowLen;
    else cy += rowLen;
  }
}

/** Build cells for a canvas mode (Rust geometry contract). */
export function buildLayout(
  tree: MockTree,
  rootId: number,
  mode: string,
  width: number,
  height: number,
  depth: number,
  colorMode: string,
): { meta: Record<string, unknown>; cells: MockCell[] } {
  const now = Math.floor(Date.now() / 1000);
  const cells: MockCell[] = [];
  const root = tree.nodes[rootId];
  const total = root.onDisk || root.logical || 1;
  const kids = childrenSorted(tree, rootId);
  // By-folder families: one pastel family per branch under the effective
  // branch root (mirrors the Rust engines' effective_branch_root).
  const branchRoot = effectiveBranchRoot(tree, rootId);
  const fam = buildFamilies(tree, rootId, branchRoot);
  const famOf = (id: number): number => fam.get(id) ?? 0;

  if (mode === "treemap") {
    squarify(kids, [0, 0, width, height], cells, tree, 0, depth, fam, colorMode, now);
  } else if (mode === "sunburst") {
    const rMax = Math.min(width, height) / 2 - 6;
    const r0 = rMax * 0.16;
    const ringGap = 1.2;
    // Center disc (matches the Rust engine): coral; the JS layer draws
    // the root name + total size centered on it.
    cells.push({
      id: rootId,
      depth: 0,
      flags: KIND_CIRCLE,
      rgba: 0xff6b4abb,
      g: [width / 2, height / 2, r0, 0, 0],
    });
    const ring = (
      items: Item[],
      rIn: number,
      rOut: number,
      a0: number,
      a1: number,
      d: number,
    ): void => {
      const sum = items.reduce((a, b) => a + b.v, 0);
      if (sum <= 0 || rOut - rIn < 2) return;
      let a = a0;
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        const span = (it.v / sum) * (a1 - a0);
        if (span > 0.006 && cells.length < MAX_CELLS) {
          const n = tree.nodes[it.node];
          cells.push({
            id: it.node,
            depth: d,
            flags: (n.isDir ? DIR_BIT : 0) | KIND_ARC,
            rgba: (colorFor(n, famOf(it.node), d, colorMode, now, i) << 8) | 0xff,
            g: [a + 0.0012, a + span - 0.0012, rIn, rOut, 0],
          });
          if (n.isDir && d < depth - 1) {
            ring(
              childrenSorted(tree, it.node),
              rOut + ringGap,
              rOut + (rMax - rOut - ringGap) / Math.max(1, depth - 1 - d),
              a,
              a + span,
              d + 1,
            );
          }
        }
        a += span;
      }
    };
    ring(kids, r0 + ringGap, rMax * 0.5, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2, 0);
  } else if (mode === "flame") {
    const rowH = Math.min(88, (height - 6) / Math.max(3, Math.min(depth, 6)));
    const rows: { item: Item; a0: number; a1: number; sib: number; d: number }[] = [];
    const layout = (items: Item[], a0: number, a1: number, d: number): void => {
      const sum = items.reduce((a, b) => a + b.v, 0);
      if (sum <= 0 || d >= Math.min(depth, 6)) return;
      let a = a0;
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        const span = (it.v / sum) * (a1 - a0);
        if (span * width > 1.5) {
          rows.push({ item: it, a0: a, a1: a + span, sib: i, d });
          if (tree.nodes[it.node].isDir) {
            layout(childrenSorted(tree, it.node), a, a + span, d + 1);
          }
        }
        a += span;
      }
    };
    layout(kids, 0, 1, 0);
    for (const r of rows.slice(0, MAX_CELLS)) {
      const n = tree.nodes[r.item.node];
      cells.push({
        id: r.item.node,
        depth: r.d,
        flags: (n.isDir ? DIR_BIT : 0) | KIND_RECT,
        rgba: (colorFor(n, famOf(r.item.node), r.d, colorMode, now, r.sib) << 8) | 0xff,
        g: [r.a0 * width + 1, r.d * rowH + 1, Math.max(1, r.a1 * width - r.a0 * width - 2), rowH - 2.5, 0],
      });
    }
  } else if (mode === "bubbles") {
    const cx = width / 2;
    const cy = height / 2;
    const R = Math.min(width, height) / 2 - 8;
    const sqrtSum = kids.reduce((a, b) => a + Math.sqrt(b.v), 0) || 1;
    let ang = -Math.PI / 2;
    for (let i = 0; i < kids.length && cells.length < MAX_CELLS; i++) {
      const k = kids[i];
      const rr = Math.min((Math.sqrt(k.v) / sqrtSum) * R * 1.75, R * 0.6);
      const dist = R - rr - 2;
      const x = cx + Math.cos(ang) * dist;
      const y = cy + Math.sin(ang) * dist;
      const n = tree.nodes[k.node];
      cells.push({
        id: k.node,
        depth: 1,
        flags: (n.isDir ? DIR_BIT : 0) | KIND_CIRCLE,
        rgba: (colorFor(n, famOf(k.node), 1, colorMode, now, i) << 8) | 0xb4,
        g: [x, y, rr, 0, 0],
      });
      if (n.isDir && depth > 1) {
        const sub = childrenSorted(tree, k.node).slice(0, 14);
        const ssum = sub.reduce((a, b) => a + Math.sqrt(b.v), 0) || 1;
        let sa = 0;
        let sj = 0;
        for (const s of sub) {
          const sr = Math.max(2.5, (Math.sqrt(s.v) / ssum) * rr * 0.62);
          const sd = rr - sr - 1.5;
          const sn = tree.nodes[s.node];
          cells.push({
            id: s.node,
            depth: 2,
            flags: (sn.isDir ? DIR_BIT : 0) | KIND_CIRCLE,
            rgba: (colorFor(sn, famOf(s.node), 2, colorMode, now, sj) << 8) | 0xd9,
            g: [x + Math.cos(sa) * sd, y + Math.sin(sa) * sd, sr, 0, 0],
          });
          sa += 0.55;
          sj += 1;
        }
      }
      ang += Math.max(0.42, (Math.sqrt(k.v) / sqrtSum) * 6.4);
    }
  } else if (mode === "mind-map") {
    const cx = width / 2;
    const cy = height / 2;
    const sum = kids.reduce((a, b) => a + b.v, 0) || 1;
    const rMax = Math.min(width, height) / 2 - 44;
    for (let i = 0; i < kids.length; i++) {
      const k = kids[i];
      const ang = (i / kids.length) * Math.PI * 2 - Math.PI / 2 + 0.18;
      const rr = Math.max(9, Math.sqrt(k.v / sum) * rMax * 0.95);
      const dist = rMax - rr;
      const x = cx + Math.cos(ang) * dist;
      const y = cy + Math.sin(ang) * dist;
      const n = tree.nodes[k.node];
      cells.push({
        id: k.node,
        depth: 1,
        flags: (n.isDir ? DIR_BIT : 0) | KIND_DOT,
        rgba: (colorFor(n, famOf(k.node), 1, colorMode, now, i) << 8) | 0xff,
        g: [x, y, rr, cx, cy],
      });
      if (n.isDir && depth > 1) {
        const sub = childrenSorted(tree, k.node).slice(0, 8);
        const ssum = sub.reduce((a, b) => a + b.v, 0) || 1;
        let sa = ang - 0.66;
        let sj = 0;
        for (const s of sub) {
          const sr = Math.max(3.5, Math.sqrt(s.v / ssum) * rr * 0.44);
          const sd = rr + sr + 14;
          const sn = tree.nodes[s.node];
          cells.push({
            id: s.node,
            depth: 2,
            flags: (sn.isDir ? DIR_BIT : 0) | KIND_DOT,
            rgba: (colorFor(sn, famOf(s.node), 2, colorMode, now, sj) << 8) | 0xcc,
            g: [x + Math.cos(sa) * sd, y + Math.sin(sa) * sd, sr, x, y],
          });
          sa += 0.27;
          sj += 1;
        }
      }
    }
  }

  const truncated = cells.length > MAX_CELLS;
  const finalCells = truncated ? cells.slice(0, MAX_CELLS) : cells;
  // Legend groups mirror the family level (children of the effective
  // branch root) so the chips match the colors on the canvas.
  const branchKids = sizeableSorted(tree, branchRoot);
  const groups = branchKids.slice(0, 8).map((k, i) => ({
    id: 0xffff0000 + i,
    name: tree.nodes[k.node].name,
    color: colorFor(tree.nodes[k.node], famOf(k.node), 0, colorMode, now, 0),
    size: k.v,
  }));

  return {
    meta: {
      mode,
      generation: tree.generation,
      node: rootId,
      width,
      height,
      depth,
      colorMode,
      cellCount: finalCells.length,
      truncated,
      center: [width / 2, height / 2] as [number, number],
      groups,
      totalBytes: total,
    },
    cells: finalCells,
  };
}

/** Encode cells into the framed binary layout buffer. */
export function encodeLayout(meta: Record<string, unknown>, cells: MockCell[]): ArrayBuffer {
  const metaJson = JSON.stringify(meta);
  const metaBytes = new TextEncoder().encode(metaJson);
  const buf = new ArrayBuffer(4 + metaBytes.length + cells.length * 32);
  const view = new DataView(buf);
  view.setUint32(0, metaBytes.length, true);
  new Uint8Array(buf, 4, metaBytes.length).set(metaBytes);
  let o = 4 + metaBytes.length;
  for (const c of cells) {
    view.setUint32(o, c.id >>> 0, true);
    view.setUint16(o + 4, c.depth, true);
    view.setUint16(o + 6, c.flags, true);
    view.setUint32(o + 8, c.rgba >>> 0, true);
    view.setFloat32(o + 12, c.g[0], true);
    view.setFloat32(o + 16, c.g[1], true);
    view.setFloat32(o + 20, c.g[2], true);
    view.setFloat32(o + 24, c.g[3], true);
    view.setFloat32(o + 28, c.g[4], true);
    o += 32;
  }
  return buf;
}
