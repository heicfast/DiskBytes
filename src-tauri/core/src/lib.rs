//! # diskbytes-core
//!
//! Platform-independent heart of `DiskBytes` (`BuildPrompt` §2, §4, §7, §13):
//! the in-memory node arena, roll-up, file categories, layout engines,
//! quick-wins matching, age analysis, snapshot diffing and duplicate
//! grouping. This crate must build on ANY host without a `WebView` shell so
//! its logic stays unit-testable everywhere (decision log D10).
//!
//! Module map (names preserved from `documentation/02_ARCHITECTURE_BLUEPRINT.md` §2):
//! - [`scan::node`] — the ≤72-byte `Node` arena, `DirExtra`, `Tree`, path rebuild
//! - [`scan::categories`] — the 9 `FileCategory` buckets (spec §4 table, exact)
//! - [`scan::rollup`] — the reverse linear roll-up + CSR child ordering
//! - [`layout`] — cell buffers + treemap/sunburst/flame/bubbles/mindmap/regroup
//! - [`quickwins`] — junk category resolution against a built tree (spec §6)
//! - [`age`] — age buckets, month heatmap, Big & Untouched (spec §7)
//! - [`snapshots`] — snapshot model, atomic write payloads, case-insensitive diff
//! - [`dupes`] — duplicate grouping + wasted-space ranking (hashing is app-side)
//! - [`format`] — byte/percent/age/duration formatting (TS twin must agree)

pub mod age;
pub mod apps;
pub mod dupes;
pub mod error;
pub mod format;
pub mod layout;
pub mod monitor;
pub mod platform;
pub mod quickwins;
pub mod scan;
pub mod snapshots;
pub mod turbo;

pub use error::CoreError;

/// Type alias used across command boundaries: every tree-viewing request
/// carries the scan generation it was issued against (spec §4).
pub type Generation = u64;
