/**
 * Platform detection (cross-platform doc §4 Option A): the SAME frontend
 * runs on Windows (WebView2) and macOS (WKWebView). Chrome differences
 * are gated on this flag — never a code fork.
 */
export type Platform = "windows" | "macos" | "other";

declare global {
  interface Window {
    __DB_PLATFORM__?: Platform;
  }
}

function detect(): Platform {
  const ua = navigator.userAgent;
  if (ua.includes("Macintosh")) return "macos";
  if (ua.includes("Windows")) return "windows";
  return "other";
}

export const PLATFORM: Platform = detect();

export const IS_MAC = PLATFORM === "macos";
export const IS_WINDOWS = PLATFORM === "windows";

/** macOS modifier in labels (⌘) vs Windows (Ctrl). */
export const MOD_KEY = IS_MAC ? "⌘" : "Ctrl";

/** Recycle Bin (Windows) vs Trash (macOS) — BuildPrompt §9 / Mac prompt §8. */
export const BIN_NAME = IS_MAC ? "Trash" : "Recycle Bin";

/** The primary file manager verb. */
export const REVEAL_NAME = IS_MAC ? "Reveal in Finder" : "Show in Explorer";

/** CTA copy on the sidebar ink button (spec §6.1 / Mac prompt §5.1). */
export const SCAN_THIS_PC = IS_MAC ? "Scan Full Mac" : "Scan This PC";
