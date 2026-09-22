//! Mind Map (spec §7 mode 6): "Branches from the root, sized by weight".
//!
//! A radial tree with curved links (JS draws a quadratic curve from each
//! dot to its parent dot position, stored in the cell); dot area ∝ share
//! of the folder. Labels are placed JS-side, biggest-first, skipping
//! collisions.

use crate::error::CoreError;
use crate::layout::{
    check_geometry, node_color, pack_rgba, Cell, ColorMode, LayoutBuffer, LayoutMeta, MAX_CELLS,
};
use crate::scan::node::Tree;

/// Minimum dot radius.
const MIN_R: f32 = 1.5;
/// Base dot radius at the root's children (scales with viewport).
const DOT_BASE: f32 = 26.0;

/// Layout the subtree under `node` as a radial mind map.
///
/// # Errors
/// - [`CoreError::InvalidGeometry`] when `width`/`height` are zero.
/// - [`CoreError::NodeNotFound`] when `node` is not in the arena.
#[allow(clippy::too_many_arguments)]
pub fn mindmap(
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
    let r_max = width.min(height) / 2.0 - 6.0;
    let mut cells: Vec<Cell> = Vec::with_capacity(512);
    let mut truncated = false;
    // Root dot.
    cells.push(Cell::dot(
        node,
        0,
        pack_rgba(0x8E8E93),
        cx,
        cy,
        12.0,
        cx,
        cy,
    ));
    if depth > 0 && total > 0 {
        layout_branches(
            tree,
            node,
            cx,
            cy,
            r_max,
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
            mode: "mindmap".into(),
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

/// Place the children of `node` on the ring at radius `ring_r`, angular
/// spans ∝ weights, then recurse within each span.
#[allow(clippy::too_many_arguments)]
fn layout_branches(
    tree: &Tree,
    node: u32,
    cx: f32,
    cy: f32,
    ring_r: f32,
    depth_here: u32,
    depth_left: u32,
    color: ColorMode,
    now: i64,
    cells: &mut Vec<Cell>,
    truncated: &mut bool,
    top_index: usize,
) {
    if depth_left == 0 || ring_r <= 4.0 {
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
    let step_r = ring_r / depth_left as f32; // per-level radius step
    let level_r = ring_r - step_r * (depth_left as f32 - 1.0);
    let mut cursor = -std::f32::consts::FRAC_PI_2; // start at 12 o'clock
    for (i, &id) in children.iter().enumerate() {
        if cells.len() >= MAX_CELLS {
            *truncated = true;
            return;
        }
        let c = tree.node(id).expect("child id");
        if c.is_removed() || c.on_disk == 0 {
            continue;
        }
        let span = c.on_disk as f32 / total as f32 * std::f32::consts::TAU;
        let mid = cursor + span / 2.0;
        let x = cx + level_r * mid.cos();
        let y = cy + level_r * mid.sin();
        // Dot radius ∝ sqrt(share of parent) — area ∝ bytes share.
        let share = c.on_disk as f32 / total as f32;
        let r = (DOT_BASE * share.sqrt()).max(MIN_R);
        let rgba = pack_rgba(match color {
            ColorMode::ByFolder => node_color(tree, id, color, now, i, depth_here as u16, i),
            ColorMode::ByType => c.category().color(),
            ColorMode::ByAge => node_color(tree, id, color, now, 0, 0, i),
        });
        cells.push(Cell::dot(id, depth_here as u16, rgba, x, y, r, cx, cy));
        if c.is_dir() && c.child_count > 0 && depth_left > 1 {
            layout_branches(
                tree,
                id,
                x,
                y,
                step_r,
                depth_here + 1,
                depth_left - 1,
                color,
                now,
                cells,
                truncated,
                if depth_here == 1 { i } else { top_index },
            );
        }
        cursor += span;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::node::{BatchEntry, Node, Tree};
    use crate::scan::rollup;

    fn build() -> Tree {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\M");
        t.append_batch(0, vec![dir("a"), dir("b")]);
        t.append_batch(1, vec![file("a1", 75, 75, 1), file("a2", 25, 25, 1)]);
        t.append_batch(2, vec![file("b1", 30, 30, 1)]);
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
    fn dots_ring_around_root_with_weight_spans() {
        let t = build();
        let buf = mindmap(&t, 0, 900.0, 700.0, 3, ColorMode::ByType, 1).unwrap();
        let l1: Vec<&Cell> = buf.cells.iter().filter(|c| c.depth == 1).collect();
        assert_eq!(l1.len(), 2);
        // a=100, b=30 → angular span ratio 100/30.
        let (a, b) = (l1[0], l1[1]);
        // Positions radiate from center; parent link points at the center.
        for c in &l1 {
            let d = ((c.g[0] - 450.0).powi(2) + (c.g[1] - 350.0).powi(2)).sqrt();
            assert!(d > 40.0, "level-1 dot should sit off-center");
            assert!(
                (c.g[3] - 450.0).abs() < 0.01 && (c.g[4] - 350.0).abs() < 0.01,
                "dot parent link should point at the root center"
            );
        }
        // Dot radii ∝ sqrt(share): share a=0.769, b=0.231.
        let ra = a.g[2] / DOT_BASE;
        let rb = b.g[2] / DOT_BASE;
        assert!((ra * ra - 100.0 / 130.0).abs() < 0.01);
        assert!((rb * rb - 30.0 / 130.0).abs() < 0.01);
    }

    #[test]
    fn invalid_geometry_rejected() {
        let t = build();
        assert!(mindmap(&t, 0, 10.0, 0.0, 2, ColorMode::ByAge, 1).is_err());
    }
}
