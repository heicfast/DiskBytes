//! Analytics IPC (doc 07 §5): the opt-out toggle surfaced to settings;
//! every event name stays behind the typed helpers in `src/lib/analytics.ts`
//! (WebView layer) and `crate::analytics` (engine layer).

use tauri::{Emitter, State};

use crate::analytics::Analytics;

/// Current opt-out state + the key-presence flag (settings copy).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsStateView {
    pub opt_out: bool,
    /// True when a PostHog project key is configured (env) — the
    /// settings screen explains "disabled" vs "opted out".
    pub enabled: bool,
}

/// Read the analytics state.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn analytics_opt_out(analytics: State<'_, Analytics>) -> AnalyticsStateView {
    AnalyticsStateView {
        opt_out: analytics.opt_out(),
        enabled: std::env::var("DISKBYTES_POSTHOG_KEY").is_ok_and(|k| !k.trim().is_empty()),
    }
}

/// Toggle opt-out (honored by both layers; persisted).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn set_analytics_opt_out(value: bool, analytics: State<'_, Analytics>, app: tauri::AppHandle) {
    analytics.set_opt_out(value);
    // Keep the WebView layer in sync (posthog-js has its own persisted
    // flag; the event lets the UI mirror the Rust-side decision).
    let _ = app.emit("analytics-optout-changed", value);
}
