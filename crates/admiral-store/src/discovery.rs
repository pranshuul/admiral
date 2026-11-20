//! Discovery by convention: every directory containing `SKILL.md` is a skill, every
//! `agents/*.md` a subagent and every `workflows/*/workflow.toml` a workflow.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::StoreError;
use crate::tree::io_error;

/// What an item is. The order is the order discovery lists them in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    Skill,
    Agent,
    Workflow,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
            ItemKind::Workflow => "workflow",
        }
    }
}

/// One discovered item, named `source/name`. `path` is relative to the source root, with `/`
/// separators (`.` for a skill at the root itself).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub source: String,
    pub kind: ItemKind,
    pub name: String,
    pub path: PathBuf,
}

impl Item {
    /// The item's name in its namespace, `source/name`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.source, self.name)
    }
}

/// Walks the source at `root` and lists its items, sorted by kind and then name.
///
/// `SKILL.md` directories count at any depth and are named after their directory (a skill at the
/// root takes the source's name). Agents and workflows are looked for under the root's `agents/`
/// and `workflows/`. The `.git` directory is not walked, and symlinks are not followed. Two items
/// of one kind with the same name would make `source/name` ambiguous, so they are refused (`E1009`).
pub fn discover(root: &Path, source: &str) -> Result<Vec<Item>, StoreError> {
    let mut items = Vec::new();
    walk(root, Path::new(""), source, &mut items)?;
    agents(root, source, &mut items)?;
    workflows(root, source, &mut items)?;
    items.sort_by(|a, b| (a.kind, &a.name, &a.path).cmp(&(b.kind, &b.name, &b.path)));
    let mut seen: BTreeMap<(ItemKind, &str), &Item> = BTreeMap::new();
    for item in &items {
        if let Some(first) = seen.insert((item.kind, &item.name), item) {
            return Err(StoreError::new(
                "E1009",
                format!(
                    "{} {:?} is defined twice in source {source:?}: {} and {}",
                    item.kind.as_str(),
                    item.id(),
                    slash(&first.path),
                    slash(&item.path),
                ),
                "Rename or remove one of them upstream; every item must be addressable as `source/name`.",
            ));
        }
    }
    Ok(items)
}

fn slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn sorted_entries(dir: &Path) -> Result<Vec<fs::DirEntry>, StoreError> {
    let mut entries = fs::read_dir(dir)
        .map_err(|e| io_error(dir, &e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io_error(dir, &e))?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

fn is_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

fn is_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

fn walk(root: &Path, rel: &Path, source: &str, items: &mut Vec<Item>) -> Result<(), StoreError> {
    let dir = root.join(rel);
    if is_file(&dir.join("SKILL.md")) {
        let name = match rel.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => source.to_string(),
        };
        let path = if rel.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            rel.to_path_buf()
        };
        items.push(Item {
            source: source.to_string(),
            kind: ItemKind::Skill,
            name,
            path,
        });
    }
    for entry in sorted_entries(&dir)? {
        let name = entry.file_name();
        if name == ".git" || !is_dir(&entry.path()) {
            continue;
        }
        walk(root, &rel.join(name), source, items)?;
    }
    Ok(())
}

fn agents(root: &Path, source: &str, items: &mut Vec<Item>) -> Result<(), StoreError> {
    let dir = root.join("agents");
    if !is_dir(&dir) {
        return Ok(());
    }
    for entry in sorted_entries(&dir)? {
        let path = entry.path();
        if is_file(&path)
            && path.extension().is_some_and(|e| e == "md")
            && let Some(stem) = path.file_stem()
        {
            items.push(Item {
                source: source.to_string(),
                kind: ItemKind::Agent,
                name: stem.to_string_lossy().into_owned(),
                path: Path::new("agents").join(entry.file_name()),
            });
        }
    }
    Ok(())
}

fn workflows(root: &Path, source: &str, items: &mut Vec<Item>) -> Result<(), StoreError> {
    let dir = root.join("workflows");
    if !is_dir(&dir) {
        return Ok(());
    }
    for entry in sorted_entries(&dir)? {
        if is_dir(&entry.path()) && is_file(&entry.path().join("workflow.toml")) {
            items.push(Item {
                source: source.to_string(),
                kind: ItemKind::Workflow,
                name: entry.file_name().to_string_lossy().into_owned(),
                path: Path::new("workflows").join(entry.file_name()),
            });
        }
    }
    Ok(())
}
