use std::path::{Path, PathBuf};

use crate::error::StoreError;

#[derive(Clone, Debug)]
pub struct Store {
    #[allow(dead_code)]
    pub(crate) dir: PathBuf,
}

impl Store {
    pub fn new(_dir: impl Into<PathBuf>) -> Store {
        todo!("red")
    }

    pub fn put(&self, _path: &Path) -> Result<String, StoreError> {
        todo!("red")
    }

    pub fn get(&self, _hash: &str) -> Result<PathBuf, StoreError> {
        todo!("red")
    }

    pub fn hashes(&self) -> Result<Vec<String>, StoreError> {
        todo!("red")
    }
}
