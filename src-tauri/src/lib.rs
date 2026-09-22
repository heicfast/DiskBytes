//! DiskBytes `Tauri` app entry (spec §2; doc 02 §2).
//!
//! The app crate owns the `WebView` shell, plugins and IPC commands; all
//! platform-independent logic lives in `diskbytes-core` so it stays
//! testable on any host (decision D10). M3 registers the scan commands
//! (doc 03 M3.7); later milestones add layout/cleanup/dupes/apps/
//! monitor/snapshot commands.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analytics;
mod commands;
mod license;
mod platform;
mod recycle;
mod state;

#[cfg(test)]
mod tests_support;

use std::sync::Arc;

use platform::HostPlatform;

use tauri::Manager;

/// Build and run the app (single window configured in `tauri.conf.json`).
///
/// # Panics
/// Panics when the `Tauri` runtime fails to start (event-loop failure,
/// window creation failure). The message surfaces in the crash log; a
/// desktop app cannot meaningfully continue without its window.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Spec M0.4: the ONLY plugin is the file/folder dialog.
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(state::AppState::new());
            app.manage(commands::layout::layout_cache());
            app.manage(commands::layout::regroup_cache());
            app.manage(commands::explore::top_cache());
            app.manage(commands::explore::age_cache());
            app.manage(commands::sidebar::quick_wins_cache());
            app.manage(commands::applications::apps_cache());
            app.manage(commands::monitor::monitor_state());
            app.manage(commands::license::license_manager());
            app.manage(analytics::Analytics::init());
            app.manage(Arc::new(HostPlatform) as Arc<HostPlatform>);
            // Dev hooks (spec §15): auto-start a scan when requested.
            commands::license::start_scheduler(&app.handle().clone());
            let hooks = commands::scan::read_dev_hooks();
            if let Some(target) = hooks.scan {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let state = handle.state::<state::AppState>();
                    let platform = handle.state::<Arc<HostPlatform>>();
                    let _ = tauri::async_runtime::block_on(commands::scan::start_scan(
                        target,
                        handle.clone(),
                        state,
                        platform,
                    ));
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::get_status,
            commands::scan::get_dev_hooks,
            commands::scan::start_scan_turbo,
            commands::layout::get_layout,
            commands::layout::get_names,
            commands::explore::get_folder_view,
            commands::explore::node_details,
            commands::explore::top_sizes,
            commands::explore::age_map,
            commands::explore::list_children,
            commands::explore::get_breadcrumb,
            commands::shell::open_node,
            commands::shell::open_url,
            commands::shell::reveal_in_explorer,
            commands::shell::copy_path,
            commands::shell::preview_text,
            commands::shell::hover_details,
            commands::cleanup::commit_cleanup,
            commands::cleanup::open_recycle_bin,
            commands::sidebar::get_drive_chips,
            commands::sidebar::get_home_path,
            commands::sidebar::disk_storage,
            commands::sidebar::is_elevated,
            commands::sidebar::restart_as_admin,
            commands::sidebar::quick_wins,
            commands::sidebar::quick_win_items,
            commands::sidebar::file_types,
            commands::dupes::find_duplicates,
            commands::applications::list_applications,
            commands::applications::uninstall_app,
            commands::applications::leftover_root_paths,
            commands::monitor::monitor_start,
            commands::monitor::monitor_stop,
            commands::license::license_status,
            commands::license::activate_license,
            commands::license::deactivate_license,
            commands::license::validate_now,
            commands::analytics_cmd::analytics_opt_out,
            commands::analytics_cmd::set_analytics_opt_out,
            commands::snapshots_cmd::list_snapshots,
            commands::snapshots_cmd::take_snapshot,
            commands::snapshots_cmd::diff_snapshots,
            commands::snapshots_cmd::delete_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
