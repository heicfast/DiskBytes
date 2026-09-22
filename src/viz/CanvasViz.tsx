/**
 * CanvasViz (spec §7 rendering performance): the heavy static layer is
 * painted ONCE per layout on one canvas; the hover highlight + selection
 * ring live on a separate overlay canvas driven by refs — mouse moves
 * NEVER re-render React or repaint the static layer. devicePixelRatio
 * and ResizeObserver are handled; hit-testing is local JS geometry.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { bytes } from "../lib/format";
import { abbreviate } from "./abbrev";
import {
  CELL_KIND, DIR_BIT, cssRgba, getLayout, getNames, type Cell, type LayoutResult,
} from "./layoutIpc";

export interface CanvasVizProps {
  generation: number;
  node: number;
  mode: "treemap" | "sunburst" | "flame" | "bubbles" | "mind-map";
  colorMode: "by-folder" | "by-type" | "by-age";
  depth: number;
  abbreviateLabels: boolean;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  onOpen: (id: number) => void;
  onContextMenu: (id: number, x: number, y: number) => void;
  onHover: (id: number | null, x: number, y: number) => void;
}

const ON_PASTEL = "#0F172A";
const ON_PASTEL_2 = "#475569";

interface ThemeColors {
  border: string;
  bg: string;
}

function readTheme(): ThemeColors {
  const cs = getComputedStyle(document.documentElement);
  return {
    border: cs.getPropertyValue("--border").trim() || "#d8d8dd",
    bg: cs.getPropertyValue("--background").trim() || "#ffffff",
  };
}

export function CanvasViz(props: CanvasVizProps) {
  const { generation, node, mode, colorMode, depth } = props;
  const shellRef = useRef<HTMLDivElement>(null);
  const staticRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [layout, setLayout] = useState<LayoutResult | null>(null);
  const loadError = useRef<string | null>(null);
  const hoverCell = useRef<Cell | null>(null);
  const theme = useRef<ThemeColors>(readTheme());

  // ResizeObserver (debounced per settle — doc 05 §6)
  useEffect(() => {
    const el = shellRef.current;
    if (!el) return;
    let t: number | null = null;
    const ro = new ResizeObserver((entries) => {
      const e = entries[0];
      if (!e) return;
      if (t !== null) window.clearTimeout(t);
      t = window.setTimeout(() => {
        setSize({ w: Math.floor(e.contentRect.width), h: Math.floor(e.contentRect.height) });
      }, 120);
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      if (t !== null) window.clearTimeout(t);
    };
  }, []);

  // Theme tracking (repaint when data-theme flips)
  useEffect(() => {
    const obs = new MutationObserver(() => {
      theme.current = readTheme();
      const s = staticRef.current;
      if (s && layout) paint(s, layout, props);
    });
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => obs.disconnect();
  });

  // Layout fetch
  const reqKey = `${generation}:${node}:${mode}:${size.w}x${size.h}:${depth}:${colorMode}`;
  const [errorText, setErrorText] = useState<string | null>(null);
  useEffect(() => {
    if (size.w < 40 || size.h < 40) return;
    let disposed = false;
    loadError.current = null;
    setErrorText(null);
    void (async () => {
      let res: Awaited<ReturnType<typeof getLayout>>;
      try {
        res = await getLayout({
          generation,
          node,
          mode,
          width: size.w,
          height: size.h,
          depth,
          color: colorMode,
        });
      } catch (e) {
        // Surface the failure — a silently blank canvas is
        // undiagnosable from CI screenshots (this is exactly how the
        // blank-treemap bug hid for two runs).
        const msg = e instanceof Error ? e.message : String(e);
        console.error("[viz] layout failed:", msg);
        if (!disposed) setErrorText(msg);
        return;
      }
      if (disposed) return;
      if (!res) {
        loadError.current = "stale";
        return;
      }
      // Batch-fetch labels for the biggest cells (spec: get_names batched).
      const labelIds = res.cells
        .filter((c) => labelable(c, mode))
        .slice(0, 400)
        .map((c) => c.id);
      // Sunburst draws the root name in the center disc — make sure it
      // resolves with the same batch (paint() re-fetches for the repaint).
      if (mode === "sunburst") labelIds.push(res.meta.node);
      await getNames(generation, labelIds).catch(() => new Map<number, string>());
      if (disposed) return;
      setLayout(res);
    })();
    return () => {
      disposed = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reqKey, size.w, size.h]);

  const cellsById = useMemo(() => {
    const m = new Map<number, Cell>();
    if (layout) for (const c of layout.cells) m.set(c.id, c);
    return m;
  }, [layout]);

  // ── Static paint (once per layout) ─────────────────────────────────
  useEffect(() => {
    const s = staticRef.current;
    if (!s || !layout || size.w < 40) return;
    paint(s, layout, props);
    // Repaint when selection changes (a user action, not hover).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layout, props.selectedId, props.abbreviateLabels]);

  // ── Overlay: hover + selection rings ───────────────────────────────
  const paintOverlay = useCallback(() => {
    const o = overlayRef.current;
    if (!o || !layout) return;
    const dpr = window.devicePixelRatio || 1;
    if (o.width !== Math.round(size.w * dpr)) {
      o.width = Math.round(size.w * dpr);
      o.height = Math.round(size.h * dpr);
    }
    o.style.width = `${size.w}px`;
    o.style.height = `${size.h}px`;
    const ctx = o.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, size.w, size.h);
    // selection ring (user action): coral outer stroke + a refined thin
    // white inner stroke (reference's selected-cell double-ring).
    const sel = props.selectedId != null ? cellsById.get(props.selectedId) : undefined;
    if (sel) {
      ctx.strokeStyle = "rgba(255,107,74,0.95)";
      ctx.lineWidth = 2.5;
      ringPath(ctx, sel, layout, mode);
      ctx.strokeStyle = "rgba(255,255,255,0.85)";
      ctx.lineWidth = 1;
      ringPath(ctx, sel, layout, mode, 3);
    }
    // hover ring (overlay canvas only — never React state)
    const hv = hoverCell.current;
    if (hv && hv !== sel) {
      ctx.strokeStyle = "rgba(29,29,31,0.85)";
      ctx.lineWidth = 1.6;
      ringPath(ctx, hv, layout, mode);
    }
  }, [layout, size, props.selectedId, cellsById, mode]);

  useEffect(() => {
    paintOverlay();
  }, [paintOverlay]);

  // ── Pointer events (refs only — no React state on move) ────────────
  useEffect(() => {
    const s = staticRef.current;
    if (!s || !layout) return;
    const hit = (x: number, y: number): Cell | null => hitCell(layout.cells, mode, x, y, layout.meta.center);
    const onMove = (e: PointerEvent) => {
      const r = s.getBoundingClientRect();
      const cell = hit(e.clientX - r.left, e.clientY - r.top);
      const prev = hoverCell.current;
      if (cell?.id !== prev?.id) {
        hoverCell.current = cell;
        paintOverlay();
        props.onHover(cell ? cell.id : null, e.clientX, e.clientY);
      } else if (cell) {
        props.onHover(cell.id, e.clientX, e.clientY);
      }
      s.style.cursor = cell ? "pointer" : "default";
    };
    const onLeave = () => {
      hoverCell.current = null;
      paintOverlay();
      props.onHover(null, 0, 0);
    };
    const onDown = (e: PointerEvent) => {
      const r = s.getBoundingClientRect();
      const cell = hit(e.clientX - r.left, e.clientY - r.top);
      props.onSelect(cell ? cell.id : null);
    };
    const onDbl = (e: MouseEvent) => {
      const r = s.getBoundingClientRect();
      const cell = hit(e.clientX - r.left, e.clientY - r.top);
      if (cell && (cell.flags & DIR_BIT) !== 0) props.onOpen(cell.id);
    };
    const onCtx = (e: MouseEvent) => {
      const r = s.getBoundingClientRect();
      const cell = hit(e.clientX - r.left, e.clientY - r.top);
      if (cell) {
        e.preventDefault();
        props.onContextMenu(cell.id, e.clientX, e.clientY);
      }
    };
    s.addEventListener("pointermove", onMove);
    s.addEventListener("pointerleave", onLeave);
    s.addEventListener("pointerdown", onDown);
    s.addEventListener("dblclick", onDbl);
    s.addEventListener("contextmenu", onCtx);
    return () => {
      s.removeEventListener("pointermove", onMove);
      s.removeEventListener("pointerleave", onLeave);
      s.removeEventListener("pointerdown", onDown);
      s.removeEventListener("dblclick", onDbl);
      s.removeEventListener("contextmenu", onCtx);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layout, mode, paintOverlay]);

  return (
    <div className="db-viz-wrap">
      <div ref={shellRef} className="db-viz-canvas-shell">
        {size.w >= 40 && size.h >= 40 && <canvas ref={staticRef} />}
        {size.w >= 40 && size.h >= 40 && <canvas ref={overlayRef} className="db-overlay-canvas" />}
        {errorText && (
          <div className="db-viz-error" role="alert">
            <strong>Couldn’t load this view.</strong>
            <span>{errorText}</span>
          </div>
        )}
      </div>
      <div className="db-viz-foot">
        <div className="db-viz-groups">
          {layout?.meta.groups.slice(0, 6).map((g) => (
            <span key={g.id}>
              <i style={{ background: cssRgba((g.color << 8) | 0xff) }} />
              {g.name}
            </span>
          ))}
        </div>
        <span className="tnum">
          {layout ? `${layout.meta.cellCount.toLocaleString()} cells` : "…"}
          {layout?.meta.truncated ? " (truncated)" : ""}
        </span>
      </div>
    </div>
  );
}

/** Which cells deserve labels (bounded — spec ≤20k cells, label the big). */
function labelable(c: Cell, mode: string): boolean {
  const kind = c.flags & 0b111;
  if (kind === CELL_KIND.HEADER) return false;
  if (mode === "treemap" || mode === "flame") {
    return c.g[2] >= 36 && c.g[3] >= 15;
  }
  if (mode === "bubbles" || mode === "mind-map") {
    return c.g[2] >= 26;
  }
  return false; // sunburst labels drawn from arc math
}

