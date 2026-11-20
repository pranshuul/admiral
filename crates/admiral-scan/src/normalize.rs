//! The SkillSpector normalizer: its JSON report to `findings/1`.

use admiral_model::findings::{
    Finding, Findings, Location, ScanMode, ScannerInfo, Severity, Verdict,
};
use serde_json::Value;

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
///
/// Reads `risk_assessment.score` (0–100) and `recommendation` (SAFE, CAUTION or DO_NOT_INSTALL),
/// every issue's `id`, `severity`, and optional `category`, `message` or `description`,
/// `confidence` and `location`; ignores every other key. Anything unreadable is `E1012`.
pub fn skillspector(
    item: &str,
    report: &str,
    raw: &str,
    mode: ScanMode,
) -> Result<Normalized, ScanError> {
    if item.len() != 64 || !item.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(ScanError::new(
            "E1004",
            format!("{item:?} is not a store id (64 lowercase hex digits)"),
            "Pass the SHA-256 tree hash of the item.",
        ));
    }
    let doc: Value = serde_json::from_str(report)
        .map_err(|e| unparseable(format!("the scanner's output is not JSON: {e}")))?;
    let assessment = doc
        .get("risk_assessment")
        .ok_or_else(|| unparseable("the report has no risk_assessment".into()))?;
    let score = score_of(assessment)?;
    let verdict = match assessment.get("recommendation").and_then(Value::as_str) {
        Some("SAFE") => Verdict::Pass,
        Some("CAUTION") => Verdict::Warn,
        Some("DO_NOT_INSTALL") => Verdict::Fail,
        other => {
            return Err(unparseable(format!(
                "risk_assessment.recommendation is {other:?}, not SAFE, CAUTION or DO_NOT_INSTALL"
            )));
        }
    };
    let mut findings = Vec::new();
    let mut instructions_surface = false;
    let issues = match doc.get("issues") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(issues)) => issues.as_slice(),
        Some(_) => return Err(unparseable("issues is not a list".into())),
    };
    for (n, issue) in issues.iter().enumerate() {
        if issue
            .get("surface")
            .and_then(Value::as_str)
            .is_some_and(|s| s.eq_ignore_ascii_case("instructions"))
        {
            instructions_surface = true;
        }
        findings.push(finding_of(n, issue)?);
    }
    let metadata = doc.get("metadata");
    let version = metadata
        .and_then(|m| m.get("skillspector_version"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(Normalized {
        findings: Findings {
            schema: "findings/1".to_string(),
            item: item.to_string(),
            scanner: ScannerInfo {
                name: "skillspector".to_string(),
                version,
            },
            score,
            verdict,
            findings,
            raw: raw.to_string(),
            mode,
            escalated_because: Vec::new(),
        },
        has_executable_scripts: metadata
            .and_then(|m| m.get("has_executable_scripts"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        instructions_surface,
        inference_usage: metadata
            .and_then(|m| m.get("inference_usage"))
            .filter(|v| !v.is_null())
            .cloned(),
    })
}

fn unparseable(message: String) -> ScanError {
    ScanError::new(
        "E1012",
        message,
        "Check that [scan] command prints SkillSpector's JSON report (--format json) and that the scanner is up to date.",
    )
}

fn score_of(assessment: &Value) -> Result<u8, ScanError> {
    let score = assessment
        .get("score")
        .and_then(Value::as_f64)
        .ok_or_else(|| unparseable("risk_assessment.score is missing or not a number".into()))?;
    if !(0.0..=100.0).contains(&score) {
        return Err(unparseable(format!(
            "risk_assessment.score {score} is outside 0-100"
        )));
    }
    Ok(score.round() as u8)
}

fn finding_of(n: usize, issue: &Value) -> Result<Finding, ScanError> {
    let text = |key: &str| issue.get(key).and_then(Value::as_str);
    let id = text("id")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| unparseable(format!("issue {n} has no id")))?;
    let severity = match text("severity").map(str::to_ascii_lowercase).as_deref() {
        Some("low") => Severity::Low,
        Some("medium") => Severity::Medium,
        Some("high") => Severity::High,
        Some("critical") => Severity::Critical,
        other => {
            return Err(unparseable(format!(
                "issue {id} has severity {other:?}, not LOW, MEDIUM, HIGH or CRITICAL"
            )));
        }
    };
    let location = issue.get("location").and_then(|loc| {
        Some(Location {
            file: loc.get("file")?.as_str()?.to_string(),
            line: loc
                .get("start_line")
                .and_then(Value::as_u64)
                .filter(|line| *line >= 1),
        })
    });
    Ok(Finding {
        id: id.to_string(),
        category: text("category").unwrap_or("unknown").to_string(),
        severity,
        message: text("message")
            .or_else(|| text("description"))
            .unwrap_or(id)
            .to_string(),
        location,
        confidence: issue
            .get("confidence")
            .and_then(Value::as_f64)
            .filter(|c| (0.0..=1.0).contains(c)),
    })
}
