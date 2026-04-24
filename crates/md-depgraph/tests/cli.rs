use assert_cmd::Command;
use predicates::prelude::*;

const PROJECT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/project");
const MULTI: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/multi");

fn cmd() -> Command {
    Command::cargo_bin("md-depgraph").unwrap()
}

// ---------------------------------------------------------------------------
// extract
// ---------------------------------------------------------------------------

#[test]
fn extract_project_exits_zero_with_json() {
    cmd()
        .args(["extract", PROJECT])
        .assert()
        .success()
        .stdout(predicate::str::starts_with('[').or(predicate::str::starts_with('{')));
}

#[test]
fn extract_project_json_is_valid() {
    let output = cmd().args(["extract", PROJECT]).output().unwrap();
    assert!(output.status.success());
    let _: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("extract output must be valid JSON");
}

#[test]
fn extract_jsonl_format_one_object_per_line() {
    let output = cmd()
        .args(["extract", PROJECT, "--format", "jsonl"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines() {
        let _: serde_json::Value =
            serde_json::from_str(line).expect("each JSONL line must be valid JSON");
    }
}

// ---------------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------------

#[test]
fn validate_project_exits_zero() {
    cmd().args(["validate", PROJECT]).assert().success();
}

#[test]
fn validate_multi_with_broken_refs_exits_nonzero() {
    cmd()
        .args(["validate", MULTI])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn validate_broken_reports_missing_file_in_stderr() {
    let output = cmd().args(["validate", MULTI]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no-such-file")
            || stderr.contains("not found")
            || stderr.contains("error:"),
        "stderr was: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// graph
// ---------------------------------------------------------------------------

#[test]
fn graph_dot_format_contains_digraph() {
    cmd()
        .args(["graph", PROJECT, "--format", "dot"])
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph {"));
}

#[test]
fn graph_json_format_is_valid() {
    let output = cmd()
        .args(["graph", PROJECT, "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("graph json must be valid JSON");
    assert!(v["nodes"].is_array());
    assert!(v["edges"].is_array());
}

#[test]
fn graph_dot_contains_edge_labels() {
    let output = cmd()
        .args(["graph", PROJECT, "--format", "dot"])
        .output()
        .unwrap();
    let dot = String::from_utf8(output.stdout).unwrap();
    // At least one edge with a directive kind label must appear.
    let has_edge_label = dot.contains("derived-from")
        || dot.contains("constrained-by")
        || dot.contains("blocked-by")
        || dot.contains("supersedes");
    assert!(has_edge_label, "DOT output had no edge labels:\n{dot}");
}
