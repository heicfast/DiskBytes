//! The `Node` arena and `Tree` (`BuildPrompt` §4 — memory matters: 1M+ nodes).
//!
//! Invariants (each load-bearing, referenced where enforced):
//! - `Node` is ≤ 72 bytes (static assertion below; spec §4).
//! - **A parent's id is always smaller than every one of its children's ids**
//!   (guaranteed by the batch-append protocol and by BFS id assignment in
//!   the Turbo engine; the reverse roll-up depends on it).
//! - Children of one parent occupy a contiguous id range
//!   `[first_child, first_child + child_count)` — one directory = exactly
//!   one batch append. Subtrees of different children interleave freely,
//!   which is why `walk` uses an explicit stack, never id-range arithmetic.
//! - Names live in one shared `Vec<u16>`; `String` only exists at the
//!   display boundary. Paths are rebuilt on demand, never stored per node.

use super::categories::FileCategory;

/// Bit positions inside `Node::flags` (u16).
mod flags {
    /// Directory bit.
    pub const IS_DIR: u16 = 1 << 0;
    /// 4-bit category field, bits 1..=4.
    pub const CATEGORY_MASK: u16 = 0b1111 << 1;
    /// `OneDrive` / cloud placeholder bit.
    pub const CLOUD_PLACEHOLDER: u16 = 1 << 5;
    /// Windows-managed (never stageable) bit.
    pub const PROTECTED: u16 = 1 << 6;
    /// Removed by tree surgery bit.
    pub const REMOVED: u16 = 1 << 7;
    /// Inside an apps root bit.
    pub const IN_APPS: u16 = 1 << 8;
}

/// One file or folder in the arena. Field-for-field per spec §4.
///
/// Sizes: `logical` = `EndOfFile` (MFT-resident tiny files may report
/// `on_disk == 0` — stored as-is); `on_disk` = `AllocationSize` /
/// allocated size. `modified`/`created` are Unix seconds (0 = unknown).
// No `#[repr(C)]`: Rust field reordering packs this to exactly 56 bytes
// (u64/i64 fields first). The repr(C) 32-byte layout contract in the spec
// applies to the layout CELL buffer, not to Node (spec §4 lists fields + the
// ≤72-byte budget only). Node never crosses a raw-memory boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    /// Element offset (u16 units) into the shared names arena.
    pub name_off: u32,
    /// Name length in UTF-16 code units (not bytes).
    pub name_len: u16,
    /// `is_dir` | category (4 bits) | `cloud_placeholder` | protected |
    /// removed | `in_apps`.
    pub flags: u16,
    /// Parent node id; `u32::MAX` for the root.
    pub parent: u32,
    /// First child id (children are contiguous from here).
    pub first_child: u32,
    /// Number of direct children.
    pub child_count: u32,
    /// Logical size (`EndOfFile`) in bytes.
    pub logical: u64,
    /// Size on disk (allocated) in bytes.
    pub on_disk: u64,
    /// Last-write time, Unix seconds (0 = unknown).
    pub modified: i64,
    /// Creation time, Unix seconds (0 = unknown).
    pub created: i64,
    /// Index into `Tree::dir_extras` when `is_dir`; `u32::MAX` for files.
    pub dir_index: u32,
}

const _: () = assert!(size_of::<Node>() <= 72, "spec §4: Node must fit 72 bytes");
const _: () = assert!(size_of::<Node>() == 56);

impl Node {
    /// A file node with no arena placement yet — engines fill the rest.
    #[must_use]
    pub fn new_file() -> Self {
        Self::with_flags(0)
    }

    /// A directory node with no arena placement yet.
    #[must_use]
    pub fn new_dir() -> Self {
        Self::with_flags(flags::IS_DIR)
    }

    fn with_flags(f: u16) -> Self {
        Self {
            name_off: 0,
            name_len: 0,
            flags: f,
            parent: u32::MAX,
            first_child: 0,
            child_count: 0,
            logical: 0,
            on_disk: 0,
            modified: 0,
            created: 0,
            dir_index: u32::MAX,
        }
    }

