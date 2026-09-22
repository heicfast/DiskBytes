//! Scanning: the node arena, categories, roll-up and the standard-engine
//! scanner (worker pool over the [`Platform`](crate::platform) seam).

pub mod categories;
pub mod node;
pub mod rollup;
pub mod scanner;
pub mod surgery;

pub use categories::FileCategory;
pub use node::{DirExtra, Node, Tree};
