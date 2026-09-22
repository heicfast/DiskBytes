//! License layer (doc 06; spec §8-licensing): Dodo Payments activation/
//! validation/deactivation over the three PUBLIC endpoints (safe in
//! shipped binaries — `security: []` in the OpenAPI), hardware binding,
//! DPAPI-cached state, the 14-day offline grace (doc 06 §3.4 decision),
//! and 24-hour revalidation.
//!
//! The Dodo **SDK** stays backend-side (doc 06 §6 decision); this client
//! is raw reqwest behind the [`DodoHttp`] seam so the state machine is
//! unit-testable with a fake transport + injected clock.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// TEST/LIVE base URLs (doc 06 §2; `DODO_PAYMENTS_MODE=test` selects TEST).
const BASE_TEST: &str = "https://test.dodopayments.com";
const BASE_LIVE: &str = "https://live.dodopayments.com";

/// Revalidation interval (doc 06 §3.4: 24 h).
pub const VALIDATION_INTERVAL_S: i64 = 24 * 60 * 60;

/// Offline grace (doc 06 §3.4 DECISION: 14 days).
pub const GRACE_DAYS: i64 = 14;

/// The HTTP seam: real = reqwest blocking; tests = fake.
pub trait DodoHttp {
    /// POST JSON → (status, body).
    fn post_json(&self, url: &str, body: &str) -> Result<(u16, String), String>;
}

/// The real transport (reqwest, 30 s timeout — doc 06 §6).
pub struct ReqwestDodo {
    client: reqwest::blocking::Client,
}

impl ReqwestDodo {
    /// Build with the documented timeout.
    ///
    /// # Errors
    /// String error when the TLS client cannot be constructed.
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("license http client: {e}"))?;
        Ok(Self { client })
    }
}

impl DodoHttp for ReqwestDodo {
    fn post_json(&self, url: &str, body: &str) -> Result<(u16, String), String> {
        let resp = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .map_err(|e| format!("network: {e}"))?;
        let status = resp.status().as_u16();
        let text = resp.text().map_err(|e| format!("network: {e}"))?;
        Ok((status, text))
    }
}

/// Typed license errors with the spec'd user copy (doc 06 §3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseError {
    /// 404 — key not found.
    InvalidKey,
    /// 403 — key inactive.
    Inactive,
    /// 422 — activation limit reached (offer deactivate-a-device).
    SeatLimit,
    /// Network failure → offline grace path.
    Network,
    /// 5xx / parse failure.
    ServerError,
}

impl std::fmt::Display for LicenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::InvalidKey => "Invalid License Key. Please check your key and try again.",
            Self::Inactive => "This license key is inactive. Contact support.",
            Self::SeatLimit => "Activation limit reached — deactivate a device to free a seat.",
            Self::Network => "Network unavailable — DiskBytes keeps working offline.",
            Self::ServerError => "Dodo Payments had a problem. Try again in a moment.",
        };
        f.write_str(s)
    }
}

impl LicenseError {
    /// Map a status code per the documented contract (doc 06 §3.1).
    #[must_use]
    pub fn from_status(status: u16) -> Self {
        match status {
            403 => Self::Inactive,
            404 => Self::InvalidKey,
            422 => Self::SeatLimit,
            _ => Self::ServerError,
        }
    }
}

/// The validate response body.
#[derive(Deserialize)]
struct ValidateResponse {
    valid: bool,
}

/// The Dodo client (3 public endpoints, no API key).
pub struct DodoClient<H: DodoHttp> {
    http: H,
    base: String,
}

impl<H: DodoHttp> DodoClient<H> {
    /// TEST mode when `DODO_PAYMENTS_MODE=test` (dev only; default LIVE).
    #[must_use]
    pub fn new(http: H) -> Self {
        let base = if std::env::var("DODO_PAYMENTS_MODE").as_deref() == Ok("test") {
            BASE_TEST
        } else {
            BASE_LIVE
        }
        .to_string();
        Self { http, base }
    }

    /// Activate a key for this device (doc 06 §3.1).
    ///
    /// # Errors
    /// [`LicenseError`] per the endpoint contract.
    pub fn activate(&self, key: &str, name: &str) -> Result<ActivateResponse, LicenseError> {
        let body = serde_json::json!({ "license_key": key, "name": name });
        let (status, text) = self
            .http
            .post_json(
                &format!("{}/licenses/activate", self.base),
                &body.to_string(),
            )
            .map_err(|_| LicenseError::Network)?;
        if status != 201 {
            return Err(LicenseError::from_status(status));
        }
        serde_json::from_str(&text).map_err(|_| LicenseError::ServerError)
    }

