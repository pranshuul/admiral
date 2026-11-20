use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::Command as Admiral;
use predicates::prelude::*;

fn upstream(dir: &Path) -> String {
    fs::create_dir_all(dir.join("skills/one")).unwrap();
    fs::write(dir.join("skills/one/SKILL.md"), "---\nname: one\n---\n").unwrap();
    fs::create_dir_all(dir.join("agents")).unwrap();
    fs::write(dir.join("agents/a.md"), "agent").unwrap();
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["add", "-A"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "commit",
            "-q",
            "-m",
            "x",
        ],
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }
    dir.to_str().unwrap().to_string()
}

fn admiral(home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    Admiral::cargo_bin("admiral")
        .unwrap()
        .env("HOME", home)
        .args(args)
        .assert()
}

#[test]
fn sources_add_clones_into_the_admiral_home_and_lists_items() {
    let tmp = tempfile::tempdir().unwrap();
    let url = upstream(&tmp.path().join("up"));
    let home = tmp.path().join("home");
    let out = admiral(&home, &["sources", "add", "mine", &url]).success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        text.contains("mine/one") && text.contains("mine/a"),
        "{text}"
    );
    assert!(home.join(".admiral/sources/mine/.git").is_dir());
}

#[test]
fn json_output_names_the_source_kind_and_items() {
    let tmp = tempfile::tempdir().unwrap();
    let url = upstream(&tmp.path().join("up"));
    let home = tmp.path().join("home");
    let out = admiral(
        &home,
        &["sources", "add", "fleet", &url, "--authored", "--json"],
    )
    .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["name"], "fleet");
    assert_eq!(v["kind"], "authored");
    assert_eq!(v["items"][0]["id"], "fleet/one");
    assert_eq!(v["items"][0]["kind"], "skill");
    assert_eq!(v["items"][1]["id"], "fleet/a");
    assert_eq!(v["items"][1]["kind"], "agent");
}

#[test]
fn dry_run_changes_nothing_but_still_refuses_a_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let url = upstream(&tmp.path().join("up"));
    let home = tmp.path().join("home");
    let out = admiral(
        &home,
        &["sources", "add", "mine", &url, "--dry-run", "--json"],
    )
    .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["dry_run"], true);
    assert_eq!(
        v["path"],
        home.join(".admiral/sources/mine").to_str().unwrap()
    );
    assert!(!home.join(".admiral").exists());

    admiral(&home, &["sources", "add", "mine", &url]).success();
    admiral(&home, &["sources", "add", "mine", &url, "--dry-run"])
        .failure()
        .stderr(predicates::str::contains("E1006"));
}

#[test]
fn a_collision_exits_nonzero_with_its_code_and_remedy() {
    let tmp = tempfile::tempdir().unwrap();
    let url = upstream(&tmp.path().join("up"));
    let home = tmp.path().join("home");
    admiral(&home, &["sources", "add", "mine", &url]).success();
    admiral(&home, &["sources", "add", "mine", &url])
        .failure()
        .stderr(predicates::str::contains("E1006").and(predicates::str::contains("remedy")));
}

#[test]
fn with_json_an_error_is_a_json_object_on_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let out = admiral(&home, &["sources", "add", "bad/name", "/x", "--json"]).failure();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["error"]["code"], "E1007");
    assert!(v["error"]["remedy"].is_string());
}
