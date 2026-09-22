/**
 * Layout IPC (spec §7; doc 03 M4.1): invoke `get_layout`, decode the
 * framed binary response `[u32 meta_len LE][meta JSON][cells 32 B each]`,
 * keep a decode cache, and batch name lookups (`get_names`) with a small
 * LRU. Results whose generation is stale are dropped (spec §9 rule).
 */
import { invoke } from "../lib/ipc";

export type ColorMode = "by-folder" | "by-type" | "by-age";
export type CanvasMode = "treemap" | "sunburst" | "flame" | "bubbles" | "mind-map";

export interface GroupDesc {
  id: number;
  name: string;
  color: number;
  size: number;
}

export interface LayoutMeta {
  mode: string;
  generation: number;
  node: number;
  width: number;
  height: number;
  depth: number;
  colorMode: ColorMode;
  cellCount: number;
  truncated: boolean;
  center: [number, number] | null;
  groups: GroupDesc[];
  totalBytes: number;
}

/** One decoded cell (32 bytes on the wire; id/depth/flags/rgba/5×f32). */
export interface Cell {
  id: number;
  depth: number;
  flags: number;
  rgba: number;
  /** Geometry — meaning depends on the mode (rect: x,y,w,h + header
   * flag; circle/arc/dot variants). */
  g: [number, number, number, number, number];
}

export interface LayoutResult {
  meta: LayoutMeta;
  cells: Cell[];
}

export interface LayoutRequest {
  generation: number;
  node: number;
  mode: CanvasMode;
  width: number;
  height: number;
  depth: number;
  color: ColorMode;
}

function reqKey(r: LayoutRequest): string {
  return `${r.generation}:${r.node}:${r.mode}:${r.width}x${r.height}:${r.depth}:${r.color}`;
}

/** Normalize the invoke payload into a byte view. The custom-protocol
 *  IPC path hands us an ArrayBuffer, but the postMessage fallback (and
 *  some webviews) deliver raw bodies as number arrays or typed arrays —
 *  a DataView on the wrong shape throws and the canvas silently blanks.
 */
function asBytes(buffer: ArrayBuffer | Uint8Array | number[]): Uint8Array {
  if (buffer instanceof ArrayBuffer) return new Uint8Array(buffer);
  if (ArrayBuffer.isView(buffer)) return new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength);
  const out = new Uint8Array(buffer.length);
  for (let i = 0; i < buffer.length; i++) out[i] = buffer[i] & 0xff;
  return out;
}

/** Decode the framed binary body. */
export function decodeLayout(buffer: ArrayBuffer | Uint8Array | number[]): LayoutResult {
  const bytes = asBytes(buffer);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const metaLen = view.getUint32(0, true);
  const metaJson = new TextDecoder().decode(bytes.subarray(4, 4 + metaLen));
  const meta = JSON.parse(metaJson) as LayoutMeta;
  const cellsStart = 4 + metaLen;
  const cellCount = Math.floor((bytes.byteLength - cellsStart) / 32);
  const cells: Cell[] = new Array(cellCount);
  for (let i = 0; i < cellCount; i++) {
    const o = cellsStart + i * 32;
    cells[i] = {
      id: view.getUint32(o, true),
      depth: view.getUint16(o + 4, true),
      flags: view.getUint16(o + 6, true),
      rgba: view.getUint32(o + 8, true),
      g: [
        view.getFloat32(o + 12, true),
        view.getFloat32(o + 16, true),
        view.getFloat32(o + 20, true),
        view.getFloat32(o + 24, true),
        view.getFloat32(o + 28, true),
      ],
    };
  }
  return { meta, cells };
}

/** JS-side decode cache (doc 03 M4.1); the Rust side caches too. */
const cache = new Map<string, LayoutResult>();

/** Fetch + decode a layout (cached). Returns null when the generation
 *  is stale (the caller re-requests with the current generation). */
