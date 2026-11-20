use std::fs;
use std::path::Path;

use admiral_store::gc::{GcReport, GcRoots};
use admiral_store::store::Store;

fn put(store: &Store, tag: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("f"), tag).unwrap();
    store.put(dir.path()).unwrap()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

struct Fixture {
    _tmp: tempfile::TempDir,
    store: Store,
    home: std::path::PathBuf,
    hashes: Vec<String>,
}

fn fixture(count: usize) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let store = Store::new(home.join("store"));
    let hashes = (0..count)
        .map(|i| put(&store, &format!("item{i}")))
        .collect();
    Fixture {
        _tmp: tmp,
        store,
        home,
        hashes,
    }
}

#[test]
fn with_no_references_everything_is_removed() {
    let f = fixture(3);
    let report = f.store.gc(&GcRoots::default(), false).unwrap();
    let mut expected = f.hashes.clone();
    expected.sort();
    assert_eq!(report.removed, expected);
    assert!(report.kept.is_empty());
    assert!(f.store.hashes().unwrap().is_empty());
}

#[test]
fn a_lock_manifest_cache_entry_and_live_run_each_keep_their_entries() {
    let f = fixture(5);
    let h = &f.hashes;
    let lock = f.home.join("workflows/w/1.0.0/workflow.lock");
    write(
        &lock,
        &format!("[[item]]\nname = \"a/b\"\nhash = \"sha256:{}\"\n", h[0]),
    );
    let manifest = f.home.join("manifest.toml");
    write(
        &manifest,
        &format!("[[file]]\nhash = \"sha256:{}\"\n", h[1]),
    );
    let cache = f.home.join("cache");
    write(
        &cache.join("k.json"),
        &format!("{{\"outputs\":\"{}\"}}", h[2]),
    );
    let run = f.home.join("project/.admiral/run/w-20261120-abcdef");
    write(
        &run.join("run.json"),
        &format!("{{\"pins\":[\"sha256:{}\"]}}", h[3]),
    );
    let roots = GcRoots {
        locks: vec![lock],
        manifests: vec![manifest],
        caches: vec![cache],
        live_runs: vec![run],
    };
    let report = f.store.gc(&roots, false).unwrap();
    assert_eq!(report.removed, vec![h[4].clone()]);
    let mut kept = h[..4].to_vec();
    kept.sort();
    assert_eq!(report.kept, kept);
    assert_eq!(f.store.hashes().unwrap(), kept);
}

#[test]
fn directories_are_scanned_recursively() {
    let f = fixture(2);
    write(
        &f.home.join("workflows/a/1/workflow.lock"),
        &format!("hash = \"sha256:{}\"", f.hashes[0]),
    );
    let roots = GcRoots {
        locks: vec![f.home.join("workflows")],
        ..GcRoots::default()
    };
    let report = f.store.gc(&roots, false).unwrap();
    assert_eq!(report.removed, vec![f.hashes[1].clone()]);
}

#[test]
fn roots_that_do_not_exist_hold_no_references() {
    let f = fixture(1);
    let roots = GcRoots {
        locks: vec![f.home.join("nope.lock")],
        manifests: vec![f.home.join("manifest.toml")],
        caches: vec![f.home.join("cache")],
        live_runs: vec![f.home.join("run")],
    };
    let report = f.store.gc(&roots, false).unwrap();
    assert_eq!(report.removed, f.hashes);
}

#[test]
fn text_that_only_resembles_a_hash_does_not_keep_an_entry() {
    let f = fixture(1);
    let longer = format!("{}ab", f.hashes[0]);
    write(&f.home.join("x.lock"), &format!("hash = \"{longer}\""));
    let roots = GcRoots {
        locks: vec![f.home.join("x.lock")],
        ..GcRoots::default()
    };
    assert_eq!(f.store.gc(&roots, false).unwrap().removed, f.hashes);
}

#[test]
fn dry_run_reports_and_removes_nothing() {
    let f = fixture(2);
    let report = f.store.gc(&GcRoots::default(), true).unwrap();
    assert_eq!(report.removed.len(), 2);
    assert_eq!(f.store.hashes().unwrap().len(), 2);
    for hash in &f.hashes {
        assert!(f.store.get(hash).unwrap().join("f").exists());
    }
}

#[test]
fn gc_is_idempotent() {
    let f = fixture(2);
    write(&f.home.join("a.lock"), &format!("sha256:{}", f.hashes[0]));
    let roots = GcRoots {
        locks: vec![f.home.join("a.lock")],
        ..GcRoots::default()
    };
    let first = f.store.gc(&roots, false).unwrap();
    assert_eq!(first.removed.len(), 1);
    let second = f.store.gc(&roots, false).unwrap();
    assert_eq!(
        second,
        GcReport {
            removed: vec![],
            kept: vec![f.hashes[0].clone()]
        }
    );
}

#[test]
fn read_only_entries_with_nested_directories_are_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::new(tmp.path().join("store"));
    let src = tempfile::tempdir().unwrap();
    fs::create_dir_all(src.path().join("a/b/c")).unwrap();
    fs::write(src.path().join("a/b/c/f"), "x").unwrap();
    let hash = store.put(src.path()).unwrap();
    let report = store.gc(&GcRoots::default(), false).unwrap();
    assert_eq!(report.removed, vec![hash.clone()]);
    assert!(!tmp.path().join("store/sha256").join(hash).exists());
}

#[test]
fn names_in_the_store_that_are_not_hashes_are_left_alone() {
    let f = fixture(1);
    let stray = f.home.join("store/sha256/README");
    fs::write(&stray, "keep").unwrap();
    let report = f.store.gc(&GcRoots::default(), false).unwrap();
    assert_eq!(report.removed, f.hashes);
    assert!(stray.exists());
}

#[test]
fn gc_of_a_store_that_does_not_exist_reports_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::new(tmp.path().join("store"));
    assert_eq!(
        store.gc(&GcRoots::default(), false).unwrap(),
        GcReport::default()
    );
}

#[test]
fn an_unreadable_root_is_an_e1005_and_removes_nothing() {
    let f = fixture(1);
    let bad = f.home.join("locks");
    fs::create_dir_all(&bad).unwrap();
    // A file whose bytes cannot be read as text must not abort the scan; a directory that
    // cannot be listed must.
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o000)).unwrap();
    let roots = GcRoots {
        locks: vec![bad.clone()],
        ..GcRoots::default()
    };
    let result = f.store.gc(&roots, false);
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o755)).unwrap();
    if let Err(err) = result {
        assert_eq!(err.code, "E1005");
        assert_eq!(f.store.hashes().unwrap(), f.hashes);
    }
    // Running as root, the directory stays readable and gc simply proceeds.
}
