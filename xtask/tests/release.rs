use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("{relative} exists: {e}"))
}

fn dist() -> toml::Table {
    read("dist-workspace.toml")
        .parse::<toml::Table>()
        .expect("dist-workspace.toml is valid TOML")
}

fn dist_table() -> toml::Table {
    dist()["dist"].as_table().expect("[dist] table").clone()
}

fn strings(table: &toml::Table, key: &str) -> Vec<String> {
    table[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is an array"))
        .iter()
        .map(|v| v.as_str().expect("string").to_owned())
        .collect()
}

#[test]
fn dist_builds_static_linux_and_macos_binaries() {
    let targets = strings(&dist_table(), "targets");
    for target in [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "aarch64-apple-darwin",
    ] {
        assert!(targets.iter().any(|t| t == target), "missing {target}");
    }
    assert!(
        targets
            .iter()
            .filter(|t| t.contains("linux"))
            .all(|t| t.ends_with("-musl")),
        "Linux targets must be statically linked (musl)"
    );
}

#[test]
fn dist_produces_shell_installer_and_homebrew_formula() {
    let installers = strings(&dist_table(), "installers");
    assert!(installers.contains(&"shell".to_owned()));
    assert!(installers.contains(&"homebrew".to_owned()));
}

#[test]
fn dist_publishes_to_a_homebrew_tap_named_admiral() {
    let table = dist_table();
    let tap = table["tap"].as_str().expect("tap");
    assert!(tap.ends_with("/homebrew-tap"), "tap is {tap}");
    assert_eq!(table["formula"].as_str(), Some("admiral"));
    assert!(strings(&table, "publish-jobs").contains(&"homebrew".to_owned()));
}

#[test]
fn dist_writes_checksums_and_signs_with_attestations() {
    let table = dist_table();
    assert_eq!(table["checksum"].as_str(), Some("sha256"));
    assert_eq!(table["github-attestations"].as_bool(), Some(true));
    assert_eq!(table["ci"].as_str(), Some("github"));
}

#[test]
fn dist_version_is_pinned() {
    let version = dist_table()["cargo-dist-version"]
        .as_str()
        .expect("version")
        .to_owned();
    assert!(
        version.split('.').count() == 3 && version.chars().all(|c| c.is_ascii_digit() || c == '.'),
        "not an exact version: {version}"
    );
}

#[test]
fn release_workflow_runs_on_version_tags_and_attests() {
    let workflow = read(".github/workflows/release.yml");
    assert!(workflow.contains("push:"));
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("\"attestations\": \"write\""));
    assert!(workflow.contains("actions/attest"));
    assert!(workflow.contains("HOMEBREW_TAP_TOKEN"));
    assert!(workflow.contains("musl"));
    assert!(workflow.contains("aarch64-apple-darwin"));
}

#[test]
fn release_workflow_pins_the_same_dist_version_as_the_config() {
    let version = dist_table()["cargo-dist-version"]
        .as_str()
        .expect("version")
        .to_owned();
    assert!(read(".github/workflows/release.yml").contains(&version));
}

#[test]
fn released_package_names_repository_and_homepage() {
    let workspace = read("Cargo.toml").parse::<toml::Table>().expect("TOML");
    let package = workspace["workspace"]["package"]
        .as_table()
        .expect("package");
    assert!(
        package["repository"]
            .as_str()
            .expect("repository")
            .starts_with("https://github.com/")
    );
    assert!(package.contains_key("homepage"));
    let cli = read("crates/admiral-cli/Cargo.toml");
    assert!(cli.contains("repository.workspace = true"));
    assert!(cli.contains("homepage.workspace = true"));
}

fn ops(args: &[&str], gh: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(root().join("ops/a0.sh")).args(args);
    command.current_dir(root());
    if let Some(gh) = gh {
        command.env("GH", gh);
    }
    command.output().expect("ops/a0.sh runs")
}

fn stub_gh(dir: &Path, repo_ok: bool, secret_ok: bool) -> PathBuf {
    let path = dir.join("gh");
    let script = format!(
        "#!/bin/sh\necho \"$@\" >> \"{log}\"\ncase \"$1 $2\" in\n\
         \"repo view\") exit {repo};;\n\
         \"secret list\") if [ {secret} = 0 ]; then echo HOMEBREW_TAP_TOKEN; fi; exit 0;;\n\
         esac\nexit 1\n",
        log = dir.join("calls.log").display(),
        repo = if repo_ok { 0 } else { 1 },
        secret = if secret_ok { 0 } else { 1 },
    );
    fs::write(&path, script).expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    path
}

#[test]
fn ops_dry_run_prints_the_release_check_and_calls_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gh = stub_gh(dir.path(), true, true);
    let out = ops(&["--dry-run"], Some(&gh));
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("release-prereqs"), "{text}");
    assert!(text.contains("homebrew-tap"), "{text}");
    assert!(text.contains("HOMEBREW_TAP_TOKEN"), "{text}");
    assert!(!dir.path().join("calls.log").exists(), "dry run called gh");
}

#[test]
fn ops_passes_when_tap_and_secret_exist_and_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gh = stub_gh(dir.path(), true, true);
    let first = ops(&[], Some(&gh));
    let second = ops(&[], Some(&gh));
    assert!(first.status.success(), "{first:?}");
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn ops_fails_naming_the_missing_tap_repository() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gh = stub_gh(dir.path(), false, true);
    let out = ops(&[], Some(&gh));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("homebrew-tap"));
}

#[test]
fn ops_fails_naming_the_missing_secret() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gh = stub_gh(dir.path(), true, false);
    let out = ops(&[], Some(&gh));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("HOMEBREW_TAP_TOKEN"));
}

#[test]
fn ops_refuses_unknown_arguments() {
    let out = ops(&["--frobnicate"], None);
    assert!(!out.status.success());
}
