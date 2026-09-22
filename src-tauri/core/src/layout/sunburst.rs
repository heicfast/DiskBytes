//! Sunburst (spec §7 mode 3): "Rings radiating out from the scan root".
//!
//! One ring per depth level; arc length ∝ size. The center disc shows the
//! folder. Cells are arcs `[a0, a1, r0, r1]`; the shared center lives in
//! `LayoutMeta::center`. Labels are drawn JS-side (radial on narrow arcs,
//! along the ring on wide ones, never upside-down).

use crate::error::CoreError;
use crate::layout::{
    check_geometry, node_color, pack_rgba, Cell, ColorMode, LayoutBuffer, LayoutMeta, MAX_CELLS,
};
use crate::scan::node::Tree;

/// Center disc radius fraction of the smaller viewport side.
const CENTER_R_FRACTION: f32 = 0.12;
/// Minimum arc span (radians) to emit a cell.
const MIN_ARC: f32 = 0.004;
/// Gap between rings and between sibling arcs (visual separation).
const RING_GAP: f32 = 1.5;
const ARC_GAP: f32 = 0.003;

/// Layout the subtree under `node` as a sunburst.
///
/// # Errors
/// - [`CoreError::InvalidGeometry`] when `width`/`height` are zero.
/// - [`CoreError::NodeNotFound`] when `node` is not in the arena.
#[allow(clippy::too_many_arguments)]
pub fn sunburst(
    tree: &Tree,
    node: u32,
    width: f32,
    height: f32,
    depth: u32,
    color: ColorMode,
    now: i64,
) -> Result<LayoutBuffer, CoreError> {
    check_geometry(width, height)?;
    let n = tree.node(node).ok_or(CoreError::NodeNotFound(node))?;
    let total = n.on_disk;
    let cx = width / 2.0;
    let cy = height / 2.0;
    let r_max = width.min(height) / 2.0 - 4.0;
    let r_center = r_max * CENTER_R_FRACTION;
    let ring_w = if depth > 0 {
        (r_max - r_center - RING_GAP * depth as f32) / depth as f32
    } else {
        0.0
    };
    let mut cells: Vec<Cell> = Vec::with_capacity(512);
    let mut truncated = false;
    // Center disc: the folder itself.
    cells.push(Cell::circle(node, 0, pack_rgba(0x8E8E93), cx, cy, r_center));
    if depth > 0 && total > 0 {
        let full = std::f32::consts::TAU;
        layout_ring(
            tree,
            node,
            full,
            0.0,
            r_center + RING_GAP,
            ring_w,
            1,
            depth,
            color,
            now,
            &mut cells,
            &mut truncated,
            0,
        );
    }
    Ok(LayoutBuffer {
        cells,
        meta: LayoutMeta {
            mode: "sunburst".into(),
            generation: tree.generation,
            node,
            width,
            height,
            depth,
            color_mode: color,
            cell_count: 0,
            truncated,
            center: Some((cx, cy)),
            groups: Vec::new(),
            total_bytes: total,
        },
    })
}

