use admiral_fixtures::hackathon_dir;
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn workflow_dir() -> PathBuf {
    hackathon_dir().join("workflows/hackathon")
}

fn workflow() -> toml::Table {
    let text = fs::read_to_string(workflow_dir().join("workflow.toml")).unwrap();
    text.parse().unwrap()
}

fn nodes(graph: &toml::Table) -> Vec<&toml::Table> {
    graph["node"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_table().unwrap())
        .collect()
}

fn slice_nodes(wf: &toml::Table) -> Vec<&toml::Table> {
    nodes(wf["subgraph"]["slice"].as_table().unwrap())
}

fn edge_targets(node: &toml::Table) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = node.get("next") {
        out.push(n.as_str().unwrap().to_string());
    }
    if let Some(on) = node.get("on") {
        for v in on.as_table().unwrap().values() {
            out.push(v.as_str().unwrap().to_string());
        }
    }
    out
}

fn reachable(entry: &str, graph: &[&toml::Table]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![entry.to_string()];
    while let Some(id) = todo.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let node = graph
            .iter()
            .find(|n| n["id"].as_str() == Some(&id))
            .unwrap_or_else(|| panic!("no node {id}"));
        todo.extend(edge_targets(node));
    }
    seen
}

#[test]
fn identifies_itself_as_the_hackathon_workflow() {
    let wf = workflow();
    assert_eq!(wf["workflow"]["name"].as_str(), Some("hackathon"));
    assert_eq!(wf["workflow"]["entry"].as_str(), Some("ingest"));
    assert!(
        wf["workflow"]["version"]
            .as_str()
            .unwrap()
            .split('.')
            .count()
            == 3
    );
    assert_eq!(wf["params"]["mode"]["values"].as_array().unwrap().len(), 2);
}

#[test]
fn declares_the_main_graph_nodes_of_the_spec() {
    let wf = workflow();
    let ids: Vec<&str> = nodes(&wf)
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "ingest",
            "scout",
            "kickoff",
            "checks",
            "ideate",
            "pick",
            "spec",
            "board-setup",
            "skeleton",
            "skeleton-check",
            "tickets",
            "build",
            "ci",
            "demo",
            "demo-check",
            "receipts",
            "scoreboard",
            "submission",
            "ship",
            "submitted",
            "partial"
        ]
    );
}

#[test]
fn every_main_node_is_reachable_and_every_edge_resolves() {
    let wf = workflow();
    let graph = nodes(&wf);
    let seen = reachable("ingest", &graph);
    assert_eq!(seen.len(), graph.len());
}

#[test]
fn every_slice_node_is_reachable_and_ends_in_an_end_node() {
    let wf = workflow();
    let graph = slice_nodes(&wf);
    let seen = reachable("clock", &graph);
    assert_eq!(seen.len(), graph.len());
    let ends: Vec<&str> = graph
        .iter()
        .filter(|n| n["kind"].as_str() == Some("end"))
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ends, ["done", "frozen"]);
}

#[test]
fn the_parallel_node_names_an_existing_subgraph() {
    let wf = workflow();
    let build = nodes(&wf)
        .into_iter()
        .find(|n| n["id"].as_str() == Some("build"))
        .unwrap();
    assert_eq!(build["kind"].as_str(), Some("parallel"));
    assert!(wf["subgraph"][build["subgraph"].as_str().unwrap()].is_table());
}

#[test]
fn every_script_gate_and_end_node_declares_what_it_must() {
    let wf = workflow();
    let all: Vec<_> = nodes(&wf).into_iter().chain(slice_nodes(&wf)).collect();
    for n in &all {
        match n["kind"].as_str().unwrap() {
            "script" | "gate" if n.contains_key("run") => {
                assert!(n.contains_key("exit"), "{}", n["id"])
            }
            "end" => assert!(["success", "failure"].contains(&n["class"].as_str().unwrap())),
            "ask" => assert!(n.contains_key("why"), "{}", n["id"]),
            _ => {}
        }
    }
}

