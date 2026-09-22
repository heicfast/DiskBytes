/**
 * Path truncation with a middle ellipsis (senior-UX rule: for paths the
 * TAIL matters most — the file/folder name — while the drive/root head
 * anchors context). The old CSS approach (`direction: rtl` +
 * `unicode-bidi: plaintext`) silently defeated itself: `plaintext`
 * re-derived an LTR paragraph from the first strong character (`C:`),
 * so long paths clipped on the RIGHT with no ellipsis at all — the
 * "broken path bar" bug. This module measures with the SAME mono font
 * as the box and elides the middle, Finder-style:
 *   C:\Users\…\System32\drivers\index.d.ts
 */

const measureCtx: CanvasRenderingContext2D | null = (() => {
  try {
    const c = document.createElement("canvas");
    return c.getContext("2d");
  } catch {
    return null; // jsdom / tests: fall back to the char-count path
  }
})();

/** Measure `s` in `font`; ~0.62em/char fallback when canvas is absent.
 * `letterSpacing` (px, 0 when "normal") is added per gap — canvas
 * `measureText` does NOT include CSS letter-spacing, but the rendered
 * text has it, so unaccounted tracking makes every measurement read
 * narrow and paths overflow (the "cut in half" bug). */
function measure(s: string, font: string, letterSpacing = 0): number {
  let w: number;
  if (measureCtx) {
    measureCtx.font = font;
    w = measureCtx.measureText(s).width;
  } else {
    w = s.length * 6.2;
  }
  return w + Math.max(0, s.length - 1) * letterSpacing;
}

/** Split a path into (head, tail) where head keeps the root + first
 * segments, tail keeps the trailing segments. Windows `\`, POSIX `/`. */
function splitPath(path: string, headSegments: number): [string, string] {
  const sep = path.includes("\\") ? "\\" : "/";
  const parts = path.split(sep).filter((p) => p.length > 0);
  if (parts.length <= headSegments + 1) return [path, ""];
  const head = parts.slice(0, headSegments).join(sep) + sep;
  const tail = parts.slice(headSegments).join(sep);
  return [head, tail];
}

/** Ellipsize `path` in the middle so it fits `maxWidth` px in `font`.
 * Keeps up to 2 leading segments + the final segment(s); shrinks the
 * tail character-by-character before dropping tail segments entirely.
 * `opts.letterSpacing` accounts for CSS tracking (see `measure`);
 * `opts.pad` reserves a safety margin (subpixel rounding, hinting). */
export function fitPath(
  path: string,
  maxWidth: number,
  font: string,
  opts?: { letterSpacing?: number; pad?: number },
): string {
  const ls = opts?.letterSpacing ?? 0;
  const pad = opts?.pad ?? 0;
  const budget = Math.max(8, maxWidth - pad);
  const m = (s: string) => measure(s, font, ls);
  if (m(path) <= budget) return path;

  // Drop tail segments until only one remains, always measuring.
  for (let head = 2; head >= 1; head--) {
    let [h, t] = splitPath(path, head);
    if (!t) break;
    for (;;) {
      const candidate = `${h}…${t}`;
      if (m(candidate) <= budget) return candidate;
      if (t.length <= 4) break; // keep at least a stub of the tail
      // Prefer dropping whole tail segments while we have >1 left.
      const sep = t.includes("\\") ? "\\" : "/";
      const idx = t.indexOf(sep);
      if (idx > 0 && t.slice(idx + 1).length > 4) {
        t = t.slice(idx + 1);
      } else {
        t = t.slice(1);
      }
    }
  }

  // Last resort: hard clip the tail with a leading ellipsis (keep at
  // most one char past the ellipsis for pathological budgets).
  const ellipsis = "…";
  let t = path;
  while (t.length > 1 && m(ellipsis + t) > budget) {
    t = t.slice(1);
  }
  return ellipsis + t;
}
