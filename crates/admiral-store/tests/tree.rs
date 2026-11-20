use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

use admiral_model::tree::TreeMode;
use admiral_store::tree::{hash_tree, list_tree};

const HELLO: &str = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03";
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn write(root: &Path, rel: &str, bytes: &[u8], mode: u32) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn single_file_hash_is_known() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", b"hello\n", 0o644);
    assert_eq!(
        hash_tree(dir.path()).unwrap(),
        "dd85b29dd1e3ab777066a2314d7801a4c3380f01f7fe564a5fcec9cfb69fbf51"
    );
}

#[test]
fn listing_is_sorted_by_full_path_with_modes_and_hashes() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "b/c", b"hello\n", 0o755);
    write(dir.path(), "b-d", b"", 0o600);
    let listing = list_tree(dir.path()).unwrap();
    let got: Vec<_> = listing
        .entries
        .iter()
        .map(|e| (e.path.as_str(), e.mode, e.sha256.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("b-d", TreeMode::Plain, EMPTY),
            ("b/c", TreeMode::Executable, HELLO)
        ]
    );
    assert_eq!(
        hash_tree(dir.path()).unwrap(),
        "8786de979792d5d1ef3d0782713f447905629f3577458c6efbfc954c6d301fd9"
    );
}

#[test]
fn root_git_directory_is_excluded() {
    let with = tempfile::tempdir().unwrap();
    let without = tempfile::tempdir().unwrap();
    write(with.path(), "a.txt", b"hello\n", 0o644);
    write(with.path(), ".git/HEAD", b"ref: refs/heads/main\n", 0o644);
    write(without.path(), "a.txt", b"hello\n", 0o644);
    assert_eq!(
        hash_tree(with.path()).unwrap(),
        hash_tree(without.path()).unwrap()
    );
}

#[test]
fn empty_directories_and_the_location_do_not_matter() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "x/y.txt", b"1", 0o644);
    write(b.path(), "x/y.txt", b"1", 0o644);
    fs::create_dir_all(a.path().join("empty/deeper")).unwrap();
    assert_eq!(hash_tree(a.path()).unwrap(), hash_tree(b.path()).unwrap());
    let none = tempfile::tempdir().unwrap();
    assert_eq!(
        hash_tree(none.path()).unwrap(),
        EMPTY,
        "the empty listing hashes to the SHA-256 of nothing"
    );
}

#[test]
fn a_mode_change_changes_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "run.sh", b"#!/bin/sh\n", 0o644);
    let before = hash_tree(dir.path()).unwrap();
    fs::set_permissions(dir.path().join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_ne!(before, hash_tree(dir.path()).unwrap());
}

#[test]
fn a_one_byte_change_changes_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a", b"abc", 0o644);
    let before = hash_tree(dir.path()).unwrap();
    write(dir.path(), "a", b"abd", 0o644);
    assert_ne!(before, hash_tree(dir.path()).unwrap());
}

#[test]
fn line_endings_are_not_normalized() {
    let lf = tempfile::tempdir().unwrap();
    let crlf = tempfile::tempdir().unwrap();
    write(lf.path(), "a", b"one\ntwo\n", 0o644);
    write(crlf.path(), "a", b"one\r\ntwo\r\n", 0o644);
    assert_ne!(
        hash_tree(lf.path()).unwrap(),
        hash_tree(crlf.path()).unwrap()
    );
}

#[test]
fn a_rename_changes_the_hash() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(a.path(), "a", b"x", 0o644);
    write(b.path(), "b", b"x", 0o644);
    assert_ne!(hash_tree(a.path()).unwrap(), hash_tree(b.path()).unwrap());
}

#[test]
fn symlinks_are_refused_with_e1003() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a", b"x", 0o644);
    symlink("a", dir.path().join("link")).unwrap();
    let err = hash_tree(dir.path()).unwrap_err();
    assert_eq!(err.code, "E1003");
    assert!(err.message.contains("link"));
    assert!(!err.remedy.is_empty());
}

#[test]
fn a_file_name_with_a_newline_is_refused_with_e1003() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a\nb", b"x", 0o644);
    assert_eq!(hash_tree(dir.path()).unwrap_err().code, "E1003");
}

#[test]
fn a_root_that_is_not_a_directory_is_refused_with_e1003() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "f", b"x", 0o644);
    assert_eq!(hash_tree(&dir.path().join("f")).unwrap_err().code, "E1003");
}

#[test]
fn a_missing_root_is_refused_with_e1005() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        hash_tree(&dir.path().join("nope")).unwrap_err().code,
        "E1005"
    );
}
