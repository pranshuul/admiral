//! `config/1`: the policy fields of `config.toml`, `policy.toml` and the project `config.toml`.
//!
//! Every field carries an `x-combine` annotation naming its combination rule; the loader's field-type
//! table is generated from those annotations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a field's values from the four layers combine into one effective value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Combine {
    /// Set of locks or rules: every layer's entries.
    Union,
    /// Numeric limit: the smallest value.
    Minimum,
    /// Permission flag: logical AND.
    And,
    /// Ordered strictness level: the strictest.
    Strictest,
    /// Allow-list: only entries every layer allows.
    Intersection,
}

impl Combine {
    /// The annotation value written in the schema.
    pub fn as_str(self) -> &'static str {
        match self {
            Combine::Union => "union",
            Combine::Minimum => "minimum",
            Combine::And => "and",
            Combine::Strictest => "strictest",
            Combine::Intersection => "intersection",
        }
    }

    /// Parses an annotation value.
    pub fn parse(text: &str) -> Option<Combine> {
        match text {
            "union" => Some(Combine::Union),
            "minimum" => Some(Combine::Minimum),
            "and" => Some(Combine::And),
            "strictest" => Some(Combine::Strictest),
            "intersection" => Some(Combine::Intersection),
            _ => None,
        }
    }
}

/// Evidence tier, loosest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    Asserted,
    Heuristic,
    Reported,
    Verified,
}

/// Which content a lock applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    Human,
    Machine,
    Any,
}

/// How a lock is enforced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Enforce {
    Context,
    Hook,
    Both,
}

/// What happens when a hook's output differs from the model's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OnDisagree {
    #[default]
    Rewrite,
    Warn,
    Fail,
}

/// One `[[lock]]` entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Lock {
    pub skill: String,
    pub audience: Audience,
    pub enforce: Enforce,
    #[serde(default)]
    pub on_disagree: OnDisagree,
    /// An organization lock marked mandatory cannot be skipped.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mandatory: bool,
}

/// Workflow-level budget limits, each a numeric limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Budget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub rounds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub cost_usd: Option<f64>,
}

/// Scanner policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Scan {
    /// Highest score (0–100) an item may have; the lower, the stricter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-combine" = "strictest", "x-strictest" = "lowest"))]
    pub threshold: Option<u8>,
}

/// The `config/1` document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "config/1")]
pub struct Config {
    #[serde(default, rename = "lock", skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("x-combine" = "union"))]
    pub locks: Vec<Lock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("x-combine" = "union"))]
    pub required_approvals: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("x-combine" = "union"))]
    pub forbidden_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub max_loops: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub max_attempts: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "minimum"))]
    pub max_concurrency: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "and"))]
    pub deploy_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "and"))]
    pub network_allowed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "and"))]
    pub allow_headless_deploy: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "strictest", "x-levels" = ["asserted", "heuristic", "reported", "verified"]))]
    pub min_evidence: Option<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan: Option<Scan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "intersection"))]
    pub deploy_targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "intersection"))]
    pub credential_names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "intersection"))]
    pub discovery_sources: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "intersection"))]
    pub decider_backends: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-combine" = "intersection"))]
    pub models: Option<Vec<String>>,
}

/// The `config/1` JSON Schema as a JSON value, including the `x-combine` annotations.
pub fn config_schema() -> serde_json::Value {
    schemars::schema_for!(Config).to_value()
}
