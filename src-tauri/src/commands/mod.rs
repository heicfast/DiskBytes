//! IPC commands (thin layer, one file per domain — doc 02 §2).
//! M3: `scan` (start_scan / get_status / get_dev_hooks).
//! M4: `layout` (get_layout raw-binary + cache, get_names batching),
//! `explore` (Folders/Top Sizes/Age Map/List/inspector datasets),
//! `shell` (Open / Show in Explorer / Copy Path / text preview).
//! M8: `applications` (registry+MSIX enumeration, uninstall flow).
//! M9: `monitor` (2s sampler). M10: `license` + `analytics_cmd`.

pub mod analytics_cmd;
pub mod applications;
pub mod cleanup;
pub mod dupes;
pub mod explore;
pub mod layout;
pub mod license;
pub mod monitor;
pub mod scan;
pub mod shell;
pub mod sidebar;
pub mod snapshots_cmd;
