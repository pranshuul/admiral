//! Loading the four configuration layers and the tighten-only combination.

use std::collections::BTreeMap;
use std::path::PathBuf;

use admiral_model::config::{Combine, Config};
use serde::{Deserialize, Serialize};

/// A configuration layer, outermost (outranking) first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Organization,
    User,
    Workflow,
    Project,
}

/// One layer's file name and text.
#[derive(Clone, Debug)]
pub struct LayerInput {
    pub layer: Layer,
    pub file: PathBuf,
    pub text: String,
}

/// Where each layer lives; a layer without a path, or whose file does not exist, is absent.
#[derive(Clone, Debug, Default)]
pub struct LayerPaths {
    pub organization: Option<PathBuf>,
    pub user: Option<PathBuf>,
    pub workflow: Option<PathBuf>,
    pub project: Option<PathBuf>,
}

/// The combination rule and ordering of one field, from the model's annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRule {
    pub combine: Combine,
    /// For `Strictest` fields ranked by name: levels from loosest to strictest.
    pub levels: Option<Vec<String>>,
    /// For numeric `Strictest` fields: the lowest value is the strictest.
    pub lowest_is_strictest: bool,
}

/// Which layer an effective value came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// The layer that decided a single value.
    Layer(Layer),
    /// For sets: the layer that contributed each element, in element order.
    Elements(Vec<Layer>),
}

/// What a finding reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingKind {
    /// A project value that would loosen a field, dropped.
    Ignored,
}

/// A finding naming the file and line it comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub file: PathBuf,
    pub line: usize,
    pub field: String,
    pub message: String,
}

/// The combined configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub config: Config,
    /// Keyed by dotted field path, for example `max_loops` or `budget.tokens`.
    pub origins: BTreeMap<String, Origin>,
    pub findings: Vec<Finding>,
}

/// A refusal with a stable code, a message and a remedy.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}\n  remedy: {remedy}")]
pub struct ConfigError {
    pub code: &'static str,
    pub message: String,
    pub remedy: String,
}

/// The field-type table, generated from the `admiral-model` annotations; keyed by dotted field path.
pub fn field_table() -> BTreeMap<String, FieldRule> {
    todo!("red")
}

/// Reads the layer files that exist.
pub fn read_layers(paths: &LayerPaths) -> Result<Vec<LayerInput>, ConfigError> {
    let _ = paths;
    todo!("red")
}

/// Parses the layers and combines them; a project value that loosens is dropped and reported.
pub fn load(inputs: &[LayerInput]) -> Result<Loaded, ConfigError> {
    let _ = inputs;
    todo!("red")
}
