use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

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
pub fn parse(_text: &str) -> Result<Scenario, String> {
    todo!("red")
}

/// Plays the scenario against the runtime; the error names the first step that differs.
pub fn run(_scenario: &Scenario, _runtime: &Runtime) -> Result<(), String> {
    todo!("red")
}
