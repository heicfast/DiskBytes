//! Monitor models + delta math (spec §12; doc 02 §7): the 2-second
//! sampler's arithmetic — CPU percentages from `GetSystemTimes` tick
//! deltas (kernel INCLUDES idle), per-process CPU normalized across
//! logical cores, process ranking (top 60 by CPU + by memory), and the
//! sample payload the UI's 120-sample ring keeps.
//!
//! The raw Win32/NT reads live in the app crate's `platform/win.rs`
//! seam; this module owns the math so the host can test it.

use serde::Serialize;

/// One process row in the sample payload.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcSample {
    /// Process id.
    pub pid: u32,
    /// Image name (e.g. `firefox.exe`).
    pub name: String,
    /// CPU % normalized across logical cores (0–100 = all cores busy).
    pub cpu_pct: f64,
    /// Working set bytes.
    pub ws: u64,
}

/// One mounted volume row (spec §12 storage card).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSample {
    /// Display root (`C:\`).
    pub root: String,
    /// Volume label.
    pub label: String,
    /// Total capacity bytes.
    pub total: u64,
    /// Free bytes.
    pub free: u64,
}

/// The full `monitor-sample` payload (one every 2 s).
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSample {
    /// Wall-clock ms between the two raw samples (0 on the first).
    pub dt_ms: f64,
    /// User-mode CPU % (all cores = 100).
    pub cpu_user_pct: f64,
    /// System/kernel-mode CPU % (idle excluded).
    pub cpu_system_pct: f64,
    /// Total busy CPU %.
    pub cpu_total_pct: f64,
    /// System-wide thread count.
    pub threads: u32,
    /// System-wide process count.
    pub processes: u32,
    /// Physical memory total bytes.
    pub mem_total: u64,
    /// Physical memory available bytes.
    pub mem_available: u64,
    /// Kernel paged pool bytes.
    pub kernel_paged: u64,
    /// Kernel non-paged pool bytes.
    pub kernel_nonpaged: u64,
    /// System cache bytes.
    pub system_cache: u64,
    /// Commit charge total bytes.
    pub commit_total: u64,
    /// Commit limit bytes.
    pub commit_limit: u64,
    /// Memory Compression private usage (`None` → "—").
    pub compressed: Option<u64>,
    /// Downstream bytes/s (filtered interfaces).
    pub net_down_bps: u64,
    /// Upstream bytes/s (filtered interfaces).
    pub net_up_bps: u64,
    /// Session downlink total bytes (since monitor start).
    pub session_in: u64,
    /// Session uplink total bytes (since monitor start).
    pub session_out: u64,
    /// Fixed + removable volumes.
    pub volumes: Vec<VolumeSample>,
    /// Top 60 by CPU union top 60 by memory.
    pub procs: Vec<ProcSample>,
    /// Total process count (the "Show all N" caption).
    pub total_procs: usize,
}

/// `GetSystemTimes` tick quadruple (100 ns units; kernel INCLUDES idle
/// — the documented `GetSystemTimes` contract).
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuTicks {
    /// Idle time (100 ns units).
    pub idle: u64,
    /// Kernel time INCLUDING idle (100 ns units).
    pub kernel: u64,
    /// User time (100 ns units).
    pub user: u64,
}

impl CpuTicks {
    /// The three CPU percentages from deltas. `system` excludes idle
    /// (kernel − idle); total = user + system. Deltas of zero → 0 %.
    #[must_use]
    pub fn pct(&self, prev: &Self) -> (f64, f64, f64) {
        let idle = self.idle.saturating_sub(prev.idle);
        let kernel = self.kernel.saturating_sub(prev.kernel);
        let user = self.user.saturating_sub(prev.user);
        let total = idle + kernel + user;
        if total == 0 {
            return (0.0, 0.0, 0.0);
        }
        let user_pct = user as f64 / total as f64 * 100.0;
        let system_pct = kernel.saturating_sub(idle) as f64 / total as f64 * 100.0;
        let total_pct = user_pct + system_pct;
        (user_pct, system_pct, total_pct)
    }
}

/// Per-process CPU % from time deltas, normalized to 0–100 across ALL
/// logical cores (spec §12): a process burning one full core on an
/// 8-core machine reads 12.5 %.
#[must_use]
pub fn proc_cpu_pct(time_delta_100ns: u64, dt_ms: f64, logical_cores: u32) -> f64 {
    if dt_ms <= 0.0 || logical_cores == 0 {
        return 0.0;
    }
    let wall_100ns = dt_ms * 10_000.0 * f64::from(logical_cores);
    (time_delta_100ns as f64 / wall_100ns * 100.0).clamp(0.0, 100.0)
}

