//! Loading the four configuration layers and the tighten-only combination.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use admiral_model::config::{Combine, Config};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml_edit::{Document, Item};

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

impl ConfigError {
    fn new(code: &'static str, message: String, remedy: &str) -> Self {
        ConfigError {
            code,
            message,
            remedy: remedy.to_string(),
        }
    }
}

/// The field-type table, generated from the `admiral-model` annotations; keyed by dotted field path.
pub fn field_table() -> BTreeMap<String, FieldRule> {
    let schema = admiral_model::config::config_schema();
    let mut table = BTreeMap::new();
    collect_rules(&schema, &schema, "", &mut table);
    table
}

fn collect_rules(
    root: &Value,
    node: &Value,
    prefix: &str,
    table: &mut BTreeMap<String, FieldRule>,
) {
    let Some(props) = node.get("properties").and_then(Value::as_object) else {
        return;
    };
    for (name, prop) in props {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        if let Some(combine) = prop
            .get("x-combine")
            .and_then(Value::as_str)
            .and_then(Combine::parse)
        {
            let levels = prop.get("x-levels").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            });
            let lowest_is_strictest =
                prop.get("x-strictest").and_then(Value::as_str) == Some("lowest");
            table.insert(
                path,
                FieldRule {
                    combine,
                    levels,
                    lowest_is_strictest,
                },
            );
        } else if let Some(target) = referenced(root, prop) {
            collect_rules(root, target, &path, table);
        }
    }
}

/// The definition a property points at, directly or as the non-null branch of an `anyOf`.
fn referenced<'a>(root: &'a Value, prop: &Value) -> Option<&'a Value> {
    let reference = prop.get("$ref").and_then(Value::as_str).or_else(|| {
        prop.get("anyOf")?
            .as_array()?
            .iter()
            .find_map(|b| b.get("$ref")?.as_str())
    })?;
    root.pointer(reference.strip_prefix('#')?)
}

/// Reads the layer files that exist.
pub fn read_layers(paths: &LayerPaths) -> Result<Vec<LayerInput>, ConfigError> {
    let mut inputs = Vec::new();
    for (layer, path) in [
        (Layer::Organization, &paths.organization),
        (Layer::User, &paths.user),
        (Layer::Workflow, &paths.workflow),
        (Layer::Project, &paths.project),
    ] {
        let Some(path) = path else { continue };
        match fs::read_to_string(path) {
            Ok(text) => inputs.push(LayerInput {
                layer,
                file: path.clone(),
                text,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(ConfigError::new(
                    "E6004",
                    format!("cannot read {}: {e}", path.display()),
                    "Check that the path is a readable file, or remove it.",
                ));
            }
        }
    }
    Ok(inputs)
}

struct Parsed<'a> {
    input: &'a LayerInput,
    doc: Document<String>,
    values: BTreeMap<String, Value>,
}

fn parse_layer<'a>(
    input: &'a LayerInput,
    table: &BTreeMap<String, FieldRule>,
) -> Result<Parsed<'a>, ConfigError> {
    let file = input.file.display();
    let doc = Document::parse(input.text.clone()).map_err(|e| {
        ConfigError::new(
            "E6001",
            format!("{file} is not valid TOML: {e}"),
            "Fix the syntax error in that file.",
        )
    })?;
    let config: Config = toml::from_str(&input.text).map_err(|e| {
        ConfigError::new(
            "E6002",
            format!("{file} does not match config/1: {e}"),
            "Correct the value's type or name; the schema is schemas/config-1.json.",
        )
    })?;
    let value = serde_json::to_value(&config).map_err(|e| {
        ConfigError::new(
            "E6002",
            format!("{file} cannot be represented: {e}"),
            "Correct the value.",
        )
    })?;
    let mut values = BTreeMap::new();
    flatten("", &value, table, &mut values);
    Ok(Parsed { input, doc, values })
}

fn flatten(
    prefix: &str,
    value: &Value,
    table: &BTreeMap<String, FieldRule>,
    out: &mut BTreeMap<String, Value>,
) {
    let Some(map) = value.as_object() else { return };
    for (key, v) in map {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if table.contains_key(&path) {
            out.insert(path, v.clone());
        } else {
            flatten(&path, v, table, out);
        }
    }
}

