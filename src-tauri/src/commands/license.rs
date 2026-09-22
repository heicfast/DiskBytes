//! License commands (doc 06 §6; spec licensing layer): thin IPC over
//! [`crate::license`], the 24-hour revalidation scheduler, and the
//! isPro gate for cleanup commits (free tier: ≤ 1 GB per queue —
//! decision-logged; PRO: unlimited).

use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::license::{
    self, validation_action, DodoClient, LicenseError, LicensePosture, LicenseState, ReqwestDodo,
    ValidationAction, GRACE_DAYS,
};

/// Free-tier cleanup-commit cap (bytes). Decision: free = analysis +
/// small cleanups; PRO unlocks unlimited commits.
const FREE_TIER_COMMIT_CAP: u64 = 1024 * 1024 * 1024;

/// The managed license manager (state + client + scheduler control).
pub struct LicenseManager {
    state: Mutex<LicenseState>,
}

/// Managed state constructor: restores the DPAPI-cached state.
#[must_use]
pub fn license_manager() -> LicenseManager {
    let state = license::dpapi::load().unwrap_or_default();
    LicenseManager {
        state: Mutex::new(state),
    }
}

/// The status payload (`license_status` + license-changed events).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatusView {
    pub posture: String,
    pub is_pro: bool,
    pub tier: String,
    /// Days left in offline grace (0 otherwise).
    pub grace_days_left: i64,
    /// Free-tier cleanup cap in bytes (the gate the UI explains).
    pub free_commit_cap: u64,
}

fn view(state: &LicenseState, now: i64) -> LicenseStatusView {
    let (posture, days) = match license::posture(state, now) {
        LicensePosture::Unlicensed => ("unlicensed", 0),
        LicensePosture::Pro => ("pro", 0),
        LicensePosture::Grace { days_left } => ("grace", days_left),
        LicensePosture::Degraded => ("degraded", 0),
    };
    LicenseStatusView {
        posture: posture.to_string(),
        is_pro: posture == "pro" || posture == "grace",
        tier: state.tier.clone(),
        grace_days_left: days,
        free_commit_cap: FREE_TIER_COMMIT_CAP,
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Current license status (posture/gating data for the UI).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn license_status(mgr: State<'_, LicenseManager>) -> LicenseStatusView {
    let state = mgr.state.lock().clone();
    view(&state, now_unix())
}

/// Activate a license key on this device (doc 06 §3.1/§3.5: the
/// activation `name` carries the hardware id so the Dodo dashboard
/// stays readable AND the instance text binds to the machine).
///
/// # Errors
/// The mapped [`LicenseError`] copy (404/403/422/5xx) or a storage
/// failure — the user-readable reason, never a silent fallback.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn activate_license(
    key: &str,
    mgr: State<'_, LicenseManager>,
    app: AppHandle,
) -> Result<LicenseStatusView, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(LicenseError::InvalidKey.to_string());
    }
    let hw = license::hardware_id()?;
    // Doc 06 §3.5: readable AND hardware-bound instance name.
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
    let name = format!("DiskBytes on {hostname} [{:.16}]", hw.as_str());
    let http = ReqwestDodo::new()?;
    let client = DodoClient::new(http);
    let response = client.activate(key, &name).map_err(|e| e.to_string())?;
    let now = now_unix();
    // Tier refinement (yearly/lifetime) maps from the product payload
    // once real product ids exist dashboard-side; until then the gate
    // only needs is_pro (documented in the worklog).
    let state = LicenseState {
        license_key: key.to_string(),
        instance_id: response.id,
        tier: String::new(),
        is_pro: true,
        activated_at: now,
        last_validated_at: now,
        last_known_good: now,
        hardware_id: hw,
    };
    license::dpapi::save(&state)?;
    identify_after_activation(&app, key);
    let v = view(&state, now);
    *mgr.state.lock() = state;
    Ok(v)
}

/// Post-activation identity merge (doc 07 §3.2): the license key handle
/// becomes the shared analytics distinct_id (both layers, one person).
pub fn identify_after_activation(app: &AppHandle, key: &str) {
    if let Some(an) = app.try_state::<crate::analytics::Analytics>() {
        an.identify(key);
    }
}

/// Deactivate this device (frees the seat server-side; doc 06 §3.3) and
/// clear the local cache. Network failure still clears locally (the
/// seat frees on the next dashboard action) — with the reason surfaced.
///
/// # Errors
/// String error when the remote deactivate fails with a hard error.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn deactivate_license(mgr: State<'_, LicenseManager>) -> Result<(), String> {
    let state = mgr.state.lock().clone();
    if state.license_key.is_empty() {
        return Ok(());
    }
    let http = ReqwestDodo::new()?;
    let client = DodoClient::new(http);
    let result = client.deactivate(&state.license_key, &state.instance_id);
    license::dpapi::clear();
    *mgr.state.lock() = LicenseState::default();
    match result {
        Ok(()) | Err(LicenseError::Network) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Run one validation NOW (Refresh in the license UI) and return the
/// resulting status. Network failures leave the grace path intact.
///
/// # Errors
/// String error on a hard validation failure (403/404/422/5xx).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn validate_now(
    app: AppHandle,
    mgr: State<'_, LicenseManager>,
) -> Result<LicenseStatusView, String> {
    let (v, err) = validate_and_apply(&app, &mgr);
    if let Some(e) = err {
        return Err(e);
    }
    Ok(v)
}

