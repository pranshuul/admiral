use admiral_model::findings::{Findings, findings_schema};

#[test]
fn schema_is_registered_as_findings_1() {
    let schemas = admiral_model::schemas();
    let text = schemas
        .get("findings-1.json")
        .expect("findings-1.json registered");
    assert!(text.ends_with('\n'));
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(value, findings_schema());
}

#[test]
fn schema_names_the_version_and_the_required_fields() {
    let schema = findings_schema();
    assert_eq!(schema["title"], "findings/1");
    assert_eq!(schema["properties"]["schema"]["const"], "findings/1");
    let required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for field in [
        "schema",
        "item",
        "scanner",
        "score",
        "verdict",
        "findings",
        "raw",
        "mode",
        "escalated_because",
    ] {
        assert!(required.contains(&field), "{field}");
    }
}

#[test]
fn a_document_round_trips_and_omits_absent_optionals() {
    let text = r#"{"schema":"findings/1","item":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "scanner":{"name":"skillspector","version":"1"},"score":3,"verdict":"pass",
        "findings":[{"id":"a","category":"c","severity":"low","message":"m"}],
        "raw":"/r","mode":"static","escalated_because":[]}"#;
    let doc: Findings = serde_json::from_str(text).unwrap();
    let back = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        back,
        serde_json::from_str::<serde_json::Value>(text).unwrap()
    );
}
