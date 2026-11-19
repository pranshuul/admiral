use std::fs;
use std::path::Path;

fn workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    fs::read_to_string(path).expect(".github/workflows/ci.yml exists")
}

#[test]
fn runs_on_linux_x86_64_linux_aarch64_and_macos_aarch64() {
    let ci = workflow();
    for runner in ["ubuntu-24.04", "ubuntu-24.04-arm", "macos-14"] {
        assert!(
            ci.contains(&format!("- {runner}")),
            "missing runner {runner}"
        );
    }
}

#[test]
fn runs_every_check() {
    let ci = workflow();
    for command in [
        "cargo build",
        "cargo test",
        "cargo clippy --all-targets -- -D warnings",
        "cargo fmt --check",
        "cargo xtask schemas-check",
    ] {
        assert!(
            ci.contains(&format!("run: {command}")),
            "missing step {command}"
        );
    }
}

#[test]
fn runs_on_push_and_pull_request() {
    let ci = workflow();
    assert!(ci.contains("push:"));
    assert!(ci.contains("pull_request:"));
}

#[test]
fn does_not_cancel_a_failing_target_early() {
    assert!(workflow().contains("fail-fast: false"));
}