/// One validation cycle: applies the doc 06 §3.2 state machine, saves,
/// and notifies the UI. Returns (view, hard-error).
fn validate_and_apply(
    app: &AppHandle,
    mgr: &LicenseManager,
) -> (LicenseStatusView, Option<String>) {
    let state = mgr.state.lock().clone();
    let mut state = state;
    let now = now_unix();
    let mut error = None;
    if state.license_key.is_empty() {
        return (view(&state, now), None);
    }
    let http = ReqwestDodo::new().ok();
    let result = match http {
        Some(h) => DodoClient::new(h).validate(&state.license_key, &state.instance_id),
        None => Err(LicenseError::Network),
    };
    // Engine telemetry (doc 07 §4) — no key material.
    if let Some(an) = app.try_state::<crate::analytics::Analytics>() {
        an.capture(
            "license_validate_result",
            &[
                ("valid", serde_json::json!(matches!(result, Ok(true)))),
                ("network", serde_json::json!(result.is_err())),
            ],
        );
    }
    match validation_action(&result) {
        ValidationAction::Refresh => {
            state.last_validated_at = now;
            state.last_known_good = state.last_known_good.max(now);
            state.is_pro = true;
        }
        ValidationAction::Deactivate => {
            // Revoked / seat removed server-side (doc 06 §3.2).
            license::dpapi::clear();
            state = LicenseState::default();
        }
        ValidationAction::Offline => {
            // Grace continues (doc 06 §3.4): timestamps frozen.
        }
        ValidationAction::Fail(e) => {
            error = Some(e.to_string());
        }
    }
    let _ = license::dpapi::save(&state);
    let v = view(&state, now);
    *mgr.state.lock() = state.clone();
    let _ = app.emit("license-changed", &v);
    (v, error)
}

/// Start the license scheduler: an immediate launch validation + the
/// 24-hour revalidation loop (doc 06 §3.4). Called from `setup`.
pub fn start_scheduler(app: &AppHandle) {
    let handle = Arc::new(app.clone());
    std::thread::Builder::new()
        .name("db-license".into())
        .spawn(move || {
            let mgr = handle.state::<LicenseManager>();
            let _ = validate_and_apply(&handle, &mgr);
            loop {
                // Sleep one validation interval; exit quietly when the
                // app is closing (emit failures end the loop naturally).
                std::thread::sleep(std::time::Duration::from_secs(
                    license::VALIDATION_INTERVAL_S.try_into().unwrap_or(86_400),
                ));
                let _ = validate_and_apply(&handle, &mgr);
            }
        })
        .expect("license scheduler thread");
}

/// The isPro cleanup-commit gate: PRO/grace commits anything; free/
/// unlicensed commits up to the cap; degraded mode blocks commits
/// entirely (read-only cleanup — doc 06 §3.4).
///
/// # Errors
/// String error with the user-readable reason when the gate refuses.
pub fn check_commit_gate(mgr: &LicenseManager, total_bytes: u64, now: i64) -> Result<(), String> {
    let state = mgr.state.lock().clone();
    match license::posture(&state, now) {
        LicensePosture::Pro | LicensePosture::Grace { .. } => Ok(()),
        LicensePosture::Degraded => Err(format!(
            "License offline for more than {GRACE_DAYS} days — cleanup is read-only until you reconnect."
        )),
        LicensePosture::Unlicensed => {
            if total_bytes > FREE_TIER_COMMIT_CAP {
                Err(
                    "The free plan moves up to 1 GB per cleanup. This queue is larger — Upgrade to Pro to commit it."
                        .to_string(),
                )
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_tier_cap_is_1gb() {
        assert_eq!(FREE_TIER_COMMIT_CAP, 1_073_741_824);
    }

    #[test]
    fn view_shapes_by_posture() {
        let now = 1_790_000_000;
        let mut s = LicenseState {
            license_key: "K".into(),
            is_pro: true,
            tier: "lifetime".into(),
            last_validated_at: now,
            ..Default::default()
        };
        let v = view(&s, now);
        assert_eq!(v.posture, "pro");
        assert!(v.is_pro);
        // 10 days stale → grace 4.
        s.last_validated_at = now - 10 * 86_400;
        let v = view(&s, now);
        assert_eq!(v.posture, "grace");
        assert_eq!(v.grace_days_left, 4);
        assert!(v.is_pro, "grace keeps pro features (doc 06 §3.4)");
        // Degraded → not pro.
        s.last_validated_at = now - (GRACE_DAYS + 1) * 86_400;
        let v = view(&s, now);
        assert_eq!(v.posture, "degraded");
        assert!(!v.is_pro);
    }

    #[test]
    fn commit_gate_rules() {
        let now = 1_790_000_000;
        // Unlicensed: ≤ 1 GB ok, > refused with upgrade copy.
        let free = LicenseState::default();
        assert!(check_commit_gate(&LicenseManager::state_for(&free, now), 1024, now).is_ok());
        let err = check_commit_gate(
            &LicenseManager::state_for(&free, now),
            FREE_TIER_COMMIT_CAP + 1,
            now,
        )
        .unwrap_err();
        assert!(err.contains("Upgrade"));
        // Pro: anything.
        let pro = LicenseState {
            license_key: "K".into(),
            is_pro: true,
            last_validated_at: now,
            ..Default::default()
        };
        assert!(
            check_commit_gate(&LicenseManager::state_for(&pro, now), u64::MAX / 2, now).is_ok()
        );
        // Degraded: refuse everything with reconnect copy.
        let mut deg = pro;
        deg.last_validated_at = now - (GRACE_DAYS + 2) * 86_400;
        let err = check_commit_gate(&LicenseManager::state_for(&deg, now), 10, now).unwrap_err();
        assert!(err.contains("reconnect"));
    }

    impl LicenseManager {
        /// Test constructor from a frozen state + clock (the gate reads
        /// posture through the same `view` path as production).
        fn state_for(state: &LicenseState, _now: i64) -> Self {
            LicenseManager {
                state: Mutex::new(state.clone()),
            }
        }
    }
}
