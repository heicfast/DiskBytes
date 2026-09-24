//! Monitor commands (spec §12; doc 03 M9): the 2-second sampler
//! thread. Started when the Monitor tab mounts (`monitor_start`) and
//! stopped on unmount (`monitor_stop`); each tick computes deltas
//! against the previous raw snapshot via `core::monitor` and emits a
//! `monitor-sample` event (the UI keeps the 120-sample ring).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use diskbytes_core::monitor::{self, MonitorSample, ProcSample, VolumeSample};
use tauri::{AppHandle, Emitter, Manager};

/// The sampler interval (spec §12: "every 2 seconds").
const INTERVAL_MS: u64 = 2_000;

/// Process rows kept per sample (top 60 by CPU ∪ top 60 by memory).
const PROC_CAP: usize = 60;

/// Shared monitor control state.
pub struct MonitorState {
    running: AtomicBool,
    /// Latest `monitor_start` wins: a stale `monitor_stop` (an older
    /// mount's cleanup racing a newer mount's start — StrictMode remount
    /// + async IPC) is ignored when the session moved on.
    session: AtomicU64,
}

/// Managed state constructor.
#[must_use]
pub fn monitor_state() -> MonitorState {
    MonitorState {
        running: AtomicBool::new(false),
        session: AtomicU64::new(0),
    }
}

/// Start the sampler. Idempotent while running (the tab mount effect
/// can fire twice under StrictMode) — every call bumps the session so
/// the newest mount owns the sampler.
///
/// # Errors
/// String error when the thread cannot be spawned.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn monitor_start(app: AppHandle, state: tauri::State<'_, MonitorState>) -> Result<u64, String> {
    let session = state.session.fetch_add(1, Ordering::SeqCst) + 1;
    if state.running.swap(true, Ordering::SeqCst) {
        return Ok(session); // already running — latest session wins
    }
    let app = Arc::new(app);
    std::thread::Builder::new()
        .name("db-monitor".into())
        .spawn(move || sampler_loop(&app))
        .map_err(|e| {
            // Spawn failed: release the flag, else every future
            // monitor_start no-ops against a dead "running" state.
            state.running.store(false, Ordering::SeqCst);
            format!("monitor thread: {e}")
        })?;
    Ok(session)
}

/// Stop the sampler (the loop exits after the current sleep). Passing
/// the `session` from `monitor_start` makes the stop a no-op when a
/// NEWER start already took ownership — an unsequenced stale stop
/// would otherwise kill a live tab's sampler.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn monitor_stop(state: tauri::State<'_, MonitorState>, session: Option<u64>) {
    if let Some(s) = session {
        if s != state.session.load(Ordering::SeqCst) {
            return; // stale stop — a newer mount owns the sampler
        }
    }
    state.running.store(false, Ordering::SeqCst);
}

/// The sampler loop: raw snapshot → delta → event, every 2 s.
type RawProcs = Vec<(u32, String, u64, u64, u64)>;

fn sampler_loop(app: &Arc<AppHandle>) {
    let running = || match app.try_state::<MonitorState>() {
        Some(s) => s.running.load(Ordering::SeqCst),
        None => true, // state gone (app shutting down): keep sampling until emit fails
    };
    let mut prev: Option<crate::platform::os::RawMonitor> = None;
    let mut prev_wall = Instant::now();
    let mut session_in: u64 = 0;
    let mut session_out: u64 = 0;
    while running() {
        std::thread::sleep(std::time::Duration::from_millis(INTERVAL_MS));
        if !running() {
            break;
        }
        let raw = crate::platform::os::monitor_raw();
        let dt_ms = prev_wall.elapsed().as_secs_f64() * 1000.0;
        prev_wall = Instant::now();

        // CPU from tick deltas (kernel INCLUDES idle — spec §12).
        let (cpu_user_pct, cpu_system_pct, cpu_total_pct) =
            raw.ticks.pct(&prev.as_ref().map_or(raw.ticks, |p| p.ticks));

        // Network rates from octet deltas; session totals accumulate.
        // dt_ms is elapsed wall time (strictly positive; clamped >= 1 ms
        // before the u64 cast, so the cast is sign-safe).
        let dt_ms_u = u64::try_from(dt_ms.max(1.0).ceil() as i64)
            .unwrap_or(1)
            .max(1);
        let (net_down_bps, net_up_bps) = match &prev {
            Some(p) => (
                monitor::tick_delta(raw.net_in, p.net_in).saturating_mul(1000) / dt_ms_u,
                monitor::tick_delta(raw.net_out, p.net_out).saturating_mul(1000) / dt_ms_u,
            ),
            None => (0, 0),
        };
        session_in = session_in.saturating_add(
            prev.as_ref()
                .map_or(0, |p| monitor::tick_delta(raw.net_in, p.net_in)),
        );
        session_out = session_out.saturating_add(
            prev.as_ref()
                .map_or(0, |p| monitor::tick_delta(raw.net_out, p.net_out)),
        );

        // Per-process CPU (normalized across logical cores).
        let cores = std::thread::available_parallelism()
            .map_or(1u32, |n| u32::try_from(n.get()).unwrap_or(1));
        let prev_procs: Option<&RawProcs> = prev.as_ref().map(|p| &p.procs);
        let procs: Vec<ProcSample> = raw
            .procs
            .iter()
            .filter(|(pid, ..)| *pid != 0)
            .map(|(pid, name, kernel, user, ws)| {
                let delta = prev_procs.and_then(|pp| {
                    pp.iter().find(|(p, ..)| p == pid).map(|(_, _, pk, pu, _)| {
                        monitor::tick_delta(*kernel, *pk)
                            .saturating_add(monitor::tick_delta(*user, *pu))
                    })
                });
                ProcSample {
                    pid: *pid,
                    name: name.clone(),
                    cpu_pct: delta.map_or(0.0, |d| monitor::proc_cpu_pct(d, dt_ms, cores)),
                    ws: *ws,
                }
            })
            .collect();
        let (procs, total_procs) = monitor::rank_processes(procs, PROC_CAP);

        let sample = MonitorSample {
            dt_ms,
            cpu_user_pct,
            cpu_system_pct,
            cpu_total_pct,
            threads: raw.threads,
            processes: raw.processes,
            mem_total: raw.mem_total,
            mem_available: raw.mem_available,
            kernel_paged: raw.kernel_paged,
            kernel_nonpaged: raw.kernel_nonpaged,
            system_cache: raw.system_cache,
            commit_total: raw.commit_total,
            commit_limit: raw.commit_limit,
            compressed: raw.compressed_ws,
            net_down_bps,
            net_up_bps,
            session_in,
            session_out,
            volumes: raw
                .volumes
                .iter()
                .map(|v| VolumeSample {
                    root: v.root.clone(),
                    label: v.label.clone(),
                    total: v.total,
                    free: v.free,
                })
                .collect(),
            procs,
            total_procs,
        };
        // Event emission is fallible only when the window is gone —
        // that also means nobody is listening; exit quietly.
        if app.emit("monitor-sample", sample).is_err() {
            break;
        }
        prev = Some(raw);
    }
    // Clear the flag so a later monitor_start can spawn a fresh loop.
    if let Some(s) = app.try_state::<MonitorState>() {
        s.running.store(false, Ordering::SeqCst);
    }
}
