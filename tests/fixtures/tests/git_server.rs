use admiral_fixtures::{GitServer, fleet_dir, materialize_repo};
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
fn creates_a_bare_repository_on_main() {
    let dir = tempfile::tempdir().unwrap();
    let server = GitServer::create(dir.path()).unwrap();
    assert_eq!(
        git(server.path(), &["rev-parse", "--is-bare-repository"]),
        "true"
    );
    assert_eq!(
        git(server.path(), &["symbolic-ref", "HEAD"]),
        "refs/heads/main"
    );
    assert!(server.url().ends_with("server.git"));
}

#[test]
fn creating_again_keeps_what_was_seeded() {
    let dir = tempfile::tempdir().unwrap();
    let server = GitServer::create(dir.path()).unwrap();
    server.seed(&fleet_dir()).unwrap();
    let again = GitServer::create(dir.path()).unwrap();
    assert_eq!(again.branches().unwrap(), vec!["main".to_string()]);
}

#[test]
fn a_seeded_fleet_can_be_cloned() {
    let dir = tempfile::tempdir().unwrap();
    let server = GitServer::create(dir.path()).unwrap();
    server.seed(&fleet_dir()).unwrap();
    let clone = dir.path().join("clone");
    git(
        dir.path(),
        &["clone", "-q", &server.url(), clone.to_str().unwrap()],
    );
    assert!(clone.join("skills").is_dir());
    assert!(clone.join("agents").is_dir());
}

#[test]
fn a_pushed_branch_shows_up_on_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let server = GitServer::create(dir.path()).unwrap();
    let work = dir.path().join("work");
    materialize_repo(&work).unwrap();
    git(&work, &["remote", "add", "origin", &server.url()]);
    git(&work, &["push", "-q", "origin", "main"]);
    git(&work, &["checkout", "-q", "-b", "feature/pr-1"]);
    git(&work, &["push", "-q", "origin", "feature/pr-1"]);
    assert_eq!(
        server.branches().unwrap(),
        vec!["feature/pr-1".to_string(), "main".to_string()]
    );
}
