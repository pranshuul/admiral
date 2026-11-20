#![allow(dead_code)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A git repository at `dir` holding `files`, committed on `main`.
pub fn repo(dir: &Path, files: &[(&str, &str)]) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", "main"]);
    for (path, text) in files {
        write(&dir.join(path), text);
    }
    commit(dir, "initial");
    dir.to_path_buf()
}

pub fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", message]);
}

pub fn url(dir: &Path) -> String {
    dir.to_str().unwrap().to_string()
}

pub const SKILL: &str = "---\nname: x\ndescription: d\n---\nbody\n";
