//! Layout engines (`BuildPrompt` §7 + doc 02 §5).
//!
//! Layouts are computed IN RUST and returned as a flat binary buffer of
//! fixed-size `#[repr(C)]` 32-byte cells plus a small JSON metadata block.
//! Never more than 20,000 cells per layout.
//!
//! Cell geometry is 5 × f32 whose meaning depends on the mode:
//! - **Treemap / Flame**: `[x, y, w, h, spare]`
//! - **Sunburst**: `[a0, a1, r0, r1, spare]` (center lives in metadata)
//! - **Bubbles**: `[cx, cy, r, spare, spare]`
//! - **Mind Map**: `[x, y, r, parent_x, parent_y]` (link = curve to parent)
//!
//! `rgba` is packed `0xRRGGBBAA` and serialized little-endian field-wise
//! (the JS twin decodes with `DataView` — see `src/viz/decode.ts`).

pub mod bubbles;
pub mod flame;
pub mod groups;
pub mod mindmap;
pub mod regroup;
pub mod sunburst;
pub mod treemap;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::scan::node::Tree;

/// Hard cell budget (spec §7: "Never return more than ~20,000 cells").
pub const MAX_CELLS: usize = 20_000;

/// A regroup/label-group tuple: `(synthetic id, name, size, color, members)`
/// where members are `(real node id, on-disk size)`.
pub type GroupTuple = (u32, String, u64, u32, Vec<(u32, u64)>);

/// Cell kind encoded in `Cell::flags` bits 0..=2.
pub mod cell_kind {
    /// Rectangle (treemap, flame).
    pub const RECT: u16 = 0;
    /// Annulus arc (sunburst).
    pub const ARC: u16 = 1;
    /// Circle (bubbles).
    pub const CIRCLE: u16 = 2;
    /// Mind-map dot (link drawn to the parent position).
    pub const DOT: u16 = 3;
    /// Label strip (treemap group header).
    pub const HEADER: u16 = 4;
    /// Extra flag (bit 3): the cell's node is a directory. Lets the JS
    /// hover chip and dblclick logic act without a round trip.
    pub const DIR_BIT: u16 = 1 << 3;
}

/// One layout cell — exactly 32 bytes (spec §7).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// Real node id or synthetic group id (`0xFFFF_0000 + n`).
    pub id: u32,
    /// Depth from the layout root.
    pub depth: u16,
    /// `cell_kind` | extra mode bits.
    pub flags: u16,
    /// `0xRRGGBBAA`.
    pub rgba: u32,
    /// Geometry (meaning per mode; see module docs).
    pub g: [f32; 5],
}

const _: () = assert!(size_of::<Cell>() == 32, "spec §7: cell must be 32 bytes");

impl Cell {
    /// Rectangle constructor.
    #[must_use]
    pub fn rect(id: u32, depth: u16, rgba: u32, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            depth,
            flags: cell_kind::RECT,
            rgba,
            g: [x, y, w, h, 0.0],
        }
    }

    /// Sunburst arc constructor.
    #[must_use]
    pub fn arc(id: u32, depth: u16, rgba: u32, a0: f32, a1: f32, r0: f32, r1: f32) -> Self {
        Self {
            id,
            depth,
            flags: cell_kind::ARC,
            rgba,
            g: [a0, a1, r0, r1, 0.0],
        }
    }

    /// Circle constructor.
    #[must_use]
    pub fn circle(id: u32, depth: u16, rgba: u32, cx: f32, cy: f32, r: f32) -> Self {
        Self {
            id,
            depth,
            flags: cell_kind::CIRCLE,
            rgba,
            g: [cx, cy, r, 0.0, 0.0],
        }
    }

    /// Mind-map dot constructor (link target = parent position).
    #[must_use]
    #[allow(clippy::too_many_arguments)] // geometry constructor — pairs with the 5-f32 cell payload
    pub fn dot(id: u32, depth: u16, rgba: u32, x: f32, y: f32, r: f32, px: f32, py: f32) -> Self {
        Self {
            id,
            depth,
            flags: cell_kind::DOT,
            rgba,
            g: [x, y, r, px, py],
        }
    }

    /// Treemap header strip constructor.
    #[must_use]
    pub fn header(id: u32, depth: u16, rgba: u32, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            depth,
            flags: cell_kind::HEADER,
            rgba,
            g: [x, y, w, h, 0.0],
        }
    }
}

/// How tree items are colored (spec §7 "Color modes").
///
/// Wire format: kebab-case ("by-folder" / "by-type" / "by-age") — the
/// JS contract (`state/vizUi` `ColorMode` + the mock backend). Without the
/// rename, serde demanded the Rust variant names (`"ByFolder"`…) and EVERY
/// `get_layout` invoke failed with "unknown variant 'by-folder'" — the
/// CI canvas frames were blank for exactly this reason (surfaced by the
/// inline viz error in FIX-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    /// Pastel family per top-level branch.
    ByFolder,
    /// Dominant category color, view regrouped per category.
    ByType,
    /// Age-bucket colors, view regrouped per bucket.
    ByAge,
}

