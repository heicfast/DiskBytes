/**
 * Tail-first path rendering for every mono path line in the app
 * (inspector path box, scanning ticker, sidebar Current View, queue
 * rows, dup/app/diff/age-map rows).
 *
 * Why JS: Chromium never draws the `text-overflow: ellipsis` glyph on
 * the LEFT for LTR content inside a `direction: rtl` block (the
 * classic head-ellipsis recipe) — long paths are silently clipped with
 * no ellipsis at all (verified live: the text hugs the LEFT edge and
 * blunt-cuts on the RIGHT). `fitPath` measures with the element's OWN
 * computed font — including weight and letter-spacing, which canvas
 * `measureText` ignores — and elides the middle (head + "…" + tail),
 * degrading to a pure tail ("…node.rs") when space is tight. The full
 * path always rides along in `title`.
 *
 * Layout contract: `.db-tail-path` is `display: block` (base.css) so
 * `clientWidth` is the real constrained box in every grid/flex cell.
 * Do not render it inside an unconstrained inline context.
 */
import { useLayoutEffect, useRef, useState } from "react";
import { fitPath } from "../lib/fitPath";

/** Computed `font` shorthand can be "" in rare Chromium states; rebuild
 * it from the longhands so measurement never silently falls back to the
 * canvas default (10px sans-serif — the classic "measured fits,
 * rendered overflows" trap). */
function computedFont(el: HTMLElement): string {
  const cs = getComputedStyle(el);
  if (cs.font) return cs.font;
  const style = cs.fontStyle === "normal" ? "" : `${cs.fontStyle} `;
  const weight = cs.fontWeight === "normal" ? "" : `${cs.fontWeight} `;
  return `${style}${weight}${cs.fontSize} ${cs.fontFamily}`;
}

function computedLetterSpacing(el: HTMLElement): number {
  const v = getComputedStyle(el).letterSpacing;
  return v === "normal" ? 0 : Number.parseFloat(v) || 0;
}

export function TailPath({ path, className }: { path: string; className?: string }) {
  const ref = useRef<HTMLSpanElement | null>(null);
  const [text, setText] = useState(path);
  const lastW = useRef(0);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!path) {
      setText("");
      return;
    }
    if (!el) return;
    const font = computedFont(el);
    const ls = computedLetterSpacing(el);
    // 1px safety pad absorbs subpixel rounding + hinting drift.
    const apply = (w: number) => setText(fitPath(path, Math.max(24, w), font, { letterSpacing: ls, pad: 1 }));
    const w = el.clientWidth;
    if (w > 0) {
      lastW.current = w;
      apply(w);
    } else if (lastW.current > 0) {
      apply(lastW.current);
    } else {
      setText(path); // measure pass next layout — full text is the safest placeholder
    }
    const ro = new ResizeObserver((entries) => {
      const cw = entries[0]?.contentRect.width ?? 0;
      if (cw > 0 && Math.abs(cw - lastW.current) > 0.5) {
        lastW.current = cw;
        apply(cw);
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [path]);

  return (
    <span ref={ref} className={className ? `db-tail-path ${className}` : "db-tail-path"} title={path}>
      {text}
    </span>
  );
}