    /// Validate a key instance (doc 06 §3.2).
    ///
    /// # Errors
    /// [`LicenseError`] on transport/server failure.
    pub fn validate(&self, key: &str, instance: &str) -> Result<bool, LicenseError> {
        let body = serde_json::json!({
            "license_key": key,
            "license_key_instance_id": instance,
        });
        let (status, text) = self
            .http
            .post_json(
                &format!("{}/licenses/validate", self.base),
                &body.to_string(),
            )
            .map_err(|_| LicenseError::Network)?;
        if status != 200 {
            return Err(LicenseError::from_status(status));
        }
        serde_json::from_str::<ValidateResponse>(&text)
            .map(|v| v.valid)
            .map_err(|_| LicenseError::ServerError)
    }

    /// Deactivate this device's instance (doc 06 §3.3).
    ///
    /// # Errors
    /// [`LicenseError`] per the endpoint contract.
    pub fn deactivate(&self, key: &str, instance: &str) -> Result<(), LicenseError> {
        let body = serde_json::json!({
            "license_key": key,
            "license_key_instance_id": instance,
        });
        let (status, _) = self
            .http
            .post_json(
                &format!("{}/licenses/deactivate", self.base),
                &body.to_string(),
            )
            .map_err(|_| LicenseError::Network)?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(LicenseError::from_status(status))
        }
    }
}

/// The activate response (fields we persist — doc 06 §3.1).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivateResponse {
    /// The license key instance id.
    #[serde(default)]
    pub id: String,
}

/// Persisted license state (DPAPI-encrypted on disk).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LicenseState {
    #[serde(default)]
    pub license_key: String,
    #[serde(default)]
    pub instance_id: String,
    /// "yearly" | "lifetime" ("" before activation).
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub is_pro: bool,
    #[serde(default)]
    pub activated_at: i64,
    #[serde(default)]
    pub last_validated_at: i64,
    /// Signed "last known good time" defense (clock-rollback guard:
    /// never moves backward — doc 06 §3.4/licensing doc §4.3).
    #[serde(default)]
    pub last_known_good: i64,
    #[serde(default)]
    pub hardware_id: String,
}

/// The runtime posture derived from state + now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicensePosture {
    /// No license on file.
    Unlicensed,
    /// Activated and fresh.
    Pro,
    /// Activated, offline, inside the grace window.
    Grace { days_left: i64 },
    /// Grace exhausted → degrade mode (read-only cleanup; banner).
    Degraded,
}

/// Derive the posture (PURE — injected clock for tests; doc 06 §3.4).
#[must_use]
pub fn posture(state: &LicenseState, now: i64) -> LicensePosture {
    if state.license_key.is_empty() || !state.is_pro {
        return LicensePosture::Unlicensed;
    }
    let reference = state.last_validated_at.max(state.last_known_good);
    let age = now - reference;
    let grace_s = GRACE_DAYS * 24 * 60 * 60;
    if age < VALIDATION_INTERVAL_S {
        return LicensePosture::Pro;
    }
    if age < grace_s {
        return LicensePosture::Grace {
            days_left: (grace_s - age + 86_399) / 86_400,
        };
    }
    LicensePosture::Degraded
}

/// What a validation outcome does to the state (doc 06 §3.2 logic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationAction {
    /// valid:true → refresh timestamps.
    Refresh,
    /// valid:false → deactivate locally (revoked/seat removed).
    Deactivate,
    /// Network error → grace continues (timestamps frozen).
    Offline,
    /// Hard error surfaced to the UI.
    Fail(LicenseError),
}

/// Apply a validation result (PURE — the state machine under test).
#[must_use]
pub fn validation_action(result: &Result<bool, LicenseError>) -> ValidationAction {
    match result {
        Ok(true) => ValidationAction::Refresh,
        Ok(false) => ValidationAction::Deactivate,
        Err(LicenseError::Network) => ValidationAction::Offline,
        Err(e) => ValidationAction::Fail(e.clone()),
    }
}

