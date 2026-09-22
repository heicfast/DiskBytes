//! Duplicate grouping and wasted-space ranking (spec §10; doc 02 §7).
//!
//! The three-pass SHA-256 pipeline is app-side (it needs `CreateFileW`,
//! rayon I/O and hardlink identity from open handles — see
//! `src-tauri/src/dupes.rs`); THIS module owns the pure data logic every
//! pass feeds into, so it stays unit-testable on any host:
//!
//! 1. [`group_by_size`] — pass 1 bucketing by logical size, dropping
//!    zero-length files (spec: "ignoring zero-length files").
//! 2. [`rank`] — pass 3 assembly: hardlink exclusion by
//!    (volume serial, file index) identity, grouping by (size, hash),
//!    wasted-space ranking `size × (count − 1)` largest first.
//!
//! The UI consumes [`DupeGroup`]s: "X could be reclaimed across N groups",
//! each group listing files with "Keep this, stage the rest" (the first
//! entry of each group is the natural "keep" anchor).

use std::collections::HashMap;

/// Minimum size a file must have to be a duplicate candidate (spec §10:
/// zero-length files are ignored — they can never waste space).
pub const MIN_CANDIDATE_SIZE: u64 = 1;

/// One file as fed back by the app-side hashing pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedFile {
    /// Display path (the staged reason text uses it verbatim).
    pub path: String,
    /// Logical size in bytes (`EndOfFile`).
    pub size: u64,
    /// Volume serial number from `GetFileInformationByHandle`.
    pub volume_serial: u64,
    /// File index from `GetFileInformationByHandle` (with the high part
    /// on FAT/exFAT already folded in by the caller).
    pub file_index: u64,
    /// Final SHA-256 digest (64 KB-prefix for small files, full contents
    /// otherwise — decided by the pipeline, opaque here).
    pub sha256: [u8; 32],
}

impl HashedFile {
    /// Hardlink identity: two entries sharing volume serial AND file index
    /// are the same physical file (spec: "hardlinks are not duplicates").
    #[must_use]
    fn hardlink_id(&self) -> (u64, u64) {
        (self.volume_serial, self.file_index)
    }
}

/// A finished duplicate group (spec §10 UI contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DupeGroup {
    /// Per-file logical size (identical across the group).
    pub size: u64,
    /// `size × (count − 1)` — what staging "the rest" would reclaim.
    pub wasted: u64,
    /// Member paths; the FIRST entry is the "keep" anchor the UI preselects.
    pub files: Vec<String>,
}

/// Pass 1 (spec §10): bucket candidate files by logical size.
///
/// Zero-length files are dropped; only buckets with ≥ 2 members survive
/// (a single file of one size is not a duplicate candidate). Order of
/// members inside a bucket follows input order.
#[must_use]
pub fn group_by_size(files: &[HashedFile]) -> Vec<Vec<&HashedFile>> {
    let mut by_size: HashMap<u64, Vec<&HashedFile>> = HashMap::new();
    for f in files {
        if f.size >= MIN_CANDIDATE_SIZE {
            by_size.entry(f.size).or_default().push(f);
        }
    }
    let mut buckets: Vec<Vec<&HashedFile>> = by_size.into_values().collect();
    buckets.retain(|b| b.len() >= 2);
    buckets
}

