//! Roll-up and child ordering (`BuildPrompt` §4).
//!
//! After a scan fills the arena, ONE reverse linear pass folds every node's
//! aggregates into its parent — `for i in (1..n).rev()` — which is sound
//! exactly because **a parent's id is always smaller than its children's**
//! (node.rs invariant). Then each folder's children are sorted
//! largest-first by on-disk size, in parallel across folders with rayon,
//! into a flat CSR-style `order: Vec<u32>` (folder holds offset+count).

use rayon::prelude::*;

use super::categories::FileCategory;
use super::node::{DirExtra, Node, Tree};

/// Fold aggregates up the tree, then build the sorted CSR `order`.
///
/// - Files contribute their own `logical`/`on_disk`, `modified` and
///   category bucket to the parent.
/// - Directories additionally contribute `file_count`, `folder_count`,
///   `type_sizes` and `max_descendant_modified`.
/// - After the pass every folder receives its dominant category.
///
/// Idempotent: safe to re-run after tree surgery (it recomputes from
/// current non-removed values — `roll_up` first zeroes the aggregate
/// fields it owns).
pub fn finalize(tree: &mut Tree) {
    let n = tree.arena.len();
    reset_aggregates(tree);
    roll_up(tree, n);
    assign_dominant_categories(tree);
    build_order(tree);
}

/// Zero the fields `roll_up` owns so a re-run after tree surgery does not
/// double-count. In this model directories carry no own bytes — their
/// `logical`/`on_disk` ARE the rolled-up sums — so both the [`DirExtra`]
/// aggregates AND every directory node's sizes reset to zero before the
/// reverse pass. File sizes stay untouched.
fn reset_aggregates(tree: &mut Tree) {
    for e in &mut tree.dir_extras {
        e.type_sizes = [0; 9];
        e.file_count = 0;
        e.folder_count = 0;
        e.max_descendant_modified = 0;
    }
    for n in &mut tree.arena {
        if n.is_dir() {
            n.logical = 0;
            n.on_disk = 0;
        }
    }
}

/// The single reverse pass (spec §4). Aggregate math is saturating so a
/// corrupt size can never wrap the totals.
fn roll_up(tree: &mut Tree, n: usize) {
    // Field-level split: arena and dir_extras are distinct fields.
    let arena: &mut Vec<Node> = &mut tree.arena;
    let extras: &mut Vec<DirExtra> = &mut tree.dir_extras;
    for i in (1..n).rev() {
        // Copy out to avoid holding a borrow while mutating the parent.
        let node = arena[i];
        if node.is_removed() {
            continue;
        }
        let parent = node.parent as usize;
        debug_assert!(
            parent < i,
            "parent id < child id invariant broken at node {i}"
        );
        let p = &mut arena[parent];
        p.logical = p.logical.saturating_add(node.logical);
        p.on_disk = p.on_disk.saturating_add(node.on_disk);
        if p.dir_index == u32::MAX {
            continue;
        }
        // Snapshot the child's folder aggregates BEFORE borrowing the
        // parent's DirExtra mutably (disjoint index access is invisible
        // to the borrow checker).
        let child_extra = if node.is_dir() && node.dir_index != u32::MAX {
            Some(extras[node.dir_index as usize])
        } else {
            None
        };
        let pe = &mut extras[p.dir_index as usize];
        pe.max_descendant_modified = pe.max_descendant_modified.max(node.modified);
        if let Some(ce) = child_extra {
            pe.folder_count += 1;
            pe.file_count += ce.file_count;
            pe.folder_count += ce.folder_count;
            for (t, v) in pe.type_sizes.iter_mut().zip(ce.type_sizes) {
                *t = t.saturating_add(v);
            }
            pe.max_descendant_modified = pe.max_descendant_modified.max(ce.max_descendant_modified);
        } else {
            pe.file_count += 1;
            let cat = node.category().as_bits() as usize;
            pe.type_sizes[cat] = pe.type_sizes[cat].saturating_add(node.on_disk);
        }
    }
    // The root folds into nothing; seed its own extras from itself.
    if arena[0].dir_index != u32::MAX {
        let r = &mut extras[arena[0].dir_index as usize];
        r.max_descendant_modified = r.max_descendant_modified.max(arena[0].modified);
    }
}

/// Folders take the category with the most bytes (spec §4 helpers).
fn assign_dominant_categories(tree: &mut Tree) {
    let dominant: Vec<u8> = tree
        .dir_extras
        .iter()
        .map(|e| {
            e.type_sizes
                .iter()
                .enumerate()
                .max_by_key(|&(idx, &sz)| (sz, std::cmp::Reverse(idx)))
                .map_or(FileCategory::Other.as_bits(), |(idx, _)| idx as u8)
        })
        .collect();
    for node in &mut tree.arena {
        if node.is_dir() && node.dir_index != u32::MAX {
            node.set_category(FileCategory::from_bits(dominant[node.dir_index as usize]));
        }
    }
}