/** Paint the static layer. */
function paint(canvas: HTMLCanvasElement, layout: LayoutResult, props: CanvasVizProps): void {
  const dpr = window.devicePixelRatio || 1;
  const w = layout.meta.width;
  const h = layout.meta.height;
  if (canvas.width !== Math.round(w * dpr)) {
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
  }
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  const mode = layout.meta.mode;
  const cx = layout.meta.center?.[0] ?? w / 2;
  const cy = layout.meta.center?.[1] ?? h / 2;

  const labelIds = layout.cells
    .filter((c) => labelable(c, mode))
    .slice(0, 400)
    .map((c) => c.id);
  if (mode === "sunburst") labelIds.push(layout.meta.node);
  void getNames(layout.meta.generation, labelIds).then(
    (names) => {
      // Names resolve async — repaint with them (still once per settle).
      drawCells(ctx, layout, props, names, w, h, cx, cy);
    },
  );
  drawCells(ctx, layout, props, new Map(), w, h, cx, cy);
}

/** The UI font family (canvas text needs it as a string). */
function uiFont(): string {
  return getComputedStyle(document.documentElement).getPropertyValue("--font-ui") || "system-ui";
}

/** Draw text with a soft halo so labels stay legible over ANY pastel
 *  fill (the audit's contrast complaint: dark text on mid-tone pastels
 *  failed the squint test). Halo first, then the ink. */
