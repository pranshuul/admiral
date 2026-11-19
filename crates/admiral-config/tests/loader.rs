use std::fs;
use std::path::PathBuf;

use admiral_config::{
    FindingKind, Layer, LayerInput, LayerPaths, Origin, field_table, load, read_layers,
};
use admiral_model::config::{Combine, Evidence};

fn input(layer: Layer, file: &str, text: &str) -> LayerInput {
    LayerInput {
        layer,
        file: PathBuf::from(file),
        text: text.to_string(),
    }
}

fn org(text: &str) -> LayerInput {
    input(Layer::Organization, "/etc/admiral/policy.toml", text)
}
fn user(text: &str) -> LayerInput {
    input(Layer::User, "/home/u/.config/admiral/config.toml", text)
}
fn workflow(text: &str) -> LayerInput {
    input(Layer::Workflow, "/home/u/.admiral/wf/workflow.toml", text)
}
fn project(text: &str) -> LayerInput {
    input(Layer::Project, "/repo/.admiral/config.toml", text)
}

#[test]
fn field_table_is_generated_from_the_model_annotations() {
    let table = field_table();
    for (field, combine) in [
        ("lock", Combine::Union),
        ("required_approvals", Combine::Union),
        ("forbidden_tools", Combine::Union),
        ("max_loops", Combine::Minimum),
        ("max_attempts", Combine::Minimum),
        ("max_concurrency", Combine::Minimum),
        ("budget.tokens", Combine::Minimum),
        ("budget.rounds", Combine::Minimum),
        ("budget.cost_usd", Combine::Minimum),
        ("deploy_enabled", Combine::And),
        ("network_allowed", Combine::And),
        ("allow_headless_deploy", Combine::And),
        ("min_evidence", Combine::Strictest),
        ("scan.threshold", Combine::Strictest),
        ("deploy_targets", Combine::Intersection),
        ("credential_names", Combine::Intersection),
        ("discovery_sources", Combine::Intersection),
        ("decider_backends", Combine::Intersection),
        ("models", Combine::Intersection),
    ] {
        assert_eq!(table[field].combine, combine, "{field}");
    }
    assert_eq!(table.len(), 19);
    assert_eq!(
        table["min_evidence"].levels.as_deref().unwrap(),
        ["asserted", "heuristic", "reported", "verified"]
    );
    assert!(table["scan.threshold"].lowest_is_strictest);
    assert!(!table["max_loops"].lowest_is_strictest);
}

