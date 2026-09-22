//! Regroup (spec §7 color modes): throwaway wrapper trees of synthetic
//! groups. NEVER mutates the real tree.
//!
//! **By type**: every file under the folder is bucketed into one synthetic
//! group per category. **By age**: six bucket groups. Synthetic ids are
//! `0xFFFF_0000 + n` and are described in the layout response's metadata.

use crate::age;
use crate::layout::GroupTuple;
use crate::scan::categories::FileCategory;
use crate::scan::node::Tree;

/// First synthetic group id (spec §7).
pub const SYNTH_BASE: u32 = 0xFFFF_0000;

/// A regrouped dataset: synthetic groups with real member files.
#[derive(Debug, Clone)]
pub struct Regrouped {
    /// `(synthetic id, group name, group size, color, members)` —
    /// members are `(real node id, on-disk size)` sorted largest-first.
    pub groups: Vec<GroupTuple>,
    /// Scan generation (echoed into layouts).
    pub generation: u64,
    /// Total bytes across groups.
    pub total: u64,
}

/// Bucket files under `node` by category (By-type mode). Cloud
/// placeholders and removed nodes are excluded; members are truncated per
/// group to keep layouts within the cell budget.
#[must_use]
pub fn by_type(tree: &Tree, node: u32, member_cap: usize) -> Regrouped {
    let mut buckets: Vec<Vec<(u32, u64)>> = vec![Vec::new(); FileCategory::COUNT];
    tree.walk(node, |id, n| {
        if !n.is_dir() && !n.is_removed() && !n.is_cloud_placeholder() {
            let cat = n.category().as_bits() as usize;
            buckets[cat].push((id, n.on_disk));
        }
    });
    let mut groups: Vec<GroupTuple> = Vec::new();
    for (cat, bucket) in buckets.iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }
        let mut members = bucket.clone();
        members.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));
        members.truncate(member_cap);
        let size: u64 = members.iter().map(|m| m.1).sum();
        let fc = FileCategory::from_bits(cat as u8);
        groups.push((
            SYNTH_BASE + groups.len() as u32,
            fc.label().to_string(),
            size,
            fc.color(),
            members,
        ));
    }
    groups.sort_unstable_by_key(|g| std::cmp::Reverse(g.2));
    let total = groups.iter().map(|g| g.2).sum();
    Regrouped {
        groups,
        generation: tree.generation,
        total,
    }
}

/// Bucket files under `node` by age bucket (By-age mode, spec §7).
#[must_use]
pub fn by_age(tree: &Tree, node: u32, now: i64, member_cap: usize) -> Regrouped {
    let mut buckets: Vec<Vec<(u32, u64)>> = vec![Vec::new(); 6];
    tree.walk(node, |id, n| {
        if !n.is_dir() && !n.is_removed() && !n.is_cloud_placeholder() {
            let b = age::bucket_of(n.modified, now);
            buckets[b].push((id, n.on_disk));
        }
    });
    let mut groups: Vec<GroupTuple> = Vec::new();
    for (b, bucket) in buckets.iter().enumerate().take(6) {
        if bucket.is_empty() {
            continue;
        }
        let mut members = bucket.clone();
        members.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));
        members.truncate(member_cap);
        let size: u64 = members.iter().map(|m| m.1).sum();
        groups.push((
            SYNTH_BASE + groups.len() as u32,
            age::BUCKETS[b].to_string(),
            size,
            crate::layout::age_bucket_color(b),
            members,
        ));
    }
    // Keep chronological order (skip empty buckets in place).
    let total = groups.iter().map(|g| g.2).sum();
    Regrouped {
        groups,
        generation: tree.generation,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::treemap::treemap_groups;
    use crate::layout::ColorMode;
    use crate::scan::node::{BatchEntry, Node, Tree};
    use crate::scan::rollup;

    fn build() -> Tree {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\R");
        t.append_batch(
            0,
            vec![
                file("a.mp4", 100, 100, 1),
                file("b.mp3", 50, 50, 2),
                file("c.txt", 30, 30, 3),
                file("d.mp4", 20, 20, 4),
            ],
        );
        rollup::finalize(&mut t);
        t
    }

    fn file(name: &str, logical: u64, on_disk: u64, modified: i64) -> BatchEntry {
        let mut node = Node::new_file();
        node.logical = logical;
        node.on_disk = on_disk;
        node.modified = modified;
        // Files are classified at insertion (node.rs invariant) — the
        // category bits must be set or every file lands in Other.
        node.set_category(crate::scan::categories::FileCategory::from_name(
            &name.encode_utf16().collect::<Vec<u16>>(),
        ));
        BatchEntry {
            name: name.encode_utf16().collect(),
            node,
        }
    }

    #[test]
    fn by_type_buckets_and_sorts() {
        let t = build();
        let r = by_type(&t, 0, 100);
        assert_eq!(r.groups.len(), 3); // Video, Audio, Document
        assert_eq!(r.groups[0].1, "Video");
        assert_eq!(r.groups[0].2, 120);
        assert_eq!(r.groups[0].4.len(), 2);
        // Synthetic ids from the reserved base.
        assert_eq!(r.groups[0].0, SYNTH_BASE);
        assert_eq!(r.groups[1].0, SYNTH_BASE + 1);
        assert_eq!(r.total, 200);
        // Members sorted desc.
        assert!(r.groups[0].4[0].1 >= r.groups[0].4[1].1);
    }

    #[test]
    fn by_age_uses_buckets() {
        let t = build();
        let now = 1_800_000_000;
        let r = by_age(&t, 0, now, 100);
        // All files modified "now-ish" far in the past? No: modified 1..4
        // with now=1.8e9 → age ~57y → bucket 5 "Over 2 years".
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].1, "Over 2 years");
        assert_eq!(r.groups[0].2, 200);
    }

    #[test]
    fn regrouped_treemap_lays_out_groups() {
        let t = build();
        let r = by_type(&t, 0, 100);
        let buf =
            treemap_groups(&r.groups, 600.0, 400.0, r.generation, 0, ColorMode::ByType).unwrap();
        assert_eq!(buf.meta.groups.len(), 3);
        assert_eq!(buf.meta.mode, "treemap");
        // Group cells + file cells present.
        assert!(buf.cells.iter().any(|c| c.id == SYNTH_BASE));
        assert!(buf.cells.iter().any(|c| c.id == 1));
    }
}
