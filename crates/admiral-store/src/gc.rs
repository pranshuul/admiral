use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::store::{is_hash, make_writable};
use crate::tree::io_error;
use crate::{Store, StoreError};

/// Where references to store entries live. Each path is a file or a directory, scanned
/// recursively; a path that does not exist holds no references. A reference is any 64-character
/// lowercase hex string, with or without a `sha256:` prefix.
#[derive(Clone, Debug, Default)]
pub struct GcRoots {
    /// `workflow.lock` and `admiral.lock` files (or the directories holding them).
    pub locks: Vec<PathBuf>,
    /// `manifest.toml` files.
    pub manifests: Vec<PathBuf>,
    /// Cache directories (`~/.admiral/cache`).
    pub caches: Vec<PathBuf>,
    /// Run directories of live runs.
    pub live_runs: Vec<PathBuf>,
}

/// What a collection removed (or would remove) and kept, as sorted hashes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GcReport {
    pub removed: Vec<String>,
    pub kept: Vec<String>,
}

/// Adds every maximal run of hex digits that is exactly a hash.
fn scan_text(bytes: &[u8], found: &mut BTreeSet<String>) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_hexdigit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            if let Ok(run) = std::str::from_utf8(&bytes[start..i])
                && is_hash(run)
            {
                found.insert(run.to_string());
            }
        } else {
            i += 1;
        }
    }
}

fn scan_path(path: &Path, found: &mut BTreeSet<String>) -> Result<(), StoreError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_error(path, &e)),
    };
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| io_error(path, &e))? {
            scan_path(&entry.map_err(|e| io_error(path, &e))?.path(), found)?;
        }
    } else if meta.is_file() {
        let bytes = fs::read(path).map_err(|e| io_error(path, &e))?;
        scan_text(&bytes, found);
    }
    Ok(())
}

impl Store {
    /// Removes entries no root references; with `dry_run`, only reports them.
    ///
    /// Every root is read before anything is removed, so an unreadable root removes nothing.
    pub fn gc(&self, roots: &GcRoots, dry_run: bool) -> Result<GcReport, StoreError> {
        let mut referenced = BTreeSet::new();
        for path in roots
            .locks
            .iter()
            .chain(&roots.manifests)
            .chain(&roots.caches)
            .chain(&roots.live_runs)
        {
            scan_path(path, &mut referenced)?;
        }
        let mut report = GcReport::default();
        for hash in self.hashes()? {
            if referenced.contains(&hash) {
                report.kept.push(hash);
            } else {
                if !dry_run {
                    let entry = self.entries_dir().join(&hash);
                    make_writable(&entry)?;
                    fs::remove_dir_all(&entry).map_err(|e| io_error(&entry, &e))?;
                }
                report.removed.push(hash);
            }
        }
        Ok(report)
    }
}
