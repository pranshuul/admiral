mod common;

use std::path::PathBuf;

use admiral_store::discovery::{Item, ItemKind, discover};
use common::{SKILL, repo};

fn ids(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .map(|i| format!("{:?} {}", i.kind, i.id()))
        .collect()
}

#[test]
fn finds_skills_agents_and_workflows_by_convention() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[
            ("skills/alpha/SKILL.md", SKILL),
            ("skills/alpha/references/notes.md", "n"),
            ("agents/reviewer.md", "agent"),
            ("workflows/flow/workflow.toml", "name = \"flow\"\n"),
            ("README.md", "readme"),
        ],
    );
    let items = discover(&root, "src").unwrap();
    assert_eq!(
        ids(&items),
        ["Skill src/alpha", "Agent src/reviewer", "Workflow src/flow"]
    );
    assert_eq!(items[0].path, PathBuf::from("skills/alpha"));
    assert_eq!(items[1].path, PathBuf::from("agents/reviewer.md"));
    assert_eq!(items[2].path, PathBuf::from("workflows/flow"));
}

#[test]
fn a_skill_directory_is_found_at_any_depth_and_named_after_its_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[
            ("plugins/p/skills/deep/SKILL.md", SKILL),
            ("top/SKILL.md", SKILL),
        ],
    );
    let items = discover(&root, "s").unwrap();
    assert_eq!(ids(&items), ["Skill s/deep", "Skill s/top"]);
    assert_eq!(items[0].path, PathBuf::from("plugins/p/skills/deep"));
}

#[test]
fn a_skill_at_the_source_root_takes_the_source_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(tmp.path(), &[("SKILL.md", SKILL)]);
    let items = discover(&root, "solo").unwrap();
    assert_eq!(ids(&items), ["Skill solo/solo"]);
    assert_eq!(items[0].path, PathBuf::from("."));
}

#[test]
fn only_direct_markdown_files_of_the_root_agents_directory_are_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[
            ("agents/one.md", "a"),
            ("agents/notes.txt", "t"),
            ("agents/nested/two.md", "a"),
            ("skills/s/agents/three.md", "a"),
        ],
    );
    assert_eq!(ids(&discover(&root, "s").unwrap()), ["Agent s/one"]);
}

#[test]
fn a_workflow_needs_its_workflow_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[
            ("workflows/has/workflow.toml", ""),
            ("workflows/lacks/README.md", ""),
            ("workflows/loose.toml", ""),
            ("workflows/has/nested/workflow.toml", ""),
        ],
    );
    assert_eq!(ids(&discover(&root, "s").unwrap()), ["Workflow s/has"]);
}

#[test]
fn the_git_directory_is_not_walked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(tmp.path(), &[("skills/a/SKILL.md", SKILL)]);
    common::write(&root.join(".git/hidden/SKILL.md"), SKILL);
    assert_eq!(ids(&discover(&root, "s").unwrap()), ["Skill s/a"]);
}

#[test]
fn a_workflow_directory_with_an_orchestrator_is_both_a_skill_and_a_workflow() {
    let hackathon =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hackathon");
    let items = discover(&hackathon, "hack").unwrap();
    let count = |kind: ItemKind| items.iter().filter(|i| i.kind == kind).count();
    assert_eq!(count(ItemKind::Agent), 8);
    assert_eq!(count(ItemKind::Skill), 3);
    assert_eq!(count(ItemKind::Workflow), 1);
    assert!(
        items
            .iter()
            .any(|i| i.id() == "hack/hackathon" && i.kind == ItemKind::Workflow)
    );
}

#[test]
fn output_is_sorted_by_kind_then_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[
            ("workflows/b/workflow.toml", ""),
            ("agents/z.md", ""),
            ("agents/a.md", ""),
            ("skills/m/SKILL.md", SKILL),
            ("skills/c/SKILL.md", SKILL),
            ("workflows/a/workflow.toml", ""),
        ],
    );
    assert_eq!(
        ids(&discover(&root, "s").unwrap()),
        [
            "Skill s/c",
            "Skill s/m",
            "Agent s/a",
            "Agent s/z",
            "Workflow s/a",
            "Workflow s/b"
        ]
    );
}

#[test]
fn two_items_of_one_kind_with_the_same_name_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[("a/same/SKILL.md", SKILL), ("b/same/SKILL.md", SKILL)],
    );
    let err = discover(&root, "s").unwrap_err();
    assert_eq!(err.code, "E1009");
    assert!(err.message.contains("s/same"));
    assert!(err.message.contains("a/same") && err.message.contains("b/same"));
}

#[test]
fn a_skill_and_an_agent_may_share_a_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = repo(
        tmp.path(),
        &[("skills/same/SKILL.md", SKILL), ("agents/same.md", "a")],
    );
    assert_eq!(
        ids(&discover(&root, "s").unwrap()),
        ["Skill s/same", "Agent s/same"]
    );
}
