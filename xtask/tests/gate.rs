//! gates/a0.sh against a fake `cargo` on PATH, so the gate's output and exit code are checked
//! without building the workspace again.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A `cargo` that appends its arguments to `calls`, fails when the arguments contain the text in
/// FAKE_CARGO_FAIL, and otherwise prints what the real harness prints for a passing scenario.
fn fake_cargo(dir: &Path) {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = format!(
        "#!/bin/sh\n\
         echo \"$*\" >> '{calls}'\n\
         case \"$*\" in *\"$FAKE_CARGO_FAIL\"*) [ -n \"$FAKE_CARGO_FAIL\" ] && {{ echo \"fake cargo failed: $*\" >&2; exit 1; }} ;; esac\n\
         case \"$*\" in *admiral-harness*) echo \"${{FAKE_HARNESS_OUT:-PASS two-node}}\" ;; esac\n\
         exit 0\n",
        calls = dir.join("calls").display()
    );
    let path = bin.join("cargo");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn gate(env: &[(&str, &str)]) -> (Output, String) {
    let dir = tempfile::tempdir().unwrap();
    fake_cargo(dir.path());
    let path = format!(
        "{}:{}",
        dir.path().join("bin").display(),
        std::env::var("PATH").unwrap()
    );
    let mut command = Command::new("bash");
    command
        .arg(root().join("gates/a0.sh"))
        .current_dir(dir.path())
        .env("PATH", path);
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command.output().expect("bash runs gates/a0.sh");
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap_or_default();
    (out, calls)
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

#[test]
fn the_gate_script_is_executable() {
    let mode = fs::metadata(root().join("gates/a0.sh"))
        .expect("gates/a0.sh exists")
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0);
}

#[test]
fn passes_both_bullets_and_exits_0() {
    let (out, _) = gate(&[]);
    assert_eq!(
        stdout(&out).lines().collect::<Vec<_>>(),
        ["PASS 1", "PASS 2"],
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn bullet_1_runs_every_ci_check_and_the_ci_definition_test() {
    let (_, calls) = gate(&[]);
    for expected in [
        "build",
        "clippy --all-targets -- -D warnings",
        "fmt --check",
        "xtask schemas-check",
        "test -p xtask --test ci",
    ] {
        assert!(
            calls.lines().any(|line| line.contains(expected)),
            "no call containing {expected:?} in\n{calls}"
        );
    }
}

#[test]
fn bullet_2_runs_the_two_node_scenario_against_the_stub_runtime() {
    let (_, calls) = gate(&[]);
    let call = calls
        .lines()
        .find(|line| line.contains("admiral-harness"))
        .unwrap_or_else(|| panic!("the harness was not run:\n{calls}"));
    assert!(call.contains("scenarios/two-node.json"), "{call}");
    assert!(call.contains("fixtures/stub-admiral.sh"), "{call}");
}

#[test]
fn a_failing_ci_check_fails_bullet_1_with_a_reason_and_still_runs_bullet_2() {
    let (out, _) = gate(&[("FAKE_CARGO_FAIL", "fmt --check")]);
    let text = stdout(&out);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with("FAIL 1 "), "{text}");
    assert!(lines[0].len() > "FAIL 1 ".len(), "reason missing: {text}");
    assert_eq!(lines[1], "PASS 2");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn a_failing_harness_run_fails_bullet_2() {
    let (out, _) = gate(&[("FAKE_CARGO_FAIL", "admiral-harness")]);
    let text = stdout(&out);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines[0], "PASS 1", "{text}");
    assert!(lines[1].starts_with("FAIL 2 "), "{text}");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn a_harness_that_does_not_report_pass_fails_bullet_2() {
    let (out, _) = gate(&[("FAKE_HARNESS_OUT", "FAIL two-node: step 1")]);
    let text = stdout(&out);
    assert!(text.lines().any(|l| l.starts_with("FAIL 2 ")), "{text}");
    assert_eq!(out.status.code(), Some(1));
}
