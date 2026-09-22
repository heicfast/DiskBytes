//! Cleanup commit commands (spec §9; doc 03 M5): the SAFETY-CRITICAL
//! flow — pre-flight + Recycle Bin move on a background thread, then
//! in-memory tree surgery (no rescan), cache clears, generation bump,
//! navigation/selection fixups and the `cleanup-committed` event.
//!
//! Zero direct-delete APIs exist in this crate (doc 09 §2 grep gate);
//! everything goes through `recycle::move_to_recycle_bin`
//! (IFileOperation, Recycle-Bin-only).

use std::sync::Arc;

use diskbytes_core::scan::node::Tree;
use diskbytes_core::scan::surgery;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::recycle::{self, StagedPath};
use crate::state::AppState;

/// One staged item from the JS queue (spec §9 shape).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitItem {
    /// Real node id (0 = path-only item).
    pub id: u32,
    /// Display path.
    pub path: String,
    /// Size on disk.
    pub size: u64,
    /// Stage reason (kept for queue parity; unused by the commit path).
    #[allow(dead_code)]
    pub reason: String,
}

/// The `cleanup-committed` payload.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCommitted {
    /// New tree generation (the UI re-keys every cache on it).
    pub generation: u64,
    /// Recycled items (incl. already-gone + nested).
    pub trashed: Vec<recycle::TrashedItem>,
    /// Refused items with reasons.
    pub failed: Vec<recycle::FailedItem>,
    /// Root stats after surgery (logical, on_disk, files, folders).
    pub stats: Option<(u64, u64, u64, u64)>,
    /// UI fixups: where navigation/selection landed after removal.
    pub current_folder: u32,
    pub selected_node: Option<u32>,
}

/// Commit the staged queue to the Recycle Bin (spec §9): pre-flight
/// refusals → IFileOperation → tree surgery without rescan.
///
/// # Errors
/// String error when the generation is stale or COM setup fails;
/// per-item problems land in the response's `failed` list.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub async fn commit_cleanup(
    generation: u64,
    items: Vec<CommitItem>,
    state: State<'_, AppState>,
    app: AppHandle,
    license: State<'_, crate::commands::license::LicenseManager>,
    analytics: State<'_, crate::analytics::Analytics>,
) -> Result<CleanupCommitted, String> {
    // The isPro gate (doc 06; spec licensing): free tier caps queue
    // bytes, degraded blocks, PRO/grace unlimited.
    let queue_total: u64 = items.iter().map(|i| i.size).sum();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    crate::commands::license::check_commit_gate(&license, queue_total, now)?;

    // Resolve the tree + protected flags under a short lock, then work
    // on the snapshot.
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

    // Join tree flags (protected) onto the staged paths.
    let staged: Vec<StagedPath> = items
        .into_iter()
        .map(|i| {
            let protected = tree
                .node(i.id)
                .is_some_and(|n| n.is_protected() || n.is_cloud_placeholder());
            // Path-less synthetic entries are refused outright (never
            // attempt a meaningless shell move).
            let empty_path = i.path.is_empty();
            StagedPath {
                id: i.id,
                path: i.path,
                size: i.size,
                protected: protected || empty_path,
            }
        })
        .collect();

    // The Recycle Bin move runs on the blocking pool (COM thread).
    let outcome =
        tauri::async_runtime::spawn_blocking(move || recycle::move_to_recycle_bin(staged))
            .await
            .map_err(|e| format!("cleanup thread failed: {e}"))??;

    // Engine telemetry (doc 07 §4): counts only, never paths.
    analytics.capture(
        "cleanup_committed",
        &[
            ("items", serde_json::json!(outcome.trashed.len())),
            ("bytes", serde_json::json!(queue_total)),
            ("failed", serde_json::json!(outcome.failed.len())),
        ],
    );

    // Tree surgery for every successfully recycled REAL node (path-only
    // items like leftovers have no node to remove).
    let removed_ids: Vec<u32> = outcome
        .trashed
        .iter()
        .map(|t| tree_lookup_id(&tree, &t.path))
        .filter(|&id| id > 0)
        .collect();

    let (new_generation, root_stats, current_folder, selected_node) = if removed_ids.is_empty() {
        (tree.generation, Some(tree.root_stats()), 0, None)
    } else {
        // CoW surgery under the WRITE lock: clone only when a reader
        // still holds an Arc (rare — caches were generation-keyed).
        let mut guard = state.tree.write();
        // Take the Arc out (briefly owning the tree exclusively), run
        // surgery on a fresh Arc via Arc::get_mut — free when no reader
        // shares it; when one does, surgery runs on a rebuilt Arc from
        // the old snapshot's data (Tree: From<&Tree> deep copy).
        let arc = guard.take().ok_or("tree vanished mid-commit")?;
        drop(guard);
        let mut owned: Tree = match Arc::try_unwrap(arc) {
            Ok(t) => t,
            Err(shared) => {
                // A blocking reader (top-sizes ranking) still holds a
                // snapshot; deep-copy once (rare, off the UI thread).
                let snapshot: &Tree = &shared;
                Tree::deep_from(snapshot)
            }
        };
        surgery::remove_subtrees(&mut owned, &removed_ids);
        let new_generation = owned.generation;
        let stats_after = owned.root_stats();
        // UI fixups: the removed navigation point walks up to a survivor.
        let current_folder = surgery::fixup_navigation(&owned, 0);
        *state.tree.write() = Some(Arc::new(owned));
        (new_generation, Some(stats_after), current_folder, None)
    };

    // Generation-keyed caches ALL drop (scan-swap path clears the same
    // set — reuse it by hand here because the tree did not rescan).
    crate::commands::layout::clear_cache(&app.state::<crate::commands::layout::Cache>());
    crate::commands::layout::clear_regroup_cache(
        &app.state::<crate::commands::layout::RegroupCache>(),
    );
    crate::commands::explore::TopCache::clear(&app.state::<crate::commands::explore::TopCache>());
    crate::commands::explore::AgeCache::clear(&app.state::<crate::commands::explore::AgeCache>());
    app.state::<crate::commands::sidebar::QuickWinsCache>()
        .clear_pub();
    app.state::<crate::commands::applications::AppsCache>()
        .clear();

    let payload = CleanupCommitted {
        generation: new_generation,
        trashed: outcome.trashed,
        failed: outcome.failed,
        stats: root_stats,
        current_folder,
        selected_node,
    };
    let _ = app.emit("cleanup-committed", &payload);
    Ok(payload)
}

