use admiral_model::tree::{TreeEntry, TreeListing, TreeMode, tree_schema};

fn entry(path: &str, mode: TreeMode, sha256: &str) -> TreeEntry {
    TreeEntry {
        path: path.to_string(),
        mode,
        sha256: sha256.to_string(),
    }
}

const HELLO: &str = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03";

#[test]
fn a_line_is_path_mode_hash_separated_by_nul_and_ended_by_newline() {
    assert_eq!(
        entry("a.txt", TreeMode::Plain, HELLO).line(),
        format!("a.txt\0-\0{HELLO}\n")
    );
    assert_eq!(
        entry("bin/run", TreeMode::Executable, HELLO).line(),
        format!("bin/run\0x\0{HELLO}\n")
    );
}

#[test]
fn render_concatenates_lines_in_order() {
    let listing = TreeListing {
        entries: vec![
            entry("a", TreeMode::Plain, HELLO),
            entry("b", TreeMode::Executable, HELLO),
        ],
    };
    assert_eq!(listing.render(), format!("a\0-\0{HELLO}\nb\0x\0{HELLO}\n"));
    assert_eq!(TreeListing::default().render(), "");
}

#[test]
fn schema_is_registered_as_tree_1() {
    let schemas = admiral_model::schemas();
    let text = schemas.get("tree-1.json").expect("tree-1.json registered");
    assert!(text.ends_with('\n'));
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(value, tree_schema());
    assert_eq!(value["title"], "tree/1");
}

#[test]
fn schema_pins_mode_and_hash_shapes() {
    let schema = tree_schema();
    let text = schema.to_string();
    assert!(text.contains("^[0-9a-f]{64}$"));
    assert!(text.contains("\"x\""));
    assert!(text.contains("\"-\""));
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn listing_round_trips_through_json() {
    let listing = TreeListing {
        entries: vec![entry("a", TreeMode::Executable, HELLO)],
    };
    let json = serde_json::to_string(&listing).unwrap();
    assert!(json.contains("\"mode\":\"x\""));
    assert_eq!(serde_json::from_str::<TreeListing>(&json).unwrap(), listing);
}
