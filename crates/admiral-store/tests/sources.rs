mod common;

use std::fs;
use std::path::Path;

use admiral_store::sources::{SourceKind, Sources};
use common::{SKILL, commit, git, repo, url, write};

struct Fixture {
    _tmp: tempfile::TempDir,
    sources: Sources,
    dir: std::path::PathBuf,
    upstream: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = repo(
        &tmp.path().join("upstream"),
        &[("skills/one/SKILL.md", SKILL), ("agents/a.md", "agent")],
    );
    let dir = tmp.path().join("home/sources");
    Fixture {
        sources: Sources::new(&dir),
        dir,
        upstream,
        _tmp: tmp,
    }
}

fn head(dir: &Path) -> String {
    git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn add_clones_into_the_sources_directory_and_lists_what_it_finds() {
    let f = fixture();
    let added = f
        .sources
        .add("mine", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    assert_eq!(added.path, f.dir.join("mine"));
    assert!(f.dir.join("mine/.git").is_dir());
    assert!(f.dir.join("mine/skills/one/SKILL.md").is_file());
    let ids: Vec<String> = added.items.iter().map(|i| i.id()).collect();
    assert_eq!(ids, ["mine/one", "mine/a"]);
    assert_eq!(head(&f.dir.join("mine")), head(&f.upstream));
}

#[test]
fn an_authored_source_is_cloned_the_same_way() {
    let f = fixture();
    let added = f
        .sources
        .add("fleet", &url(&f.upstream), SourceKind::Authored)
        .unwrap();
    assert_eq!(added.items.len(), 2);
    assert!(f.dir.join("fleet/.git").is_dir());
}

#[test]
fn a_second_source_with_the_same_alias_is_refused_and_the_first_is_untouched() {
    let f = fixture();
    f.sources
        .add("mine", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    let other = repo(
        &f.upstream.parent().unwrap().join("other"),
        &[("skills/zzz/SKILL.md", SKILL)],
    );
    let err = f
        .sources
        .add("mine", &url(&other), SourceKind::Authored)
        .unwrap_err();
    assert_eq!(err.code, "E1006");
    assert!(err.message.contains("mine"));
    assert!(!err.remedy.is_empty());
    assert!(!f.dir.join("mine/skills/zzz").exists());
    assert_eq!(head(&f.dir.join("mine")), head(&f.upstream));
}

#[test]
fn aliases_that_differ_only_in_case_collide() {
    let f = fixture();
    f.sources
        .add("Mine", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    let err = f
        .sources
        .add("mine", &url(&f.upstream), SourceKind::Consumed)
        .unwrap_err();
    assert_eq!(err.code, "E1006");
}

#[test]
fn a_collision_is_refused_before_anything_is_fetched() {
    let f = fixture();
    f.sources
        .add("mine", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    let err = f
        .sources
        .add("mine", "/nonexistent/repository", SourceKind::Consumed)
        .unwrap_err();
    assert_eq!(err.code, "E1006");
}

#[test]
fn names_that_are_not_plain_aliases_are_refused() {
    let f = fixture();
    for name in ["", "a/b", ".hidden", "..", "has space", "-x"] {
        let err = f
            .sources
            .add(name, &url(&f.upstream), SourceKind::Consumed)
            .unwrap_err();
        assert_eq!(err.code, "E1007", "{name:?}");
    }
    assert!(!f.dir.exists() || fs::read_dir(&f.dir).unwrap().next().is_none());
}

#[test]
fn a_clone_that_fails_leaves_nothing_behind() {
    let f = fixture();
    let err = f
        .sources
        .add("bad", "/nonexistent/repository", SourceKind::Consumed)
        .unwrap_err();
    assert_eq!(err.code, "E1008");
    assert!(!f.dir.join("bad").exists());
    assert!(
        fs::read_dir(&f.dir)
            .map(|mut r| r.next().is_none())
            .unwrap_or(true)
    );
}

#[test]
fn a_source_whose_items_collide_is_refused_and_not_kept() {
    let f = fixture();
    let dup = repo(
        &f.upstream.parent().unwrap().join("dup"),
        &[("a/same/SKILL.md", SKILL), ("b/same/SKILL.md", SKILL)],
    );
    let err = f
        .sources
        .add("dup", &url(&dup), SourceKind::Consumed)
        .unwrap_err();
    assert_eq!(err.code, "E1009");
    assert!(!f.dir.join("dup").exists());
    assert!(
        fs::read_dir(&f.dir)
            .map(|mut r| r.next().is_none())
            .unwrap_or(true)
    );
}

#[test]
fn pulling_a_consumed_source_hard_resets_it_to_upstream() {
    let f = fixture();
    f.sources
        .add("c", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    let clone = f.dir.join("c");
    // Local damage: an edit, an untracked-then-committed file and a local commit.
    write(&clone.join("skills/one/SKILL.md"), "edited");
    write(&clone.join("local.txt"), "mine");
    commit(&clone, "local work");
    write(&clone.join("agents/a.md"), "dirty");
    // Upstream moves on.
    write(&f.upstream.join("skills/two/SKILL.md"), SKILL);
    commit(&f.upstream, "add two");

    let items = f.sources.pull("c", SourceKind::Consumed).unwrap();
    assert_eq!(head(&clone), head(&f.upstream));
    assert!(!clone.join("local.txt").exists());
    assert_eq!(
        fs::read_to_string(clone.join("agents/a.md")).unwrap(),
        "agent"
    );
    assert_eq!(git(&clone, &["status", "--porcelain"]), "");
    let ids: Vec<String> = items.iter().map(|i| i.id()).collect();
    assert_eq!(ids, ["c/one", "c/two", "c/a"]);
}

#[test]
fn a_consumed_source_follows_a_rewritten_upstream() {
    let f = fixture();
    f.sources
        .add("c", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    write(&f.upstream.join("agents/a.md"), "amended");
    git(&f.upstream, &["add", "-A"]);
    git(&f.upstream, &["commit", "-q", "--amend", "-m", "rewritten"]);
    f.sources.pull("c", SourceKind::Consumed).unwrap();
    let clone = f.dir.join("c");
    assert_eq!(head(&clone), head(&f.upstream));
    assert_eq!(
        fs::read_to_string(clone.join("agents/a.md")).unwrap(),
        "amended"
    );
}

#[test]
fn pulling_an_authored_source_merges_and_keeps_local_commits() {
    let f = fixture();
    f.sources
        .add("a", &url(&f.upstream), SourceKind::Authored)
        .unwrap();
    let clone = f.dir.join("a");
    git(&clone, &["config", "user.name", "t"]);
    git(&clone, &["config", "user.email", "t@example.com"]);
    git(&clone, &["config", "commit.gpgsign", "false"]);
    write(&clone.join("agents/mine.md"), "local");
    commit(&clone, "local work");
    write(&f.upstream.join("skills/two/SKILL.md"), SKILL);
    commit(&f.upstream, "add two");

    let items = f.sources.pull("a", SourceKind::Authored).unwrap();
    assert!(clone.join("agents/mine.md").is_file());
    assert!(clone.join("skills/two/SKILL.md").is_file());
    assert_eq!(
        git(&clone, &["rev-list", "--parents", "-n1", "HEAD"])
            .split(' ')
            .count(),
        3
    );
    let ids: Vec<String> = items.iter().map(|i| i.id()).collect();
    assert_eq!(ids, ["a/one", "a/two", "a/a", "a/mine"]);
}

#[test]
fn pulling_an_authored_source_fast_forwards_when_nothing_is_local() {
    let f = fixture();
    f.sources
        .add("a", &url(&f.upstream), SourceKind::Authored)
        .unwrap();
    write(&f.upstream.join("agents/new.md"), "n");
    commit(&f.upstream, "new");
    f.sources.pull("a", SourceKind::Authored).unwrap();
    assert_eq!(head(&f.dir.join("a")), head(&f.upstream));
}

#[test]
fn a_conflicting_authored_pull_is_an_error_and_the_clone_is_left_for_the_user() {
    let f = fixture();
    f.sources
        .add("a", &url(&f.upstream), SourceKind::Authored)
        .unwrap();
    let clone = f.dir.join("a");
    git(&clone, &["config", "user.name", "t"]);
    git(&clone, &["config", "user.email", "t@example.com"]);
    git(&clone, &["config", "commit.gpgsign", "false"]);
    write(&clone.join("agents/a.md"), "local");
    commit(&clone, "local");
    write(&f.upstream.join("agents/a.md"), "upstream");
    commit(&f.upstream, "upstream");
    let err = f.sources.pull("a", SourceKind::Authored).unwrap_err();
    assert_eq!(err.code, "E1008");
    assert!(!err.remedy.is_empty());
}

#[test]
fn pulling_an_unknown_source_is_refused() {
    let f = fixture();
    let err = f.sources.pull("nope", SourceKind::Consumed).unwrap_err();
    assert_eq!(err.code, "E1010");
}

#[test]
fn names_lists_the_registered_clones_sorted() {
    let f = fixture();
    assert!(f.sources.names().unwrap().is_empty());
    f.sources
        .add("b", &url(&f.upstream), SourceKind::Consumed)
        .unwrap();
    f.sources
        .add("a", &url(&f.upstream), SourceKind::Authored)
        .unwrap();
    assert_eq!(f.sources.names().unwrap(), ["a", "b"]);
}
