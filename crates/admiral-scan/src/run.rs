//! Running the scanner and judging its result.

use admiral_model::config::Scan;
use admiral_model::findings::Findings;

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
        let _ = scan;
        todo!("red")
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
        todo!("red")
    }
}

/// The result of asking for a scan.
#[derive(Clone, Debug, PartialEq)]
pub enum ScanOutcome {
    /// No scanner is configured.
    Unscanned,
    Scanned(Box<Scanned>),
}
