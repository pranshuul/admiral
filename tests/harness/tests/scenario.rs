mod common;

use admiral_harness::{FORMAT, Scenario, parse};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[test]
fn parses_the_two_node_scenario() {
    let scenario = parse(&common::two_node_text()).unwrap();
    assert_eq!(scenario.format, FORMAT);
    assert_eq!(scenario.name, "two-node");
    assert_eq!(scenario.steps.len(), 2);
    let first = &scenario.steps[0];
    assert_eq!(first.expect_next["node"], "plan");
    let advance = first.advance.as_ref().unwrap();
    assert_eq!(advance.node, "plan");
    assert_eq!(advance.result, json!({ "plan": "one step" }));
    assert_eq!(advance.expect["node"], "build");
}

#[test]
fn rejects_another_format_version() {
    let text = common::two_node_text().replace("scenario/1", "scenario/2");
    let err = parse(&text).unwrap_err();
    assert!(err.contains("scenario/1"), "{err}");
}

#[test]
fn rejects_unknown_fields() {
    let text = common::two_node_text().replace("\"name\"", "\"surprise\": 1, \"name\"");
    assert!(parse(&text).is_err());
}

#[test]
fn rejects_a_scenario_without_steps() {
    let err = parse(r#"{"format":"scenario/1","name":"empty","steps":[]}"#).unwrap_err();
    assert!(err.contains("steps"), "{err}");
}

#[test]
fn rejects_an_empty_name() {
    let err =
        parse(r#"{"format":"scenario/1","name":"","steps":[{"expect_next":{}}]}"#).unwrap_err();
    assert!(err.contains("name"), "{err}");
}

#[test]
fn rejects_text_that_is_not_json() {
    assert!(parse("format = 'scenario/1'").is_err());
}

fn keys(value: &Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn committed_schema_describes_the_scenario_type() {
    let text = std::fs::read_to_string(common::crate_dir().join("scenario-1.schema.json")).unwrap();
    let schema: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(schema["$id"], FORMAT);
    assert_eq!(schema["properties"]["format"]["const"], FORMAT);

    // A scenario with every optional field set serializes to exactly the fields the schema lists.
    let full: Scenario = serde_json::from_value(json!({
        "format": FORMAT,
        "name": "full",
        "steps": [{
            "child": "item-1",
            "expect_next": {},
            "advance": { "node": "n", "result": {}, "usage": {}, "ack": [], "expect": {} }
        }]
    }))
    .unwrap();
    let value = serde_json::to_value(&full).unwrap();
    assert_eq!(keys(&value), keys(&schema["properties"]));
    let step = &value["steps"][0];
    assert_eq!(
        keys(step),
        keys(&schema["properties"]["steps"]["items"]["properties"])
    );
    let advance_props =
        &schema["properties"]["steps"]["items"]["properties"]["advance"]["properties"];
    assert_eq!(keys(&step["advance"]), keys(advance_props));
}

#[test]
fn every_committed_scenario_parses() {
    for entry in std::fs::read_dir(common::crate_dir().join("scenarios")).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
