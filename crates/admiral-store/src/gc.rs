use std::path::PathBuf;

use crate::error::StoreError;
use crate::store::Store;

#[derive(Clone, Debug, Default)]
pub struct GcRoots {
    pub locks: Vec<PathBuf>,
    pub manifests: Vec<PathBuf>,
    pub caches: Vec<PathBuf>,
    pub live_runs: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GcReport {
    pub removed: Vec<String>,
    pub kept: Vec<String>,
}

impl Store {
    pub fn gc(&self, _roots: &GcRoots, _dry_run: bool) -> Result<GcReport, StoreError> {
        todo!("red")
    }
}