/// The device identity for seat binding (doc 06 §3.5):
/// `SHA-256(MachineGuid ‖ system-drive volume serial ‖ CPUID)` — the
/// inputs come from the platform seam (MachineGuid + volume serial +
/// CPUID on Windows; IOPlatformUUID + fsid + CPUID on macOS), so the
/// same binding story holds on both OSes.
///
/// # Errors
/// String error when the underlying platform reads fail (all three
/// inputs are required — no partial identity).
pub fn hardware_id() -> Result<String, String> {
    let machine_guid = crate::platform::os::machine_guid().ok_or("MachineGuid unreadable")?;
    let serial = crate::platform::os::system_drive_serial().ok_or("volume serial unreadable")?;
    let cpuid = crate::platform::os::cpuid_brand().ok_or("CPUID unavailable")?;
    let mut hasher = Sha256::new();
    hasher.update(machine_guid.as_bytes());
    hasher.update(serial.to_le_bytes());
    hasher.update(cpuid.as_bytes());
    let digest = hasher.finalize();
    Ok(hex(&digest))
}

/// Lowercase hex (32 bytes → 64 chars).
fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// DPAPI-encrypted persistence (licensing doc §5.4: the local cache is
/// protected with `CryptProtectData`, tied to the Windows user; on macOS
/// the same contract is served by a Keychain-delimited item via the
/// platform seam — see `platform::os::dpapi_protect`).
pub mod dpapi {
    use super::LicenseState;

    /// Where the encrypted license blob lives.
    #[must_use]
    pub fn license_path() -> std::path::PathBuf {
        crate::platform::os::app_data_dir().join("license.bin")
    }

    /// Encrypt + persist the state.
    ///
    /// # Errors
    /// String error when DPAPI or the write fails.
    pub fn save(state: &LicenseState) -> Result<(), String> {
        let json = serde_json::to_vec(state).map_err(|e| format!("serialize: {e}"))?;
        let blob = crate::platform::os::dpapi_protect(&json)?;
        std::fs::write(license_path(), blob).map_err(|e| format!("write license: {e}"))
    }

    /// Load + decrypt the state (`None` when absent/corrupt — fresh start).
    #[must_use]
    pub fn load() -> Option<LicenseState> {
        let blob = std::fs::read(license_path()).ok()?;
        let json = crate::platform::os::dpapi_unprotect(&blob).ok()?;
        serde_json::from_slice(&json).ok()
    }