/// Layout result: cells + JSON metadata (group descriptions, totals).
#[derive(Debug, Clone)]
pub struct LayoutBuffer {
    /// The cells (≤ [`MAX_CELLS`]).
    pub cells: Vec<Cell>,
    /// JSON metadata payload (see `LayoutMeta`).
    pub meta: LayoutMeta,
}

impl LayoutBuffer {
    /// Serialize cells to the raw little-endian binary body (IPC contract).
    #[must_use]
    pub fn cells_to_bytes(&self) -> Vec<u8> {
        // Safe field-wise serialization (no unsafe transmute needed; the
        // buffer is small and produced once per layout, then cached).
        let mut out = Vec::with_capacity(self.cells.len() * 32);
        for c in &self.cells {
            out.extend_from_slice(&c.id.to_le_bytes());
            out.extend_from_slice(&c.depth.to_le_bytes());
            out.extend_from_slice(&c.flags.to_le_bytes());
            out.extend_from_slice(&c.rgba.to_le_bytes());
            for f in c.g {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
        out
    }
}

/// Layout metadata returned as JSON alongside the binary cells.
///
/// Wire format: camelCase keys — the JS `LayoutMeta` contract
/// (`layoutIpc.ts`) and the mock backend's encoder. The core tests assert
/// the exact JSON keys so a future rename cannot silently drift the
/// binary IPC contract again.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutMeta {
    /// Layout mode that produced this buffer.
    pub mode: String,
    /// Scan generation (the UI drops stale results).
    pub generation: u64,
    /// Folder node the layout was computed for.
    pub node: u32,
    /// Viewport width in CSS px.
    pub width: f32,
    /// Viewport height in CSS px.
    pub height: f32,
    /// Depth setting used.
    pub depth: u32,
    /// Color mode used.
    pub color_mode: ColorMode,
    /// Number of cells emitted.
    pub cell_count: u32,
    /// True when the cell budget forced skipping smaller items.
    pub truncated: bool,
    /// Sunburst center `(cx, cy)` (other modes ignore it).
    pub center: Option<(f32, f32)>,
    /// Synthetic group legend (regroup modes; empty otherwise).
    pub groups: Vec<GroupDesc>,
    /// Total on-disk bytes of the laid-out subtree (100% reference).
    pub total_bytes: u64,
}

/// A synthetic group description (regroup modes, spec §7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDesc {
    /// Synthetic id `0xFFFF_0000 + n`.
    pub id: u32,
    /// Display name ("Video", "Last 7 days", …).
    pub name: String,
    /// `0xRRGGBB`.
    pub color: u32,
    /// Group size on disk.
    pub size: u64,
}

/// Pack a color as `0xRRGGBBAA` with full alpha.
#[must_use]
pub fn pack_rgba(rgb: u32) -> u32 {
    (rgb << 8) | 0xFF
}

/// Age bucket colors (spec §7 By age), bucket index 0..=5.
#[must_use]
pub const fn age_bucket_color(bucket: usize) -> u32 {
    const COLORS: [u32; 6] = [0x34D399, 0x60A5FA, 0x818CF8, 0xA78BFA, 0xF472B6, 0xF87171];
    // `Ord::min` is not const-callable; an if/else is (and clamps > 5 to 5).
    COLORS[if bucket > 5 { 5 } else { bucket }]
}

/// Pastel families for By-folder coloring (spec §7): blue, teal, violet,
/// amber, rose, green, sky, slate — shades vary by depth+index.
#[must_use]
pub fn folder_family_color(top_index: usize, depth: u16, index: usize) -> u32 {
    const FAMILIES: [(u8, u8, u8); 8] = [
        (147, 197, 253), // blue
        (153, 246, 228), // teal
        (196, 181, 253), // violet
        (253, 230, 138), // amber
        (253, 205, 211), // rose
        (187, 247, 208), // green
        (186, 230, 253), // sky
        (203, 213, 225), // slate
    ];
    let (r, g, b) = FAMILIES[top_index % FAMILIES.len()];
    // Shade: darken slightly with depth and vary by sibling index so
    // neighbors separate (stays inside the pastel family).
    let shade =
        1.0 - f32::from((depth.min(4)) as u8) * 0.06 - if index % 2 == 1 { 0.05 } else { 0.0 };
    let r = (f32::from(r) * shade).clamp(0.0, 255.0) as u32;
    let g = (f32::from(g) * shade).clamp(0.0, 255.0) as u32;
    let b = (f32::from(b) * shade).clamp(0.0, 255.0) as u32;
    (r << 16) | (g << 8) | b
}

/// Resolve the cell color for a real node under the given color mode.
#[must_use]
pub fn node_color(
    tree: &Tree,
    id: u32,
    mode: ColorMode,
    now: i64,
    top_index: usize,
    depth: u16,
    index: usize,
) -> u32 {
    match mode {
        ColorMode::ByFolder => folder_family_color(top_index, depth, index),
        ColorMode::ByType => {
            let cat = tree.dominant_category(id);
            cat.color()
        }
        ColorMode::ByAge => age_bucket_color(crate::age::bucket_of(
            tree.node(id).map_or(0, |n| n.modified),
            now,
        )),
    }
}