export async function getLayout(req: LayoutRequest): Promise<LayoutResult | null> {
  const key = reqKey(req);
  const hit = cache.get(key);
  if (hit) return hit;
  try {
    const raw = (await invoke("get_layout", { req })) as
      | ArrayBuffer
      | Uint8Array
      | number[];
    const result = decodeLayout(raw);
    if (result.meta.generation !== req.generation) return null;
    cache.set(key, result);
    // Simple eviction: keep the cache bounded.
    if (cache.size > 48) {
      const first = cache.keys().next().value;
      if (first !== undefined) cache.delete(first);
    }
    return result;
  } catch (e) {
    if (String(e).includes("stale generation")) return null;
    throw e;
  }
}

/** Drop cached layouts for a generation (called on scan-done). */
export function invalidateLayouts(): void {
  cache.clear();
  namesLru.clear();
}

/** Batched name lookups with a tiny LRU (doc 03 M4.1). */
const namesLru = new Map<number, string>();

export async function getNames(generation: number, ids: number[]): Promise<Map<number, string>> {
  const out = new Map<number, string>();
  const missing: number[] = [];
  for (const id of ids) {
    if (namesLru.has(id)) {
      const name = namesLru.get(id)!;
      // LRU touch.
      namesLru.delete(id);
      namesLru.set(id, name);
      out.set(id, name);
    } else {
      missing.push(id);
    }
  }
  if (missing.length > 0) {
    const fetched = (await invoke<string[]>("get_names", { generation, ids: missing }).catch(
      () => null,
    )) as string[] | null;
    if (fetched) {
      for (let i = 0; i < missing.length && i < fetched.length; i++) {
        namesLru.set(missing[i], fetched[i]);
        out.set(missing[i], fetched[i]);
      }
      // Bound the LRU (label-heavy modes touch thousands).
      while (namesLru.size > 4096) {
        const oldest = namesLru.keys().next().value;
        if (oldest === undefined) break;
        namesLru.delete(oldest);
      }
    }
  }
  return out;
}

/** rgba 0xRRGGBBAA → "rgba(r,g,b,a)" canvas string. */
export function cssRgba(rgba: number): string {
  const r = (rgba >>> 24) & 0xff;
  const g = (rgba >>> 16) & 0xff;
  const b = (rgba >>> 8) & 0xff;
  const a = (rgba & 0xff) / 255;
  return `rgba(${r},${g},${b},${a})`;
}

/** Cell kind flags (mirrors core layout::cell_kind). */
export const CELL_KIND = {
  RECT: 0,
  ARC: 1,
  CIRCLE: 2,
  DOT: 3,
  HEADER: 4,
} as const;

/** Extra cell flag: the node is a directory (mirrors core DIR_BIT). */
export const DIR_BIT = 1 << 3;

/** Hover-chip details (mirrors Rust `HoverDetails`). */
export interface HoverDetailsData {
  id: number;
  name: string;
  isDir: boolean;
  size: number;
  shareOfScan: number;
  fileCount: number;
  isCloud: boolean;
  isProtected: boolean;
  category: string;
  categoryColor: number;
}

const hoverCache = new Map<number, HoverDetailsData>();

/** Fetch hover details (cached per id until the generation changes). */
export async function getHoverDetails(
  generation: number,
  id: number,
): Promise<HoverDetailsData | null> {
  const hit = hoverCache.get(id);
  if (hit) return hit;
  try {
    const d = (await invoke<HoverDetailsData>("hover_details", { generation, id })) as HoverDetailsData;
    hoverCache.set(id, d);
    while (hoverCache.size > 1024) {
      const oldest = hoverCache.keys().next().value;
      if (oldest === undefined) break;
      hoverCache.delete(oldest);
    }
    return d;
  } catch {
    return null;
  }
}

/** Drop hover details on scan swap. */
export function invalidateHoverCache(): void {
  hoverCache.clear();
}
