//! The scanner interface: run `[scan] command` on an item, normalize its output to `findings/1`,
//! compare the score with the effective threshold, and escalate to an AI scan when it is needed.

use std::path::Path;

pub mod normalize;
pub mod run;

pub use normalize::{Normalized, skillspector};
pub use run::{DEFAULT_COMMAND, Escalation, ScanOutcome, ScanRequest, ScanSettings, Scanned};

/// A refusal with a stable code, a message and a remedy.
///
/// Codes: `E1004` a malformed item id, `E1011` the scanner failed to run (not found, timed out,
/// exit code other than 0 or 1, or its report could not be saved), `E1012` the scanner's output
/// could not be understood (not JSON, or missing `risk_assessment.score`). Both E1011 and E1012
/// refuse the install; neither is ever treated as a pass.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}\n  remedy: {remedy}")]
pub struct ScanError {
    pub code: &'static str,
    pub message: String,
    pub remedy: String,
}

impl ScanError {
    pub fn new(code: &'static str, message: String, remedy: &str) -> Self {
        ScanError {
            code,
            message,
            remedy: remedy.to_string(),
        }
    }
}

/// Scans the item `item` (its store id) whose files are under `root`; raw reports and the
/// `findings/1` document are saved in `out_dir`.
pub fn scan(
    item: &str,
    root: &Path,
    out_dir: &Path,
    settings: &ScanSettings,
    request: &ScanRequest,
) -> Result<ScanOutcome, ScanError> {
    let _ = (item, root, out_dir, settings, request);
    todo!("red")
}
