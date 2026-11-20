//! Sources: git clones under `~/.admiral/sources/<name>`, added, discovered and pulled.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::StoreError;
use crate::discovery::{Item, discover};
use crate::tree::io_error;

/// How a source is kept up to date. Authored sources are the user's own: pull merges. Consumed
/// sources are read-only: pull hard-resets them to upstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Authored,
    Consumed,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Authored => "authored",
            SourceKind::Consumed => "consumed",
        }
    }
}

/// The sources directory, `~/.admiral/sources`.
///
/// Codes: `E1006` alias collision, `E1007` a name that is not a plain alias, `E1008` a git
/// failure (clone, fetch, reset or merge), `E1009` ambiguous item names (see `discover`), `E1010`
/// no such source.
#[derive(Clone, Debug)]
pub struct Sources {
    dir: PathBuf,
}

/// A source that was just added.
#[derive(Clone, Debug)]
pub struct AddedSource {
    pub path: PathBuf,
    pub items: Vec<Item>,
}

impl Sources {
    pub fn new(dir: impl Into<PathBuf>) -> Sources {
        Sources { dir: dir.into() }
    }

    /// Where the source called `name` lives.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// The registered aliases, sorted.
    pub fn names(&self) -> Result<Vec<String>, StoreError> {
        let read = match fs::read_dir(&self.dir) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io_error(&self.dir, &e)),
        };
        let mut out = Vec::new();
        for entry in read {
            let entry = entry.map_err(|e| io_error(&self.dir, &e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') && entry.path().is_dir() {
                out.push(name);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Refuses a name that is not a plain alias or that another source already holds (compared
    /// without regard to case, so the clone is safe on case-insensitive file systems). Touches
    /// nothing on disk.
    pub fn check_alias(&self, name: &str) -> Result<(), StoreError> {
        let plain = name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && name
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric());
        if !plain {
            return Err(StoreError::new(
                "E1007",
                format!("{name:?} is not a valid source alias"),
                "Use letters, digits, `.`, `_` and `-`, starting with a letter or digit.",
            ));
        }
        let lower = name.to_lowercase();
        if let Some(existing) = self
            .names()?
            .into_iter()
            .find(|existing| existing.to_lowercase() == lower)
        {
            return Err(StoreError::new(
                "E1006",
                format!("source alias {name:?} is already registered (as {existing:?})"),
                "Choose a different name for this source; two sources cannot register the same alias.",
            ));
        }
        Ok(())
    }

    /// Clones `url` into `<sources>/<name>` and discovers its items.
    ///
    /// Both kinds are cloned alike; the kind only decides how `pull` updates the clone.
    ///
    /// The alias is checked before anything is fetched, and a clone that fails or whose items
    /// are ambiguous leaves nothing behind.
    pub fn add(&self, name: &str, url: &str, _kind: SourceKind) -> Result<AddedSource, StoreError> {
        self.check_alias(name)?;
        fs::create_dir_all(&self.dir).map_err(|e| io_error(&self.dir, &e))?;
        let staging = self.dir.join(format!(".tmp-{name}-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|e| io_error(&staging, &e))?;
        }
        let result = git(
            None,
            &["clone", "--quiet", "--", url, &staging.to_string_lossy()],
            "Check the URL and your access to it, then retry.",
        )
        .and_then(|_| discover(&staging, name));
        let items = match result {
            Ok(items) => items,
            Err(e) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(e);
            }
        };
        let path = self.path(name);
        if let Err(e) = fs::rename(&staging, &path) {
            let _ = fs::remove_dir_all(&staging);
            return Err(io_error(&path, &e));
        }
        Ok(AddedSource { path, items })
    }

    /// Brings the source up to date and returns its items afterwards.
    ///
    /// A consumed source is hard-reset to its upstream branch, discarding local changes and
    /// following a rewritten history. An authored source is pulled with a merge (never a rebase),
    /// so local commits stay; a conflict is an error and leaves the clone for the user to resolve.
    pub fn pull(&self, name: &str, kind: SourceKind) -> Result<Vec<Item>, StoreError> {
        let path = self.path(name);
        if !path.join(".git").exists() {
            return Err(StoreError::new(
                "E1010",
                format!("no source named {name:?}"),
                "Add it first with `admiral sources add <name> <url>`.",
            ));
        }
        match kind {
            SourceKind::Consumed => {
                let remedy = "Check your network and access to the upstream, then retry.";
                git(Some(&path), &["fetch", "--quiet"], remedy)?;
                git(
                    Some(&path),
                    &["reset", "--hard", "--quiet", "@{upstream}"],
                    "Make sure the clone's branch tracks an upstream branch that still exists.",
                )?;
            }
            SourceKind::Authored => {
                git(
                    Some(&path),
                    &["pull", "--quiet", "--no-rebase", "--no-edit"],
                    "Resolve the conflict or fix the problem in the clone, commit, then pull again.",
                )?;
            }
        }
        discover(&path, name)
    }
}

fn git(dir: Option<&Path>, args: &[&str], remedy: &str) -> Result<(), StoreError> {
    let mut command = Command::new("git");
    if let Some(dir) = dir {
        command.arg("-C").arg(dir);
    }
    let output = command.args(args).output().map_err(|e| {
        StoreError::new(
            "E1008",
            format!("cannot run git: {e}"),
            "Install git 2.30 or later and make sure it is on PATH.",
        )
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(StoreError::new(
        "E1008",
        format!(
            "git {} failed: {}",
            args.iter()
                .take_while(|a| **a != "--")
                .copied()
                .collect::<Vec<_>>()
                .join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        remedy,
    ))
}
