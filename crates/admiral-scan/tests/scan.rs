use std::fs;
use std::path::{Path, PathBuf};

use admiral_model::config::Scan;
use admiral_model::findings::{Findings, ScanMode, Verdict};
use admiral_scan::{Escalation, ScanOutcome, ScanRequest, ScanSettings, Scanned, scan};

const ITEM: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAFE: &str = include_str!("fixtures/safe.json");
const CAUTION: &str = include_str!("fixtures/caution.json");
const DANGER: &str = include_str!("fixtures/danger.json");

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("item")).unwrap();
        Env { dir }
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("item")
    }
    fn out(&self) -> PathBuf {
        self.dir.path().join("out")
    }
    /// A fake scanner: prints `stdout`, exits `code`; records its arguments and the provider.
    fn fake(&self, name: &str, stdout: &str, code: i32) -> Vec<String> {
        let report = self.dir.path().join(format!("{name}.report"));
        fs::write(&report, stdout).unwrap();
        let script = self.dir.path().join(format!("{name}.sh"));
        let log = self.dir.path().join("calls.log");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\necho \"$SKILLSPECTOR_PROVIDER|$*\" >> '{}'\ncat '{}'\nexit {code}\n",
                log.display(),
                report.display()
            ),
        )
        .unwrap();
        vec![
            "sh".into(),
            script.display().to_string(),
            "{path}".into(),
            "--no-llm".into(),
        ]
    }
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("calls.log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
    fn settings(&self, command: Option<Vec<String>>, threshold: Option<u8>) -> ScanSettings {
        ScanSettings {
            command,
            timeout_secs: 20,
            threshold,
            deep_provider: "claude_cli".into(),
            deep_max_per_day: 20,
        }
    }
    fn run(
        &self,
        settings: &ScanSettings,
        request: &ScanRequest,
    ) -> Result<ScanOutcome, admiral_scan::ScanError> {
        scan(ITEM, &self.root(), &self.out(), settings, request)
    }
}

fn scanned(outcome: ScanOutcome) -> Scanned {
    match outcome {
        ScanOutcome::Scanned(s) => *s,
        ScanOutcome::Unscanned => panic!("expected a scan"),
    }
}

#[test]
fn no_scanner_configured_returns_unscanned_and_runs_nothing() {
    let env = Env::new();
    let none = env.run(&env.settings(None, Some(50)), &ScanRequest::default());
    assert_eq!(none.unwrap(), ScanOutcome::Unscanned);
    let empty = env.run(
        &env.settings(Some(vec![]), Some(50)),
        &ScanRequest::default(),
    );
    assert_eq!(empty.unwrap(), ScanOutcome::Unscanned);
    assert!(!env.out().exists());
}

#[test]
fn a_safe_item_passes_and_its_report_and_findings_are_saved() {
    let env = Env::new();
    let cmd = env.fake("a", SAFE, 0);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(50)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.findings.verdict, Verdict::Pass);
    assert!(!s.above_threshold);
    assert!(!s.refused());
    assert_eq!(s.escalation, Escalation::NotNeeded);
    assert_eq!(
        fs::read_to_string(&s.findings.raw).unwrap(),
        SAFE,
        "the raw report is saved byte for byte"
    );
    assert!(Path::new(&s.findings.raw).starts_with(env.out()));
    let saved: Findings = serde_json::from_str(
        &fs::read_to_string(env.out().join(format!("{ITEM}.findings.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, s.findings);
}

#[test]
fn the_path_placeholder_is_replaced_by_the_absolute_root_without_a_shell() {
    let env = Env::new();
    let cmd = env.fake("a", SAFE, 0);
    env.run(&env.settings(Some(cmd), None), &ScanRequest::default())
        .unwrap();
    let calls = env.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], format!("|{} --no-llm", env.root().display()));
}

#[test]
fn exit_one_is_a_valid_result_and_above_threshold_is_refused() {
    let env = Env::new();
    let cmd = env.fake("a", DANGER, 1);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(50)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.findings.score, 91);
    assert!(s.above_threshold);
    assert!(s.refused());
}

#[test]
fn score_equal_to_the_threshold_is_not_above_it() {
    let env = Env::new();
    let cmd = env.fake("a", CAUTION, 0);
    let req = ScanRequest {
        deep_scans_today: 20,
        ..Default::default()
    };
    let s = scanned(
        env.run(&env.settings(Some(cmd.clone()), Some(38)), &req)
            .unwrap(),
    );
    assert!(!s.above_threshold);
    let s = scanned(env.run(&env.settings(Some(cmd), Some(37)), &req).unwrap());
    assert!(s.above_threshold);
    assert!(s.refused());
}

