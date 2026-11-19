use admiral_fixtures::fleet_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// The `key: value` lines between the `---` fences at the top of a Markdown file.
fn frontmatter(path: &Path) -> Vec<(String, String)> {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "{} has no frontmatter",
        path.display()
    );
    let mut out = Vec::new();
    for line in lines {
        if line == "---" {
            return out;
        }
        let (k, v) = line.split_once(':').unwrap();
        out.push((k.trim().to_string(), v.trim().to_string()));
    }
    panic!("{} frontmatter is not closed", path.display());
}

fn get<'a>(fm: &'a [(String, String)], key: &str) -> &'a str {
    fm.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("no {key}"))
}

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    v.sort();
    v
}

#[test]
fn has_exactly_two_skills_each_with_a_skill_md() {
    let skills = sorted_entries(&fleet_dir().join("skills"));
    assert_eq!(skills.len(), 2);
    for dir in skills {
        let fm = frontmatter(&dir.join("SKILL.md"));
        assert_eq!(get(&fm, "name"), dir.file_name().unwrap().to_str().unwrap());
        assert!(!get(&fm, "description").is_empty());
    }
}

#[test]
fn has_exactly_one_agent_named_after_its_file() {
    let agents = sorted_entries(&fleet_dir().join("agents"));
    assert_eq!(agents.len(), 1);
    let path = &agents[0];
    assert_eq!(path.extension().unwrap(), "md");
    let fm = frontmatter(path);
    assert_eq!(
        get(&fm, "name"),
        path.file_stem().unwrap().to_str().unwrap()
    );
    assert!(!get(&fm, "description").is_empty());
}

#[test]
fn has_a_readme_and_no_workflows() {
    assert!(fleet_dir().join("README.md").is_file());
    assert!(!fleet_dir().join("workflows").exists());
}
