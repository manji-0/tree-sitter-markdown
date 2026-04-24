use std::path::Path;

use md_depgraph::{extract, graph::Graph, DirectiveKind};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/project");

#[test]
fn extract_from_impl_md() {
    let path = Path::new(FIXTURES).join("impl.md");
    let directives = extract::extract_file(&path).unwrap();
    assert_eq!(directives.len(), 4, "expected 4 directives in impl.md");

    let kinds: Vec<_> = directives.iter().map(|d| d.kind).collect();
    assert!(kinds.contains(&DirectiveKind::DerivedFrom));
    assert!(kinds.contains(&DirectiveKind::BlockedBy));
    assert!(kinds.contains(&DirectiveKind::ConstrainedBy));
    assert!(kinds.contains(&DirectiveKind::Supersedes));
}

#[test]
fn source_sections_inferred() {
    let path = Path::new(FIXTURES).join("impl.md");
    let directives = extract::extract_file(&path).unwrap();

    // "<!-- derived-from spec.md#rationale -->" is at document scope (before any heading)
    let derived = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::DerivedFrom)
        .unwrap();
    // It appears right after the h1 heading "# Implementation", so source_section
    // should be "implementation" (slug of the h1).
    assert_eq!(derived.source_section.as_deref(), Some("implementation"));

    // "<!-- blocked-by tasks.md -->" is under "## Setup"
    let blocked = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::BlockedBy)
        .unwrap();
    assert_eq!(blocked.source_section.as_deref(), Some("setup"));
}

#[test]
fn same_file_anchor_has_no_target_file() {
    let path = Path::new(FIXTURES).join("impl.md");
    let directives = extract::extract_file(&path).unwrap();

    let supersedes = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::Supersedes)
        .unwrap();
    assert!(supersedes.target_file.is_none());
    assert_eq!(supersedes.target_section.as_deref(), Some("setup"));
}

#[test]
fn graph_from_directives_has_correct_edge_count() {
    let fixtures = Path::new(FIXTURES);
    let mut all = Vec::new();
    for file in md_depgraph::walker::markdown_files(fixtures) {
        all.extend(extract::extract_file(&file).unwrap());
    }
    let graph = Graph::from_directives(&all);
    assert_eq!(graph.edges.len(), 4);
}

#[test]
fn dot_output_contains_edge_labels() {
    let path = Path::new(FIXTURES).join("impl.md");
    let directives = extract::extract_file(&path).unwrap();
    let graph = Graph::from_directives(&directives);
    let dot = graph.to_dot();
    assert!(dot.contains("derived-from"));
    assert!(dot.contains("blocked-by"));
    assert!(dot.contains("digraph"));
}

#[test]
fn json_output_is_valid() {
    let path = Path::new(FIXTURES).join("impl.md");
    let directives = extract::extract_file(&path).unwrap();
    let graph = Graph::from_directives(&directives);
    let json = graph.to_json().unwrap();
    let _: serde_json::Value = serde_json::from_str(&json).unwrap();
}