/// The union of top-60-by-CPU and top-60-by-memory process rows,
/// sorted by CPU (the table's default), plus the total count.
#[must_use]
pub fn rank_processes(mut procs: Vec<ProcSample>, cap: usize) -> (Vec<ProcSample>, usize) {
    let total = procs.len();
    if procs.len() <= cap * 2 {
        procs.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return (procs, total);
    }
    let mut by_cpu: Vec<usize> = (0..procs.len()).collect();
    by_cpu.sort_by(|&a, &b| {
        procs[b]
            .cpu_pct
            .partial_cmp(&procs[a].cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut by_mem: Vec<usize> = (0..procs.len()).collect();
    by_mem.sort_by_key(|&i| std::cmp::Reverse(procs[i].ws));
    let mut keep = vec![false; procs.len()];
    for &i in by_cpu.iter().take(cap) {
        keep[i] = true;
    }
    for &i in by_mem.iter().take(cap) {
        keep[i] = true;
    }
    let mut out: Vec<ProcSample> = procs
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, p)| p.clone())
        .collect();
    out.sort_by(|a, b| {
        b.cpu_pct
            .partial_cmp(&a.cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (out, total)
}

/// 100 ns-tick deltas between two u64 counters (saturating).
#[must_use]
pub fn tick_delta(now: u64, prev: u64) -> u64 {
    now.saturating_sub(prev)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct_approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn cpu_pct_math_kernel_includes_idle() {
        // 100 ticks wall: idle 50, kernel 80 (incl. idle), user 20.
        let prev = CpuTicks::default();
        let now = CpuTicks {
            idle: 50,
            kernel: 80,
            user: 20,
        };
        let (u, s, t) = now.pct(&prev);
        // total wall = idle + kernel + user = 150; but busy = kernel-
        // idle + user = 50. The denominator must be the SUM (150):
        assert!(pct_approx(u, 20.0 / 150.0 * 100.0));
        assert!(pct_approx(s, 30.0 / 150.0 * 100.0)); // 80-50 = 30
        assert!(pct_approx(t, 50.0 / 150.0 * 100.0));
        assert!(pct_approx(t, u + s));
    }

    #[test]
    fn cpu_pct_zero_delta_is_zero() {
        let t = CpuTicks {
            idle: 10,
            kernel: 20,
            user: 5,
        };
        assert_eq!(t.pct(&t), (0.0, 0.0, 0.0));
    }

    #[test]
    fn cpu_pct_backward_time_is_zero() {
        // Counter wrap / clock skew: saturating deltas → 0.
        let prev = CpuTicks {
            idle: 100,
            kernel: 100,
            user: 100,
        };
        let now = CpuTicks {
            idle: 10,
            kernel: 10,
            user: 10,
        };
        assert_eq!(now.pct(&prev), (0.0, 0.0, 0.0));
    }

    #[test]
    fn proc_cpu_normalized_across_cores() {
        // One full core for 1000 ms on 8 cores → 12.5 %.
        let one_core_1s = 10_000_000u64; // 1 s of 100 ns units
        assert!(pct_approx(proc_cpu_pct(one_core_1s, 1000.0, 8), 12.5));
        // All 8 cores busy → 100 %.
        assert!(pct_approx(proc_cpu_pct(one_core_1s * 8, 1000.0, 8), 100.0));
        // Degenerate inputs.
        assert!(pct_approx(proc_cpu_pct(1000, 0.0, 8), 0.0));
        assert!(pct_approx(proc_cpu_pct(1000, 10.0, 0), 0.0));
    }

    #[test]
    fn ranking_unions_top_by_cpu_and_memory() {
        let mut procs: Vec<ProcSample> = (0..100)
            .map(|i| ProcSample {
                pid: i,
                name: format!("p{i}"),
                // CPU descending with pid
                cpu_pct: f64::from(100 - i),
                // memory ASCENDING with pid (inverse of cpu)
                ws: u64::from(i) * 1024,
            })
            .collect();
        let (ranked, total) = rank_processes(std::mem::take(&mut procs), 60);
        assert_eq!(total, 100);
        // 100 ≤ 120 → everything kept, sorted by CPU.
        assert_eq!(ranked.len(), 100);
        assert_eq!(ranked[0].pid, 0);

        // Now with cap 10: union of top-10 CPU (pids 0..10) and
        // top-10 memory (pids 90..100) = 20 rows.
        let mut procs: Vec<ProcSample> = (0..100)
            .map(|i| ProcSample {
                pid: i,
                name: format!("p{i}"),
                cpu_pct: f64::from(100 - i),
                ws: u64::from(i) * 1024,
            })
            .collect();
        let (ranked, total) = rank_processes(std::mem::take(&mut procs), 10);
        assert_eq!(total, 100);
        assert_eq!(ranked.len(), 20);
        let ids: Vec<u32> = ranked.iter().map(|p| p.pid).collect();
        assert!(ids.contains(&0)); // top cpu
        assert!(ids.contains(&99)); // top memory
        assert!(!ids.contains(&50)); // middle of both
    }

    #[test]
    fn tick_deltas_saturate() {
        assert_eq!(tick_delta(10, 3), 7);
        assert_eq!(tick_delta(3, 10), 0);
    }
}
