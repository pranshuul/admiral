use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use admiral_model::tree::TreeMode;

use crate::StoreError;
use crate::tree::{io_error, list_tree};

/// A store directory, `~/.admiral/store`, holding entries at `sha256/<hash>/`.
#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

pub(crate) fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    pub(crate) fn entries_dir(&self) -> PathBuf {
        self.dir.join("sha256")
    }

    /// Copies the tree at `path` into the store read-only and returns its hash.
    ///
    /// Directories become 0555, files 0444, or 0555 when executable. Putting bytes already
    /// stored changes nothing.
    pub fn put(&self, path: &Path) -> Result<String, StoreError> {
        let listing = list_tree(path)?;
        let hash = crate::tree::hash_tree(path)?;
        let entries = self.entries_dir();
        let target = entries.join(&hash);
        if target.is_dir() {
            return Ok(hash);
        }
        fs::create_dir_all(&entries).map_err(|e| io_error(&entries, &e))?;
        let staging = entries.join(format!(".tmp-{hash}-{}", std::process::id()));
        let result = self.stage(path, &listing, &staging, &target);
        if result.is_err() {
            let _ = make_writable(&staging);
            let _ = fs::remove_dir_all(&staging);
        }
        result?;
        Ok(hash)
    }

    fn stage(
        &self,
        src: &Path,
        listing: &admiral_model::tree::TreeListing,
        staging: &Path,
        target: &Path,
    ) -> Result<(), StoreError> {
        if staging.exists() {
            make_writable(staging)?;
            fs::remove_dir_all(staging).map_err(|e| io_error(staging, &e))?;
        }
        fs::create_dir(staging).map_err(|e| io_error(staging, &e))?;
        for entry in &listing.entries {
            let from = src.join(&entry.path);
            let to = staging.join(&entry.path);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| io_error(parent, &e))?;
            }
            fs::copy(&from, &to).map_err(|e| io_error(&from, &e))?;
            let mode = match entry.mode {
                TreeMode::Executable => 0o555,
                TreeMode::Plain => 0o444,
            };
            fs::set_permissions(&to, fs::Permissions::from_mode(mode))
                .map_err(|e| io_error(&to, &e))?;
        }
        let mut dirs = Vec::new();
        collect_dirs(staging, &mut dirs)?;
        // Deepest first, so a directory is sealed only after its children.
        for dir in dirs.iter().rev() {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o555))
                .map_err(|e| io_error(dir, &e))?;
        }
        fs::set_permissions(staging, fs::Permissions::from_mode(0o555))
            .map_err(|e| io_error(staging, &e))?;
        match fs::rename(staging, target) {
            Ok(()) => Ok(()),
            // A concurrent put of the same bytes won the race: the entry exists, which is the goal.
            Err(_) if target.is_dir() => {
                make_writable(staging)?;
                fs::remove_dir_all(staging).map_err(|e| io_error(staging, &e))
            }
            Err(e) => Err(io_error(target, &e)),
        }
    }

    /// The path of the entry for `hash` (`<hex>` or `sha256:<hex>`).
    pub fn get(&self, hash: &str) -> Result<PathBuf, StoreError> {
        let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
        if !is_hash(hex) {
            return Err(StoreError::new(
                "E1004",
                format!("{hash:?} is not a SHA-256 hash"),
                "Use 64 lowercase hex characters, optionally prefixed with `sha256:`.",
            ));
        }
        let path = self.entries_dir().join(hex);
        if path.is_dir() {
            Ok(path)
        } else {
            Err(StoreError::new(
                "E1002",
                format!("no store entry for sha256:{hex}"),
                "Install or publish the item that references it, so it is fetched into the store.",
            ))
        }
    }

    /// Every entry's hash, sorted.
    pub fn hashes(&self) -> Result<Vec<String>, StoreError> {
        let dir = self.entries_dir();
        let read = match fs::read_dir(&dir) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io_error(&dir, &e)),
        };
        let mut out = Vec::new();
        for entry in read {
            let entry = entry.map_err(|e| io_error(&dir, &e))?;
            if let Some(name) = entry.file_name().to_str()
                && is_hash(name)
                && entry.path().is_dir()
            {
                out.push(name.to_string());
            }
        }
        out.sort();
        Ok(out)
    }
}

fn collect_dirs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), StoreError> {
    for entry in fs::read_dir(dir).map_err(|e| io_error(dir, &e))? {
        let path = entry.map_err(|e| io_error(dir, &e))?.path();
        if path.is_dir() {
            out.push(path.clone());
            collect_dirs(&path, out)?;
        }
    }
    Ok(())
}

/// Gives the owner write access to every directory under `dir` (and `dir`), so it can be deleted.
pub(crate) fn make_writable(dir: &Path) -> Result<(), StoreError> {
    if !dir.is_dir() {
        return Ok(());
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).map_err(|e| io_error(dir, &e))?;
    for entry in fs::read_dir(dir).map_err(|e| io_error(dir, &e))? {
        let path = entry.map_err(|e| io_error(dir, &e))?.path();
        if path.is_dir() {
            make_writable(&path)?;
        }
    }
    Ok(())
}
