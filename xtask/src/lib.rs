use std::collections::BTreeMap;
use std::path::Path;

/// Compares generated schemas with the `*.json` files in `dir`; returns one message per difference.
pub fn check_schemas(generated: &BTreeMap<String, String>, dir: &Path) -> Result<(), Vec<String>> {
    let _ = (generated, dir);
    todo!("red")
}