function haloText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  ink: string,
): void {
  ctx.save();
  ctx.lineWidth = 2.5;
  ctx.strokeStyle = "rgba(255,255,255,0.55)";
  ctx.lineJoin = "round";
  ctx.strokeText(text, x, y);
  ctx.fillStyle = ink;
  ctx.fillText(text, x, y);
  ctx.restore();
}

function drawCells(
  ctx: CanvasRenderingContext2D,
  layout: LayoutResult,
  props: CanvasVizProps,
  names: Map<number, string>,
  w: number,
  h: number,
  cx: number,
  cy: number,
): void {
  const mode = layout.meta.mode;
  const bg = getComputedStyle(document.documentElement).getPropertyValue("--border").trim() || "#d8d8dd";

  // mind-map: draw links first — each takes the CHILD's own family color
  // at ~45% opacity (reference: colored bezier links, not uniform gray).
  if (mode === "mind-map") {
    for (const c of layout.cells) {
      if ((c.flags & 0b111) !== CELL_KIND.DOT) continue;
      const [x, y, , px, py] = c.g;
      const lr = (c.rgba >>> 24) & 0xff;
      const lg = (c.rgba >>> 16) & 0xff;
      const lb = (c.rgba >>> 8) & 0xff;
      ctx.strokeStyle = `rgba(${lr},${lg},${lb},0.45)`;
      ctx.lineWidth = 1.8;
      ctx.beginPath();
      const mx = (x + px) / 2 + (y - py) * 0.12;
      const my = (y + py) / 2 - (x - px) * 0.12;
      ctx.moveTo(px, py);
      ctx.quadraticCurveTo(mx, my, x, y);
      ctx.stroke();
    }
  }
  // sunburst: subtle ring separators
  if (mode === "sunburst") {
    ctx.strokeStyle = bg;
    for (const c of layout.cells) {
      if ((c.flags & 0b111) !== CELL_KIND.ARC) continue;
      ctx.lineWidth = 0.8;
      ctx.beginPath();
      ctx.arc(cx, cy, c.g[3], 0, Math.PI * 2);
      ctx.stroke();
      break; // one circle per ring is enough (first cell of each ring)
    }
  }

  for (const c of layout.cells) {
    const kind = c.flags & 0b111;
    const fill = cssRgba(c.rgba);
    if (kind === CELL_KIND.RECT) {
      const [x, y, rw, rh] = c.g;
      ctx.fillStyle = fill;
      ctx.fillRect(x, y, rw, rh);
      // Treemap: white gap separators between the pastel blocks (the
      // reference's look); flame keeps the hairline dark stroke.
      if (mode === "treemap") {
        ctx.strokeStyle = "rgba(255,255,255,0.55)";
        ctx.lineWidth = 1.5;
      } else {
        ctx.strokeStyle = "rgba(29,29,31,0.10)";
        ctx.lineWidth = 1;
      }
      ctx.strokeRect(x + 0.5, y + 0.5, rw - 1, rh - 1);
      if (rw >= 44 && rh >= 16) {
        const name = names.get(c.id);
        if (name) {
          const label = props.abbreviateLabels ? abbreviate(name) : name;
          // Size tiering + halo (design audit: small cells had illegible
          // low-contrast text — bigger cells deserve bigger, bolder
          // labels; the halo keeps them readable on any pastel).
          const big = rw >= 150 && rh >= 46;
          const mid = rw >= 84 && rh >= 26;
          const fontPx = big ? 12.5 : mid ? 11 : 10;
          const weight = big ? 700 : 600;
          ctx.font = `${weight} ${fontPx}px ${uiFont()}`;
          ctx.textBaseline = "top";
          haloText(
            ctx,
            clipLabel(ctx, label, rw - 10),
            x + 5,
            y + 4,
            ON_PASTEL,
          );
          if ((c.flags & DIR_BIT) !== 0 && rw >= 60 && rh >= 30) {
            ctx.fillStyle = ON_PASTEL_2;
            ctx.font = "500 9px " + uiFont();
          }
        }
      }
    } else if (kind === CELL_KIND.ARC) {
      const [a0, a1, r0, r1] = c.g;
      ctx.fillStyle = fill;
      ctx.beginPath();
      ctx.arc(cx, cy, r1, a0, a1);
      ctx.arc(cx, cy, r0, a1, a0, true);
      ctx.closePath();
      ctx.fill();
      ctx.strokeStyle = "rgba(29,29,31,0.08)";
      ctx.lineWidth = 0.8;
      ctx.stroke();
      // labels: along-ring when wide, radial when narrow — never upside-down
      const span = a1 - a0;
      const midA = (a0 + a1) / 2;
      if (span > 0.16 && r1 - r0 > 13) {
        const name = names.get(c.id);
        if (name) {
          const label = props.abbreviateLabels ? abbreviate(name) : name;
          const rr = (r0 + r1) / 2;
          const flip = Math.cos(midA) < 0;
          ctx.save();
          ctx.translate(cx + Math.cos(midA) * rr, cy + Math.sin(midA) * rr);
          ctx.rotate(flip ? midA + Math.PI : midA);
          ctx.fillStyle = ON_PASTEL;
          ctx.font = "600 9px " + getComputedStyle(document.documentElement).getPropertyValue("--font-ui");
          ctx.textBaseline = "middle";
          const maxW = span * rr - 6;
          ctx.fillText(clipLabel(ctx, label, maxW), flip ? -2 : 2, 0);
          ctx.restore();
        }
      }
    } else if (kind === CELL_KIND.CIRCLE) {
      const [x, y, r] = c.g;
      ctx.fillStyle = fill;
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = "rgba(29,29,31,0.16)";
      ctx.lineWidth = 1.2;
      ctx.stroke();
      // The sunburst center disc carries the dedicated white center
      // label below — skip the generic dark-ink circle label for it.
      const isSunburstCenter = mode === "sunburst" && c.id === layout.meta.node;
      if (r >= 30 && !isSunburstCenter) {
        const name = names.get(c.id);
        if (name) {
          const label = props.abbreviateLabels ? abbreviate(name) : name;
          ctx.fillStyle = ON_PASTEL;
          ctx.font = "600 10px " + getComputedStyle(document.documentElement).getPropertyValue("--font-ui");
          ctx.textBaseline = "middle";
          ctx.fillText(clipLabel(ctx, label, r * 1.6), x, y);
        }
      }
    } else if (kind === CELL_KIND.DOT) {
      const [x, y, r] = c.g;
      ctx.fillStyle = fill;
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = "rgba(29,29,31,0.28)";
      ctx.lineWidth = 1.4;
      ctx.stroke();
      if (r >= 20) {
        const name = names.get(c.id);
        if (name) {
          const label = props.abbreviateLabels ? abbreviate(name) : name;
          ctx.fillStyle = ON_PASTEL;
          ctx.font = "600 10px " + getComputedStyle(document.documentElement).getPropertyValue("--font-ui");
          ctx.textBaseline = "middle";
          const left = x > w / 2;
          ctx.textAlign = left ? "right" : "left";
          ctx.fillText(clipLabel(ctx, label, 110), x + (left ? -r - 5 : r + 5), y - 6);
          ctx.textAlign = "left";
        }
      }
    }
  }

  // Sunburst center label: the root folder name + total size in white,
  // centered on the coral disc (reference's center treatment).
  if (mode === "sunburst") {
    let discR = NaN;
    for (const c of layout.cells) {
      if ((c.flags & 0b111) !== CELL_KIND.ARC) continue;
      if (Number.isNaN(discR) || c.g[2] < discR) discR = c.g[2];
    }
    if (Number.isNaN(discR)) discR = Math.min(w, h) * 0.07;
    const rootName = names.get(layout.meta.node);
    ctx.save();
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    // White ink on coral: a soft dark shadow keeps it crisp without a
    // heavy halo ring.
    ctx.shadowColor = "rgba(0,0,0,0.25)";
    ctx.shadowBlur = 4;
    ctx.fillStyle = "#FFFFFF";
    if (rootName) {
      ctx.font = "600 11px " + uiFont();
      ctx.fillText(clipLabel(ctx, rootName, discR * 1.7), cx, cy - 8);
    }
    ctx.font = "700 15px " + uiFont();
    ctx.fillText(bytes(layout.meta.totalBytes), cx, cy + (rootName ? 7 : 0));
    ctx.restore();
  }
}

