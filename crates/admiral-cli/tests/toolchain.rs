use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_toml(name: &str) -> toml::Table {
    let text = fs::read_to_string(root().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    text.parse().unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn toolchain_is_pinned_to_an_exact_stable_version() {
    let t = read_toml("rust-toolchain.toml");
    let tc = t["toolchain"].as_table().expect("[toolchain] table");
    let channel = tc["channel"].as_str().expect("channel");
    let parts: Vec<&str> = channel.split('.').collect();
    assert_eq!(parts.len(), 3, "channel must be x.y.z, got {channel}");
    assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()), "{channel}");
}

#[test]
fn toolchain_installs_rustfmt_and_clippy() {
    let t = read_toml("rust-toolchain.toml");
    let comps: Vec<&str> = t["toolchain"]["components"]
        .as_array()
        .expect("components")
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(comps.contains(&"rustfmt"));
    assert!(comps.contains(&"clippy"));
}

#[test]
fn running_compiler_matches_the_pin() {
    let t = read_toml("rust-toolchain.toml");
    let channel = t["toolchain"]["channel"].as_str().unwrap().to_string();
    let out = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("rustc");
    let v = String::from_utf8(out.stdout).unwrap();
    assert!(v.starts_with(&format!("rustc {channel} ")), "{v}");
}

#[test]
fn rustfmt_config_exists() {
    let t = read_toml("rustfmt.toml");
    assert_eq!(t["edition"].as_str(), Some("2024"));
}

#[test]
fn workspace_denies_warnings() {
    let t = read_toml("Cargo.toml");
    assert_eq!(
        t["workspace"]["lints"]["rust"]["warnings"].as_str(),
        Some("deny")
    );
    assert_eq!(
        t["workspace"]["lints"]["clippy"]["all"]["level"].as_str(),
        Some("deny")
    );
}

#[test]
fn every_member_inherits_workspace_lints() {
    let ws = read_toml("Cargo.toml");
    let members = ws["workspace"]["members"].as_array().unwrap();
    assert!(!members.is_empty());
    for m in members {
        let m = m.as_str().unwrap();
        let t = read_toml(&format!("{m}/Cargo.toml"));
        assert_eq!(t["lints"]["workspace"].as_bool(), Some(true), "{m}");
    }
}