/// Recursive ring layout: children of `node` inside the angular span
/// `(a0..a1)` on ring at radius `r0`, width `ring_w`.
#[allow(clippy::too_many_arguments)]
fn layout_ring(
    tree: &Tree,
    node: u32,
    span: f32,
    start_angle: f32,
    r0: f32,
    ring_w: f32,
    depth_here: u32,
    depth_left: u32,
    color: ColorMode,
    now: i64,
    cells: &mut Vec<Cell>,
    truncated: &mut bool,
    top_index: usize,
) {
    if depth_left == 0 || ring_w <= 0.5 {
        return;
    }
    let children = tree.children_sorted(node);
    let total: u64 = children
        .iter()
        .map(|&id| tree.node(id).map_or(0, |c| c.on_disk))
        .sum();
    if total == 0 {
        return;
    }
    let mut cursor = start_angle;
    let usable = span - ARC_GAP * children.len().min(64) as f32;
    let gap = ARC_GAP;
    for (i, &id) in children.iter().enumerate() {
        if cells.len() >= MAX_CELLS {
            *truncated = true;
            return;
        }
        let c = tree.node(id).expect("child id");
        if c.is_removed() || c.on_disk == 0 {
            continue;
        }
        let arc = c.on_disk as f32 / total as f32 * usable.max(0.0);
        if arc < MIN_ARC {
            continue; // Skip invisible slivers; proportions stay honest.
        }
        let a0 = cursor;
        let a1 = cursor + arc;
        let rgba = pack_rgba(match color {
            ColorMode::ByFolder => node_color(tree, id, color, now, i, depth_here as u16, i),
            ColorMode::ByType => c.category().color(),
            ColorMode::ByAge => node_color(tree, id, color, now, 0, 0, i),
        });
        cells.push(Cell::arc(
            id,
            depth_here as u16,
            rgba,
            a0,
            a1,
            r0,
            r0 + ring_w,
        ));
        if c.is_dir() && c.child_count > 0 && depth_left > 1 {
            layout_ring(
                tree,
                id,
                arc,
                a0,
                r0 + ring_w + RING_GAP,
                ring_w,
                depth_here + 1,
                depth_left - 1,
                color,
                now,
                cells,
                truncated,
                if depth_here == 1 { i } else { top_index },
            );
        }
        cursor = a1 + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::node::{BatchEntry, Node, Tree};
    use crate::scan::rollup;

    fn build() -> Tree {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\S");
        t.append_batch(
            0,
            vec![
                dir("a"),
                dir("b"),
                file("f1.bin", 100, 100, 1),
                file("f2.bin", 50, 50, 1),
            ],
        );
        t.append_batch(1, vec![file("a1", 160, 160, 1), file("a2", 30, 30, 1)]);
        t.append_batch(2, vec![file("b1", 20, 20, 1)]);
        rollup::finalize(&mut t);
        t
    }

    fn dir(name: &str) -> BatchEntry {
        let mut node = Node::new_dir();
        node.modified = 1;
        BatchEntry {
            name: name.encode_utf16().collect(),
            node,
        }
    }

    fn file(name: &str, logical: u64, on_disk: u64, modified: i64) -> BatchEntry {
        let mut node = Node::new_file();
        node.logical = logical;
        node.on_disk = on_disk;
        node.modified = modified;
        BatchEntry {
            name: name.encode_utf16().collect(),
            node,
        }
    }

    #[test]
    fn arcs_proportional_and_nested() {
        let t = build();
        let buf = sunburst(&t, 0, 800.0, 800.0, 3, ColorMode::ByFolder, 1).unwrap();
        assert!(buf.cells.len() >= 6);
        assert_eq!(buf.meta.center, Some((400.0, 400.0)));
        // Ring-1 arcs cover ~TAU and children nest inside parent spans.
        let ring1: Vec<&Cell> = buf
            .cells
            .iter()
            .filter(|c| c.flags == crate::layout::cell_kind::ARC && c.depth == 1)
            .collect();
        assert_eq!(ring1.len(), 4);
        let total_span: f32 = ring1.iter().map(|c| c.g[1] - c.g[0]).sum();
        assert!((total_span - std::f32::consts::TAU).abs() < 0.2);
        // Proportions: 190 (a) / 20 (b) / 100 / 50 of 360 total.
        let a = ring1.iter().find(|c| c.id == 1).unwrap();
        let f1 = ring1.iter().find(|c| c.id == 3).unwrap();
        let span_a = a.g[1] - a.g[0];
        let span_f1 = f1.g[1] - f1.g[0];
        assert!((span_a / span_f1 - 1.9).abs() < 0.02, "190 vs 100 ratio");
    }

    #[test]
    fn invalid_geometry_rejected() {
        let t = build();
        assert!(sunburst(&t, 0, 0.0, 100.0, 3, ColorMode::ByType, 1).is_err());
    }
}