/// Find the node id for a display path (cheap parent-chain walk is not
/// possible without an index; scan the arena once per commit — commits
/// are rare and the arena walk is branch-predictable).
fn tree_lookup_id(tree: &Tree, path: &str) -> u32 {
    if path.is_empty() {
        return 0;
    }
    for (idx, n) in tree.arena.iter().enumerate() {
        if n.is_removed() || n.parent == u32::MAX {
            continue;
        }

        // Full-path compare via the parent-chain walk (no String per node).
        if path_matches(tree, idx, path) {
            return idx as u32;
        }
    }
    0
}

/// Fast full-path equality without building every String: walk the
/// parent chain comparing name slices right-to-left.
fn path_matches(tree: &Tree, idx: usize, path: &str) -> bool {
    // Building one String per candidate is wasteful at 1M nodes; the
    // parent-walk compare avoids it: collect ancestor name slices and
    // compare against the path from the end.
    let mut chain: Vec<(usize, usize)> = Vec::with_capacity(8); // (off, len)
    let mut id = idx as u32;
    loop {
        let n = &tree.arena[id as usize];
        chain.push((n.name_off as usize, usize::from(n.name_len)));
        if n.parent == u32::MAX {
            break;
        }
        id = n.parent;
    }
    // Compare the chain (root→leaf) joined by '\' against `path`.
    let mut consumed = 0usize;
    for (i, (off, len)) in chain.iter().rev().enumerate() {
        if i > 0 {
            match path.as_bytes().get(consumed) {
                Some(b'\\') => consumed += 1,
                _ => return false,
            }
        }
        let name = &tree.names[*off..off + len];
        for &u in name {
            let Some(c) = path[consumed..].chars().next() else {
                return false;
            };
            if u32::from(c) != u32::from(u) {
                return false;
            }
            consumed += c.len_utf8();
        }
    }
    consumed == path.len()
}

/// Open the Recycle Bin folder (the confirmation dialog's link —
/// spec §9 `shell:RecycleBinFolder`).
///
/// # Errors
/// String error when the shell cannot open it.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // State extraction is the tauri command contract
pub fn open_recycle_bin() -> Result<(), String> {
    crate::platform::HostPlatform::open_path("shell:RecycleBinFolder")
}