function clipLabel(ctx: CanvasRenderingContext2D, label: string, maxW: number): string {
  if (maxW <= 12) return "";
  if (ctx.measureText(label).width <= maxW) return label;
  let out = label;
  while (out.length > 1 && ctx.measureText(`${out}…`).width > maxW) {
    out = out.slice(0, -1);
  }
  return `${out}…`;
}

/** Hit-test a point against the mode's geometry. */
function hitCell(cells: Cell[], _mode: string, x: number, y: number, center: [number, number] | null): Cell | null {
  // topmost = last drawn → iterate reversed.
  const cx = center?.[0] ?? 0;
  const cy = center?.[1] ?? 0;
  for (let i = cells.length - 1; i >= 0; i--) {
    const c = cells[i];
    const kind = c.flags & 0b111;
    if (kind === CELL_KIND.RECT) {
      if (x >= c.g[0] && x <= c.g[0] + c.g[2] && y >= c.g[1] && y <= c.g[1] + c.g[3]) return c;
    } else if (kind === CELL_KIND.CIRCLE || kind === CELL_KIND.DOT) {
      const dx = x - c.g[0];
      const dy = y - c.g[1];
      if (dx * dx + dy * dy <= c.g[2] * c.g[2]) return c;
    } else if (kind === CELL_KIND.ARC) {
      const dx = x - cx;
      const dy = y - cy;
      const r = Math.sqrt(dx * dx + dy * dy);
      if (r >= c.g[2] && r <= c.g[3]) {
        let a = Math.atan2(dy, dx);
        const a0 = c.g[0];
        const a1 = c.g[1];
        // normalize a into [a0, a0 + 2π)
        let norm = a;
        while (norm < a0) norm += Math.PI * 2;
        while (norm > a0 + Math.PI * 2) norm -= Math.PI * 2;
        if (norm <= a1) return c;
        a = norm;
        void a;
      }
    }
  }
  return null;
}

