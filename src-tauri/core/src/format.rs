//! Byte/percent/age/duration formatting (`BuildPrompt` §14).
//!
//! **binary (1024) base with 3 significant digits** on Windows — matching
//! Explorer's Properties dialog (`StrFormatByteSize`-like output):
//! `"118 GB"`, `"5.12 GB"`, `"33.7 MB"`, `"1023 B"`, `"0 B"`.
//! The TypeScript twin lives at `src/lib/format.ts`; both are unit-tested
//! against the same boundary fixtures (spec §17) and MUST agree.

/// Format a byte count: binary base, 3 significant digits.
///
/// Examples (also asserted in the tests below, mirrored in TS):
/// `0 → "0 B"`, `1023 → "1023 B"`, `1024 → "1.00 KB"`,
/// `35_338_675 → "33.7 MB"`, `126_663_778_304 → "118 GB"`.
#[must_use]
pub fn bytes(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    // Largest k such that bytes >= 1024^k, k >= 1.
    let mut k = 1usize;
    let mut unit: f64 = 1024.0;
    while bytes as f64 >= unit * 1024.0 && k + 1 < UNITS.len() {
        unit *= 1024.0;
        k += 1;
    }
    let mut value = bytes as f64 / unit;
    // Rounding may cross the unit boundary (e.g. 1023.99 MB → 1024): bump once.
    if value >= 1023.5 && k + 1 < UNITS.len() {
        k += 1;
        value /= 1024.0;
    }
    // 3 significant digits: >=100 → integer, >=10 → 1 decimal, else 2 decimals.
    if value >= 99.995 {
        format!("{value:.0} {}", UNITS[k])
    } else if value >= 9.9995 {
        format!("{value:.1} {}", UNITS[k])
    } else {
        format!("{value:.2} {}", UNITS[k])
    }
}

/// Format a percentage of a whole; values below 0.1% render as `"<0.1%"`.
#[must_use]
pub fn percent(fraction: f64) -> String {
    let pct = fraction * 100.0;
    if pct < 0.1 {
        return "<0.1%".to_string();
    }
    // One decimal, dropped when it rounds to an integer ("12.3%",
    // "50%", "100%").
    let tenths = (pct * 10.0).round();
    if (tenths % 10.0).abs() < f64::EPSILON {
        format!("{pct:.0}%")
    } else {
        format!("{pct:.1}%")
    }
}

/// Relative age from a Unix-seconds timestamp to `now`.
///
/// `"Just now"`, `"42 seconds ago"`, `"3 days ago"`, `"7 months ago"`, …
/// `modified == 0` means unknown → `"—"`.
#[must_use]
pub fn relative_age(modified: i64, now: i64) -> String {
    if modified <= 0 {
        return "—".to_string();
    }
    let delta = now - modified;
    if delta < 0 {
        // Future timestamps (clock skew) read as "Just now" — honest enough
        // for display, never a silent data change.
        return "Just now".to_string();
    }
    if delta < 10 {
        "Just now".to_string()
    } else if delta < 60 {
        format!("{delta} seconds ago")
    } else if delta < 3600 {
        format!("{} minutes ago", delta / 60)
    } else if delta < 86_400 {
        format!("{} hours ago", delta / 3600)
    } else if delta < 7 * 86_400 {
        format!("{} days ago", delta / 86_400)
    } else if delta < 30 * 86_400 {
        format!("{} weeks ago", delta / (7 * 86_400))
    } else if delta < 365 * 86_400 {
        format!("{} months ago", delta / (30 * 86_400))
    } else {
        format!("{} years ago", delta / (365 * 86_400))
    }
}

/// Format an elapsed duration: `"5.5s"`, `"12s"`, `"1m 03s"`, `"2h 04m"`.
#[must_use]
pub fn duration(millis: u64) -> String {
    if millis < 1000 {
        format!("{millis}ms")
    } else if millis < 10_000 {
        format!("{:.1}s", millis as f64 / 1000.0)
    } else if millis < 60_000 {
        format!("{}s", millis / 1000)
    } else if millis < 3_600_000 {
        format!("{}m {:02}s", millis / 60_000, (millis / 1000) % 60)
    } else {
        format!("{}h {:02}m", millis / 3_600_000, (millis / 60_000) % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_boundaries() {
        // Same fixtures as src/lib/format.test.ts — both twins must agree.
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1), "1 B");
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(1023), "1023 B");
        assert_eq!(bytes(1024), "1.00 KB");
        assert_eq!(bytes(1024 * 1024), "1.00 MB");
        assert_eq!(bytes(1_048_575), "1.00 MB"); // 1023.99 KB rounds across the bump
        assert_eq!(bytes(33_700_000), "32.1 MB");
        assert_eq!(bytes(35_338_675), "33.7 MB"); // spec example
        assert_eq!(bytes(5_497_563_032), "5.12 GB"); // spec example
        assert_eq!(bytes(126_663_778_304), "118 GB"); // spec example
        assert_eq!(bytes(1024u64.pow(4)), "1.00 TB");
        assert_eq!(bytes(999 * 1024u64.pow(3)), "999 GB");
    }

    #[test]
    fn percent_boundaries() {
        assert_eq!(percent(0.0), "<0.1%");
        assert_eq!(percent(0.000_9), "<0.1%"); // 0.09%
        assert_eq!(percent(0.001), "0.1%");
        assert_eq!(percent(0.1234), "12.3%");
        assert_eq!(percent(0.5), "50%");
        assert_eq!(percent(1.0), "100%");
    }

    #[test]
    fn relative_ages() {
        let now = 1_800_000_000;
        assert_eq!(relative_age(0, now), "—");
        assert_eq!(relative_age(now, now), "Just now");
        assert_eq!(relative_age(now - 45, now), "45 seconds ago");
        assert_eq!(relative_age(now - 300, now), "5 minutes ago");
        assert_eq!(relative_age(now - 7200, now), "2 hours ago");
        assert_eq!(relative_age(now - 3 * 86_400, now), "3 days ago"); // spec example
        assert_eq!(relative_age(now - 14 * 86_400, now), "2 weeks ago");
        assert_eq!(relative_age(now - 90 * 86_400, now), "3 months ago");
        assert_eq!(relative_age(now - 730 * 86_400, now), "2 years ago");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(0), "0ms");
        assert_eq!(duration(240), "240ms");
        assert_eq!(duration(5_500), "5.5s"); // spec example
        assert_eq!(duration(12_300), "12s");
        assert_eq!(duration(63_000), "1m 03s");
        assert_eq!(duration(7_440_000), "2h 04m");
    }
}
