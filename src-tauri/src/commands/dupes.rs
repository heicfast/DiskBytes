//! Duplicates commands (spec §10; doc 03 M8): the 3-pass flow
//! (size-grouping, 64 KiB prefix SHA-256, full hashing for matches)
//! streamed in 1 MiB chunks. Hardlink exclusion via
//! (volume-serial, file-index); cloud placeholders never open (R7.3);
//! wasted-space ranking per the spec.

use std::collections::HashMap;
use std::sync::Arc;

use diskbytes_core::dupes::{self, DupeGroup, HashedFile};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::State;

use crate::state::AppState;

/// Prefix-hash chunk (64 KiB, spec §10).
const PREFIX: u64 = 64 * 1024;
/// Full-hash streaming chunk (1 MiB, spec §10).
const CHUNK: usize = 1024 * 1024;

/// One duplicate-group row for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupeGroupView {
    /// Group id (index).
    pub id: usize,
    /// Paths of the group's members.
    pub paths: Vec<String>,
    /// Per-file size.
    pub size: u64,
    /// Member count.
    pub count: u64,
    /// Wasted space = size × (count − 1).
    pub wasted: u64,
}

/// The duplicates response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupesResult {
    pub generation: u64,
    pub groups: Vec<DupeGroupView>,
    /// Total wasted bytes.
    pub wasted_total: u64,
    /// Files considered.
    pub files: u64,
}

/// Hash a file per the spec passes: the 64 KiB prefix, then the full
/// stream when the file exceeds it. Returns the digest (opaque to the
/// core). `None` = unreadable (skipped honestly).
fn hash_file(path: &std::path::Path, size: u64) -> Option<[u8; 32]> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut remaining = if size > PREFIX { u64::MAX } else { size };
    let mut buf = vec![0u8; CHUNK];
    loop {
        let cap = if remaining == u64::MAX {
            buf.len()
        } else {
            (remaining.min(buf.len() as u64)) as usize
        };
        if cap == 0 {
            break;
        }
        let n = f.read(&mut buf[..cap]).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        if remaining != u64::MAX {
            remaining = remaining.saturating_sub(n as u64);
        }
    }
    let digest: [u8; 32] = hasher.finalize().into();
    Some(digest)
}

/// Hardlink identity via the platform seam `win::hardlink_identity`
/// (spec §10: hardlinks are NOT duplicates). None = unavailable
/// (treated unique). Non-Windows builds have no hardlinks to detect.
#[cfg(windows)]
fn hardlink_identity(path: &std::path::Path) -> Option<(u64, u64)> {
    crate::platform::os::hardlink_identity(path)
}

#[cfg(not(windows))]
fn hardlink_identity(_path: &std::path::Path) -> Option<(u64, u64)> {
    None
}

/// Find duplicates in the current tree (spec §10 3-pass).
///
/// # Errors
/// String error when no scan exists or the generation is stale.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub async fn find_duplicates(
    generation: u64,
    state: State<'_, AppState>,
) -> Result<DupesResult, String> {
    let tree = {
        let guard = state.tree.read();
        let Some(tree) = guard.as_ref() else {
            return Err("no scan yet".into());
        };
        if tree.generation != generation {
            return Err(format!(
                "stale generation {} (current {})",
                generation, tree.generation
            ));
        }
        Arc::clone(tree)
    };
    let result = tauri::async_runtime::spawn_blocking(move || compute_dupes(&tree))
        .await
        .map_err(|e| format!("dupes thread failed: {e}"))?;
    Ok(result)
}

/// The full pipeline (spec §10 3-pass): collect → size groups →
/// prefix/full hashes → hardlink exclusion → wasted-space ranking via
/// the core.
fn compute_dupes(tree: &diskbytes_core::scan::node::Tree) -> DupesResult {
    // Collect live files (cloud placeholders NEVER opened — R7.3).
    struct Candidate {
        path: String,
        size: u64,
        id: u32,
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    tree.walk(tree.root, |id, n| {
        if !n.is_dir() && !n.is_removed() && !n.is_cloud_placeholder() && n.logical > 0 {
            candidates.push(Candidate {
                path: tree.node_path(id),
                size: n.logical,
                id,
            });
        }
    });
    let total_files = candidates.len() as u64;

    // Pass 1: size buckets (candidate level).
    let mut by_size: HashMap<u64, Vec<&Candidate>> = HashMap::new();
    for c in &candidates {
        by_size.entry(c.size).or_default().push(c);
    }

    // Passes 2+3: hash + hardlink identity → HashedFile (core contract).
    let mut hashed: Vec<HashedFile> = Vec::new();
    for (size, bucket) in &by_size {
        if bucket.len() < 2 {
            continue; // Single size = no duplicate candidates.
        }
        for c in bucket {
            if let Some(digest) = hash_file(std::path::Path::new(&c.path), *size) {
                let (vs, fi) = hardlink_identity(std::path::Path::new(&c.path))
                    .unwrap_or((u64::MAX, u64::from(c.id)));
                hashed.push(HashedFile {
                    path: c.path.clone(),
                    size: *size,
                    volume_serial: vs,
                    file_index: fi,
                    sha256: digest,
                });
            }
        }
    }

    // Core ranking (hardlink exclusion + wasted-space sort).
    let groups: Vec<DupeGroup> = dupes::rank(&hashed);
    let (wasted_total, group_count) = dupes::totals(&groups);
    let views: Vec<DupeGroupView> = groups
        .into_iter()
        .take(200)
        .enumerate()
        .map(|(id, g)| {
            let count = g.files.len() as u64;
            DupeGroupView {
                id,
                paths: g.files,
                size: g.size,
                count,
                wasted: g.wasted,
            }
        })
        .collect();
    let _ = group_count;
    DupesResult {
        generation: tree.generation,
        groups: views,
        wasted_total,
        files: total_files,
    }
}
