//! Applications / app-uninstaller models + matching (spec §11; doc 02 §7).
//!
//! Pure logic only: normalization, [`UserAssist`] decoding, leftover
//! matching, icon-`DisplayIcon` parsing and base64 (icon data URLs).
//! The Windows I/O (registry keys, the MSIX package manager, registry
//! `UserAssist` reads, `SHGetFileInfoW`, size walks, process closing,
//! uninstaller launch) lives in the app crate's `platform/win.rs`
//! seam (doc 02 §2) and feeds this module's pure functions.

use serde::Serialize;

/// Where an installed-app entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppSource {
    /// A registry `Uninstall` key (3 hives, spec §11).
    Registry,
    /// An MSIX / Store package (`PackageManager.FindPackagesForUser`).
    Msix,
}

/// One installed application (the row model).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    /// Stable id: the registry key name or the package full name.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Publisher subject.
    pub publisher: String,
    /// Version string.
    pub version: String,
    /// Where the entry came from.
    pub source: AppSource,
    /// Install folder ("" when unknown).
    pub install_location: String,
    /// `UninstallString` (registry; "" for MSIX).
    pub uninstall_string: String,
    /// `QuietUninstallString` when present.
    pub quiet_uninstall_string: String,
    /// MSIX package full name ("" for registry apps).
    pub package_full_name: String,
    /// Last-used Unix seconds ("—" in the UI when `None`).
    pub last_used: Option<i64>,
    /// `data:image/png;base64,…` icon ("" when extraction failed).
    pub icon: String,
    /// Allocated size of the install folder (bundle), 0 when unknown.
    pub bundle_size: u64,
    /// Leftover groups matched in the 5 roots.
    pub leftovers: Vec<LeftoverGroup>,
    /// bundle + leftovers.
    pub total: u64,
}

/// One leftover group: matched directories inside one root.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverGroup {
    /// Root label (`Local AppData`, `Roaming AppData`, `LocalLow
    /// AppData`, `ProgramData`, `Store package data`).
    pub label: &'static str,
    /// The matched directories (absolute paths) with their sizes.
    pub paths: Vec<LeftoverPath>,
    /// Sum of `paths` sizes.
    pub size: u64,
}

/// One leftover directory.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverPath {
    /// Absolute path of the matched directory.
    pub path: String,
    /// Allocated size on disk.
    pub size: u64,
}

/// Leftover root labels (spec §11 group captions), in listing order.
pub const ROOT_LABELS: [&str; 5] = [
    "Local AppData",
    "Roaming AppData",
    "LocalLow AppData",
    "ProgramData",
    "Store package data",
];

/// The identity triple + publisher used for leftover matching.
#[derive(Debug, Clone, Default)]
pub struct AppIdentity {
    /// Registry `DisplayName` / MSIX display name.
    pub display_name: String,
    /// Final component of `InstallLocation`.
    pub install_folder: String,
    /// MSIX package family name ("" for registry apps).
    pub family_name: String,
    /// Publisher (registry `Publisher` / MSIX publisher subject).
    pub publisher: String,
}

impl AppIdentity {
    /// The exact-normalized match candidates for a directory name.
    fn names(&self) -> [String; 3] {
        [
            normalize_name(&self.display_name),
            normalize_name(&self.install_folder),
            normalize_name(&self.family_name),
        ]
    }
}

/// One child of a leftover root (the shared listing; built once).
#[derive(Debug, Clone, Default)]
pub struct RootChild {
    /// Original-casing folder name (display + staging path join).
    pub name: String,
    /// Allocated size of this folder.
    pub size: u64,
    /// Children of a publisher-named folder (populated ONLY for
    /// children whose normalized name equals some app's publisher —
    /// the "one level inside a folder named after the publisher"
    /// rule, e.g. `Roaming\Mozilla\Firefox`).
    pub nested: Vec<(String, u64)>,
}

/// One listed root.
#[derive(Debug, Clone, Default)]
pub struct RootListing {
    /// Index into [`ROOT_LABELS`].
    pub label_idx: usize,
    /// Absolute root path.
    pub path: String,
    /// Direct children (one listing shared across all apps).
    pub children: Vec<RootChild>,
}

