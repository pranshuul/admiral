//! The SkillSpector normalizer: its JSON report to `findings/1`.

use admiral_model::findings::{Findings, ScanMode};

use crate::ScanError;

/// A normalized report with the facts the escalation rules read from the raw one.
#[derive(Clone, Debug, PartialEq)]
pub struct Normalized {
    pub findings: Findings,
    /// `metadata.has_executable_scripts`.
    pub has_executable_scripts: bool,
    /// Whether any issue is on the `instructions` surface.
    pub instructions_surface: bool,
    /// `metadata.inference_usage`, when the scan used AI analysis.
    pub inference_usage: Option<serde_json::Value>,
}

/// Normalizes SkillSpector's JSON `report` for the item `item`; `raw` is where the report is saved.
pub fn skillspector(
    item: &str,
    report: &str,
    raw: &str,
    mode: ScanMode,
) -> Result<Normalized, ScanError> {
    let _ = (item, report, raw, mode);
    todo!("red")
}
