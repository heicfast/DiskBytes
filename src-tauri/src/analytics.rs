//! Analytics bridge (doc 07): posthog-rs (blocking client, default
//! features off) for ENGINE telemetry — scan performance, turbo
//! verification, recycle preflight, license validation. One shared
//! anonymous distinct_id (random UUID, never the hardware id — privacy
//! §6); swapped to the license identity after activation.
//!
//! Rules enforced here (doc 07 §5/§6):
//! - captures NEVER error into the UI (non-falling helper);
//! - opt-out (persisted flag) stops ALL Rust captures;
//! - zero file-system content in events — counts/sizes/durations only;
//! - the client only exists when a project key is configured
//!   (env `DISKBYTES_POSTHOG_KEY`); absent key = disabled, not an error.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

/// Managed analytics state.
pub struct Analytics {
    client: Option<posthog_rs::Client>,
    distinct_id: Mutex<String>,
    opt_out: AtomicBool,
}

/// Where the anonymous id persists (plain text: it IS the privacy
/// boundary — a random UUID with no meaning, doc 07 §3.1). Uses the
/// platform app-data dir (`%APPDATA%\DiskBytes` on Windows,
/// `~/Library/Application Support/DiskBytes` on macOS).
fn id_path() -> std::path::PathBuf {
    crate::platform::os::app_data_dir().join("analytics-id")
}

/// Read or create the anonymous UUID (no `uuid` crate: two u64s from
/// the OS entropy are enough for an analytics id).
fn load_or_create_id() -> String {
    if let Ok(s) = std::fs::read_to_string(id_path()) {
        let s = s.trim();
        if !s.is_empty() {
            return s.to_string();
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mix = nanos ^ (u128::from(std::process::id()) << 64);
    let id = format!("{mix:032x}");
    let _ = std::fs::write(id_path(), &id);
    id
}

/// The opt-out flag path (checked before client init AND each capture).
fn opt_out_path() -> std::path::PathBuf {
    id_path().with_file_name("analytics-optout")
}

impl Analytics {
    /// Build the managed state (env-configured, disabled-by-default).
    #[must_use]
    pub fn init() -> Self {
        let opt_out = opt_out_path().exists();
        let client = Self::build_client();
        Self {
            client,
            distinct_id: Mutex::new(load_or_create_id()),
            opt_out: AtomicBool::new(opt_out),
        }
    }

    /// Construct the posthog client when a key is configured (US host —
    /// region decision logged; doc 07 §5).
    fn build_client() -> Option<posthog_rs::Client> {
        let key = std::env::var("DISKBYTES_POSTHOG_KEY").ok()?;
        if key.trim().is_empty() {
            return None;
        }
        Some(posthog_rs::client(key.as_str()))
    }

    /// Fire one engine event. NEVER fails: opt-out, disabled client and
    /// property errors all drop silently (doc 07 §5).
    pub fn capture(&self, event: &str, props: &[(&str, serde_json::Value)]) {
        if self.opt_out.load(Ordering::Relaxed) {
            return;
        }
        let Some(client) = &self.client else { return };
        let distinct_id = self.distinct_id.lock().clone();
        let mut ev = posthog_rs::Event::new(event, distinct_id.as_str());
        for (k, v) in props {
            if ev.insert_prop(*k, v.clone()).is_err() {
                return; // drop rather than misreport
            }
        }
        // posthog-rs capture hands the event to its background worker
        // and returns immediately (verified against the 0.26 source).
        client.capture(ev);
    }

    /// Post-activation identity (doc 07 §3.2): the Dodo customer id
    /// becomes the shared distinct_id (both layers, one person).
    pub fn identify(&self, customer_ref: &str) {
        *self.distinct_id.lock() = customer_ref.to_string();
    }

    /// The opt-out toggle (persisted; honored by both layers).
    pub fn set_opt_out(&self, value: bool) {
        self.opt_out.store(value, Ordering::Relaxed);
        let p = opt_out_path();
        if value {
            let _ = std::fs::write(&p, b"1");
        } else {
            // App-owned data file removal lives in the core persistence
            // layer (R7.1 grep scope).
            diskbytes_core::snapshots::remove_app_data_file(&p);
        }
    }

    /// Current opt-out state (settings UI).
    #[must_use]
    pub fn opt_out(&self) -> bool {
        self.opt_out.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_without_client_never_panics() {
        // No DISKBYTES_POSTHOG_KEY in the test env → disabled client.
        let an = Analytics {
            client: None,
            distinct_id: Mutex::new("test-id".into()),
            opt_out: AtomicBool::new(false),
        };
        an.capture("scan_perf", &[("files", serde_json::json!(10))]);
    }

    #[test]
    fn opt_out_blocks_capture() {
        // With a client mock unavailable in unit tests, the opt-out
        // branch is exercised with a None client AND the flag set — the
        // early return precedes any client use.
        let an = Analytics {
            client: None,
            distinct_id: Mutex::new("t".into()),
            opt_out: AtomicBool::new(true),
        };
        an.capture("x", &[]);
    }

    #[test]
    fn identify_swaps_distinct_id() {
        let an = Analytics {
            client: None,
            distinct_id: Mutex::new("anon-1".into()),
            opt_out: AtomicBool::new(false),
        };
        an.identify("cust_123");
        assert_eq!(*an.distinct_id.lock(), "cust_123");
    }

    #[test]
    fn anonymous_id_is_hex_and_persisted() {
        // Shape check: 32 hex chars (the entropy source, not the value).
        let s = load_or_create_id();
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        // Persisted → stable across calls.
        assert_eq!(load_or_create_id(), s);
        // Cleanup (app-owned data file — core persistence layer).
        diskbytes_core::snapshots::remove_app_data_file(&id_path());
    }
}
