//! `findings/1`: a scanner's output, normalized.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The overall judgment of a scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

/// How serious one finding is.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Which kind of scan produced the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    /// The default command, without AI analysis.
    Static,
    /// A rerun with AI analysis.
    Deep,
}

/// The scanner that produced the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ScannerInfo {
    pub name: String,
    pub version: String,
}

/// Where a finding is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Location {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub line: Option<u64>,
}

/// One issue a scanner reported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub category: String,
    pub severity: Severity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: Option<f64>,
}

/// The `findings/1` document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "findings/1", deny_unknown_fields)]
pub struct Findings {
    #[schemars(extend("const" = "findings/1"))]
    pub schema: String,
    /// The store id (tree hash) of the scanned item.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub item: String,
    pub scanner: ScannerInfo,
    #[schemars(range(min = 0, max = 100))]
    pub score: u8,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    /// Path of the saved raw report.
    pub raw: String,
    pub mode: ScanMode,
    /// Why a deep scan was run; empty for a static scan.
    pub escalated_because: Vec<String>,
}

/// The `findings/1` JSON Schema as a JSON value.
pub fn findings_schema() -> serde_json::Value {
    todo!("red")
}
