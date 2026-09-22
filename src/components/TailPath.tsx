/**
 * Tail-first path rendering for the small mono path lines (sidebar
 * Current View, queue rows, dup/app/diff rows, canvas labels).
 *
 * Why JS: Chromium never draws the `text-overflow: ellipsis` glyph on
 * the LEFT for LTR content inside a `direction: rtl` block (the
 * classic head-ellipsis recipe) — long paths silently clipped with no
 * ellipsis at all. `fitPath` measures with the element's OWN computed
 * font via canvas and elides the middle (head segments + "…" + tail),
 * degrading to a pure tail ("…node.rs") when space is tight. The full
 * path always rides along in `title`.
 */
import { useLayoutEffect, useRef, useState } from "react";
import { fitPath } from "../lib/fitPath";

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
    const font = getComputedStyle(el).font;
    const apply = (w: number) => setText(fitPath(path, Math.max(24, w), font));
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
