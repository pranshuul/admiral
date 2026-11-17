use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The crates of the build plan's Architecture table.
const ARCHITECTURE_CRATES: [&str; 14] = [
    "admiral-model",
    "admiral-config",
    "admiral-store",
    "admiral-scan",
    "admiral-adapter",
    "admiral-validate",
    "admiral-runtime",
    "admiral-decider",
    "admiral-cli",
    "admiral-tui",
    "admiral-lifecycle",
    "admiral-board",
    "admiral-dashboard",
    "admiral-observe",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn manifest(path: &Path) -> toml::Table {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    text.parse()
        .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

#[test]
fn workspace_has_every_architecture_crate() {
    let root = workspace_root();
    let members: BTreeSet<String> = manifest(&root.join("Cargo.toml"))["workspace"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap().to_owned())
        .collect();
    for name in ARCHITECTURE_CRATES {
        let dir = format!("crates/{name}");
        assert!(members.contains(&dir), "{dir} is not a workspace member");
        let package = manifest(&root.join(&dir).join("Cargo.toml"));
        assert_eq!(package["package"]["name"].as_str(), Some(name));
    }
}

#[test]
fn every_crate_but_the_cli_is_a_library() {
    let root = workspace_root();
    for name in ARCHITECTURE_CRATES {
        if name == "admiral-cli" {
            continue;
        }
        let dir = root.join("crates").join(name);
        assert!(dir.join("src/lib.rs").is_file(), "{name} has no src/lib.rs");
        assert!(
            !dir.join("src/main.rs").exists(),
            "{name} must not have a binary"
        );
    }
}
