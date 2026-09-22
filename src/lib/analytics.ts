/**
 * WebView analytics layer (doc 07): posthog-js with autocapture OFF,
 * no session recording (privacy §6: file names on screen), localStorage
 * persistence and offline buffering. The typed `track()` helper is the
 * ONLY call surface — no scattered string literals.
 *
 * The API key comes from `VITE_POSTHOG_KEY` (client-side keys are
 * public by design, doc 07 §5). Absent key = disabled, never an error.
 */
import posthog from "posthog-js";

const POSTHOG_KEY = (import.meta.env.VITE_POSTHOG_KEY as string | undefined) ?? "";
const POSTHOG_HOST = "https://us.i.posthog.com"; // region decision: US

let initialized = false;

/** All WebView event names (doc 07 §4 dictionary — extend via decision log). */
export const EVENTS = {
  tabOpened: "tab_opened",
  vizModeSelected: "viz_mode_selected",
  colorModeChanged: "color_mode_changed",
  depthSliderMoved: "depth_slider_moved",
  scanStarted: "scan_started",
  scanCompleted: "scan_completed",
  quickWinsCategoryOpened: "quick_wins_category_opened",
  quickWinsAddAll: "quick_wins_add_all",
  cleanupStaged: "cleanup_staged",
  cleanupCommitted: "cleanup_committed",
  duplicatesScanCompleted: "duplicates_scan_completed",
  uninstallRun: "uninstall_run",
  snapshotTaken: "snapshot_taken",
  snapshotDiffed: "snapshot_diffed",
  checkoutOpened: "checkout_opened",
  licenseActivated: "license_activated",
  licenseError: "license_error",
  themeChanged: "theme_changed",
  searchUsed: "search_used",
} as const;

export type EventName = (typeof EVENTS)[keyof typeof EVENTS];

function init(): void {
  if (initialized || !POSTHOG_KEY) return;
  initialized = true;
  posthog.init(POSTHOG_KEY, {
    api_host: POSTHOG_HOST,
    autocapture: false, // deliberate (doc 07 §5)
    capture_pageview: false, // no pages in a desktop app
    persistence: "localStorage",
    disable_session_recording: true, // never in a disk analyzer
    loaded: (ph) => {
      if (import.meta.env.DEV) ph.debug();
    },
  });
}

init();

/** The opt-out toggle (synced with the Rust layer via localStorage). */
export function setWebViewOptOut(value: boolean): void {
  if (!initialized) return;
  if (value) posthog.opt_out_capturing();
  else posthog.opt_in_capturing();
}

/** Typed capture — queue-before-ready, never throws, never blocks render. */
export function track(
  name: EventName,
  props: Record<string, string | number | boolean | null>,
): void {
  if (!initialized) return;
  try {
    posthog.capture(name, props);
  } catch {
    /* analytics must never error into the UI (doc 07 §5) */
  }
}

/** Post-activation identity merge (doc 07 §3.2). */
export function identifyUser(customerRef: string, email?: string): void {
  if (!initialized) return;
  try {
    posthog.identify(customerRef, email ? { email } : undefined);
  } catch {
    /* never surface */
  }
}
