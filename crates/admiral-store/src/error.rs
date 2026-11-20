#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}\n  remedy: {remedy}")]
pub struct StoreError {
    pub code: &'static str,
    pub message: String,
    pub remedy: String,
}

impl StoreError {
    pub fn new(_code: &'static str, _message: String, _remedy: &str) -> Self {
        todo!("red")
    }
}
