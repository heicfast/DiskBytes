//! Snapshots commands (spec §13; doc 03 M9): take/list/diff/delete
//! round-trip over the core snapshot store (atomic writes, ≥1 MiB
//! folders, case-insensitive keys, top-200 diff).

use serde::Serialize;
use tauri::State;

use diskbytes_core::snapshots::{self, Snapshot, SnapshotSummary};
use std::path::PathBuf;

/// The snapshots directory (app data; created on demand).
/// Minimum folder size stored in snapshots (spec §13).
const MIN_1MIB: u64 = 1024 * 1024;

fn snapshots_dir() -> PathBuf {
    let base = std::env::var("APPDATA").map_or_else(
        |_| PathBuf::from("."),
        |d| PathBuf::from(d).join("DiskBytes"),
    );
    let dir = base.join("snapshots");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn index_path() -> PathBuf {
    snapshots_dir().join("index.json")
}

/// One snapshot row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotView {
    pub id: String,
    pub root: String,
    pub taken_at: i64,
    pub total: u64,
    pub folders: u64,
}

impl From<&SnapshotSummary> for SnapshotView {
    fn from(s: &SnapshotSummary) -> Self {
        Self {
            id: s.id.clone(),
            root: s.root.clone(),
            taken_at: s.date,
            total: s.total,
            folders: s.folder_count,
        }
    }
}

/// List saved snapshots (index summaries). Absence = empty list.
#[tauri::command]
pub fn list_snapshots() -> Vec<SnapshotView> {
    let path = index_path();
    if !path.exists() {
        return Vec::new();
    }
    let idx: Vec<SnapshotSummary> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    idx.iter().map(std::convert::Into::into).collect()
}

/// Take a snapshot of the current tree (folders ≥ 1 MiB; atomic write).
///
/// # Errors
/// String error when no scan exists or the write fails.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn take_snapshot(
    generation: u64,
    state: State<'_, crate::state::AppState>,
) -> Result<SnapshotView, String> {
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
    // Folders ≥ 1 MiB (spec §13 — MIN_1MIB above).
    let root_path = tree
        .roots
        .first()
        .map_or_else(|| tree.name(tree.root), |r| r.path.clone());
    let mut pairs: Vec<(String, u64)> = Vec::new();
    for id in 0..tree.len() as u32 {
        let Some(n) = tree.node(id) else { continue };
        if n.is_dir() && n.on_disk >= MIN_1MIB {
            pairs.push((tree.node_path(id), n.on_disk));
        }
    }
    let taken_at = now_unix();
    let id = format!(
        "{}-{}",
        root_path.replace('\\', "-").replace(':', ""),
        taken_at
    );
    let snap = Snapshot::build(id, root_path, taken_at, pairs);
    let view: SnapshotView = (&snapshots::summarize(&snap)).into();
    let file = snapshots_dir().join(format!("{}.json", snap.id));
    snapshots::write_json_atomic(&file, &snap)
        .map_err(|e| format!("Couldn't write the snapshot: {e:?}"))?;
    // Index update (atomic).
    let mut idx: Vec<SnapshotSummary> = std::fs::read_to_string(index_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    idx.push(snapshots::summarize(&snap));
    snapshots::write_json_atomic(&index_path(), &idx)
        .map_err(|e| format!("Couldn't update the snapshot index: {e:?}"))?;
    Ok(view)
}

/// One changed-folder row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeView {
    pub path: String,
    pub before: u64,
    pub after: u64,
    pub delta: i64,
}

/// The diff response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffView {
    pub same_root: bool,
    pub total_before: u64,
    pub total_after: u64,
    pub changes: Vec<ChangeView>,
}

/// Diff two snapshots (case-insensitive, top-200 by |delta|).
///
/// # Errors
/// String error when a snapshot file cannot be read.
#[tauri::command]
pub fn diff_snapshots(before_id: &str, after_id: &str) -> Result<DiffView, String> {
    let load = |id: &str| -> Result<Snapshot, String> {
        let p = snapshots_dir().join(format!("{id}.json"));
        snapshots::read_snapshot(&p).map_err(|e| format!("Couldn't read {id}: {e:?}"))
    };
    let before = load(before_id)?;
    let after = load(after_id)?;
    let d = snapshots::diff(&before, &after);
    Ok(DiffView {
        same_root: !d.different_roots,
        total_before: d.totals.0,
        total_after: d.totals.1,
        changes: d
            .changes
            .into_iter()
            .take(200)
            .map(|c| ChangeView {
                path: c.path,
                before: c.before,
                after: c.after,
                delta: c.delta,
            })
            .collect(),
    })
}

/// Delete a snapshot (app-owned data file; the core persistence layer
/// owns the removal + index rewrite — the app crate keeps ZERO
/// direct-delete APIs, R7.1).
///
/// # Errors
/// String error when the file exists but cannot be removed, or the
/// index rewrite fails.
#[tauri::command]
pub fn delete_snapshot(id: &str) -> Result<(), String> {
    snapshots::delete_snapshot(&snapshots_dir(), id)
        .map_err(|e| format!("Couldn't delete the snapshot: {e:?}"))
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}
