//! Running the scanner and judging its result.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use admiral_model::config::Scan;
use admiral_model::findings::{Findings, ScanMode, Verdict};

use crate::ScanError;
use crate::normalize::{Normalized, skillspector};

/// The command `admiral init` writes: SkillSpector's static scan.
pub const DEFAULT_COMMAND: [&str; 6] = [
    "skillspector",
    "scan",
    "{path}",
    "--format",
    "json",
    "--no-llm",
];

/// The scan settings after layering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanSettings {
    /// `None`: no scanner is configured.
    pub command: Option<Vec<String>>,
    pub timeout_secs: u64,
    /// Effective threshold; `None` when no layer sets one.
    pub threshold: Option<u8>,
    pub deep_provider: String,
    pub deep_max_per_day: u32,
}

impl ScanSettings {
    /// Settings from an effective `[scan]` table, defaults applied.
    pub fn from_config(scan: Option<&Scan>) -> Self {
        ScanSettings {
            command: scan.and_then(|s| s.command.clone()),
            timeout_secs: scan.and_then(|s| s.timeout_secs).unwrap_or(300),
            threshold: scan.and_then(|s| s.threshold),
            deep_provider: scan
                .and_then(|s| s.deep_provider.clone())
                .unwrap_or_else(|| "claude_cli".into()),
            deep_max_per_day: scan.and_then(|s| s.deep_max_per_day).unwrap_or(20),
        }
    }
}

/// What the caller knows about the scan being asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanRequest {
    /// The user asked for a deep scan (`--deep`, or chosen in the trust review).
    pub deep: bool,
    /// First install from a source not seen before.
    pub first_from_source: bool,
    /// The item comes from an authored source.
    pub authored: bool,
    /// Automatic deep scans already run today.
    pub deep_scans_today: u32,
}

/// What happened about escalation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Escalation {
    NotNeeded,
    Done,
    /// The deep scan could not run or be read; the static result stands.
    Failed(String),
    /// Wanted, but today's allowance is spent: ask in the trust review.
    BudgetExhausted,
    /// Wanted, but the item is from an authored source and the provider is not local.
    WithheldAuthored,
}

/// A completed scan.
#[derive(Clone, Debug, PartialEq)]
pub struct Scanned {
    pub findings: Findings,
    /// `score > threshold`.
    pub above_threshold: bool,
    pub escalation: Escalation,
    /// `metadata.inference_usage` of the deep scan, for the event log.
    pub inference_usage: Option<serde_json::Value>,
}

impl Scanned {
    /// The install is refused unless the trust review approves: above threshold or verdict `fail`.
    pub fn refused(&self) -> bool {
        self.above_threshold || self.findings.verdict == Verdict::Fail
    }
}

/// The result of asking for a scan.
#[derive(Clone, Debug, PartialEq)]
pub enum ScanOutcome {
    /// No scanner is configured.
    Unscanned,
    Scanned(Box<Scanned>),
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
    validate_item(item)?;
    let argv = match &settings.command {
        Some(argv) if !argv.is_empty() => argv,
        _ => return Ok(ScanOutcome::Unscanned),
    };
    let root = std::path::absolute(root)
        .map_err(|e| failed(format!("cannot resolve {}: {e}", root.display())))?;
    fs::create_dir_all(out_dir)
        .map_err(|e| failed(format!("cannot create {}: {e}", out_dir.display())))?;

    let timeout = Duration::from_secs(settings.timeout_secs);
    let static_raw = out_dir.join(format!("{item}.static.raw.json"));
    let report = run_command(argv, &root, &[], timeout)?;
    save(&static_raw, &report)?;
    let first = skillspector(
        item,
        &report,
        &static_raw.to_string_lossy(),
        ScanMode::Static,
    )?;

    let reasons = reasons(&first, settings, request);
    let mut escalation = Escalation::NotNeeded;
    let mut chosen = first.clone();
    let mut inference_usage = None;
    if !reasons.is_empty() {
        let automatic_only = !request.deep;
        if automatic_only
            && request.authored
            && !LOCAL_PROVIDERS.contains(&settings.deep_provider.as_str())
        {
            escalation = Escalation::WithheldAuthored;
        } else if automatic_only && request.deep_scans_today >= settings.deep_max_per_day {
            escalation = Escalation::BudgetExhausted;
        } else {
            let deep_argv: Vec<String> =
                argv.iter().filter(|a| *a != "--no-llm").cloned().collect();
            let deep_raw = out_dir.join(format!("{item}.deep.raw.json"));
            let env = [("SKILLSPECTOR_PROVIDER", settings.deep_provider.as_str())];
            let attempt = run_command(&deep_argv, &root, &env, timeout).and_then(|text| {
                save(&deep_raw, &text)?;
                skillspector(item, &text, &deep_raw.to_string_lossy(), ScanMode::Deep)
            });
            match attempt {
                Ok(mut deep) => {
                    deep.findings.escalated_because = reasons;
                    inference_usage = deep.inference_usage.clone();
                    chosen = deep;
                    escalation = Escalation::Done;
                }
                Err(e) => escalation = Escalation::Failed(e.to_string()),
            }
        }
    }