#[test]
fn the_four_layers_parse_into_one_structure_with_origins() {
    let loaded = load(&[
        org("max_loops = 9\n"),
        user("max_attempts = 4\n"),
        workflow("network_allowed = true\n"),
        project("max_concurrency = 2\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.max_loops, Some(9));
    assert_eq!(loaded.config.max_attempts, Some(4));
    assert_eq!(loaded.config.max_concurrency, Some(2));
    assert_eq!(
        loaded.origins["max_loops"],
        Origin::Layer(Layer::Organization)
    );
    assert_eq!(loaded.origins["max_attempts"], Origin::Layer(Layer::User));
    assert_eq!(
        loaded.origins["max_concurrency"],
        Origin::Layer(Layer::Project)
    );
    assert!(loaded.findings.is_empty());
}

#[test]
fn input_order_does_not_matter_and_absent_layers_are_fine() {
    let loaded = load(&[project("max_loops = 2\n"), user("max_loops = 5\n")]).unwrap();
    assert_eq!(loaded.config.max_loops, Some(2));
    assert_eq!(loaded.origins["max_loops"], Origin::Layer(Layer::Project));
    let empty = load(&[]).unwrap();
    assert_eq!(empty.config, Default::default());
    assert!(empty.origins.is_empty());
}

#[test]
fn union_collects_every_layer_with_element_origins() {
    let lock = |skill: &str| {
        format!("[[lock]]\nskill = \"{skill}\"\naudience = \"human\"\nenforce = \"both\"\n")
    };
    let loaded = load(&[
        org(&format!(
            "forbidden_tools = [\"rm\", \"dd\"]\n{}",
            lock("org/pii")
        )),
        user(&format!(
            "forbidden_tools = [\"dd\", \"curl\"]\n{}",
            lock("style/unslop")
        )),
        project(&lock("org/pii")),
    ])
    .unwrap();
    assert_eq!(loaded.config.forbidden_tools, ["rm", "dd", "curl"]);
    assert_eq!(
        loaded.origins["forbidden_tools"],
        Origin::Elements(vec![Layer::Organization, Layer::Organization, Layer::User])
    );
    let skills: Vec<_> = loaded
        .config
        .locks
        .iter()
        .map(|l| l.skill.as_str())
        .collect();
    assert_eq!(skills, ["org/pii", "style/unslop"]);
    assert_eq!(
        loaded.origins["lock"],
        Origin::Elements(vec![Layer::Organization, Layer::User])
    );
    assert!(loaded.findings.is_empty());
}

#[test]
fn minimum_takes_the_smallest_and_records_its_layer() {
    let loaded = load(&[
        org("max_loops = 10\n[budget]\ntokens = 1000\n"),
        user("max_loops = 5\n[budget]\ntokens = 2000\nrounds = 8\ncost_usd = 4.5\n"),
        workflow("max_loops = 7\n"),
        project("max_loops = 3\n[budget]\nrounds = 6\ncost_usd = 2.5\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.max_loops, Some(3));
    assert_eq!(loaded.origins["max_loops"], Origin::Layer(Layer::Project));
    let budget = loaded.config.budget.unwrap();
    assert_eq!(budget.tokens, Some(1000));
    assert_eq!(budget.rounds, Some(6));
    assert_eq!(budget.cost_usd, Some(2.5));
    assert_eq!(
        loaded.origins["budget.tokens"],
        Origin::Layer(Layer::Organization)
    );
    assert_eq!(
        loaded.origins["budget.rounds"],
        Origin::Layer(Layer::Project)
    );
    assert_eq!(
        loaded.origins["budget.cost_usd"],
        Origin::Layer(Layer::Project)
    );
    assert!(loaded.findings.is_empty());
}

#[test]
fn and_is_false_if_any_layer_says_false() {
    let loaded = load(&[
        org("deploy_enabled = true\nnetwork_allowed = true\n"),
        user("deploy_enabled = false\nnetwork_allowed = true\n"),
        workflow("allow_headless_deploy = true\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.deploy_enabled, Some(false));
    assert_eq!(loaded.origins["deploy_enabled"], Origin::Layer(Layer::User));
    assert_eq!(loaded.config.network_allowed, Some(true));
    assert_eq!(
        loaded.origins["network_allowed"],
        Origin::Layer(Layer::Organization)
    );
    assert_eq!(loaded.config.allow_headless_deploy, Some(true));
}

#[test]
fn strictest_picks_the_higher_evidence_and_the_lower_scan_threshold() {
    let loaded = load(&[
        org("min_evidence = \"reported\"\n[scan]\nthreshold = 60\n"),
        user("min_evidence = \"heuristic\"\n[scan]\nthreshold = 40\n"),
        project("min_evidence = \"verified\"\n[scan]\nthreshold = 50\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.min_evidence, Some(Evidence::Verified));
    assert_eq!(
        loaded.origins["min_evidence"],
        Origin::Layer(Layer::Project)
    );
    assert_eq!(loaded.config.scan.unwrap().threshold, Some(40));
    assert_eq!(loaded.origins["scan.threshold"], Origin::Layer(Layer::User));
    assert_eq!(loaded.findings.len(), 1);
    assert_eq!(loaded.findings[0].field, "scan.threshold");
}

#[test]
fn intersection_keeps_only_what_every_layer_allows() {
    let loaded = load(&[
        org("models = [\"fast\", \"deep\", \"mid\"]\n"),
        user("models = [\"deep\", \"mid\", \"other\"]\ndeploy_targets = [\"prod\"]\n"),
        workflow("models = [\"mid\", \"deep\"]\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.models.as_deref().unwrap(), ["deep", "mid"]);
    assert_eq!(loaded.origins["models"], Origin::Layer(Layer::User));
    assert_eq!(loaded.config.deploy_targets.as_deref().unwrap(), ["prod"]);
    assert_eq!(loaded.origins["deploy_targets"], Origin::Layer(Layer::User));
    assert!(loaded.config.credential_names.is_none());
    assert!(!loaded.origins.contains_key("credential_names"));
}

#[test]
fn a_project_value_that_loosens_is_dropped_with_file_and_line() {
    let loaded = load(&[
        user("max_loops = 3\nmin_evidence = \"reported\"\n"),
        project("# project\nmax_loops = 8\nmin_evidence = \"asserted\"\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.max_loops, Some(3));
    assert_eq!(loaded.config.min_evidence, Some(Evidence::Reported));
    assert_eq!(loaded.origins["max_loops"], Origin::Layer(Layer::User));
    assert_eq!(loaded.findings.len(), 2);
    let f = &loaded.findings[0];
    assert_eq!(f.kind, FindingKind::Ignored);
    assert_eq!(f.file, PathBuf::from("/repo/.admiral/config.toml"));
    assert_eq!((f.field.as_str(), f.line), ("max_loops", 2));
    assert!(f.message.contains("max_loops"));
    let g = &loaded.findings[1];
    assert_eq!((g.field.as_str(), g.line), ("min_evidence", 3));
    assert_eq!(
        serde_json::to_value(g.kind).unwrap(),
        serde_json::json!("ignored")
    );
}

#[test]
fn a_project_flag_is_ignored_unless_the_layers_above_allow_it() {
    let loaded = load(&[
        user("network_allowed = false\ndeploy_enabled = true\n"),
        project("network_allowed = true\ndeploy_enabled = true\nallow_headless_deploy = true\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.network_allowed, Some(false));
    assert_eq!(loaded.config.deploy_enabled, Some(true));
    assert_eq!(loaded.config.allow_headless_deploy, None);
    let fields: Vec<_> = loaded
        .findings
        .iter()
        .map(|f| (f.field.as_str(), f.line))
        .collect();
    assert_eq!(
        fields,
        [("network_allowed", 1), ("allow_headless_deploy", 3)]
    );
    let tightening = load(&[
        user("deploy_enabled = true\n"),
        project("deploy_enabled = false\n"),
    ])
    .unwrap();
    assert_eq!(tightening.config.deploy_enabled, Some(false));
    assert_eq!(
        tightening.origins["deploy_enabled"],
        Origin::Layer(Layer::Project)
    );
    assert!(tightening.findings.is_empty());
}

#[test]
fn project_allow_list_entries_outside_the_upper_list_are_dropped_one_by_one() {
    let loaded = load(&[
        user("deploy_targets = [\"staging\", \"prod\"]\n"),
        project("deploy_targets = [\n  \"staging\",\n  \"moon\",\n]\n"),
    ])
    .unwrap();
    assert_eq!(
        loaded.config.deploy_targets.as_deref().unwrap(),
        ["staging"]
    );
    assert_eq!(
        loaded.origins["deploy_targets"],
        Origin::Layer(Layer::Project)
    );
    assert_eq!(loaded.findings.len(), 1);
    assert_eq!(loaded.findings[0].line, 3);
    assert!(loaded.findings[0].message.contains("moon"));
    let unrestricted = load(&[project("models = [\"a\"]\n")]).unwrap();
    assert_eq!(unrestricted.config.models.as_deref().unwrap(), ["a"]);
    assert!(unrestricted.findings.is_empty());
}

#[test]
fn a_project_cannot_loosen_a_budget_or_scan_threshold() {
    let loaded = load(&[
        org("[budget]\ntokens = 100\n[scan]\nthreshold = 30\n"),
        project("[budget]\ntokens = 200\n[scan]\nthreshold = 90\n"),
    ])
    .unwrap();
    assert_eq!(loaded.config.budget.unwrap().tokens, Some(100));
    assert_eq!(loaded.config.scan.unwrap().threshold, Some(30));
    let fields: Vec<_> = loaded
        .findings
        .iter()
        .map(|f| (f.field.as_str(), f.line))
        .collect();
    assert_eq!(fields, [("budget.tokens", 2), ("scan.threshold", 4)]);
}

#[test]
fn loosening_in_a_non_project_layer_is_not_a_finding() {
    let loaded = load(&[org("max_loops = 2\n"), user("max_loops = 9\n")]).unwrap();
    assert_eq!(loaded.config.max_loops, Some(2));
    assert!(loaded.findings.is_empty());
}

#[test]
fn bad_files_are_typed_errors_with_a_remedy() {
    let err = load(&[user("max_loops = \n")]).unwrap_err();
    assert_eq!(err.code, "E6001");
    assert!(err.message.contains("/home/u/.config/admiral/config.toml"));
    assert!(!err.remedy.is_empty());
    let err = load(&[project("max_loops = \"many\"\n")]).unwrap_err();
    assert_eq!(err.code, "E6002");
    assert!(err.message.contains("/repo/.admiral/config.toml"));
    let err = load(&[user("max_loops = 1\n"), user("max_loops = 2\n")]).unwrap_err();
    assert_eq!(err.code, "E6003");
    assert!(err.to_string().starts_with("E6003: "));
}

#[test]
fn read_layers_skips_missing_files_and_reads_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let user_file = dir.path().join("user.toml");
    fs::write(&user_file, "max_loops = 4\n").unwrap();
    let inputs = read_layers(&LayerPaths {
        organization: Some(dir.path().join("missing.toml")),
        user: Some(user_file.clone()),
        workflow: None,
        project: None,
    })
    .unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].layer, Layer::User);
    assert_eq!(inputs[0].file, user_file);
    assert_eq!(inputs[0].text, "max_loops = 4\n");
    let err = read_layers(&LayerPaths {
        project: Some(dir.path().to_path_buf()),
        ..Default::default()
    })
    .unwrap_err();
    assert_eq!(err.code, "E6004");
}

// --- property test: a project layer never makes any effective value looser ---

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn maybe(&mut self) -> bool {
        self.below(2) == 0
    }
}

fn random_layer(rng: &mut Rng) -> String {
    let mut out = String::new();
    for key in ["max_loops", "max_attempts", "max_concurrency"] {
        if rng.maybe() {
            out += &format!("{key} = {}\n", rng.below(10));
        }
    }
    for key in ["deploy_enabled", "network_allowed", "allow_headless_deploy"] {
        if rng.maybe() {
            out += &format!("{key} = {}\n", rng.maybe());
        }
    }
    if rng.maybe() {
        let tiers = ["asserted", "heuristic", "reported", "verified"];
        out += &format!("min_evidence = \"{}\"\n", tiers[rng.below(4) as usize]);
    }
    for key in ["forbidden_tools", "required_approvals"] {
        if rng.maybe() {
            let items: Vec<String> = (0..rng.below(4))
                .map(|_| format!("\"t{}\"", rng.below(5)))
                .collect();
            out += &format!("{key} = [{}]\n", items.join(", "));
        }
    }
    for key in ["models", "deploy_targets", "credential_names"] {
        if rng.maybe() {
            let items: Vec<String> = (0..rng.below(5))
                .map(|_| format!("\"m{}\"", rng.below(5)))
                .collect();
            out += &format!("{key} = [{}]\n", items.join(", "));
        }
    }
    if rng.maybe() {
        out += &format!("[budget]\ntokens = {}\n", rng.below(100));
    }
    if rng.maybe() {
        out += &format!("[scan]\nthreshold = {}\n", rng.below(101));
    }
    out
}

fn rank(e: Option<Evidence>) -> i32 {
    match e {
        None => -1,
        Some(Evidence::Asserted) => 0,
        Some(Evidence::Heuristic) => 1,
        Some(Evidence::Reported) => 2,
        Some(Evidence::Verified) => 3,
    }
}

fn subset(new: &[String], old: &[String]) -> bool {
    new.iter().all(|x| old.contains(x))
}

#[test]
fn combining_any_project_layer_never_yields_a_looser_value() {
    let mut rng = Rng(7);
    for case in 0..3000 {
        let upper = [
            org(&random_layer(&mut rng)),
            user(&random_layer(&mut rng)),
            workflow(&random_layer(&mut rng)),
        ];
        let proj = project(&random_layer(&mut rng));
        let base = load(&upper).unwrap().config;
        let mut all = upper.to_vec();
        all.push(proj);
        let with = load(&all).unwrap().config;
        let ctx = format!("case {case}");
        for (new, old) in [
            (with.max_loops, base.max_loops),
            (with.max_attempts, base.max_attempts),
            (with.max_concurrency, base.max_concurrency),
        ] {
            assert!(new.unwrap_or(u32::MAX) <= old.unwrap_or(u32::MAX), "{ctx}");
        }
        for (new, old) in [
            (with.deploy_enabled, base.deploy_enabled),
            (with.network_allowed, base.network_allowed),
            (with.allow_headless_deploy, base.allow_headless_deploy),
        ] {
            assert!(!new.unwrap_or(false) || old.unwrap_or(false), "{ctx}");
        }
        assert!(rank(with.min_evidence) >= rank(base.min_evidence), "{ctx}");
        let threshold = |c: &admiral_model::config::Config| {
            c.scan.as_ref().and_then(|s| s.threshold).unwrap_or(255)
        };
        assert!(threshold(&with) <= threshold(&base), "{ctx}");
        assert!(
            subset(&base.forbidden_tools, &with.forbidden_tools),
            "{ctx}"
        );
        assert!(
            subset(&base.required_approvals, &with.required_approvals),
            "{ctx}"
        );
        let tokens = |c: &admiral_model::config::Config| {
            c.budget.as_ref().and_then(|b| b.tokens).unwrap_or(u64::MAX)
        };
        assert!(tokens(&with) <= tokens(&base), "{ctx}");
        for (new, old) in [
            (&with.models, &base.models),
            (&with.deploy_targets, &base.deploy_targets),
            (&with.credential_names, &base.credential_names),
        ] {
            match (new, old) {
                (_, None) => {}
                (Some(n), Some(o)) => assert!(subset(n, o), "{ctx}"),
                (None, Some(_)) => panic!("{ctx}: allow-list lost"),
            }
        }
    }
}
