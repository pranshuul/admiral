//! `tree/1`: the canonical listing a store entry's hash is computed over.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Whether a listed file is executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TreeMode {
    /// Executable: written `x` in the listing.
    #[serde(rename = "x")]
    Executable,
    /// Not executable: written `-` in the listing.
    #[serde(rename = "-")]
    Plain,
}

/// One file of a tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct TreeEntry {
    /// Path relative to the item root, `/` separated.
    #[schemars(length(min = 1))]
    pub path: String,
    pub mode: TreeMode,
    /// SHA-256 of the file's bytes, lowercase hex.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
}

impl TreeEntry {
    /// The listing line: `<path>\0<mode>\0<sha256>\n`.
    pub fn line(&self) -> String {
        todo!("red")
    }
}

/// The `tree/1` document: every file, sorted by path.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "tree/1", deny_unknown_fields)]
pub struct TreeListing {
    pub entries: Vec<TreeEntry>,
}

impl TreeListing {
    /// The canonical text that is hashed: every entry's line, in order.
    pub fn render(&self) -> String {
        todo!("red")
    }
}

/// The `tree/1` JSON Schema as a JSON value.
pub fn tree_schema() -> serde_json::Value {
    todo!("red")
}
