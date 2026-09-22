/**
 * Theme hook (BuildPrompt §14).
 *
 * `data-theme="light|dark"` on the document root carries the actual
 * palette (tokens.css). This hook flips it, asks Tauri to set the native
 * window theme (so native menus/tooltips match), and persists the choice
 * in localStorage. First launch follows `prefers-color-scheme`; the
 * pre-mount inline script in index.html applies the saved theme before
 * React mounts so there is no flash.
 */
import { useCallback, useEffect, useState } from "react";

export type Theme = "light" | "dark";

const STORAGE_KEY = "diskbytes.theme";

/** The theme the pre-mount script already applied (or the OS preference). */
function currentDomTheme(): Theme {
  const dom = document.documentElement.getAttribute("data-theme");
  if (dom === "dark" || dom === "light") return dom;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/**
 * Read + control the app theme.
 *
 * - `theme` — the active palette ("light" | "dark").
 * - `toggle()` — flip, persist, and sync the native window theme.
 * - `isDark` — convenience for icon selection (sun/moon).
 */
export function useTheme(): { theme: Theme; isDark: boolean; toggle: () => void } {
  const [theme, setTheme] = useState<Theme>(currentDomTheme);

  // Keep React state in sync if the attribute changes elsewhere.
  useEffect(() => {
    const observer = new MutationObserver(() => setTheme(currentDomTheme()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);

  const toggle = useCallback(() => {
    // Analytics fires before the state change so both themes report.
    import("../lib/analytics").then(({ EVENTS, track }) =>
      track(EVENTS.themeChanged, { theme: "pending" }),
    );
    const next: Theme = theme === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    try {
      window.localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // localStorage can throw in hardened WebView profiles; the theme
      // still applies for this session, it just will not persist.
    }
    // Native side: match window chrome (menus, tooltips). Fire-and-forget —
    // analytics never blocks and neither does this (doc 07 offline rule).
    import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(next))
      .catch(() => {
        // Running under Vitest / plain browser (no Tauri IPC) — CSS still
        // switched; nothing to report.
      });
  }, [theme]);

  return { theme, isDark: theme === "dark", toggle };
}
