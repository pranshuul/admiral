use std::path::{Path, PathBuf};

use crate::StoreError;

/// What an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    Skill,
    Agent,
    Workflow,
}

/// One discovered item, named `source/name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub source: String,
    pub kind: ItemKind,
    pub name: String,
    pub path: PathBuf,
}

impl Item {
    pub fn id(&self) -> String {
        todo!("red")
    }
}

pub fn discover(root: &Path, source: &str) -> Result<Vec<Item>, StoreError> {
    let _ = (root, source);
    todo!("red")
}
