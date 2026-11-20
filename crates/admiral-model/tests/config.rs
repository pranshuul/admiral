use admiral_model::config::{Combine, Config, config_schema};

#[test]
fn schema_is_registered_as_config_1() {
    let schemas = admiral_model::schemas();
    let text = schemas
        .get("config-1.json")
        .expect("config-1.json registered");
    assert!(text.ends_with('\n'));
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(value, config_schema());
}

#[test]
fn schema_annotates_every_policy_field_with_its_rule() {
    let schema = config_schema();
    let props = &schema["properties"];
    for (field, rule) in [
        ("lock", "union"),
        ("required_approvals", "union"),
        ("forbidden_tools", "union"),
        ("max_loops", "minimum"),
        ("max_attempts", "minimum"),
        ("max_concurrency", "minimum"),
        ("deploy_enabled", "and"),
        ("network_allowed", "and"),
        ("allow_headless_deploy", "and"),
        ("min_evidence", "strictest"),
        ("deploy_targets", "intersection"),
        ("credential_names", "intersection"),
        ("discovery_sources", "intersection"),
        ("decider_backends", "intersection"),
        ("models", "intersection"),
    ] {
        assert_eq!(props[field]["x-combine"], rule, "{field}");
        assert!(Combine::parse(rule).is_some());
    }
}

#[test]
fn combine_names_round_trip() {
    for c in [
        Combine::Union,
        Combine::Minimum,
        Combine::And,
        Combine::Strictest,
        Combine::Intersection,
    ] {
        assert_eq!(Combine::parse(c.as_str()), Some(c));
    }
    assert_eq!(Combine::parse("loosest"), None);
}

#[test]
fn config_parses_the_spec_lock_example() {
    let text = r#"
max_loops = 3
min_evidence = "verified"
[[lock]]
skill = "style/unslop"
audience = "human"
enforce = "both"
on_disagree = "rewrite"
[budget]
tokens = 4000000
[scan]
threshold = 40
"#;
    let config: Config = toml::from_str(text).unwrap();
    assert_eq!(config.locks.len(), 1);
    assert_eq!(config.max_loops, Some(3));
    assert_eq!(config.scan.unwrap().threshold, Some(40));
}

#[test]
fn scan_table_carries_the_scanner_command_and_deep_settings() {
    let config: Config = toml::from_str(
        "[scan]\nthreshold = 40\ncommand = [\"skillspector\", \"scan\", \"{path}\"]\n\
         timeout_secs = 60\ndeep_provider = \"ollama\"\ndeep_max_per_day = 5\n",
    )
    .unwrap();
    let scan = serde_json::to_value(config.scan.unwrap()).unwrap();
    assert_eq!(scan["threshold"], 40);
    assert_eq!(scan["command"][2], "{path}");
    assert_eq!(scan["timeout_secs"], 60);
    assert_eq!(scan["deep_provider"], "ollama");
    assert_eq!(scan["deep_max_per_day"], 5);
}
