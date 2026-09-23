/**
 * Shared arrow-key list/grid navigation (Explorer/Finder parity):
 * Up/Down (and Left/Right in grids) move the selection, Enter activates
 * (open folder), Right/Left expand/collapse in outlines. Skips events
 * aimed at form fields and any open overlay (menus/dialogs own their
 * keys — Esc parity per the focus-trap work).
 */
import { useEffect, useRef } from "react";

export interface ArrowNavConfig {
  /** Rows/items count (0 disables). */
  count: number;
  /** Currently selected id (start position; null starts at 0). */
  selectedId: number | null;
  idOf: (index: number) => number;
  /** Selection moved — modes scroll the row into view + select. */
  onMove: (index: number) => void;
  /** Enter — same semantics as double-click. */
  onActivate?: (index: number) => void;
  /** Right = open, Left = close (outline disclosure). No-op when the
   *  row cannot expand (the mode decides). */
  onExpand?: (index: number, open: boolean) => void;
  /** Grid mode: Left/Right also step (default linear). */
  grid?: { columns: () => number };
}

function overlayOpen(): boolean {
  return Boolean(
    document.querySelector(
      '[role="menu"], [role="dialog"], [class*="overlay"], [class*="popover"], [class*="modal"]',
    ),
  );
}

export function useArrowNav(config: ArrowNavConfig): void {
  const ref = useRef(config);
  ref.current = config;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      if (overlayOpen()) return;
      const c = ref.current;
      if (!c.count) return;
      const idx = (() => {
        if (c.selectedId == null) return -1;
        for (let i = 0; i < c.count; i++) if (c.idOf(i) === c.selectedId) return i;
        return -1;
      })();
      const cols = c.grid ? Math.max(1, c.grid.columns()) : 1;
      const step = (delta: number) => {
        // Nothing selected yet: any arrow key lands on the FIRST item
        // (Explorer/Finder grid behavior — never skip a full row).
        const next = idx < 0 ? 0 : Math.min(c.count - 1, Math.max(0, idx + delta));
        if (next !== idx) {
          e.preventDefault();
          c.onMove(next);
        }
      };
      switch (e.key) {
        case "ArrowDown":
          step(cols);
          return;
        case "ArrowUp":
          step(-cols);
          return;
        case "ArrowRight":
          if (c.grid) {
            step(1);
            return;
          }
          if (c.onExpand && idx >= 0) {
            e.preventDefault();
            c.onExpand(idx, true);
          }
          return;
        case "ArrowLeft":
          if (c.grid) {
            step(-1);
            return;
          }
          if (c.onExpand && idx >= 0) {
            e.preventDefault();
            c.onExpand(idx, false);
          }
          return;
        case "Enter":
          if (c.onActivate && idx >= 0) {
            e.preventDefault();
            c.onActivate(idx);
          }
          return;
        default:
          return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