/// The 1-based line of a field's value, or of one element of its array.
fn line_of(parsed: &Parsed, path: &str, element: Option<usize>) -> usize {
    let mut item: Option<&Item> = Some(parsed.doc.as_item());
    for part in path.split('.') {
        item = item.and_then(|i| i.get(part));
    }
    let span = item.and_then(|item| match element {
        Some(n) => item.as_array()?.get(n)?.span(),
        None => item.span(),
    });
    let offset = span.map_or(0, |s| s.start);
    parsed.input.text[..offset].matches('\n').count() + 1
}

fn compare_numbers(a: &Value, b: &Value) -> Ordering {
    match (a.as_u64(), b.as_u64()) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(Ordering::Equal),
    }
}

/// Compares two strictness levels: `Greater` means `a` is stricter.
fn compare_strictness(rule: &FieldRule, a: &Value, b: &Value) -> Ordering {
    if let Some(levels) = &rule.levels {
        let rank = |v: &Value| levels.iter().position(|l| Some(l.as_str()) == v.as_str());
        rank(a).cmp(&rank(b))
    } else if rule.lowest_is_strictest {
        compare_numbers(b, a)
    } else {
        compare_numbers(a, b)
    }
}

fn elements(value: &Value) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for v in value.as_array().into_iter().flatten() {
        if !out.contains(v) {
            out.push(v.clone());
        }
    }
    out
}

/// A scalar field's effective value so far and the layer that decided it.
fn fold_scalar(rule: &FieldRule, entries: &[(Layer, &Value)]) -> Option<(Value, Layer)> {
    let mut best: Option<(&Value, Layer)> = None;
    for &(layer, v) in entries {
        best = match best {
            None => Some((v, layer)),
            Some((cur, from)) => {
                let replace = match rule.combine {
                    Combine::Minimum => compare_numbers(v, cur) == Ordering::Less,
                    Combine::And => v == &Value::Bool(false) && cur == &Value::Bool(true),
                    _ => compare_strictness(rule, v, cur) == Ordering::Greater,
                };
                if replace {
                    Some((v, layer))
                } else {
                    Some((cur, from))
                }
            }
        };
    }
    best.map(|(v, l)| (v.clone(), l))
}

/// Intersects lists in layer order; the origin is the last layer that narrowed the list.
fn fold_intersection(lists: &[(Layer, Vec<Value>)]) -> Option<(Vec<Value>, Layer)> {
    let mut current: Option<(Vec<Value>, Layer)> = None;
    for (layer, list) in lists {
        current = match current {
            None => Some((list.clone(), *layer)),
            Some((cur, from)) => {
                let kept: Vec<Value> = cur.iter().filter(|v| list.contains(v)).cloned().collect();
                let origin = if kept.len() < cur.len() { *layer } else { from };
                Some((kept, origin))
            }
        };
    }
    current
}

/// Parses the layers and combines them; a project value that loosens is dropped and reported.
pub fn load(inputs: &[LayerInput]) -> Result<Loaded, ConfigError> {
    let table = field_table();
    let mut ordered: Vec<&LayerInput> = inputs.iter().collect();
    ordered.sort_by_key(|i| i.layer);
    if let Some(pair) = ordered.windows(2).find(|w| w[0].layer == w[1].layer) {
        return Err(ConfigError::new(
            "E6003",
            format!(
                "layer {:?} was given twice: {} and {}",
                pair[0].layer,
                pair[0].file.display(),
                pair[1].file.display()
            ),
            "Provide each of the four layers at most once.",
        ));
    }
    let layers = ordered
        .into_iter()
        .map(|i| parse_layer(i, &table))
        .collect::<Result<Vec<_>, _>>()?;

    let mut effective = Value::Object(Default::default());
    let mut origins = BTreeMap::new();
    let mut findings = Vec::new();
    for (path, rule) in &table {
        let present: Vec<(&Parsed, &Value)> = layers
            .iter()
            .filter_map(|l| l.values.get(path).map(|v| (l, v)))
            .collect();
        let outcome = match rule.combine {
            Combine::Union => combine_union(&present),
            Combine::Intersection => combine_intersection(path, &present, &mut findings),
            _ => combine_scalar(path, rule, &present, &mut findings),
        };
        if let Some((value, origin)) = outcome {
            insert_path(&mut effective, path, value);
            origins.insert(path.clone(), origin);
        }
    }
    findings.sort_by(|a, b| (&a.file, a.line, &a.field).cmp(&(&b.file, b.line, &b.field)));
    let config = serde_json::from_value(effective).map_err(|e| {
        ConfigError::new(
            "E6002",
            format!("the combined configuration is invalid: {e}"),
            "Correct the layers' values.",
        )
    })?;
    Ok(Loaded {
        config,
        origins,
        findings,
    })
}

