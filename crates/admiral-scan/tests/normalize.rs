use admiral_model::findings::{ScanMode, Severity, Verdict};
use admiral_scan::skillspector;

const ITEM: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAFE: &str = include_str!("fixtures/safe.json");
const CAUTION: &str = include_str!("fixtures/caution.json");
const DANGER: &str = include_str!("fixtures/danger.json");

fn norm(report: &str) -> Result<admiral_scan::Normalized, admiral_scan::ScanError> {
    skillspector(ITEM, report, "/out/raw.json", ScanMode::Static)
}

#[test]
fn safe_maps_to_pass_with_score_and_scanner() {
    let n = norm(SAFE).unwrap();
    let f = &n.findings;
    assert_eq!(f.schema, "findings/1");
    assert_eq!(f.item, ITEM);
    assert_eq!(f.scanner.name, "skillspector");
    assert_eq!(f.scanner.version, "0.3.1");
    assert_eq!(f.score, 4);
    assert_eq!(f.verdict, Verdict::Pass);
    assert!(f.findings.is_empty());
    assert_eq!(f.raw, "/out/raw.json");
    assert_eq!(f.mode, ScanMode::Static);
    assert!(f.escalated_because.is_empty());
    assert!(!n.has_executable_scripts);
    assert!(!n.instructions_surface);
}

#[test]
fn caution_maps_to_warn_and_issues_are_converted() {
    let n = norm(CAUTION).unwrap();
    assert_eq!(n.findings.verdict, Verdict::Warn);
    assert_eq!(n.findings.score, 38);
    assert!(n.has_executable_scripts);
    let first = &n.findings.findings[0];
    assert_eq!(first.id, "EXF-002");
    assert_eq!(first.category, "data_exfiltration");
    assert_eq!(first.severity, Severity::Medium);
    assert_eq!(first.message, "Posts file contents to a remote host");
    assert_eq!(first.confidence, Some(0.7));
    let loc = first.location.as_ref().unwrap();
    assert_eq!(loc.file, "scripts/send.sh");
    assert_eq!(loc.line, Some(12));
}

#[test]
fn message_is_preferred_then_description_then_id() {
    let n = norm(CAUTION).unwrap();
    assert_eq!(n.findings.findings[1].message, "Unpinned dependency");
    assert_eq!(n.findings.findings[1].location.as_ref().unwrap().line, None);
    let report = r#"{"risk_assessment":{"score":1,"recommendation":"SAFE"},
        "issues":[{"id":"X-1","category":"c","severity":"HIGH"}]}"#;
    let n = norm(report).unwrap();
    assert_eq!(n.findings.findings[0].message, "X-1");
    assert_eq!(n.findings.findings[0].severity, Severity::High);
    assert_eq!(n.findings.scanner.version, "unknown");
    assert!(n.findings.findings[0].location.is_none());
}

#[test]
fn danger_maps_to_fail_and_reports_the_instructions_surface_and_usage() {
    let n = norm(DANGER).unwrap();
    assert_eq!(n.findings.verdict, Verdict::Fail);
    assert_eq!(n.findings.score, 91);
    assert_eq!(n.findings.findings[0].severity, Severity::Critical);
    assert!(n.instructions_surface);
    assert_eq!(n.inference_usage.unwrap()["input_tokens"], 1200);
}

#[test]
fn deep_mode_is_recorded() {
    let n = skillspector(ITEM, SAFE, "r", ScanMode::Deep).unwrap();
    assert_eq!(n.findings.mode, ScanMode::Deep);
}

#[test]
fn output_that_is_not_json_is_e1012() {
    let e = norm("not json at all").unwrap_err();
    assert_eq!(e.code, "E1012");
    assert!(!e.remedy.is_empty());
}

#[test]
fn a_missing_or_invalid_score_is_e1012() {
    for report in [
        r#"{"risk_assessment":{"recommendation":"SAFE"}}"#,
        r#"{"issues":[]}"#,
        r#"{"risk_assessment":{"score":101,"recommendation":"SAFE"}}"#,
        r#"{"risk_assessment":{"score":-1,"recommendation":"SAFE"}}"#,
        r#"{"risk_assessment":{"score":"high","recommendation":"SAFE"}}"#,
        r#"[1,2]"#,
    ] {
        assert_eq!(norm(report).unwrap_err().code, "E1012", "{report}");
    }
}

#[test]
fn an_unknown_recommendation_or_severity_is_e1012_never_a_pass() {
    for report in [
        r#"{"risk_assessment":{"score":1}}"#,
        r#"{"risk_assessment":{"score":1,"recommendation":"MAYBE"}}"#,
        r#"{"risk_assessment":{"score":1,"recommendation":"SAFE"},"issues":[{"id":"a","category":"c","severity":"SEVERE"}]}"#,
        r#"{"risk_assessment":{"score":1,"recommendation":"SAFE"},"issues":[{"category":"c","severity":"LOW"}]}"#,
    ] {
        assert_eq!(norm(report).unwrap_err().code, "E1012", "{report}");
    }
}

#[test]
fn a_malformed_item_id_is_e1004() {
    let e = skillspector("short", SAFE, "r", ScanMode::Static).unwrap_err();
    assert_eq!(e.code, "E1004");
}
