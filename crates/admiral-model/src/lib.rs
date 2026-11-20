use std::collections::BTreeMap;

pub mod config;
pub mod findings;
pub mod tree;

/// Every JSON Schema generated from the model, keyed by file name under `schemas/`.
///
/// Each phase that freezes a format adds it here.
pub fn schemas() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert(
        "config-1.json".to_string(),
        render(&config::config_schema()),
    );
    out.insert(
        "findings-1.json".to_string(),
        render(&findings::findings_schema()),
    );
    out.insert("tree-1.json".to_string(), render(&tree::tree_schema()));
    out
}

/// Pretty JSON with sorted keys and a trailing newline.
fn render(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("a JSON value always serializes");
    text.push('\n');
    text
}
