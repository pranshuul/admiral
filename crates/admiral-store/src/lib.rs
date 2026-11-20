//! The content-addressed store (tree hashing, `put`, `get`, `gc`) and sources with discovery.

pub mod discovery;
pub mod error;
pub mod gc;
pub mod sources;
pub mod store;
pub mod tree;

pub use discovery::{Item, ItemKind, discover};
pub use error::StoreError;
pub use gc::{GcReport, GcRoots};
pub use sources::{AddedSource, SourceKind, Sources};
pub use store::Store;
pub use tree::{hash_tree, list_tree};
