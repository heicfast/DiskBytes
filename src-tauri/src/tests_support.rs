//! Shared test fixtures for the app crate (compiled only under
//! `cfg(test)`; registered in `lib.rs`).

#![allow(dead_code)] // test helper surface

use diskbytes_core::scan::categories::FileCategory;
use diskbytes_core::scan::node::{BatchEntry, Node};

/// A directory `BatchEntry` (mtime = 1).
pub fn dir_entry(name: &str) -> BatchEntry {
    let mut node = Node::new_dir();
    node.modified = 1;
    BatchEntry {
        name: name.encode_utf16().collect(),
        node,
    }
}

/// A file `BatchEntry` classified from its name (the insertion invariant
/// — category bits must be set or every file lands in Other).
pub fn file_entry(name: &str, size: u64, modified: i64) -> BatchEntry {
    let mut node = Node::new_file();
    node.logical = size;
    node.on_disk = size;
    node.modified = modified;
    node.set_category(FileCategory::from_name(
        &name.encode_utf16().collect::<Vec<u16>>(),
    ));
    BatchEntry {
        name: name.encode_utf16().collect(),
        node,
    }
}
