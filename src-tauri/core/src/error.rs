//! Typed errors for the core crate (`BuildPrompt` §4/§9; doc 04 §3).
//!
//! No stringly-typed failures: every variant maps to a user-readable reason
//! the IPC layer can render. The only permitted *behavioral* fallback in the
//! whole product (Turbo → standard engine, with a stated reason) is expressed
//! by [`CoreError::TurboUnavailable`], never silently.

use thiserror::Error;

/// Errors produced by core tree/layout/snapshot operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The request referenced a node id that does not exist in the arena.
    #[error("node {0} not found in this scan")]
    NodeNotFound(u32),
    /// The caller's generation is stale: a newer scan replaced the tree.
    #[error("stale generation {0}; the scan was replaced")]
    StaleGeneration(u64),
    /// A synthetic regroup id was used where a real node id is required.
    #[error("synthetic group {0:#x} is not a real node")]
    SyntheticNode(u32),
    /// The layout request produced more cells than the 20,000-cell budget
    /// (spec §7) — the caller must lower depth or narrow the subtree.
    #[error("layout would emit {0} cells; budget is 20000")]
    TooManyCells(usize),
    /// Invalid geometry was requested (zero-size viewport etc.).
    #[error("invalid layout geometry {width}x{height}")]
    InvalidGeometry {
        /// Requested width in CSS pixels.
        width: u32,
        /// Requested height in CSS pixels.
        height: u32,
    },
    /// Turbo engine preconditions failed; the reason is user-visible
    /// (the ONE sanctioned fallback: run the standard engine, stating this).
    #[error("turbo engine unavailable: {0}")]
    TurboUnavailable(String),
    /// A snapshot file could not be parsed.
    #[error("snapshot parse error: {0}")]
    SnapshotParse(String),
    /// Age-map/quick-wins resolution failure carrying a reason.
    #[error("resolution error: {0}")]
    Resolution(String),
}

impl CoreError {
    /// Machine-readable kind tag used by the IPC layer to pick UI treatment.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NodeNotFound(_) => "node_not_found",
            Self::StaleGeneration(_) => "stale_generation",
            Self::SyntheticNode(_) => "synthetic_node",
            Self::TooManyCells(_) => "too_many_cells",
            Self::InvalidGeometry { .. } => "invalid_geometry",
            Self::TurboUnavailable(_) => "turbo_unavailable",
            Self::SnapshotParse(_) => "snapshot_parse",
            Self::Resolution(_) => "resolution",
        }
    }
}
