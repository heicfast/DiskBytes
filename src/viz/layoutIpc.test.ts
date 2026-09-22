/**
 * Layout IPC decode tests (spec §7 / doc 03 M4.1): the framed
 * `[u32 meta_len LE][meta JSON][32 B cells][8 B sizes per cell]` contract
 * must decode from EVERY transport shape — the custom-protocol path hands
 * us an ArrayBuffer, the postMessage fallback can deliver number arrays or
 * typed arrays. A DataView on the wrong shape throws and the canvas
 * silently blanks (the CI blank-treemap bug). The sizes tail is optional
 * (legacy frames decode with size 0).
 */
import { describe, expect, it } from "vitest";
import { decodeLayout } from "./layoutIpc";

interface TCell {
  id: number;
  depth: number;
  flags: number;
  rgba: number;
  g: [number, number, number, number, number];
}

function frame(
  meta: Record<string, unknown>,
  cells: TCell[],
  sizes?: number[],
): ArrayBuffer {
  const metaJson = new TextEncoder().encode(JSON.stringify(meta));
  const tail = sizes ? cells.length * 8 : 0;
  const out = new ArrayBuffer(4 + metaJson.length + cells.length * 32 + tail);
  const view = new DataView(out);
  view.setUint32(0, metaJson.length, true);
  new Uint8Array(out, 4).set(metaJson);
  let o = 4 + metaJson.length;
  for (const c of cells) {
    view.setUint32(o, c.id, true);
    view.setUint16(o + 4, c.depth, true);
    view.setUint16(o + 6, c.flags, true);
    view.setUint32(o + 8, c.rgba, true);
    const g = c.g;
    view.setFloat32(o + 12, g[0], true);
    view.setFloat32(o + 16, g[1], true);
    view.setFloat32(o + 20, g[2], true);
    view.setFloat32(o + 24, g[3], true);
    view.setFloat32(o + 28, g[4], true);
    o += 32;
  }
  if (sizes) {
    for (const s of sizes) {
      // u64 LE via two u32 halves (DataView has no setUint64).
      view.setUint32(o, s % 4294967296, true);
      view.setUint32(o + 4, Math.floor(s / 4294967296), true);
      o += 8;
    }
  }
  return out;
}

const META = {
  mode: "treemap",
  generation: 7,
  node: 3,
  width: 800,
  height: 600,
  depth: 4,
  colorMode: "by-folder",
  cellCount: 1,
  truncated: false,
  center: null,
  groups: [],
  totalBytes: 4096,
};

const CELLS: TCell[] = [
  {
    id: 42,
    depth: 1,
    flags: 0x8,
    rgba: 0x7f_ff_00_00,
    g: [10, 20, 100, 80, 0],
  },
];

describe("decodeLayout transport shapes", () => {
  it("decodes an ArrayBuffer (custom-protocol path)", () => {
    const res = decodeLayout(frame(META, CELLS));
    expect(res.meta.generation).toBe(7);
    expect(res.meta.mode).toBe("treemap");
    expect(res.cells).toHaveLength(1);
    expect(res.cells[0].id).toBe(42);
    expect(res.cells[0].depth).toBe(1);
    expect(res.cells[0].flags).toBe(0x8);
    expect(res.cells[0].rgba).toBe(0x7f_ff_00_00);
    expect(res.cells[0].g[0]).toBeCloseTo(10);
    expect(res.cells[0].g[3]).toBeCloseTo(80);
  });

  it("decodes a Uint8Array view with a nonzero byteOffset", () => {
    const whole = new Uint8Array(frame(META, CELLS));
    // Slice with a 3-byte offset to force byteOffset != 0.
    const shifted = new Uint8Array(3 + whole.length);
    shifted.set(whole, 3);
    const view = shifted.subarray(3);
    const res = decodeLayout(view);
    expect(res.cells[0].id).toBe(42);
    expect(res.cells[0].g[1]).toBeCloseTo(20);
  });

  it("decodes a plain number array (postMessage fallback shape)", () => {
    const numbers = Array.from(new Uint8Array(frame(META, CELLS)));
    const res = decodeLayout(numbers);
    expect(res.meta.generation).toBe(7);
    expect(res.cells[0].id).toBe(42);
    expect(res.cells[0].g[4]).toBeCloseTo(0);
  });

  it("reports zero cells when only the header is present", () => {
    const res = decodeLayout(frame({ ...META, cellCount: 0 }, []));
    expect(res.cells).toHaveLength(0);
    expect(res.meta.cellCount).toBe(0);
  });

  it("keeps every field through a full 32-byte cell round trip", () => {
    const cell: TCell[] = [
      {
        id: 0xffff_0001,
        depth: 9,
        flags: 0x1234,
        rgba: 0xdead_beef,
        g: [1.5, -2.25, 3e-7, 65504, 0.5],
      },
    ];
    const res = decodeLayout(frame(META, cell));
    const c = res.cells[0];
    expect(c.id).toBe(0xffff_0001);
    expect(c.depth).toBe(9);
    expect(c.flags).toBe(0x1234);
    expect(c.rgba).toBe(0xdead_beef);
    expect(c.g[0]).toBeCloseTo(1.5);
    expect(c.g[1]).toBeCloseTo(-2.25);
    expect(c.g[2]).toBeCloseTo(3e-7, 5);
    expect(c.g[3]).toBeCloseTo(65504);
    expect(c.g[4]).toBeCloseTo(0.5);
    // No sizes tail → size decodes as 0 (legacy frames stay readable).
    expect(c.size).toBe(0);
  });

  it("decodes the u64 sizes tail in cell order", () => {
    const cells: TCell[] = [
      { id: 1, depth: 1, flags: 0, rgba: 0, g: [0, 0, 10, 10, 0] },
      { id: 2, depth: 1, flags: 0, rgba: 0, g: [10, 0, 10, 10, 0] },
      { id: 0xffff_0000, depth: 0, flags: 0, rgba: 0, g: [0, 10, 20, 4, 0] },
    ];
    const meta = { ...META, cellCount: 3 };
    // 4.5 TB exercises the high u32 half (> 2^32).
    const sizes = [4096, 12_884_901_888_832, 7];
    const res = decodeLayout(frame(meta, cells, sizes));
    expect(res.cells).toHaveLength(3);
    expect(res.cells[0].size).toBe(4096);
    expect(res.cells[1].size).toBe(12_884_901_888_832);
    expect(res.cells[2].size).toBe(7);
  });

  it("does not confuse the cell count when a sizes tail is present", () => {
    // Legacy length-derived counting would read (32N + 8N)/32 > N cells;
    // meta.cellCount must win.
    const cells: TCell[] = [
      { id: 1, depth: 1, flags: 0, rgba: 0, g: [0, 0, 10, 10, 0] },
      { id: 2, depth: 1, flags: 0, rgba: 0, g: [10, 0, 10, 10, 0] },
    ];
    const res = decodeLayout(frame({ ...META, cellCount: 2 }, cells, [1, 2]));
    expect(res.cells).toHaveLength(2);
    expect(res.cells[1].size).toBe(2);
  });
});
