use admiral_fixtures::{materialize_repo, mixed_repo_dir};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn holds_source_in_five_languages_with_their_manifests() {
    let root = mixed_repo_dir();
    for file in [
        "Cargo.toml",
        "src/main.rs",
        "pyproject.toml",
        "app/__init__.py",
        "package.json",
        "web/index.js",
        "go.mod",
        "cmd/main.go",
        "README.md",
    ] {
        assert!(root.join(file).is_file(), "{file} is missing");
    }
}

#[test]
fn each_language_has_a_test_file() {
    let root = mixed_repo_dir();
    for file in [
        "tests/smoke.rs",
        "tests/test_app.py",
        "web/index.test.js",
        "cmd/main_test.go",
    ] {
        assert!(root.join(file).is_file(), "{file} is missing");
    }
}

#[test]
fn materializes_as_one_commit_on_main_with_a_clean_tree() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("repo");
    materialize_repo(&dest).unwrap();
    assert_eq!(git(&dest, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(git(&dest, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git(&dest, &["status", "--porcelain"]), "");
    assert!(git(&dest, &["ls-files"]).contains("web/index.js"));
}

#[test]
fn materializing_twice_gives_the_same_tree() {
    let dir = tempfile::tempdir().unwrap();
    materialize_repo(&dir.path().join("a")).unwrap();
    materialize_repo(&dir.path().join("b")).unwrap();
    assert_eq!(
        git(&dir.path().join("a"), &["rev-parse", "HEAD"]),
        git(&dir.path().join("b"), &["rev-parse", "HEAD"])
    );
}