#[test]
fn a_fail_verdict_is_refused_even_with_no_threshold_or_a_low_score() {
    let env = Env::new();
    let report = DANGER.replace("\"score\": 91", "\"score\": 10");
    let cmd = env.fake("a", &report, 0);
    let s = scanned(
        env.run(&env.settings(Some(cmd), None), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.findings.verdict, Verdict::Fail);
    assert!(!s.above_threshold);
    assert!(s.refused());
}

#[test]
fn a_warn_with_no_deep_provider_run_is_not_refused() {
    let env = Env::new();
    let cmd = env.fake("a", CAUTION, 0);
    let req = ScanRequest {
        deep_scans_today: 20,
        ..Default::default()
    };
    let s = scanned(env.run(&env.settings(Some(cmd), Some(90)), &req).unwrap());
    assert_eq!(s.findings.verdict, Verdict::Warn);
    assert!(!s.refused());
}

#[test]
fn exit_two_a_missing_command_and_a_timeout_are_e1011() {
    let env = Env::new();
    let cmd = env.fake("a", "boom", 2);
    let e = env
        .run(&env.settings(Some(cmd), Some(50)), &ScanRequest::default())
        .unwrap_err();
    assert_eq!(e.code, "E1011");

    let missing = vec!["/nonexistent/skillspector".to_string(), "{path}".into()];
    let e = env
        .run(&env.settings(Some(missing), None), &ScanRequest::default())
        .unwrap_err();
    assert_eq!(e.code, "E1011");

    let slow = env.dir.path().join("slow.sh");
    fs::write(&slow, "#!/bin/sh\nsleep 30\n").unwrap();
    let mut settings = env.settings(Some(vec!["sh".into(), slow.display().to_string()]), None);
    settings.timeout_secs = 1;
    let started = std::time::Instant::now();
    let e = env.run(&settings, &ScanRequest::default()).unwrap_err();
    assert_eq!(e.code, "E1011");
    assert!(started.elapsed().as_secs() < 15, "the child was killed");
}

#[test]
fn unparseable_output_is_e1012_and_nothing_is_saved_as_findings() {
    let env = Env::new();
    let cmd = env.fake("a", "<html>", 0);
    let e = env
        .run(&env.settings(Some(cmd), None), &ScanRequest::default())
        .unwrap_err();
    assert_eq!(e.code, "E1012");
    assert!(!env.out().join(format!("{ITEM}.findings.json")).exists());
}

#[test]
fn a_malformed_item_id_is_e1004_before_anything_runs() {
    let env = Env::new();
    let cmd = env.fake("a", SAFE, 0);
    let e = scan(
        "../x",
        &env.root(),
        &env.out(),
        &env.settings(Some(cmd), None),
        &ScanRequest::default(),
    )
    .unwrap_err();
    assert_eq!(e.code, "E1004");
    assert!(env.calls().is_empty());
}

#[test]
fn settings_default_from_config_and_read_the_scan_table() {
    let d = ScanSettings::from_config(None);
    assert_eq!(d.command, None);
    assert_eq!(d.timeout_secs, 300);
    assert_eq!(d.threshold, None);
    assert_eq!(d.deep_provider, "claude_cli");
    assert_eq!(d.deep_max_per_day, 20);

    let config: admiral_model::config::Config = toml_parse(
        "[scan]\nthreshold = 40\ncommand = [\"x\", \"{path}\"]\ntimeout_secs = 9\n\
         deep_provider = \"ollama\"\ndeep_max_per_day = 3\n",
    );
    let s = ScanSettings::from_config(config.scan.as_ref());
    assert_eq!(s.command, Some(vec!["x".to_string(), "{path}".to_string()]));
    assert_eq!(s.timeout_secs, 9);
    assert_eq!(s.threshold, Some(40));
    assert_eq!(s.deep_provider, "ollama");
    assert_eq!(s.deep_max_per_day, 3);
    let _: Option<&Scan> = config.scan.as_ref();
}

fn toml_parse(text: &str) -> admiral_model::config::Config {
    let value: toml::Value = toml::from_str(text).unwrap();
    value.try_into().unwrap()
}

#[test]
fn the_default_command_is_skillspector_static() {
    assert_eq!(
        admiral_scan::DEFAULT_COMMAND,
        [
            "skillspector",
            "scan",
            "{path}",
            "--format",
            "json",
            "--no-llm"
        ]
    );
}

// --- escalation ---

/// A fake that answers the static run with `static_report` and any run without `--no-llm` with
/// `deep_report`.
fn two_phase(
    env: &Env,
    static_report: &str,
    static_code: i32,
    deep_report: &str,
    deep_code: i32,
) -> Vec<String> {
    let sp = env.dir.path().join("static.report");
    let dp = env.dir.path().join("deep.report");
    fs::write(&sp, static_report).unwrap();
    fs::write(&dp, deep_report).unwrap();
    let script = env.dir.path().join("two.sh");
    let log = env.dir.path().join("calls.log");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$SKILLSPECTOR_PROVIDER|$*\" >> '{log}'\n\
             case \"$*\" in *--no-llm*) cat '{sp}'; exit {static_code};; *) cat '{dp}'; exit {deep_code};; esac\n",
            log = log.display(),
            sp = sp.display(),
            dp = dp.display()
        ),
    )
    .unwrap();
    vec![
        "sh".into(),
        script.display().to_string(),
        "{path}".into(),
        "--format".into(),
        "json".into(),
        "--no-llm".into(),
    ]
}