fn insert_path(root: &mut Value, path: &str, value: Value) {
    let mut node = root;
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        let map = node
            .as_object_mut()
            .expect("effective is built from objects");
        if parts.peek().is_none() {
            map.insert(part.to_string(), value);
            return;
        }
        node = map
            .entry(part.to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
}

fn ignored(parsed: &Parsed, path: &str, element: Option<usize>, message: String) -> Finding {
    Finding {
        kind: FindingKind::Ignored,
        file: parsed.input.file.clone(),
        line: line_of(parsed, path, element),
        field: path.to_string(),
        message,
    }
}

fn combine_union(present: &[(&Parsed, &Value)]) -> Option<(Value, Origin)> {
    let mut items: Vec<Value> = Vec::new();
    let mut from = Vec::new();
    for (parsed, value) in present {
        for item in elements(value) {
            if !items.contains(&item) {
                items.push(item);
                from.push(parsed.input.layer);
            }
        }
    }
    if items.is_empty() {
        None
    } else {
        Some((Value::Array(items), Origin::Elements(from)))
    }
}

fn combine_scalar(
    path: &str,
    rule: &FieldRule,
    present: &[(&Parsed, &Value)],
    findings: &mut Vec<Finding>,
) -> Option<(Value, Origin)> {
    let upper: Vec<(Layer, &Value)> = present
        .iter()
        .filter(|(p, _)| p.input.layer != Layer::Project)
        .map(|(p, v)| (p.input.layer, *v))
        .collect();
    let mut entries = upper.clone();
    if let Some((parsed, value)) = present
        .iter()
        .find(|(p, _)| p.input.layer == Layer::Project)
    {
        let above = fold_scalar(rule, &upper).map(|(v, _)| v);
        let loosens = match (rule.combine, &above) {
            (Combine::And, above) => {
                *value == &Value::Bool(true) && above.as_ref() != Some(&Value::Bool(true))
            }
            (Combine::Minimum, Some(above)) => compare_numbers(value, above) == Ordering::Greater,
            (_, Some(above)) => compare_strictness(rule, value, above) == Ordering::Less,
            (_, None) => false,
        };
        if loosens {
            findings.push(ignored(
                parsed,
                path,
                None,
                format!(
                    "project value {value} for {path} would loosen the value set above it and is ignored"
                ),
            ));
        } else {
            entries.push((Layer::Project, value));
        }
    }
    fold_scalar(rule, &entries).map(|(v, l)| (v, Origin::Layer(l)))
}

fn combine_intersection(
    path: &str,
    present: &[(&Parsed, &Value)],
    findings: &mut Vec<Finding>,
) -> Option<(Value, Origin)> {
    let upper: Vec<(Layer, Vec<Value>)> = present
        .iter()
        .filter(|(p, _)| p.input.layer != Layer::Project)
        .map(|(p, v)| (p.input.layer, elements(v)))
        .collect();
    let above = fold_intersection(&upper).map(|(list, _)| list);
    let mut lists = upper;
    if let Some((parsed, value)) = present
        .iter()
        .find(|(p, _)| p.input.layer == Layer::Project)
    {
        let raw = value.as_array().map(Vec::as_slice).unwrap_or_default();
        if let Some(above) = &above {
            for (n, item) in raw.iter().enumerate() {
                if !above.contains(item) {
                    findings.push(ignored(
                        parsed,
                        path,
                        Some(n),
                        format!(
                            "project entry {item} is not allowed by the layers above and is ignored in {path}"
                        ),
                    ));
                }
            }
        }
        let kept: Vec<Value> = elements(value)
            .into_iter()
            .filter(|v| above.as_ref().is_none_or(|a| a.contains(v)))
            .collect();
        lists.push((Layer::Project, kept));
    }
    fold_intersection(&lists).map(|(list, layer)| (Value::Array(list), Origin::Layer(layer)))
}