/// Pass 3 assembly: hardlink exclusion + (size, hash) grouping +
/// wasted-space ranking (spec §10).
///
/// - Hardlinks: within one size bucket, entries whose
///   `(volume_serial, file_index)` was already seen are dropped —
///   deduplication happens BEFORE grouping so a hardlink pair never
///   forms a group of "duplicates" with itself.
/// - A group survives only with ≥ 2 distinct physical files.
/// - Output is sorted by wasted space, largest first; ties break by
///   size (larger first) then by the first member's path so the order
///   is deterministic for tests and the UI.
#[must_use]
pub fn rank(files: &[HashedFile]) -> Vec<DupeGroup> {
    // Pass-1 size buckets (zero-length already dropped by group_by_size).
    let mut groups: Vec<DupeGroup> = Vec::new();
    for bucket in group_by_size(files) {
        // Hardlink exclusion inside the bucket.
        let mut seen_hardlinks = std::collections::HashSet::new();
        let mut members: Vec<&HashedFile> = Vec::with_capacity(bucket.len());
        for f in bucket {
            if seen_hardlinks.insert(f.hardlink_id()) {
                members.push(f);
            }
        }
        if members.len() < 2 {
            continue;
        }
        // (size, hash) sub-grouping.
        let mut by_hash: HashMap<[u8; 32], Vec<&HashedFile>> = HashMap::new();
        for f in members {
            by_hash.entry(f.sha256).or_default().push(f);
        }
        for hash_members in by_hash.into_values() {
            if hash_members.len() < 2 {
                continue;
            }
            let size = hash_members[0].size;
            let count = hash_members.len() as u64;
            groups.push(DupeGroup {
                size,
                wasted: size * (count - 1),
                files: hash_members.iter().map(|f| f.path.clone()).collect(),
            });
        }
    }
    groups.sort_by(|a, b| {
        b.wasted
            .cmp(&a.wasted)
            .then(b.size.cmp(&a.size))
            .then_with(|| a.files[0].cmp(&b.files[0]))
    });
    groups
}

/// Total reclaimable bytes and group count (header line
/// "X could be reclaimed across N groups").
#[must_use]
pub fn totals(groups: &[DupeGroup]) -> (u64, usize) {
    (groups.iter().map(|g| g.wasted).sum(), groups.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, size: u64, serial: u64, index: u64, seed: u8) -> HashedFile {
        HashedFile {
            path: path.to_string(),
            size,
            volume_serial: serial,
            file_index: index,
            sha256: [seed; 32],
        }
    }

    #[test]
    fn zero_length_and_singletons_are_not_candidates() {
        let files = vec![
            f("empty.txt", 0, 1, 1, 0xAA),
            f("only-one.bin", 100, 1, 2, 0xBB),
        ];
        assert!(rank(&files).is_empty());
        assert!(group_by_size(&files).is_empty());
    }

    #[test]
    fn hardlinks_are_not_duplicates() {
        // Two hardlinks: same (volume serial, file index), same hash.
        let files = vec![
            f("a/link1.dat", 500, 7, 42, 0xCC),
            f("b/link2.dat", 500, 7, 42, 0xCC),
        ];
        let groups = rank(&files);
        assert!(groups.is_empty(), "hardlink pair must not be reported");
    }

    #[test]
    fn same_size_different_hash_is_not_a_group() {
        let files = vec![
            f("a/x.dat", 500, 7, 42, 0x01),
            f("b/y.dat", 500, 7, 43, 0x02),
        ];
        assert!(rank(&files).is_empty());
    }

    #[test]
    fn wasted_space_math_and_ranking_order() {
        // Group A: 3 files × 100 → wasted 200.
        // Group B: 2 files × 900 → wasted 900 (must rank first).
        let files = vec![
            f("a1", 100, 1, 1, 0x10),
            f("a2", 100, 1, 2, 0x10),
            f("a3", 100, 1, 3, 0x10),
            f("b1", 900, 1, 4, 0x20),
            f("b2", 900, 1, 5, 0x20),
        ];
        let groups = rank(&files);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].wasted, 900);
        assert_eq!(groups[0].size, 900);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[1].wasted, 200);
        assert_eq!(groups[1].files.len(), 3);
        assert_eq!(totals(&groups), (1100, 2));
    }

    #[test]
    fn hardlink_exclusion_prunes_before_grouping() {
        // Three physical entries, two of them the same physical file.
        // One true duplicate remains + the hardlink clone is dropped.
        let files = vec![
            f("real1.doc", 50, 2, 10, 0x30),
            f("real2.doc", 50, 2, 11, 0x30),
            f("hard2.doc", 50, 2, 11, 0x30), // hardlink of real2
        ];
        let groups = rank(&files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files, vec!["real1.doc", "real2.doc"]);
        assert_eq!(groups[0].wasted, 50);
    }

    #[test]
    fn keep_anchor_is_first_member() {
        let files = vec![f("z-last", 10, 1, 1, 0x40), f("a-first", 10, 1, 2, 0x40)];
        let groups = rank(&files);
        // Input order preserved inside the group (a-first was fed second).
        assert_eq!(groups[0].files, vec!["z-last", "a-first"]);
    }
}
