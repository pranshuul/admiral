use std::path::Path;

use admiral_model::tree::TreeListing;

use crate::error::StoreError;

pub fn list_tree(_root: &Path) -> Result<TreeListing, StoreError> {
    todo!("red")
}

pub fn hash_tree(_root: &Path) -> Result<String, StoreError> {
    todo!("red")
}
