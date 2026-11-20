use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use admiral_store::store::Store;
use admiral_store::tree::hash_tree;

fn write(root: &Path, rel: &str, bytes: &[u8], mode: u32) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn item() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "SKILL.md", b"# skill\n", 0o644);
    write(dir.path(), "scripts/run.sh", b"#!/bin/sh\n", 0o755);
    write(dir.path(), ".git/HEAD", b"ref\n", 0o644);
    dir
}

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        out.push(path.clone());
        if path.is_dir() {
            walk(&path, out);
        }
    }
}

#[test]
fn put_returns_the_tree_hash_and_copies_to_sha256_dir() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let src = item();
    let hash = store.put(src.path()).unwrap();
    assert_eq!(hash, hash_tree(src.path()).unwrap());
    let entry = store_dir.path().join("sha256").join(&hash);
    assert_eq!(store.get(&hash).unwrap(), entry);
    assert_eq!(fs::read(entry.join("SKILL.md")).unwrap(), b"# skill\n");
    assert_eq!(
        fs::read(entry.join("scripts/run.sh")).unwrap(),
        b"#!/bin/sh\n"
    );
    assert!(!entry.join(".git").exists());
}

#[test]
fn the_entry_rehashes_to_its_own_name() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let hash = store.put(item().path()).unwrap();
    assert_eq!(hash_tree(&store.get(&hash).unwrap()).unwrap(), hash);
}

#[test]
fn entries_are_read_only_dirs_0555_files_0444_or_0555() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let hash = store.put(item().path()).unwrap();
    let entry = store.get(&hash).unwrap();
    assert_eq!(mode(&entry), 0o555);
    assert_eq!(mode(&entry.join("scripts")), 0o555);
    assert_eq!(mode(&entry.join("SKILL.md")), 0o444);
    assert_eq!(mode(&entry.join("scripts/run.sh")), 0o555);
}

#[test]
fn putting_the_same_bytes_twice_stores_once() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let a = store.put(item().path()).unwrap();
    let b = store.put(item().path()).unwrap();
    assert_eq!(a, b);
    assert_eq!(store.hashes().unwrap(), vec![a]);
}

#[test]
fn put_leaves_nothing_but_entries_in_the_store() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    store.put(item().path()).unwrap();
    let names: Vec<_> = fs::read_dir(store_dir.path().join("sha256"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].len(), 64);
}

#[test]
fn put_does_not_touch_the_source() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let src = item();
    store.put(src.path()).unwrap();
    assert_eq!(mode(&src.path().join("SKILL.md")), 0o644);
    assert!(src.path().join(".git/HEAD").exists());
}

#[test]
fn get_accepts_the_sha256_prefix_used_in_locks() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let hash = store.put(item().path()).unwrap();
    assert_eq!(
        store.get(&format!("sha256:{hash}")).unwrap(),
        store.get(&hash).unwrap()
    );
}

#[test]
fn get_of_a_missing_entry_is_e1002() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let err = store.get(&"0".repeat(64)).unwrap_err();
    assert_eq!(err.code, "E1002");
    assert!(!err.remedy.is_empty());
}

#[test]
fn get_of_a_malformed_hash_is_e1004() {
    let store = Store::new(tempfile::tempdir().unwrap().path());
    for bad in [
        "",
        "abc",
        "../etc",
        &"G".repeat(64),
        &"A".repeat(64),
        &"a".repeat(65),
    ] {
        assert_eq!(store.get(bad).unwrap_err().code, "E1004", "{bad}");
    }
}

#[test]
fn hashes_of_a_store_that_does_not_exist_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("store"));
    assert_eq!(store.hashes().unwrap(), Vec::<String>::new());
}

#[test]
fn a_failed_put_leaves_no_entry() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::new(store_dir.path());
    let src = item();
    std::os::unix::fs::symlink("SKILL.md", src.path().join("link")).unwrap();
    assert_eq!(store.put(src.path()).unwrap_err().code, "E1003");
    assert_eq!(store.hashes().unwrap(), Vec::<String>::new());
    let mut all = Vec::new();
    if store_dir.path().join("sha256").exists() {
        walk(&store_dir.path().join("sha256"), &mut all);
    }
    assert!(all.is_empty());
}
