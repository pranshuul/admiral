use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xtask::check_schemas;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("admiral-xtask-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn generated(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn equal_schemas_pass() {
    let dir = scratch("equal");
    fs::write(dir.join("a.json"), "{}\n").unwrap();
    assert_eq!(
        check_schemas(&generated(&[("a.json", "{}\n")]), &dir),
        Ok(())
    );
}

#[test]
fn empty_generated_and_missing_directory_pass() {
    let dir = scratch("missing").join("none");
    assert_eq!(check_schemas(&BTreeMap::new(), &dir), Ok(()));
}

#[test]
fn changed_content_is_reported() {
    let dir = scratch("changed");
    fs::write(dir.join("a.json"), "{}\n").unwrap();
    let errors = check_schemas(&generated(&[("a.json", "{\"x\":1}\n")]), &dir).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("a.json"));
}

#[test]
fn generated_but_not_committed_is_reported() {
    let dir = scratch("uncommitted");
    let errors = check_schemas(&generated(&[("a.json", "{}\n")]), &dir).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("a.json"));
}

#[test]
fn committed_but_not_generated_is_reported() {
    let dir = scratch("stale");
    fs::write(dir.join("old.json"), "{}\n").unwrap();
    fs::write(dir.join("README.md"), "ignored").unwrap();
    let errors = check_schemas(&BTreeMap::new(), &dir).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("old.json"));
}

#[test]
fn committed_schemas_equal_generated_schemas() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    assert_eq!(
        check_schemas(&admiral_model::schemas(), &root.join("schemas")),
        Ok(())
    );
}