/// Build the CSR `order`: per-folder children sorted largest-first by
/// on-disk size (live children only), parallel across folders (spec §4).
fn build_order(tree: &mut Tree) {
    // (first_child, child_count, dir_index) per folder with children.
    let folders: Vec<(u32, u32, u32)> = tree
        .arena
        .iter()
        .filter(|n| n.is_dir() && n.child_count > 0 && n.dir_index != u32::MAX)
        .map(|n| (n.first_child, n.child_count, n.dir_index))
        .collect();
    // Parallel across folders: each task sorts its own child list; the
    // results are flattened serially afterwards (each folder's slice is
    // disjoint, so this is race-free without unsafe shared mutation).
    let arena: &Vec<Node> = &tree.arena;
    let sorted_per_folder: Vec<Vec<u32>> = folders
        .par_iter()
        .map(|&(fc, cc, _di)| {
            let mut kids: Vec<u32> = (fc..fc + cc)
                .filter(|&id| !arena[id as usize].is_removed())
                .collect();
            kids.sort_unstable_by(|&a, &b| {
                arena[b as usize]
                    .on_disk
                    .cmp(&arena[a as usize].on_disk)
                    .then_with(|| a.cmp(&b))
            });
            kids
        })
        .collect();
    // Flatten + record (offset, count) per folder.
    tree.order.clear();
    tree.order
        .reserve(sorted_per_folder.iter().map(Vec::len).sum::<usize>());
    for (&(_, _, di), kids) in folders.iter().zip(sorted_per_folder.iter()) {
        let off = tree.order.len() as u32;
        tree.order.extend_from_slice(kids);
        let e = &mut tree.dir_extras[di as usize];
        e.order_offset = off;
        e.order_count = kids.len() as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::node::{BatchEntry, Node, Tree};

    fn file(name: &str, logical: u64, on_disk: u64, modified: i64) -> BatchEntry {
        let mut node = Node::new_file();
        node.logical = logical;
        node.on_disk = on_disk;
        node.modified = modified;
        let u16name: Vec<u16> = name.encode_utf16().collect();
        node.set_category(FileCategory::from_name(&u16name));
        BatchEntry {
            name: u16name,
            node,
        }
    }

    fn dir(name: &str, modified: i64) -> BatchEntry {
        let mut node = Node::new_dir();
        node.modified = modified;
        BatchEntry {
            name: name.encode_utf16().collect(),
            node,
        }
    }

    #[test]
    fn rollup_on_hand_built_tree() {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\Base");
        t.append_batch(0, vec![dir("A", 100), dir("B", 50)]);
        t.append_batch(
            1,
            vec![file("a1.bin", 100, 128, 700), file("a2.txt", 30, 32, 800)],
        );
        t.append_batch(2, vec![file("b1.mp4", 500, 512, 900)]);

        finalize(&mut t);

        let (logical, on_disk, files, folders) = t.root_stats();
        assert_eq!(logical, 630);
        assert_eq!(on_disk, 672);
        assert_eq!(files, 3);
        assert_eq!(folders, 2);

        let ea = &t.dir_extras[t.node(1).unwrap().dir_index as usize];
        assert_eq!(ea.file_count, 2);
        assert_eq!(ea.folder_count, 0);
        assert_eq!(ea.max_descendant_modified, 800);
        // .bin → Other, .txt → Document.
        assert_eq!(ea.type_sizes[FileCategory::Other.as_bits() as usize], 128);
        assert_eq!(ea.type_sizes[FileCategory::Document.as_bits() as usize], 32);

        let eb = &t.dir_extras[t.node(2).unwrap().dir_index as usize];
        assert_eq!(eb.type_sizes[FileCategory::Video.as_bits() as usize], 512);
        assert_eq!(eb.max_descendant_modified, 900);

        assert_eq!(t.dominant_category(1), FileCategory::Other);
        assert_eq!(t.dominant_category(2), FileCategory::Video);
        // Root dominant: Video (512) > Other (128).
        assert_eq!(t.dominant_category(0), FileCategory::Video);

        // Sorted children (by on_disk desc): B (512) before A (160).
        assert_eq!(t.children_sorted(0), &[2, 1]);
        assert_eq!(t.children_sorted(1), &[3, 4]);
        assert_eq!(t.children_sorted(2), &[5]);
    }

    #[test]
    fn rollup_is_idempotent() {
        let mut t = Tree::new(2);
        t.add_root_path(0, "C:\\");
        t.append_batch(0, vec![dir("A", 1)]);
        t.append_batch(1, vec![file("x.mp3", 10, 10, 5)]);
        finalize(&mut t);
        let first = t.root_stats();
        finalize(&mut t);
        assert_eq!(first, t.root_stats());
    }

    #[test]
    fn rollup_respects_removed_flags() {
        let mut t = Tree::new(3);
        t.add_root_path(0, "C:\\");
        t.append_batch(0, vec![file("keep", 10, 10, 5), file("gone", 90, 90, 5)]);
        t.arena[2].set_removed(true);
        finalize(&mut t);
        let (logical, on_disk, files, _) = t.root_stats();
        assert_eq!(logical, 10);
        assert_eq!(on_disk, 10);
        assert_eq!(files, 1);
        assert_eq!(t.children_sorted(0), &[1]);
    }
}