    /// True when this node represents a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.flags & flags::IS_DIR != 0
    }

    /// `OneDrive` / cloud-files placeholder (never opened, previewed or hashed).
    #[must_use]
    pub fn is_cloud_placeholder(&self) -> bool {
        self.flags & flags::CLOUD_PLACEHOLDER != 0
    }

    /// Windows-managed item — never stageable for cleanup (spec §4).
    #[must_use]
    pub fn is_protected(&self) -> bool {
        self.flags & flags::PROTECTED != 0
    }

    /// Marked removed by post-recycle tree surgery (spec §9).
    #[must_use]
    pub fn is_removed(&self) -> bool {
        self.flags & flags::REMOVED != 0
    }

    /// Inside an apps root (`Program Files`, `%LOCALAPPDATA%\Programs`, …).
    #[must_use]
    pub fn in_apps_root(&self) -> bool {
        self.flags & flags::IN_APPS != 0
    }

    /// The file category bucket (0 = Other until classified).
    #[must_use]
    pub fn category(&self) -> FileCategory {
        FileCategory::from_bits(((self.flags & flags::CATEGORY_MASK) >> 1) as u8)
    }

    /// Set the category bits (folders get their dominant category after
    /// roll-up; files are classified at insertion).
    pub fn set_category(&mut self, cat: FileCategory) {
        self.flags = (self.flags & !flags::CATEGORY_MASK) | (u16::from(cat.as_bits()) << 1);
    }

    /// Set the cloud-placeholder bit.
    pub fn set_cloud_placeholder(&mut self, on: bool) {
        set_flag(&mut self.flags, flags::CLOUD_PLACEHOLDER, on);
    }

    /// Set the protected bit.
    pub fn set_protected(&mut self, on: bool) {
        set_flag(&mut self.flags, flags::PROTECTED, on);
    }

    /// Set the removed bit (tree surgery only).
    pub fn set_removed(&mut self, on: bool) {
        set_flag(&mut self.flags, flags::REMOVED, on);
    }

    /// Set the in-apps-root bit.
    pub fn set_in_apps_root(&mut self, on: bool) {
        set_flag(&mut self.flags, flags::IN_APPS, on);
    }
}

/// The separator a root path's family uses (windows roots carry `\\`,
/// POSIX roots `/`); inference keeps ONE core for both platforms.
fn root_sep(root: &str) -> char {
    if root.contains('\\') {
        '\\'
    } else {
        '/'
    }
}

fn set_flag(flags: &mut u16, bit: u16, on: bool) {
    if on {
        *flags |= bit;
    } else {
        *flags &= !bit;
    }
}

/// Per-folder extras (spec §4): files don't pay for these.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirExtra {
    /// Bytes on disk per [`FileCategory`] (9 slots).
    pub type_sizes: [u64; 9],
    /// Total descendant file count.
    pub file_count: u64,
    /// Total descendant folder count.
    pub folder_count: u64,
    /// Most recent descendant modification (Unix seconds).
    pub max_descendant_modified: i64,
    /// Offset into `Tree::order` (CSR children-sorted-largest-first).
    pub order_offset: u32,
    /// Number of children in the `order` slice.
    pub order_count: u32,
}

/// A scan root with its on-disk path (display form, no `\\?\` prefix).
#[derive(Debug, Clone)]
pub struct RootRef {
    /// Node id of the root.
    pub node: u32,
    /// Display path, e.g. `C:\` or `D:\Work` (no verbatim prefix).
    pub path: String,
}

/// Entry fed by engines for one directory's children (the single-lock batch
/// append protocol; ids become a contiguous range).
#[derive(Debug, Clone)]
pub struct BatchEntry {
    /// Child name in UTF-16 (moved into the shared names arena).
    pub name: Vec<u16>,
    /// Pre-built node (`parent/first_child` fixed by the tree).
    pub node: Node,
}

