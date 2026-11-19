use assert_cmd::Command;

#[test]
fn version_flag_prints_name_and_version() {
    Command::cargo_bin("admiral")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("admiral {}\n", env!("CARGO_PKG_VERSION")))
        .stderr("");
}
