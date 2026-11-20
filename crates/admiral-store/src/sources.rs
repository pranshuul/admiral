use std::path::{Path, PathBuf};

use crate::StoreError;
use crate::discovery::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Authored,
    Consumed,
}

#[derive(Clone, Debug)]
pub struct Sources {
    dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct AddedSource {
    pub path: PathBuf,
    pub items: Vec<Item>,
}

impl Sources {
    pub fn new(dir: impl Into<PathBuf>) -> Sources {
        Sources { dir: dir.into() }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        let _ = (&self.dir, name);
        todo!("red")
    }

    pub fn names(&self) -> Result<Vec<String>, StoreError> {
        todo!("red")
    }

    pub fn add(&self, name: &str, url: &str, kind: SourceKind) -> Result<AddedSource, StoreError> {
        let _ = (name, url, kind, Path::new(""));
        todo!("red")
    }

    pub fn pull(&self, name: &str, kind: SourceKind) -> Result<Vec<Item>, StoreError> {
        let _ = (name, kind);
        todo!("red")
    }
}