/// The finished scan tree. Wrapped in `Arc` behind an `RwLock` by the app;
/// dropped on a background thread when replaced (spec §4).
#[derive(Debug, Default)]
pub struct Tree {
    /// Node arena; ids are indices.
    pub arena: Vec<Node>,
    /// Shared UTF-16 name storage (exact round-trip paths).
    pub names: Vec<u16>,
    /// Side table for folders, indexed by `Node::dir_index`.
    pub dir_extras: Vec<DirExtra>,
    /// CSR order: per-folder children sorted largest-first by on-disk size.
    pub order: Vec<u32>,
    /// Scan roots carrying a real path (folder scan: one; This PC: one per
    /// drive). The synthetic This-PC node has none.
    pub roots: Vec<RootRef>,
    /// Root node id (0 for both real roots and the This-PC synthetic root).
    pub root: u32,
    /// Monotonic scan generation; IPC callers tag requests with it.
    pub generation: u64,
    /// Display label for the synthetic This PC root.
    pub this_pc_label: Option<String>,
}

impl Tree {
    /// Deep copy (the surgery copy-on-write path: a blocking reader still shares
    /// old snapshot, so the commit builds its own tree). Rare — commits
    /// are user-rare and this runs off the UI thread.
    #[must_use]
    pub fn deep_from(other: &Self) -> Self {
        Self {
            arena: other.arena.clone(),
            names: other.names.clone(),
            dir_extras: other.dir_extras.clone(),
            order: other.order.clone(),
            roots: other.roots.clone(),
            root: other.root,
            generation: other.generation,
            this_pc_label: other.this_pc_label.clone(),
        }
    }

    /// Empty tree at `generation` with the root directory node at id 0.
    #[must_use]
    pub fn new(generation: u64) -> Self {
        let mut root = Node::new_dir();
        root.parent = u32::MAX;
        root.dir_index = 0;
        Self {
            arena: vec![root],
            names: Vec::new(),
            dir_extras: vec![DirExtra::default()],
            order: Vec::new(),
            roots: Vec::new(),
            root: 0,
            generation,
            this_pc_label: None,
        }
    }

