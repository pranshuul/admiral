use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use admiral_model::tree::{TreeEntry, TreeListing, TreeMode};
use sha2::{Digest, Sha256};

use crate::StoreError;

pub(crate) fn io_error(path: &Path, e: &io::Error) -> StoreError {
    StoreError::new(
        "E1005",
        format!("{}: {e}", path.display()),
        "Check that the path exists and is readable and writable, then retry.",
    )
}

fn unhashable(path: &Path, why: &str) -> StoreError {
    StoreError::new(
        "E1003",
        format!("{}: {why}", path.display()),
        "Store a directory of regular files with plain names; remove or replace anything else.",
    )
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_file(path: &Path) -> Result<String, StoreError> {
    let mut file = fs::File::open(path).map_err(|e| io_error(path, &e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| io_error(path, &e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn collect(dir: &Path, prefix: &str, out: &mut Vec<TreeEntry>) -> Result<(), StoreError> {
    let read = fs::read_dir(dir).map_err(|e| io_error(dir, &e))?;
    for entry in read {
        let entry = entry.map_err(|e| io_error(dir, &e))?;
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            return Err(unhashable(&path, "the name is not valid UTF-8"));
        };
        if prefix.is_empty() && name == ".git" {
            continue;
        }
        if name.contains('\n') || name.contains('\0') {
            return Err(unhashable(&path, "the name contains a newline or NUL"));
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let meta = fs::symlink_metadata(&path).map_err(|e| io_error(&path, &e))?;
        let kind = meta.file_type();
        if kind.is_symlink() {
            return Err(unhashable(&path, "symbolic links are not stored"));
        } else if kind.is_dir() {
            collect(&path, &rel, out)?;
        } else if kind.is_file() {
            let mode = if meta.permissions().mode() & 0o100 != 0 {
                TreeMode::Executable
            } else {
                TreeMode::Plain
            };
            out.push(TreeEntry {
                path: rel,
                mode,
                sha256: hash_file(&path)?,
            });
        } else {
            return Err(unhashable(&path, "not a regular file or directory"));
        }
    }
    Ok(())
}

/// The canonical listing of the item at `root`: every file except the root's `.git`, sorted by
/// relative path.
pub fn list_tree(root: &Path) -> Result<TreeListing, StoreError> {
    let meta = fs::metadata(root).map_err(|e| io_error(root, &e))?;
    if !meta.is_dir() {
        return Err(unhashable(root, "the item root is not a directory"));
    }
    let mut entries = Vec::new();
    collect(root, "", &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(TreeListing { entries })
}

/// The SHA-256 of the canonical listing of `root`, as 64 lowercase hex characters.
pub fn hash_tree(root: &Path) -> Result<String, StoreError> {
    let listing = list_tree(root)?;
    Ok(to_hex(&Sha256::digest(listing.render().as_bytes())))
}
