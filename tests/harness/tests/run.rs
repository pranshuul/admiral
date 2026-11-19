mod common;

use admiral_harness::{Runtime, parse, run};

fn scenario(text: &str) -> admiral_harness::Scenario {
    parse(text).unwrap()
}

#[test]
fn drives_the_two_node_workflow_through_next_and_advance() {
    let (_dir, runtime) = common::stub_runtime();
    run(&scenario(&common::two_node_text()), &runtime).unwrap();
}

#[test]
fn advances_with_each_result_in_a_file() {
    let (dir, runtime) = common::stub_runtime();
    run(&scenario(&common::two_node_text()), &runtime).unwrap();
    let log = std::fs::read_to_string(dir.path().join(".stub-log")).unwrap();
    assert!(log.contains("advance advance plan --result "), "{log}");
    assert!(log.contains("--json"), "{log}");
    assert!(log.contains(r#"{"plan":"one step"}"#), "{log}");
    assert!(log.contains(r#"{"built":true}"#), "{log}");
}

#[test]
fn passes_child_usage_and_ack_to_advance() {
    let text = r#"{
      "format": "scenario/1", "name": "flags",
      "steps": [{
        "child": "item-7",
        "expect_next": {"node":"plan","kind":"agent","task":"write the plan"},
        "advance": {
          "node": "plan", "result": {},
          "usage": {"tokens": 12}, "ack": [{"directive_id": "d1", "resolution": "followed"}],
          "expect": {"node":"build","kind":"agent","task":"build the plan"}
        }
      }]
    }"#;
    let (dir, runtime) = common::stub_runtime();
    run(&scenario(text), &runtime).unwrap();
    let log = std::fs::read_to_string(dir.path().join(".stub-log")).unwrap();
    assert!(log.contains("--child item-7"), "{log}");
    assert!(log.contains(r#"--usage {"tokens":12}"#), "{log}");
    assert!(
        log.contains(r#"--ack [{"directive_id":"d1","resolution":"followed"}]"#),
        "{log}"
    );
}

#[test]
fn a_step_without_advance_only_checks_next() {
    let text = r#"{
      "format": "scenario/1", "name": "next-only",
      "steps": [{"expect_next": {"node":"plan","kind":"agent","task":"write the plan"}}]
    }"#;
    let (dir, runtime) = common::stub_runtime();
    run(&scenario(text), &runtime).unwrap();
    assert!(!dir.path().join(".stub-log").exists());
}

#[test]
fn fails_on_an_unexpected_next_response() {
    let text = common::two_node_text().replace("write the plan", "write something else");
    let (_dir, runtime) = common::stub_runtime();
    let err = run(&scenario(&text), &runtime).unwrap_err();
    assert!(err.contains("step 1"), "{err}");
    assert!(err.contains("next"), "{err}");
    assert!(err.contains("write something else"), "{err}");
    assert!(err.contains("write the plan"), "{err}");
}

#[test]
fn fails_on_an_unexpected_advance_response() {
    let text = common::two_node_text().replace(r#""kind": "end""#, r#""kind": "ask""#);
    let (_dir, runtime) = common::stub_runtime();
    let err = run(&scenario(&text), &runtime).unwrap_err();
    assert!(err.contains("step 2"), "{err}");
    assert!(err.contains("advance"), "{err}");
}

#[test]
fn fails_when_the_runtime_refuses_an_advance() {
    let text = common::two_node_text().replacen(
        r#""node": "plan",
        "result""#,
        r#""node": "build",
        "result""#,
        1,
    );
    let (_dir, runtime) = common::stub_runtime();
    let err = run(&scenario(&text), &runtime).unwrap_err();
    assert!(err.contains("step 1"), "{err}");
    assert!(err.contains("exit"), "{err}");
    assert!(err.contains("current node is plan"), "{err}");
}

#[test]
fn fails_when_the_runtime_prints_something_that_is_not_json() {
    let (_dir, mut runtime) = common::stub_runtime();
    runtime.program = "/bin/echo".into();
    let err = run(&scenario(&common::two_node_text()), &runtime).unwrap_err();
    assert!(err.contains("step 1"), "{err}");
    assert!(err.contains("JSON"), "{err}");
}

#[test]
fn fails_when_the_runtime_cannot_be_started() {
    let (dir, _) = common::stub_runtime();
    let runtime = Runtime {
        program: dir.path().join("no-such-runtime"),
        dir: dir.path().to_path_buf(),
    };
    let err = run(&scenario(&common::two_node_text()), &runtime).unwrap_err();
    assert!(err.contains("no-such-runtime"), "{err}");
}