#[test]
fn every_script_a_node_runs_exists_and_is_executable() {
    let wf = workflow();
    let mut count = 0;
    for n in nodes(&wf).into_iter().chain(slice_nodes(&wf)) {
        if let Some(run) = n.get("run") {
            let path = workflow_dir().join(run.as_str().unwrap());
            let mode = fs::metadata(&path)
                .unwrap_or_else(|_| panic!("{} missing", path.display()))
                .permissions()
                .mode();
            assert!(mode & 0o111 != 0, "{} is not executable", path.display());
            count += 1;
        }
    }
    assert_eq!(count, 9);
}

#[test]
fn every_script_is_valid_bash() {
    let dir = workflow_dir().join("scripts");
    let mut count = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let status = Command::new("bash").arg("-n").arg(&path).status().unwrap();
        assert!(status.success(), "{}", path.display());
        count += 1;
    }
    assert_eq!(count, 9);
}

#[test]
fn every_schema_a_node_names_exists_and_parses() {
    let wf = workflow();
    let mut paths = BTreeSet::new();
    for n in nodes(&wf).into_iter().chain(slice_nodes(&wf)) {
        for key in ["output", "ask", "decide"] {
            if let Some(t) = n.get(key).and_then(|v| v.as_table()) {
                for field in ["schema", "question"] {
                    if let Some(p) = t.get(field) {
                        paths.insert(p.as_str().unwrap().to_string());
                    }
                }
            }
        }
        if let Some(p) = n.get("attempts").and_then(|a| a.get("question")) {
            paths.insert(p.as_str().unwrap().to_string());
        }
    }
    paths.insert(wf["harness"]["rubric"].as_str().unwrap().to_string());
    assert_eq!(paths.len(), 13, "{paths:?}");
    for p in paths {
        let text =
            fs::read_to_string(workflow_dir().join(&p)).unwrap_or_else(|_| panic!("{p} missing"));
        serde_json::from_str::<serde_json::Value>(&text).unwrap_or_else(|e| panic!("{p}: {e}"));
    }
}

#[test]
fn every_fleet_agent_and_skill_a_node_names_ships_in_the_package() {
    let wf = workflow();
    let mut agents = BTreeSet::new();
    let mut skills = BTreeSet::new();
    for n in nodes(&wf).into_iter().chain(slice_nodes(&wf)) {
        if let Some(a) = n.get("agent") {
            agents.insert(
                a.as_str()
                    .unwrap()
                    .strip_prefix("fleet/")
                    .unwrap()
                    .to_string(),
            );
        }
        if let Some(s) = n.get("skill") {
            skills.insert(
                s.as_str()
                    .unwrap()
                    .strip_prefix("fleet/")
                    .unwrap()
                    .to_string(),
            );
        }
    }
    assert_eq!(agents.len(), 8);
    assert_eq!(skills.len(), 2);
    for a in agents {
        assert!(
            hackathon_dir().join(format!("agents/{a}.md")).is_file(),
            "agent {a}"
        );
    }
    for s in skills {
        assert!(
            hackathon_dir()
                .join(format!("skills/{s}/SKILL.md"))
                .is_file(),
            "skill {s}"
        );
    }
}

#[test]
fn ships_the_orchestrator_skill_scouting_seed_and_eval() {
    let dir = workflow_dir();
    assert!(dir.join("SKILL.md").is_file());
    let seed: toml::Table = fs::read_to_string(dir.join("references/scouting.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(!seed.is_empty());
    let eval = dir.join("evals");
    assert!(eval.is_dir());
    assert!(fs::read_dir(eval).unwrap().count() >= 1);
}

#[test]
fn the_package_holds_no_literal_secret() {
    let out = Command::new("grep")
        .args([
            "-rEi",
            "(api[_-]?key|token|secret)[\"']? *[:=] *[\"'][A-Za-z0-9]{16,}",
        ])
        .arg(hackathon_dir())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn the_checks_script_detects_the_stack_of_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    admiral_fixtures::copy_tree(&admiral_fixtures::mixed_repo_dir(), dir.path()).unwrap();
    let out = Command::new(workflow_dir().join("scripts/checks.sh"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    for stack in ["rust", "python", "node", "go"] {
        assert!(stdout.contains(stack), "{stdout}");
    }
    let empty = tempfile::tempdir().unwrap();
    let out = Command::new(workflow_dir().join("scripts/checks.sh"))
        .current_dir(empty.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}