/// Case-insensitive + space/punctuation-insensitive comparison key
/// (spec §11: "matched case-insensitively, ignoring spaces and
/// punctuation, by normalized … name"; EXACT equality on this key —
/// never a substring, so `Adobe` cannot match `Adobe Reader`).
#[must_use]
pub fn normalize_name(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

/// ROT13 (`UserAssist` value names are ROT13-encoded).
#[must_use]
pub fn rot13(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='M' | 'a'..='m' => char::from_u32(u32::from(c) + 13).unwrap_or(c),
            'N'..='Z' | 'n'..='z' => char::from_u32(u32::from(c) - 13).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// FILETIME (100 ns ticks since 1601-01-01) at data offset 60 → Unix
/// seconds (`UserAssist` `Count` binary layout; spec §11). `None` when
/// the blob is shorter than 68 bytes or the timestamp is zero.
///
/// # Panics
/// Never — the slice bounds are checked before the fixed-size
/// conversion (the `expect` below is unreachable).
#[must_use]
pub fn userassist_last_run(data: &[u8]) -> Option<i64> {
    if data.len() < 68 {
        return None;
    }
    #[allow(clippy::indexing_slicing)] // length checked above
    let ticks = u64::from_le_bytes(data[60..68].try_into().expect("slice length checked above"));
    if ticks == 0 {
        return None;
    }
    // FILETIME → Unix: (ticks / 10^7) - 11_644_473_600 seconds.
    let secs = ticks / 10_000_000;
    i64::try_from(secs).ok().map(|s| s - 11_644_473_600)
}

/// Best last-used for an install location from decoded `UserAssist`
/// entries: the newest timestamp among executables whose path is
/// inside `install_location` (case-insensitive, separator-safe).
#[must_use]
pub fn last_used_for_install(entries: &[(String, i64)], install_location: &str) -> Option<i64> {
    let loc = normalize_path(install_location);
    if loc.is_empty() {
        return None;
    }
    entries
        .iter()
        .filter(|(exe, _)| {
            let e = normalize_path(exe);
            e.len() > loc.len()
                && e.starts_with(&loc)
                && e.as_bytes().get(loc.len()) == Some(&b'\\')
        })
        .map(|(_, t)| *t)
        .max()
}

/// Path normalization for comparisons: lowercase, forward→back slashes,
/// drop a trailing separator.
#[must_use]
fn normalize_path(p: &str) -> String {
    let mut s: String = p.to_ascii_lowercase().replace('/', "\\");
    while s.ends_with('\\') {
        s.pop();
    }
    s
}

/// Parse a registry `DisplayIcon` value into a usable icon path:
/// strips surrounding quotes and a trailing `,index` suffix
/// (`"C:\App\app.exe",0` → `C:\App\app.exe`). `None` when empty.
#[must_use]
pub fn parse_display_icon(value: &str) -> Option<String> {
    let mut v = value.trim();
    if v.is_empty() {
        return None;
    }
    if let Some(stripped) = v.strip_prefix('"') {
        v = stripped;
        // Cut at the closing quote; anything after (",0") is the index.
        v = v.split('"').next().unwrap_or("");
    } else {
        // Unquoted: cut at ",N" if present (only after a path char).
        if let Some(pos) = v.rfind(',') {
            if v[pos + 1..].chars().all(|c| c.is_ascii_digit())
                && !v[pos + 1..].is_empty()
                && pos > 2
            {
                v = &v[..pos];
            }
        }
    }
    let v = v.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// Match one app's leftovers against the shared root listings.
/// Exact normalized equality only; a group is emitted per root with
/// ≥1 match, paths joined onto the root path.
#[must_use]
pub fn find_leftovers(app: &AppIdentity, roots: &[RootListing]) -> Vec<LeftoverGroup> {
    let [n_display, n_folder, n_family] = app.names();
    let n_publisher = normalize_name(&app.publisher);
    let mut out: Vec<LeftoverGroup> = Vec::new();
    for root in roots {
        let mut paths: Vec<LeftoverPath> = Vec::new();
        let root_sep = if root.path.ends_with('\\') { "" } else { "\\" };
        for child in &root.children {
            let n_child = normalize_name(&child.name);
            let direct = n_child == n_display
                || n_child == n_folder
                || n_child == n_family
                || (!n_publisher.is_empty() && n_child == n_publisher);
            if direct {
                paths.push(LeftoverPath {
                    path: format!("{}{}{}", root.path, root_sep, child.name),
                    size: child.size,
                });
            }
            // Publisher-nested: Roaming\Mozilla\Firefox — the publisher
            // child itself is only a container, but when a nested child
            // matches the app name it is a leftover.
            if !n_publisher.is_empty() && n_child == n_publisher {
                for (nested, size) in &child.nested {
                    let n_nested = normalize_name(nested);
                    if n_nested == n_display || n_nested == n_folder || n_nested == n_family {
                        paths.push(LeftoverPath {
                            path: format!("{}{}{}\\{}", root.path, root_sep, child.name, nested),
                            size: *size,
                        });
                    }
                }
            }
        }
        if !paths.is_empty() {
            let size = paths.iter().map(|p| p.size).sum();
            out.push(LeftoverGroup {
                label: ROOT_LABELS[root.label_idx.min(ROOT_LABELS.len() - 1)],
                paths,
                size,
            });
        }
    }
    out
}

/// Which root children need publisher-nested listings? The I/O layer
/// calls this once per root after building the direct children so the
/// shared listing includes one level inside publisher folders.
#[must_use]
pub fn publisher_children_to_expand(
    roots: &[RootListing],
    apps: &[AppIdentity],
) -> Vec<(usize, String)> {
    let publishers: Vec<String> = apps
        .iter()
        .map(|a| normalize_name(&a.publisher))
        .filter(|p| !p.is_empty())
        .collect();
    let mut out = Vec::new();
    for (ri, root) in roots.iter().enumerate() {
        for child in &root.children {
            let n = normalize_name(&child.name);
            if publishers.contains(&n) {
                out.push((ri, child.name.clone()));
            }
        }
    }
    out
}

/// Standard base64 (RFC 4648, padding) — icon PNGs become
/// `data:image/png;base64,…` data URLs without adding a crate.
#[must_use]
pub fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).map_or(0, |&b| u32::from(b));
        let b2 = chunk.get(2).map_or(0, |&b| u32::from(b));
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 0x3F] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 0x3F] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3F] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3F] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_case_space_punctuation() {
        assert_eq!(normalize_name("Adobe Reader 11.0"), "adobereader110");
        assert_eq!(normalize_name("  Foo-Bar_v2!! "), "foobarv2");
        assert_eq!(normalize_name("µTorrent"), "torrent");
        // Non-ASCII letters are dropped (ASCII-alnum key) — deliberate:
        // the key only needs to be injective enough for exact match.
        assert_eq!(normalize_name("Приложение"), "");
    }

    #[test]
    fn adobe_never_substring_matches() {
        // The rule that kills the classic bug: "Adobe" must not match
        // every Adobe product — matching is EXACT on the normalized key.
        let adobe = AppIdentity {
            display_name: "Adobe".into(),
            install_folder: "Adobe".into(),
            family_name: String::new(),
            publisher: String::new(),
        };
        let root = RootListing {
            label_idx: 0,
            path: "C:\\Users\\u\\AppData\\Local".into(),
            children: vec![
                RootChild {
                    name: "Adobe".into(),
                    size: 10,
                    nested: vec![],
                },
                RootChild {
                    name: "Adobe Acrobat DC".into(),
                    size: 500,
                    nested: vec![],
                },
                RootChild {
                    name: "Adobe Illustrator".into(),
                    size: 700,
                    nested: vec![],
                },
            ],
        };
        let groups = find_leftovers(&adobe, &[root]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].paths.len(), 1); // only exact "Adobe"
        assert_eq!(
            groups[0].paths[0].path,
            "C:\\Users\\u\\AppData\\Local\\Adobe"
        );
        assert_eq!(groups[0].size, 10);
    }

    #[test]
    fn publisher_nested_one_level() {
        // Roaming\Mozilla\Firefox (publisher container + nested match).
        let firefox = AppIdentity {
            display_name: "Mozilla Firefox".into(),
            install_folder: "Firefox".into(),
            family_name: String::new(),
            publisher: "Mozilla".into(),
        };
        let root = RootListing {
            label_idx: 1,
            path: "C:\\Users\\u\\AppData\\Roaming".into(),
            children: vec![RootChild {
                name: "Mozilla".into(),
                size: 999, // the container's own size
                nested: vec![
                    ("Firefox".to_string(), 450),
                    ("Thunderbird".to_string(), 300),
                ],
            }],
        };
        let groups = find_leftovers(&firefox, &[root]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, "Roaming AppData");
        // BOTH the publisher container (direct publisher match) and the
        // nested Firefox folder are leftovers.
        assert_eq!(groups[0].paths.len(), 2);
        assert!(groups[0]
            .paths
            .iter()
            .any(|p| p.path.ends_with("Mozilla\\Firefox") && p.size == 450));
        assert_eq!(groups[0].size, 999 + 450);
    }

    #[test]
    fn msix_family_name_matches_packages_root() {
        let app = AppIdentity {
            display_name: "Spotify".into(),
            install_folder: String::new(),
            family_name: "SpotifyAB.SpotifyMusic".into(),
            publisher: String::new(),
        };
        let root = RootListing {
            label_idx: 4,
            path: "C:\\Users\\u\\AppData\\Local\\Packages".into(),
            children: vec![RootChild {
                name: "SpotifyAB.SpotifyMusic_zpdnekdrzaja0".into(),
                size: 250,
                nested: vec![],
            }],
        };
        // Family names have a publisher hash suffix — the spec still
        // requires exact match, so only entries whose normalized name IS
        // the family name (no suffix) match. This one does not:
        let groups = find_leftovers(&app, std::slice::from_ref(&root));
        assert!(groups.is_empty());
        // Exact family name (no suffix) matches.
        let root2 = RootListing {
            label_idx: 4,
            path: "C:\\Users\\u\\AppData\\Local\\Packages".into(),
            children: vec![RootChild {
                name: "SpotifyAB.SpotifyMusic".into(),
                size: 250,
                nested: vec![],
            }],
        };
        let groups2 = find_leftovers(&app, &[root2]);
        assert_eq!(groups2.len(), 1);
        assert_eq!(groups2[0].label, "Store package data");
    }

    #[test]
    fn rot13_roundtrip_and_samples() {
        assert_eq!(rot13("Uryyb"), "Hello");
        assert_eq!(rot13("Hello"), "Uryyb");
        // UserAssist-style value name (ROT13 with GUID punctuation fixed).
        assert_eq!(
            rot13("{6D809377-6AF0-444B-8957-A3773F02200E}"),
            "{6Q809377-6NS0-444O-8957-N3773S02200R}"
        );
        assert_eq!(rot13(&rot13("Any ASCII x1y2")), "Any ASCII x1y2");
    }

    #[test]
    fn userassist_filetime_at_offset_60() {
        // 2026-09-21T00:00:00Z = 1_790_067_200 Unix.
        let unix: u64 = u64::try_from(1_790_067_200i64 + 11_644_473_600).unwrap();
        let ticks = unix * 10_000_000;
        let mut data = [0u8; 72];
        data[60..68].copy_from_slice(&ticks.to_le_bytes());
        assert_eq!(userassist_last_run(&data), Some(1_790_067_200));
        // Zero timestamp → None; short blob → None.
        assert_eq!(userassist_last_run(&[0u8; 72]), None);
        assert_eq!(userassist_last_run(&[0u8; 40]), None);
    }

    #[test]
    fn last_used_matches_only_exes_inside_install() {
        let entries = vec![
            ("C:\\Program Files\\App\\app.exe".to_string(), 100),
            ("C:\\Program Files\\App\\helper.exe".to_string(), 300),
            ("C:\\Program Files\\Other\\x.exe".to_string(), 900),
            ("C:\\Program Files\\App2\\y.exe".to_string(), 999), // prefix trap
        ];
        assert_eq!(
            last_used_for_install(&entries, r"C:\Program Files\App\"),
            Some(300)
        );
        assert_eq!(
            last_used_for_install(&entries, "c:\\PROGRAM FILES\\app"),
            Some(300)
        );
        assert_eq!(last_used_for_install(&entries, ""), None);
    }

    #[test]
    fn display_icon_parsing() {
        assert_eq!(
            parse_display_icon(r#""C:\App\app.exe",0"#).as_deref(),
            Some(r"C:\App\app.exe")
        );
        assert_eq!(
            parse_display_icon(r"C:\App\app.exe,1").as_deref(),
            Some(r"C:\App\app.exe")
        );
        assert_eq!(
            parse_display_icon(r#""C:\App\icon.ico""#).as_deref(),
            Some(r"C:\App\icon.ico")
        );
        assert_eq!(parse_display_icon("   ").as_deref(), None);
        // Path containing commas (no digit tail) stays intact.
        assert_eq!(
            parse_display_icon(r"C:\a,b\app.exe").as_deref(),
            Some(r"C:\a,b\app.exe")
        );
    }

    #[test]
    fn base64_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // 4 bytes = two chunks: "iVBO" + "Rw==".
        assert_eq!(base64_encode(&[0x89, b'P', b'N', b'G']), "iVBORw==");
    }

    #[test]
    fn publisher_expand_selection() {
        let apps = vec![AppIdentity {
            display_name: "Firefox".into(),
            install_folder: "Firefox".into(),
            family_name: String::new(),
            publisher: "Mozilla".into(),
        }];
        let roots = vec![RootListing {
            label_idx: 1,
            path: "C:\\Roaming".into(),
            children: vec![
                RootChild {
                    name: "Mozilla".into(),
                    size: 0,
                    nested: vec![],
                },
                RootChild {
                    name: "Unrelated".into(),
                    size: 0,
                    nested: vec![],
                },
            ],
        }];
        let expand = publisher_children_to_expand(&roots, &apps);
        assert_eq!(expand, vec![(0, "Mozilla".to_string())]);
    }
}