/** Ring/outline path for selection + hover. `inset > 0` strokes an
 *  inner variant (rect inset by `inset` px, circle r - inset, arc radii
 *  pulled in by `inset`) — used for the white inner selection ring. */
function ringPath(
  ctx: CanvasRenderingContext2D,
  c: Cell,
  layout: LayoutResult,
  _mode: string,
  inset = 0,
): void {
  const kind = c.flags & 0b111;
  ctx.beginPath();
  if (kind === CELL_KIND.RECT) {
    const i = inset > 0 ? inset : 1;
    if (c.g[2] - 2 * i > 1 && c.g[3] - 2 * i > 1) {
      ctx.rect(c.g[0] + i, c.g[1] + i, c.g[2] - 2 * i, c.g[3] - 2 * i);
    }
  } else if (kind === CELL_KIND.CIRCLE || kind === CELL_KIND.DOT) {
    const r = inset > 0 ? c.g[2] - inset : c.g[2] + 1.5;
    if (r > 0.5) ctx.arc(c.g[0], c.g[1], r, 0, Math.PI * 2);
  } else if (kind === CELL_KIND.ARC) {
    const [cx, cy] = layout.meta.center ?? [0, 0];
    const ro = inset > 0 ? c.g[3] - inset : c.g[3] + 1;
    const ri = inset > 0 ? c.g[2] + inset : c.g[2] - 1;
    if (ro > ri && ri > 0.5) {
      ctx.arc(cx, cy, ro, c.g[0], c.g[1]);
      ctx.arc(cx, cy, ri, c.g[1], c.g[0], true);
      ctx.closePath();
    }
  }
  ctx.stroke();
}
