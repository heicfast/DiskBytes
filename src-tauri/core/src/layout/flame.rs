//! Flame (spec §7 mode 4): "Depth top to bottom, size left to right".
//!
//! Each row is a depth level; each block sits beneath its parent's span;
//! blocks under 1 px wide are skipped. Cell geometry is a rect
//! `[x, y, w, h]` with the row height `H / depth`.

use crate::error::CoreError;
use crate::layout::{
    check_geometry, node_color, pack_rgba, Cell, ColorMode, LayoutBuffer, LayoutMeta, MAX_CELLS,
};
use crate::scan::node::Tree;

/// Minimum block width in px (spec: "Skip blocks under 1px wide").
const MIN_W: f32 = 1.0;
/// Horizontal gap between sibling blocks.
const GAP_X: f32 = 0.5;

/// Layout the subtree under `node` as a flame/icicle chart.
///
/// # Errors
/// - [`CoreError::InvalidGeometry`] when `width`/`height` are zero.
/// - [`CoreError::NodeNotFound`] when `node` is not in the arena.
#[allow(clippy::too_many_arguments)]
pub fn flame(
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
    let row_h = if depth > 0 {
        height / depth as f32
    } else {
        height
    };
    let mut cells: Vec<Cell> = Vec::with_capacity(512);
    let mut truncated = false;
    // The root block spans the full width on row 0 when depth >= 1.
    if depth > 0 && total > 0 {
        cells.push(Cell::rect(
            node,
            0,
            pack_rgba(0x8E8E93),
            0.0,
            0.0,
            width,
            row_h,
        ));
        layout_row(
            tree,
            node,
            0.0,
            width,
            1,
            depth,
            row_h,
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
            mode: "flame".into(),
            generation: tree.generation,
            node,
            width,
            height,
            depth,
            color_mode: color,
            cell_count: 0,
            truncated,
            center: None,
            groups: Vec::new(),
            total_bytes: total,
        },
    })
}

/// Recursive row layout: children of `node` inside x-span `(x0..x1)` on
/// row `depth_here`, each beneath its parent's span.
#[allow(clippy::too_many_arguments)]
fn layout_row(
    tree: &Tree,
    node: u32,
    x0: f32,
    x1: f32,
    depth_here: u32,
    depth_left: u32,
    row_h: f32,
    color: ColorMode,
    now: i64,
    cells: &mut Vec<Cell>,
    truncated: &mut bool,
    top_index: usize,
) {
    if depth_left == 0 {
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
    let y = (depth_here - 1) as f32 * row_h;
    let span = x1 - x0;
    let n_gaps = children.len().saturating_sub(1) as f32;
    let usable = (span - GAP_X * n_gaps).max(0.0);
    let mut cursor = x0;
    for (i, &id) in children.iter().enumerate() {
        if cells.len() >= MAX_CELLS {
            *truncated = true;
            return;
        }
        let c = tree.node(id).expect("child id");
        if c.is_removed() || c.on_disk == 0 {
            continue;
        }
        let w = c.on_disk as f32 / total as f32 * usable;
        if w < MIN_W {
            continue; // Skip sub-1px blocks.
        }
        let rgba = pack_rgba(match color {
            ColorMode::ByFolder => node_color(tree, id, color, now, i, depth_here as u16, i),
            ColorMode::ByType => c.category().color(),
            ColorMode::ByAge => node_color(tree, id, color, now, 0, 0, i),
        });
        cells.push(Cell::rect(id, depth_here as u16, rgba, cursor, y, w, row_h));
        if c.is_dir() && c.child_count > 0 && depth_left > 1 {
            layout_row(
                tree,
                id,
                cursor,
                cursor + w,
                depth_here + 1,
                depth_left - 1,
                row_h,
                color,
                now,
                cells,
                truncated,
                if depth_here == 1 { i } else { top_index },
            );
        }
        cursor += w + GAP_X;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::node::{BatchEntry, Node, Tree};
    use crate::scan::rollup;

    fn build() -> Tree {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\F");
        t.append_batch(
            0,
            vec![
                dir("a"),
                file("f1.bin", 100, 100, 1),
                file("f2.bin", 50, 50, 1),
            ],
        );
        t.append_batch(1, vec![file("a1", 60, 60, 1), file("a2", 30, 30, 1)]);
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
    fn blocks_align_under_parents() {
        let t = build();
        let buf = flame(&t, 0, 1000.0, 300.0, 3, ColorMode::ByType, 1).unwrap();
        let row1: Vec<&Cell> = buf.cells.iter().filter(|c| c.depth == 1).collect();
        assert_eq!(row1.len(), 3);
        // Sizes: a=90, f1=100, f2=50 → widths 375, 416.67, 208.3.
        let wa = row1.iter().find(|c| c.id == 1).unwrap().g[2];
        let wf1 = row1.iter().find(|c| c.id == 2).unwrap().g[2];
        assert!((wa / wf1 - 0.9).abs() < 0.01);
        // Children of `a` sit within a's x-span on row 2.
        let a_x = row1.iter().find(|c| c.id == 1).unwrap().g[0];
        let a_w = row1.iter().find(|c| c.id == 1).unwrap().g[2];
        let row2: Vec<&Cell> = buf.cells.iter().filter(|c| c.depth == 2).collect();
        for c in row2 {
            assert!(c.g[0] >= a_x - 0.5 && c.g[0] + c.g[2] <= a_x + a_w + 0.5);
            assert!((c.g[1] - 100.0).abs() < 0.5); // row 2 y
        }
        // No sub-1px blocks.
        assert!(buf.cells.iter().all(|c| c.g[2] >= MIN_W - f32::EPSILON));
    }
}