/// Validate geometry before layout math (doc 04 §4 "assert bounds early").
pub(crate) fn check_geometry(width: f32, height: f32) -> Result<(), CoreError> {
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return Err(CoreError::InvalidGeometry {
            width: width.max(0.0) as u32,
            height: height.max(0.0) as u32,
        });
    }
    Ok(())
}

/// The 6px² skip threshold for treemap cells (spec §7).
const MIN_CELL_AREA: f32 = 6.0;

/// Whether a rect is large enough to emit (spec §7 treemap rule).
#[must_use]
pub(crate) fn rect_visible(w: f32, h: f32) -> bool {
    w > 0.0 && h > 0.0 && w * h >= MIN_CELL_AREA
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Field-wise little-endian bytes of a cell (test twin of
    /// `LayoutBuffer::cells_to_bytes`).
    fn cell_to_le_bytes(c: &Cell) -> [u8; 32] {
        let mut out = [0u8; 32];
        let mut i = 0;
        out[i..i + 4].copy_from_slice(&c.id.to_le_bytes());
        i += 4;
        out[i..i + 2].copy_from_slice(&c.depth.to_le_bytes());
        i += 2;
        out[i..i + 2].copy_from_slice(&c.flags.to_le_bytes());
        i += 2;
        out[i..i + 4].copy_from_slice(&c.rgba.to_le_bytes());
        i += 4;
        for f in c.g {
            out[i..i + 4].copy_from_slice(&f.to_le_bytes());
            i += 4;
        }
        out
    }

    #[test]
    fn wire_json_keys_match_the_js_contract() {
        // The binary IPC contract: `LayoutMeta` serializes CAMELCASE and
        // `ColorMode` KEBAB — the JS `LayoutMeta`/`ColorMode` types
        // (layoutIpc.ts) + the mock's encoder. These exact keys once
        // drifted (`cell_count`, `"ByFolder"`) and every canvas layout
        // failed silently (CI blank treemap) — the keys are now pinned.
        let meta = LayoutMeta {
            mode: "treemap".into(),
            generation: 7,
            node: 3,
            width: 800.0,
            height: 600.0,
            depth: 4,
            color_mode: ColorMode::ByFolder,
            cell_count: 12,
            truncated: false,
            center: None,
            groups: vec![GroupDesc {
                id: 1,
                name: "g".into(),
                color: 0x112233,
                size: 5,
            }],
            total_bytes: 4096,
        };
        let json = serde_json::to_value(&meta).expect("meta serializes");
        for key in [
            "mode",
            "generation",
            "node",
            "width",
            "height",
            "depth",
            "colorMode",
            "cellCount",
            "truncated",
            "center",
            "groups",
            "totalBytes",
        ] {
            assert!(json.get(key).is_some(), "meta JSON must carry `{key}`");
        }
        assert_eq!(json["colorMode"], serde_json::json!("by-folder"));
        // The request side: kebab variants must round-trip.
        for (wire, variant) in [
            ("\"by-folder\"", ColorMode::ByFolder),
            ("\"by-type\"", ColorMode::ByType),
            ("\"by-age\"", ColorMode::ByAge),
        ] {
            let parsed: ColorMode = serde_json::from_str(wire).expect("kebab parses");
            assert_eq!(parsed, variant);
        }
        assert!(serde_json::from_str::<ColorMode>("\"ByFolder\"").is_err());
    }

    #[test]
    fn cell_is_32_bytes() {
        assert_eq!(size_of::<Cell>(), 32);
        let c = Cell::rect(1, 2, 0xFF0000FF, 1.0, 2.0, 3.0, 4.0);
        // Bit patterns: constructor values round-trip exactly.
        assert_eq!(c.g[0].to_bits(), 1.0f32.to_bits());
        assert_eq!(c.g[3].to_bits(), 4.0f32.to_bits());
    }

    #[test]
    fn cell_serialization_roundtrip_field_wise() {
        let c = Cell::arc(0x1234, 3, 0xAABBCCDD, 0.1, 1.2, 5.0, 6.0);
        let bytes = cell_to_le_bytes(&c);
        let buf = LayoutBuffer {
            cells: vec![c],
            meta: LayoutMeta {
                mode: "sunburst".into(),
                generation: 1,
                node: 0,
                width: 100.0,
                height: 100.0,
                depth: 5,
                color_mode: ColorMode::ByType,
                cell_count: 1,
                truncated: false,
                center: Some((50.0, 50.0)),
                groups: vec![],
                total_bytes: 42,
            },
        };
        assert_eq!(buf.cells_to_bytes().len(), 32);
        assert_eq!(bytes[0], 0x34);
        assert_eq!(bytes[1], 0x12);
    }

    #[test]
    fn colors_pack() {
        assert_eq!(pack_rgba(0xFF6B4A), 0xFF6B4AFF);
        assert_eq!(age_bucket_color(0), 0x34D399);
        assert_eq!(age_bucket_color(5), 0xF87171);
        assert_eq!(age_bucket_color(99), 0xF87171); // clamped
    }
}
