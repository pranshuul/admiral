use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// The scenario format version this harness reads.
pub const FORMAT: &str = "scenario/1";

/// A scripted run: for each step, the `next` response the runtime must give and the
/// result to `advance` with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub format: String,
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child: Option<String>,
    pub expect_next: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance: Option<Advance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Advance {
    pub node: String,
    pub result: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ack: Option<Value>,
    pub expect: Value,
}

/// The runtime under test: a program that answers `next` and `advance`, run in `dir`.
#[derive(Debug, Clone)]
pub struct Runtime {
    pub program: PathBuf,
    pub dir: PathBuf,
}

/// Reads and checks a scenario file's text.
pub fn parse(text: &str) -> Result<Scenario, String> {
    let scenario: Scenario =
        serde_json::from_str(text).map_err(|e| format!("not a valid scenario: {e}"))?;
    if scenario.format != FORMAT {
        return Err(format!(
            "unsupported format \"{}\"; this harness reads {FORMAT}",
            scenario.format
        ));
    }
    if scenario.name.is_empty() {
        return Err("name must not be empty".to_string());
    }
    if scenario.steps.is_empty() {
        return Err("steps must hold at least one step".to_string());
    }
    Ok(scenario)
}

/// Plays the scenario against the runtime; the error names the first step that differs.
pub fn run(scenario: &Scenario, runtime: &Runtime) -> Result<(), String> {
    let results =
        tempfile::tempdir().map_err(|e| format!("cannot make a results directory: {e}"))?;
    for (index, step) in scenario.steps.iter().enumerate() {
        let n = index + 1;
        let mut args: Vec<String> = vec!["next".into()];
        if let Some(child) = &step.child {
            args.extend(["--child".into(), child.clone()]);
        }
        args.push("--json".into());
        let got = call(runtime, &args).map_err(|e| format!("step {n}: next {e}"))?;
        if got != step.expect_next {
            return Err(differs(n, "next", &step.expect_next, &got));
        }
        let Some(advance) = &step.advance else {
            continue;
        };
        let file = results.path().join(format!("step-{n}.json"));
        let body = serde_json::to_string(&advance.result).map_err(|e| format!("step {n}: {e}"))?;
        std::fs::write(&file, body)
            .map_err(|e| format!("step {n}: cannot write {}: {e}", file.display()))?;
        let mut args: Vec<String> = vec![
            "advance".into(),
            advance.node.clone(),
            "--result".into(),
            file.display().to_string(),
        ];
        if let Some(child) = &step.child {
            args.extend(["--child".into(), child.clone()]);
        }
        for (flag, value) in [("--usage", &advance.usage), ("--ack", &advance.ack)] {
            if let Some(value) = value {
                let text = serde_json::to_string(value).map_err(|e| format!("step {n}: {e}"))?;
                args.extend([flag.into(), text]);
            }
        }
        args.push("--json".into());
        let got = call(runtime, &args).map_err(|e| format!("step {n}: advance {e}"))?;
        if got != advance.expect {
            return Err(differs(n, "advance", &advance.expect, &got));
        }
    }
    Ok(())
}

fn differs(step: usize, what: &str, expected: &Value, got: &Value) -> String {
    format!("step {step}: {what} response differs: expected {expected}, got {got}")
}

/// Runs the runtime once and parses its standard output as JSON.
fn call(runtime: &Runtime, args: &[String]) -> Result<Value, String> {
    let output = Command::new(&runtime.program)
        .args(args)
        .current_dir(&runtime.dir)
        .output()
        .map_err(|e| format!("cannot start {}: {e}", runtime.program.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "failed: runtime exit status {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        format!("printed output that is not JSON ({e}): {}", stdout.trim())
    })
}
