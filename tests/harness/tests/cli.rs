mod common;

use assert_cmd::Command;

fn harness() -> Command {
    Command::cargo_bin("admiral-harness").unwrap()
}

fn scenario_path() -> String {
    common::crate_dir()
        .join("scenarios/two-node.json")
        .display()
        .to_string()
}

fn stub_path() -> String {
    common::crate_dir()
        .join("fixtures/stub-admiral.sh")
        .display()
        .to_string()
}

#[test]
fn passes_the_two_node_scenario() {
    let dir = tempfile::tempdir().unwrap();
    let out = harness()
        .args([&scenario_path(), "--admiral", &stub_path(), "--dir"])
        .arg(dir.path())
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "PASS two-node");
}

#[test]
fn runs_in_the_current_directory_by_default() {
    let dir = tempfile::tempdir().unwrap();
    harness()
        .current_dir(dir.path())
        .args([&scenario_path(), "--admiral", &stub_path()])
        .assert()
        .success();
    assert!(dir.path().join(".stub-state").exists());
}

#[test]
fn reports_the_failing_step_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let text = common::two_node_text().replace("build the plan", "build something else");
    let file = dir.path().join("bad.json");
    std::fs::write(&file, text).unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = harness()
        .args([file.to_str().unwrap(), "--admiral", &stub_path(), "--dir"])
        .arg(work.path())
        .assert()
        .code(1);
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.starts_with("FAIL two-node: step 1"), "{stdout}");
}

#[test]
fn exits_2_when_the_scenario_cannot_be_read() {
    let out = harness().arg("/no/such/scenario.json").assert().code(2);
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("/no/such/scenario.json"), "{stderr}");
}

#[test]
fn exits_2_when_the_scenario_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.json");
    std::fs::write(&file, r#"{"format":"scenario/9","name":"x","steps":[]}"#).unwrap();
    harness().arg(&file).assert().code(2);
}
