use std::process::Command;

fn xtask(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn schemas_check_passes_on_the_repository() {
    assert!(xtask(&["schemas-check"]).status.success());
}

#[test]
fn unknown_command_is_a_usage_error() {
    let out = xtask(&["nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
}
