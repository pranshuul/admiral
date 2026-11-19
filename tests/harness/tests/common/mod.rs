#![allow(dead_code)]
use admiral_harness::Runtime;
use std::path::{Path, PathBuf};

pub fn crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn two_node_text() -> String {
    std::fs::read_to_string(crate_dir().join("scenarios/two-node.json")).unwrap()
}

/// The stub runtime, run in a fresh directory that lives as long as the returned guard.
pub fn stub_runtime() -> (tempfile::TempDir, Runtime) {
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime {
        program: crate_dir().join("fixtures/stub-admiral.sh"),
        dir: dir.path().to_path_buf(),
    };
    (dir, runtime)
}
