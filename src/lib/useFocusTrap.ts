/**
 * Focus trap for modal dialogs (a11y): while `active`, Tab wraps inside
 * `ref` so keyboard users never tab into content the scrim obscures.
 * Focus moves into the dialog on open and restores to the opener on
 * close (the WAI-ARIA dialog pattern).
 */
import { useEffect, type RefObject } from "react";

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function useFocusTrap(ref: RefObject<HTMLElement | null>, active: boolean): void {
  useEffect(() => {
    if (!active) return;
    const el = ref.current;
    if (!el) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const focusables = [...el.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
        (n) => n.offsetParent !== null || n === document.activeElement,
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const inside = el.contains(document.activeElement);
      if (e.shiftKey && (document.activeElement === first || !inside)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (document.activeElement === last || !inside)) {
        e.preventDefault();
        first.focus();
      }
    };
    el.addEventListener("keydown", onKey);
    // Pull focus into the dialog on open; restore the opener on close.
    const prev = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const target = el.querySelector<HTMLElement>(FOCUSABLE);
    target?.focus();
    return () => {
      el.removeEventListener("keydown", onKey);
      prev?.focus?.();
    };
  }, [active, ref]);
}
