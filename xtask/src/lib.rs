use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Compares generated schemas with the `*.json` files in `dir`; returns one message per difference.
/// A missing directory counts as holding no schemas.
pub fn check_schemas(generated: &BTreeMap<String, String>, dir: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut committed = BTreeMap::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.ends_with(".json") {
                            match fs::read_to_string(entry.path()) {
                                Ok(text) => {
                                    committed.insert(name, text);
                                }
                                Err(e) => errors.push(format!("cannot read {name}: {e}")),
                            }
                        }
                    }
                    Err(e) => errors.push(format!("cannot list {}: {e}", dir.display())),
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => errors.push(format!("cannot list {}: {e}", dir.display())),
    }
    for (name, text) in generated {
        match committed.get(name) {
            None => errors.push(format!("{name}: generated but not committed")),
            Some(old) if old != text => {
                errors.push(format!("{name}: committed schema differs from generated"));
            }
            Some(_) => {}
        }
    }
    for name in committed.keys() {
        if !generated.contains_key(name) {
            errors.push(format!("{name}: committed but not generated"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