#[test]
fn a_warn_escalates_and_the_deep_result_replaces_the_static_one() {
    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, SAFE, 0);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::Done);
    assert_eq!(s.findings.mode, ScanMode::Deep);
    assert_eq!(s.findings.verdict, Verdict::Pass);
    assert!(s.findings.escalated_because.contains(&"warn".to_string()));
    let calls = env.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].starts_with('|'), "static scan sets no provider");
    assert!(calls[1].starts_with("claude_cli|"));
    assert!(!calls[1].contains("--no-llm"));
    assert!(calls[1].contains("--format json"));
    let raws: Vec<_> = fs::read_dir(env.out())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("raw"))
        .collect();
    assert_eq!(raws.len(), 2, "both raw reports are kept: {raws:?}");
    let saved: Findings = serde_json::from_str(
        &fs::read_to_string(env.out().join(format!("{ITEM}.findings.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(saved.mode, ScanMode::Deep);
}

#[test]
fn each_reason_triggers_escalation() {
    let env = Env::new();
    let with_issue = SAFE.replace(
        "\"issues\": []",
        r#""issues": [{"id": "X-1", "category": "c", "severity": "LOW", "surface": "scripts"}]"#,
    );
    let near = with_issue.replace("\"score\": 4", "\"score\": 35");
    // within 15 points of the threshold, either side
    for (report, threshold, reason) in [
        (near.as_str(), 50, "near_threshold"),
        (near.as_str(), 20, "near_threshold"),
    ] {
        let env2 = Env::new();
        let cmd = two_phase(&env2, report, 0, SAFE, 0);
        let s = scanned(
            env2.run(
                &env2.settings(Some(cmd), Some(threshold)),
                &ScanRequest::default(),
            )
            .unwrap(),
        );
        assert_eq!(s.escalation, Escalation::Done, "{reason} {threshold}");
        assert_eq!(s.findings.escalated_because, [reason]);
    }
    // an instructions-surface finding
    let cmd = two_phase(&env, DANGER, 1, SAFE, 0);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(0)), &ScanRequest::default())
            .unwrap(),
    );
    assert!(
        s.findings
            .escalated_because
            .contains(&"instructions_surface".to_string())
    );
    // executable scripts
    let env3 = Env::new();
    let scripts = with_issue.replace(
        "\"has_executable_scripts\": false",
        "\"has_executable_scripts\": true",
    );
    let cmd = two_phase(&env3, &scripts, 0, SAFE, 0);
    let s = scanned(
        env3.run(&env3.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.findings.escalated_because, ["executable_scripts"]);
    // a clean pass with scripts and no findings is still never escalated automatically
    let env5 = Env::new();
    let clean_scripts = SAFE.replace(
        "\"has_executable_scripts\": false",
        "\"has_executable_scripts\": true",
    );
    let cmd = two_phase(&env5, &clean_scripts, 0, SAFE, 0);
    let s = scanned(
        env5.run(&env5.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::NotNeeded);
    // first install from a source
    let env4 = Env::new();
    let cmd = two_phase(&env4, &with_issue, 0, SAFE, 0);
    let req = ScanRequest {
        first_from_source: true,
        ..Default::default()
    };
    let s = scanned(env4.run(&env4.settings(Some(cmd), Some(90)), &req).unwrap());
    assert_eq!(s.findings.escalated_because, ["first_from_source"]);
}

#[test]
fn a_clean_pass_is_never_escalated_automatically_but_is_when_asked() {
    let env = Env::new();
    let cmd = two_phase(&env, SAFE, 0, SAFE, 0);
    let s = scanned(
        env.run(
            &env.settings(Some(cmd.clone()), Some(90)),
            &ScanRequest::default(),
        )
        .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::NotNeeded);
    assert_eq!(s.findings.mode, ScanMode::Static);
    assert_eq!(env.calls().len(), 1);

    let env = Env::new();
    let cmd = two_phase(&env, SAFE, 0, SAFE, 0);
    let req = ScanRequest {
        deep: true,
        ..Default::default()
    };
    let s = scanned(env.run(&env.settings(Some(cmd), Some(90)), &req).unwrap());
    assert_eq!(s.escalation, Escalation::Done);
    assert_eq!(s.findings.escalated_because, ["requested"]);
}

#[test]
fn a_failed_deep_scan_leaves_the_static_result_standing() {
    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, "boom", 2);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert!(matches!(s.escalation, Escalation::Failed(ref why) if why.contains("E1011")));
    assert_eq!(s.findings.mode, ScanMode::Static);
    assert_eq!(s.findings.verdict, Verdict::Warn);
    assert!(s.findings.escalated_because.is_empty());

    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, "{not json", 0);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert!(matches!(s.escalation, Escalation::Failed(ref why) if why.contains("E1012")));
    assert_eq!(s.findings.mode, ScanMode::Static);
}

