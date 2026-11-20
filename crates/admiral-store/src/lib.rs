//! The content-addressed store: tree hashing, `put`, `get` and `gc`.

pub mod discovery;
pub mod error;
pub mod gc;
pub mod sources;
pub mod store;
pub mod tree;

pub use error::StoreError;
pub use gc::{GcReport, GcRoots};
pub use store::Store;
pub use tree::{hash_tree, list_tree};