    /// Remove the stored license (deactivation cleanup; app-owned data
    /// file removal lives in the core persistence layer — R7.1).
    pub fn clear() {
        diskbytes_core::snapshots::remove_app_data_file(&license_path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Fake transport scripted per call.
    struct FakeHttp {
        responses: RefCell<Vec<Result<(u16, String), String>>>,
        urls: RefCell<Vec<String>>,
    }

    impl FakeHttp {
        fn new(rs: Vec<Result<(u16, String), String>>) -> Self {
            Self {
                responses: RefCell::new(rs),
                urls: RefCell::new(Vec::new()),
            }
        }
    }

    impl DodoHttp for FakeHttp {
        fn post_json(&self, url: &str, _body: &str) -> Result<(u16, String), String> {
            self.urls.borrow_mut().push(url.to_string());
            self.responses
                .borrow_mut()
                .pop()
                .expect("scripted response available")
        }
    }

    #[test]
    fn activate_maps_statuses_and_body() {
        // 201 + body → instance id.
        let c = DodoClient::new(FakeHttp::new(vec![Ok((
            201,
            r#"{"id":"lki_123","name":"x"}"#.into(),
        ))]));
        let r = c.activate("CNFX-1", "DiskBytes on PC").unwrap();
        assert_eq!(r.id, "lki_123");
        // 404/403/422 → typed errors with spec'd copy.
        for (status, expected) in [
            (404, LicenseError::InvalidKey),
            (403, LicenseError::Inactive),
            (422, LicenseError::SeatLimit),
            (500, LicenseError::ServerError),
        ] {
            let c = DodoClient::new(FakeHttp::new(vec![Ok((status, "{}".into()))]));
            assert_eq!(c.activate("k", "n").unwrap_err(), expected);
        }
    }

    #[test]
    fn activate_error_copy_is_spec_text() {
        assert_eq!(
            LicenseError::InvalidKey.to_string(),
            "Invalid License Key. Please check your key and try again."
        );
    }

    fn d2(h: FakeHttp) -> DodoClient<FakeHttp> {
        DodoClient::new(h)
    }

    #[test]
    fn validate_parses_bool_and_errors() {
        let c = DodoClient::new(FakeHttp::new(vec![Ok((200, r#"{"valid":true}"#.into()))]));
        assert!(c.validate("k", "i").unwrap());
        let c = d2(FakeHttp::new(vec![Ok((200, r#"{"valid":false}"#.into()))]));
        assert!(!c.validate("k", "i").unwrap());
        // Transport failure → Network (the grace path trigger).
        let c = DodoClient::new(FakeHttp::new(vec![Err("down".into())]));
        assert_eq!(c.validate("k", "i").unwrap_err(), LicenseError::Network);
    }

    #[test]
    fn deactivate_accepts_2xx_only() {
        let c = DodoClient::new(FakeHttp::new(vec![Ok((204, String::new()))]));
        assert!(c.deactivate("k", "i").is_ok());
        let c = DodoClient::new(FakeHttp::new(vec![Ok((404, String::new()))]));
        assert_eq!(
            c.deactivate("k", "i").unwrap_err(),
            LicenseError::InvalidKey
        );
    }

    #[test]
    fn test_mode_selects_test_base() {
        std::env::set_var("DODO_PAYMENTS_MODE", "test");
        let c = DodoClient::new(FakeHttp::new(vec![Ok((204, String::new()))]));
        let _ = c.deactivate("k", "i");
        std::env::remove_var("DODO_PAYMENTS_MODE");
        let urls = c.http.urls.borrow();
        assert!(urls[0].starts_with("https://test.dodopayments.com"));
    }

    const NOW: i64 = 1_790_000_000;

    fn pro_state(last_validated: i64, last_good: i64) -> LicenseState {
        LicenseState {
            license_key: "CNFX-9".into(),
            instance_id: "lki_1".into(),
            tier: "lifetime".into(),
            is_pro: true,
            activated_at: NOW - 100,
            last_validated_at: last_validated,
            last_known_good: last_good,
            hardware_id: "hw".into(),
        }
    }

    #[test]
    fn posture_fresh_pro() {
        assert_eq!(
            posture(&pro_state(NOW - 3_600, 0), NOW),
            LicensePosture::Pro
        );
    }

    #[test]
    fn posture_grace_counts_days_left() {
        // 10 days stale → 4 days left (of 14).
        let stale = NOW - 10 * 86_400;
        assert_eq!(
            posture(&pro_state(stale, 0), NOW),
            LicensePosture::Grace { days_left: 4 }
        );
    }

    #[test]
    fn posture_grace_boundary_and_degrade() {
        // Day 14 boundary → Degraded (age >= grace).
        let stale = NOW - GRACE_DAYS * 86_400;
        assert_eq!(posture(&pro_state(stale, 0), NOW), LicensePosture::Degraded);
        // One second under → Grace day 1.
        let stale = NOW - GRACE_DAYS * 86_400 + 1;
        assert_eq!(
            posture(&pro_state(stale, 0), NOW),
            LicensePosture::Grace { days_left: 1 }
        );
    }

    #[test]
    fn posture_uses_the_newer_of_validated_and_known_good() {
        // last_validated stale, last_known_good fresh → Pro.
        let s = pro_state(NOW - 20 * 86_400, NOW - 1_000);
        assert_eq!(posture(&s, NOW), LicensePosture::Pro);
    }

    #[test]
    fn posture_unlicensed_when_no_key() {
        assert_eq!(
            posture(&LicenseState::default(), NOW),
            LicensePosture::Unlicensed
        );
        let mut s = pro_state(NOW, NOW);
        s.is_pro = false;
        assert_eq!(posture(&s, NOW), LicensePosture::Unlicensed);
    }

    #[test]
    fn validation_action_state_machine() {
        assert_eq!(validation_action(&Ok(true)), ValidationAction::Refresh);
        assert_eq!(validation_action(&Ok(false)), ValidationAction::Deactivate);
        assert_eq!(
            validation_action(&Err(LicenseError::Network)),
            ValidationAction::Offline
        );
        assert_eq!(
            validation_action(&Err(LicenseError::SeatLimit)),
            ValidationAction::Fail(LicenseError::SeatLimit)
        );
    }

    #[test]
    fn hardware_id_hex_shape() {
        // Pure-shape check of the hex helper (the real id needs Windows).
        assert_eq!(hex(&[0x00, 0xff, 0x10]), "00ff10");
        assert_eq!(hex(&[0xab; 32]).len(), 64);
    }
}