#[test]
fn deep_usage_is_returned_for_the_event_log() {
    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, DANGER, 1);
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(90)), &ScanRequest::default())
            .unwrap(),
    );
    assert_eq!(s.inference_usage.unwrap()["output_tokens"], 300);
    assert_eq!(s.findings.verdict, Verdict::Fail);
}

#[test]
fn the_daily_budget_stops_automatic_escalation_but_not_a_request() {
    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, SAFE, 0);
    let spent = ScanRequest {
        deep_scans_today: 20,
        ..Default::default()
    };
    let s = scanned(
        env.run(&env.settings(Some(cmd.clone()), Some(90)), &spent)
            .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::BudgetExhausted);
    assert_eq!(s.findings.mode, ScanMode::Static);
    assert_eq!(env.calls().len(), 1);

    let asked = ScanRequest {
        deep: true,
        deep_scans_today: 20,
        ..Default::default()
    };
    let s = scanned(env.run(&env.settings(Some(cmd), Some(90)), &asked).unwrap());
    assert_eq!(s.escalation, Escalation::Done);
}

#[test]
fn an_authored_item_escalates_automatically_only_to_a_local_provider() {
    let env = Env::new();
    let cmd = two_phase(&env, CAUTION, 0, SAFE, 0);
    let req = ScanRequest {
        authored: true,
        ..Default::default()
    };
    let s = scanned(
        env.run(&env.settings(Some(cmd.clone()), Some(90)), &req)
            .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::WithheldAuthored);
    assert_eq!(env.calls().len(), 1);

    let mut local = env.settings(Some(cmd.clone()), Some(90));
    local.deep_provider = "ollama".into();
    let s = scanned(env.run(&local, &req).unwrap());
    assert_eq!(s.escalation, Escalation::Done);
    assert!(env.calls().last().unwrap().starts_with("ollama|"));

    let explicit = ScanRequest {
        authored: true,
        deep: true,
        ..Default::default()
    };
    let s = scanned(
        env.run(&env.settings(Some(cmd), Some(90)), &explicit)
            .unwrap(),
    );
    assert_eq!(s.escalation, Escalation::Done);
}

#[test]
fn a_clean_pass_is_not_escalated_by_scripts_first_source_or_nearness() {
    let env = Env::new();
    let clean = SAFE.replace("\"score\": 4", "\"score\": 45").replace(
        "\"has_executable_scripts\": false",
        "\"has_executable_scripts\": true",
    );
    let cmd = two_phase(&env, &clean, 0, SAFE, 0);
    let req = ScanRequest {
        first_from_source: true,
        ..Default::default()
    };
    let s = scanned(env.run(&env.settings(Some(cmd), Some(50)), &req).unwrap());
    assert_eq!(s.escalation, Escalation::NotNeeded);
    assert_eq!(env.calls().len(), 1);
}