    /// Number of nodes in the arena (progress UI).
    #[must_use]
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// True when the arena holds nothing but the root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arena.len() <= 1
    }

    /// Borrow a node; `None` when out of range (command layer maps this to
    /// `CoreError::NodeNotFound`).
    #[must_use]
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.arena.get(id as usize)
    }

    /// The node's UTF-16 name slice (zero-copy into the arena).
    #[must_use]
    pub fn name_u16(&self, id: u32) -> &[u16] {
        let n = &self.arena[id as usize];
        &self.names[n.name_off as usize..(n.name_off + u32::from(n.name_len)) as usize]
    }

    /// The node's name as an owned `String` (display boundary only).
    #[must_use]
    pub fn name(&self, id: u32) -> String {
        String::from_utf16_lossy(self.name_u16(id))
    }

    /// Set (or replace) a node's display name by appending to the name
    /// arena and repointing the node — safe at any time, and the ONLY
    /// way to name a root node (roots carry their on-disk path via
    /// `add_root_path`, but the arena name drives every display surface:
    /// Folders header, breadcrumb crumb, inspector title).
    ///
    /// The previous slice (if any) becomes unreachable dead bytes; in
    /// production roots are named once before children exist, so there
    /// is no waste. `name_len` is capped at `u16::MAX` by truncation.
    pub fn set_name(&mut self, id: u32, name: &str) {
        let encoded: Vec<u16> = name.encode_utf16().collect();
        let off = self.names.len() as u32;
        self.names.extend_from_slice(&encoded);
        let n = &mut self.arena[id as usize];
        n.name_off = off;
        n.name_len = u16::try_from(encoded.len()).unwrap_or(u16::MAX);
    }

    /// Names of many ids in one batch (the `get_names(ids)` IPC contract).
    ///
    /// Out-of-arena ids — synthetic regroup ids (`SYNTH_BASE + n`, the
    /// by-type/by-age group cells) or anything stale — resolve to an
    /// empty string. This is a bulk display path fed straight from
    /// layout cells; it must NEVER panic the process (CI run 35707319531
    /// crashed exactly here when a by-type layout's group ids reached
    /// `name()`'s direct arena index).
    #[must_use]
    pub fn names_batch(&self, ids: &[u32]) -> Vec<String> {
        ids.iter()
            .map(|&id| {
                if usize::try_from(id).is_ok_and(|i| i < self.arena.len()) {
                    self.name(id)
                } else {
                    String::new()
                }
            })
            .collect()
    }

    /// Append one directory's children atomically; returns the contiguous
    /// id range base and sets `parent`/`first_child`/`child_count`.
    ///
    /// Invariants kept here:
    /// - parent id < every child id (caller appends parents first — engines
    ///   enumerate top-down);
    /// - each directory is appended exactly ONCE, so its children form a
    ///   single contiguous range (debug-asserted below).
    pub fn append_batch(&mut self, parent: u32, entries: Vec<BatchEntry>) -> u32 {
        let base = self.arena.len() as u32;
        debug_assert!(
            self.arena[parent as usize].is_dir(),
            "append_batch into a non-directory"
        );
        {
            let pn = &mut self.arena[parent as usize];
            if pn.child_count == 0 {
                pn.first_child = base;
            } else {
                // A second batch for the same parent would break child
                // contiguity — engines must never do this.
                debug_assert_eq!(
                    pn.first_child + pn.child_count,
                    base,
                    "children of one parent must stay contiguous"
                );
            }
            pn.child_count += entries.len() as u32;
        }
        for e in entries {
            let mut n = e.node;
            let name_off = self.names.len() as u32;
            self.names.extend_from_slice(&e.name);
            n.name_off = name_off;
            n.name_len = e.name.len() as u16;
            n.parent = parent;
            if n.is_dir() {
                n.dir_index = self.dir_extras.len() as u32;
                self.dir_extras.push(DirExtra::default());
            }
            self.arena.push(n);
        }
        base
    }

    /// Attach the on-disk path to a root node (folder scan: root; This PC:
    /// each drive node).
    pub fn add_root_path(&mut self, node: u32, path: &str) {
        self.roots.push(RootRef {
            node,
            path: path.to_string(),
        });
    }

    /// Convert a display path to its verbatim engine form: `\\?\` on
    /// Windows, identity on macOS/POSIX (cross-platform doc §3 — the
    /// mac engine takes plain paths).
    #[must_use]
    pub fn verbatim(path: &str) -> String {
        if path.starts_with(r"\\?\") {
            return path.to_string();
        }
        if cfg!(windows) {
            format!(r"\\?\{path}")
        } else {
            path.to_string()
        }
    }

    /// Rebuild a node's display path on demand (spec §4: walk up to the
    /// nearest node carrying a root path; append the names below it).
    ///
    /// The synthetic This-PC root itself yields `"This PC"`.
    #[must_use]
    pub fn node_path(&self, id: u32) -> String {
        // Collect the chain from `id` up to (and including) a RootRef node.
        let mut chain: Vec<u32> = Vec::with_capacity(16);
        let mut cur = id;
        let mut root_path: Option<&str> = None;
        loop {
            chain.push(cur);
            if let Some(r) = self.roots.iter().find(|r| r.node == cur) {
                root_path = Some(r.path.as_str());
                break;
            }
            let p = self.arena[cur as usize].parent;
            if p == u32::MAX {
                break;
            }
            cur = p;
        }
        match root_path {
            Some(rp) => {
                let sep = root_sep(rp);
                let mut out = String::from(rp);
                // Names below the root (chain reversed, root excluded); the
                // separator is added only when a name actually follows (the
                // root itself must not gain a trailing separator).
                for &nid in chain.iter().rev().skip(1) {
                    if !(out.ends_with('\\') || out.ends_with('/')) {
                        out.push(sep);
                    }
                    out.push_str(&self.name(nid));
                }
                out
            }
            None if chain.len() == 1 => self
                .this_pc_label
                .clone()
                .unwrap_or_else(|| self.name(chain[0])),
            None => {
                // Defensive: a non-synthetic chain should always terminate at
                // a RootRef. Join names rather than inventing a path.
                let sep = root_sep(&self.name(chain[0]));
                let mut out = String::new();
                for &nid in chain.iter().rev() {
                    out.push_str(&self.name(nid));
                    out.push(sep);
                }
                out.pop();
                out
            }
        }
    }

    /// Children ids of `id` sorted largest-first by on-disk size (the CSR
    /// slice — every ranked view consumes this). Valid after `rollup::finalize`.
    #[must_use]
    pub fn children_sorted(&self, id: u32) -> &[u32] {
        let n = &self.arena[id as usize];
        if n.child_count == 0 || n.dir_index == u32::MAX {
            return &[];
        }
        let e = &self.dir_extras[n.dir_index as usize];
        let s = e.order_offset as usize;
        &self.order[s..s + e.order_count as usize]
    }

    /// Share of the parent's on-disk size (1.0 for the root, 0.0 unknown).
    #[must_use]
    pub fn share_of_parent(&self, id: u32) -> f64 {
        let n = &self.arena[id as usize];
        if n.parent == u32::MAX {
            return 1.0;
        }
        let p = &self.arena[n.parent as usize];
        if p.on_disk == 0 {
            return 0.0;
        }
        n.on_disk as f64 / p.on_disk as f64
    }

    /// `logical − on_disk` when positive (compressed/sparse savings).
    #[must_use]
    pub fn compression_savings(&self, id: u32) -> u64 {
        let n = &self.arena[id as usize];
        n.logical.saturating_sub(n.on_disk)
    }

    /// `on_disk − logical` when positive (cluster overhead).
    #[must_use]
    pub fn cluster_overhead(&self, id: u32) -> u64 {
        let n = &self.arena[id as usize];
        n.on_disk.saturating_sub(n.logical)
    }

    /// The category with the most bytes (folders: dominant by rolled-up
    /// type sizes after roll-up).
    #[must_use]
    pub fn dominant_category(&self, id: u32) -> FileCategory {
        self.arena[id as usize].category()
    }

    /// Top-N (category, bytes) pairs for the folder, largest first.
    #[must_use]
    pub fn top_categories(&self, id: u32, n: usize) -> Vec<(FileCategory, u64)> {
        let node = &self.arena[id as usize];
        let mut out: Vec<(FileCategory, u64)> = Vec::with_capacity(9);
        if node.is_dir() && node.dir_index != u32::MAX {
            let e = &self.dir_extras[node.dir_index as usize];
            for (i, &sz) in e.type_sizes.iter().enumerate() {
                if sz > 0 {
                    out.push((FileCategory::from_bits(i as u8), sz));
                }
            }
        } else if !node.is_dir() {
            out.push((node.category(), node.on_disk));
        }
        out.sort_unstable_by_key(|p| std::cmp::Reverse(p.1));
        out.truncate(n);
        out
    }

    /// True when `desc` is `anc` or lies below it. Parent ids always shrink,
    /// so the walk terminates at the first id ≤ `anc`.
    #[must_use]
    pub fn is_descendant_of(&self, desc: u32, anc: u32) -> bool {
        let mut cur = desc;
        while cur > anc {
            let p = self.arena[cur as usize].parent;
            if p == u32::MAX {
                return false;
            }
            cur = p;
        }
        cur == anc
    }

    /// Depth-first traversal of the live (non-removed) subtree under `id`,
    /// excluding `id` itself. Stack-based: subtrees of sibling children
    /// interleave in the arena, so id-range arithmetic is NOT sound here.
    pub fn walk<F: FnMut(u32, &Node)>(&self, id: u32, mut f: F) {
        let first = self.arena[id as usize].first_child;
        let count = self.arena[id as usize].child_count;
        if count == 0 {
            return;
        }
        let mut stack: Vec<u32> = (first..first + count).rev().collect();
        while let Some(nid) = stack.pop() {
            let n = &self.arena[nid as usize];
            if n.is_removed() {
                continue;
            }
            f(nid, n);
            if n.is_dir() && n.child_count > 0 {
                let fc = n.first_child;
                stack.extend((fc..fc + n.child_count).rev());
            }
        }
    }

    /// All live file ids under `id` (duplicates / quick-wins inputs).
    #[must_use]
    pub fn all_files(&self, id: u32) -> Vec<u32> {
        let mut out = Vec::new();
        self.walk(id, |nid, n| {
            if !n.is_dir() {
                out.push(nid);
            }
        });
        out
    }

    /// Root stats snapshot `(logical, on_disk, files, folders)` for the
    /// sidebar header.
    #[must_use]
    pub fn root_stats(&self) -> (u64, u64, u64, u64) {
        let r = &self.arena[self.root as usize];
        if r.dir_index == u32::MAX {
            (r.logical, r.on_disk, 0, 0)
        } else {
            let e = &self.dir_extras[r.dir_index as usize];
            (r.logical, r.on_disk, e.file_count, e.folder_count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, logical: u64, on_disk: u64, modified: i64) -> BatchEntry {
        let mut node = Node::new_file();
        node.logical = logical;
        node.on_disk = on_disk;
        node.modified = modified;
        node.created = modified;
        BatchEntry {
            name: name.encode_utf16().collect(),
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
    fn node_layout_is_56_bytes() {
        assert_eq!(size_of::<Node>(), 56);
        assert!(size_of::<Node>() <= 72);
    }

    #[test]
    fn names_batch_never_panics_on_synthetic_or_stale_ids() {
        // Regression (CI run 35707319531): by-type/by-age layouts emit
        // synthetic group ids (SYNTH_BASE + n); `get_names` fed them
        // straight into `name()`'s direct arena index and PANICKED the
        // whole app mid-tour. The bulk path must resolve them (and any
        // stale id) to empty strings instead.
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\Base");
        t.set_name(0, "Base");
        t.append_batch(0, vec![file("a.bin", 10, 10, 1)]);
        let out = t.names_batch(&[0, 1, crate::layout::regroup::SYNTH_BASE + 3, u32::MAX]);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], "Base");
        assert_eq!(out[1], "a.bin");
        assert_eq!(out[2], "", "synthetic group id resolves to empty");
        assert_eq!(out[3], "", "far-out-of-range id resolves to empty");
    }

    #[test]
    fn batch_append_contiguous_and_ordered() {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\Base");
        let base = t.append_batch(
            0,
            vec![
                dir("Alpha", 100),
                file("a.txt", 10, 12, 500),
                dir("Beta", 200),
            ],
        );
        assert_eq!(base, 1);
        let root = t.node(0).unwrap();
        assert_eq!(root.first_child, 1);
        assert_eq!(root.child_count, 3);
        let a = t.append_batch(1, vec![file("x.bin", 100, 128, 700)]);
        assert_eq!(a, 4);
        assert_eq!(t.node(1).unwrap().first_child, 4);
        assert_eq!(t.node(1).unwrap().child_count, 1);
        assert_eq!(t.name(4), "x.bin");
        assert_eq!(t.node(4).unwrap().parent, 1);
        // The append invariant: root(0) < Alpha(1) < x.bin(4) — parent
        // ids are always smaller than their children's ids.
        assert!(t.node(4).unwrap().parent < 4);
    }

    #[test]
    fn interleaved_subtrees_still_walk_correctly() {
        // Simulate worker interleaving: children of A appended AFTER a
        // sibling B's subtree — subtree contiguity must not be assumed.
        let mut t = Tree::new(9);
        t.add_root_path(0, "C:\\");
        t.append_batch(0, vec![dir("A", 1), dir("B", 1)]); // ids 1, 2
        t.append_batch(2, vec![file("b1", 1, 1, 1), file("b2", 1, 1, 1)]); // 3, 4
        t.append_batch(1, vec![file("a1", 1, 1, 1)]); // id 5 — NOT contiguous with 1
        let mut seen = Vec::new();
        t.walk(0, |id, _| seen.push(id));
        // DFS order: A, a1, B, b1, b2
        assert_eq!(seen, vec![1, 5, 2, 3, 4]);
        assert_eq!(t.all_files(1), vec![5]);
        assert_eq!(t.all_files(2), vec![3, 4]);
        assert_eq!(t.all_files(0), vec![5, 3, 4]);
    }

    #[test]
    fn path_rebuild_walks_to_root_ref() {
        let mut t = Tree::new(1);
        t.add_root_path(0, "C:\\Base");
        t.append_batch(0, vec![dir("Alpha", 1)]);
        t.append_batch(1, vec![dir("Deep", 1)]);
        t.append_batch(2, vec![file("x.bin", 1, 1, 1)]);
        assert_eq!(t.node_path(0), "C:\\Base");
        assert_eq!(t.node_path(1), "C:\\Base\\Alpha");
        assert_eq!(t.node_path(2), "C:\\Base\\Alpha\\Deep");
        assert_eq!(t.node_path(3), "C:\\Base\\Alpha\\Deep\\x.bin");
        let mut t2 = Tree::new(2);
        t2.add_root_path(0, "C:\\");
        t2.append_batch(0, vec![dir("Users", 1)]);
        assert_eq!(t2.node_path(1), "C:\\Users");
    }

    #[test]
    fn this_pc_synthetic_root_has_no_path() {
        let mut t = Tree::new(3);
        t.this_pc_label = Some("This PC".into());
        t.append_batch(0, vec![dir("C:", 1), dir("D:", 1)]);
        t.add_root_path(1, "C:\\");
        assert_eq!(t.node_path(0), "This PC");
        assert_eq!(t.node_path(1), "C:\\");
        assert!(t.is_descendant_of(1, 0));
    }

    #[test]
    fn descendant_check() {
        let mut t = Tree::new(4);
        t.add_root_path(0, "C:\\");
        t.append_batch(0, vec![dir("A", 1), dir("B", 1)]);
        t.append_batch(1, vec![file("a1", 1, 1, 1)]);
        t.append_batch(2, vec![file("b1", 1, 1, 1)]);
        assert!(t.is_descendant_of(1, 0));
        assert!(t.is_descendant_of(3, 1));
        assert!(t.is_descendant_of(4, 2));
        assert!(!t.is_descendant_of(4, 1));
        assert!(!t.is_descendant_of(3, 2));
        assert!(!t.is_descendant_of(0, 3));
    }

    #[test]
    fn savings_and_overhead_helpers() {
        let mut t = Tree::new(6);
        t.add_root_path(0, "C:\\");
        t.append_batch(
            0,
            vec![file("sparse", 100, 40, 1), file("padded", 100, 128, 1)],
        );
        assert_eq!(t.compression_savings(1), 60);
        assert_eq!(t.cluster_overhead(1), 0);
        assert_eq!(t.compression_savings(2), 0);
        assert_eq!(t.cluster_overhead(2), 28);
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_prefixing() {
        assert_eq!(Tree::verbatim("C:\\Users"), r"\\?\C:\Users");
        assert_eq!(Tree::verbatim(r"\\?\C:\Users"), r"\\?\C:\Users");
    }

    #[cfg(not(windows))]
    #[test]
    fn verbatim_is_identity_on_posix() {
        // Cross-platform doc §3: the mac engine takes plain paths.
        assert_eq!(Tree::verbatim("/Applications"), "/Applications");
        // An already-prefixed path stays untouched (round-trip safety).
        assert_eq!(Tree::verbatim(r"\\?\C:\Users"), r"\\?\C:\Users");
    }
}
