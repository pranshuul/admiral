/// A refusal with a stable code, a message and a remedy.
///
/// Codes: `E1002` no such entry, `E1003` a tree that cannot be hashed, `E1004` a malformed hash,
/// `E1005` a file system failure. (`E1001`, hash mismatch, belongs to install.)
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}\n  remedy: {remedy}")]
pub struct StoreError {
    pub code: &'static str,
    pub message: String,
    pub remedy: String,
}

impl StoreError {
    pub fn new(code: &'static str, message: String, remedy: &str) -> Self {
        StoreError {
            code,
            message,
            remedy: remedy.to_string(),
        }
    }
}