    let findings = chosen.findings;
    let document = serde_json::to_value(&findings)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .map_err(|e| failed(format!("cannot write findings: {e}")))?;
    save(
        &out_dir.join(format!("{item}.findings.json")),
        &(document + "\n"),
    )?;
    Ok(ScanOutcome::Scanned(Box::new(Scanned {
        above_threshold: settings.threshold.is_some_and(|t| findings.score > t),
        findings,
        escalation,
        inference_usage,
    })))
}

/// Providers that keep the item's files on the machine.
const LOCAL_PROVIDERS: [&str; 1] = ["ollama"];

/// Why the result warrants an AI scan: the automatic reasons, which a clean pass with no findings
/// never has, then `requested`.
fn reasons(first: &Normalized, settings: &ScanSettings, request: &ScanRequest) -> Vec<String> {
    let mut out = Vec::new();
    let f = &first.findings;
    let clean = f.verdict == Verdict::Pass && f.findings.is_empty();
    if !clean {
        if f.verdict == Verdict::Warn {
            out.push("warn");
        }
        if settings
            .threshold
            .is_some_and(|t| f.score.abs_diff(t) <= 15)
        {
            out.push("near_threshold");
        }
        if first.instructions_surface {
            out.push("instructions_surface");
        }
        if first.has_executable_scripts {
            out.push("executable_scripts");
        }
        if request.first_from_source {
            out.push("first_from_source");
        }
    }
    if request.deep {
        out.push("requested");
    }
    out.into_iter().map(String::from).collect()
}

fn validate_item(item: &str) -> Result<(), ScanError> {
    if item.len() == 64 && item.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        Ok(())
    } else {
        Err(ScanError::new(
            "E1004",
            format!("{item:?} is not a store id (64 lowercase hex digits)"),
            "Pass the SHA-256 tree hash of the item.",
        ))
    }
}

fn failed(message: String) -> ScanError {
    ScanError::new(
        "E1011",
        message,
        "Check that [scan] command runs, or raise [scan] timeout_secs. A failed scan is never a pass.",
    )
}

fn save(path: &Path, text: &str) -> Result<(), ScanError> {
    fs::write(path, text).map_err(|e| failed(format!("cannot write {}: {e}", path.display())))
}

/// Runs `argv` (no shell) with `{path}` replaced by `root`; returns stdout when the exit code is
/// 0 or 1. Kills the child at the timeout.
fn run_command(
    argv: &[String],
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<String, ScanError> {
    let args: Vec<String> = argv
        .iter()
        .map(|a| {
            if a == "{path}" {
                root.to_string_lossy().into_owned()
            } else {
                a.clone()
            }
        })
        .collect();
    let name = args[0].clone();
    let mut child = Command::new(&name)
        .args(&args[1..])
        .envs(env.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| failed(format!("cannot run {name}: {e}")))?;
    let (tx, rx) = mpsc::channel();
    for (which, pipe) in [
        (
            0,
            child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn Read + Send>),
        ),
        (
            1,
            child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn Read + Send>),
        ),
    ] {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            let _ = tx.send((which, bytes));
        });
    }
    drop(tx);

    let deadline = Instant::now() + timeout;
    let timed_out = |child: &mut std::process::Child| {
        let _ = child.kill();
        let _ = child.wait();
        failed(format!(
            "{name} did not finish within {} seconds",
            timeout.as_secs()
        ))
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => return Err(timed_out(&mut child)),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(failed(format!("cannot wait for {name}: {e}"))),
        }
    };
    let mut streams = [Vec::new(), Vec::new()];
    for _ in 0..2 {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok((which, bytes)) => streams[which] = bytes,
            Err(_) => return Err(timed_out(&mut child)),
        }
    }
    let stdout = String::from_utf8_lossy(&streams[0]).into_owned();
    match status.code() {
        Some(0 | 1) => Ok(stdout),
        code => {
            let stderr = String::from_utf8_lossy(&streams[1]);
            let tail: String = stderr.trim().chars().take(300).collect();
            Err(failed(format!(
                "{name} exited with {} (only 0 and 1 are results): {tail}",
                code.map_or("a signal".to_string(), |c| c.to_string())
            )))
        }
    }
}
