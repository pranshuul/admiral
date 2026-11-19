use std::collections::BTreeMap;

/// Every JSON Schema generated from the model, keyed by file name under `schemas/`.
///
/// Phase 0 introduces no file format, so the set is empty; each phase that freezes a format adds it here.
pub fn schemas() -> BTreeMap<String, String> {
    BTreeMap::new()
}
