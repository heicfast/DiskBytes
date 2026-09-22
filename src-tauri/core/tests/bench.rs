//! Manual M2 gate bench (doc 03: "1M synthetic nodes insert < 2 s; bench
//! ignored in CI, run manually"). Run with:
//! `cargo test -p diskbytes-core --release -- --ignored bench_1m --nocapture`

use diskbytes_core::scan::node::{BatchEntry, Node, Tree};

#[test]
#[ignore = "manual bench"]
fn bench_1m_nodes_insert() {
    let start = std::time::Instant::now();
    let mut t = Tree::new(1);
    t.add_root_path(0, "C:\\bench");
    // 1000 dirs × 1000 files each = 1,000,000 files + 1001 dirs.
    let file = |i: u32| BatchEntry {
        name: format!("file_{i}").encode_utf16().collect(),
        node: {
            let mut n = Node::new_file();
            n.logical = u64::from(i) * 17;
            n.on_disk = u64::from(i) * 17 + 4096;
            n.modified = 1_700_000_000 + i64::from(i);
            n
        },
    };
    let chunk: Vec<BatchEntry> = (0..1000).map(file).collect();
    let mut parent = 0;
    for d in 0..1000 {
        let dir = BatchEntry {
            name: format!("dir_{d}").encode_utf16().collect(),
            node: Node::new_dir(),
        };
        let base = t.append_batch(parent, vec![dir]);
        parent = base;
        t.append_batch(base, chunk.clone());
    }
    let insert_ms = start.elapsed().as_millis();
    println!("1M nodes inserted in {insert_ms} ms");
    assert!(
        insert_ms < 2000,
        "M2 gate: 1M insert took {insert_ms} ms (> 2 s)"
    );
}
