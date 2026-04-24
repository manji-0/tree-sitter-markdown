use std::path::Path;

use md_depgraph::{extract, graph::Graph, resolve, walker, DirectiveKind, ResolveError};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/project");
const MULTI: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/multi");

// ---------------------------------------------------------------------------
// project/ fixtures (single-file focused)
// ---------------------------------------------------------------------------

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

    let derived = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::DerivedFrom)
        .unwrap();
    assert_eq!(derived.source_section.as_deref(), Some("implementation"));

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
fn graph_from_project_dir_has_correct_edge_count() {
    let fixtures = Path::new(FIXTURES);
    let all = collect_all(fixtures);
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

// ---------------------------------------------------------------------------
// multi/ fixtures — cross-document scenarios
// ---------------------------------------------------------------------------

fn collect_valid_multi() -> Vec<md_depgraph::Directive> {
    // Collect from a.md, b.md, c.md only (exclude broken.md).
    ["a.md", "b.md", "c.md"]
        .iter()
        .flat_map(|name| {
            let path = Path::new(MULTI).join(name);
            extract::extract_file(&path).unwrap()
        })
        .collect()
}

#[test]
fn multi_doc_total_directive_count() {
    // a.md: 3 directives, b.md: 3 directives
    let directives = collect_valid_multi();
    assert_eq!(directives.len(), 6);
}

#[test]
fn multi_doc_cross_file_targets_resolved() {
    let directives = collect_valid_multi();
    // Every cross-file directive should have a non-None target_file.
    let cross_file: Vec<_> = directives
        .iter()
        .filter(|d| d.target_file.is_some())
        .collect();
    // 5 cross-file refs (derived-from b#gamma, constrained-by b#gamma,
    //                     blocked-by c, supersedes a#alpha,
    //                     derived-from a#beta, constrained-by a#beta)
    // — all 6 are cross-file (none are same-file anchors in valid fixtures)
    assert_eq!(cross_file.len(), 6);
}

#[test]
fn multi_doc_multiple_directives_in_one_section() {
    // b.md ## Delta has both derived-from and constrained-by.
    let path = Path::new(MULTI).join("b.md");
    let directives = extract::extract_file(&path).unwrap();

    let delta_dirs: Vec<_> = directives
        .iter()
        .filter(|d| d.source_section.as_deref() == Some("delta"))
        .collect();
    assert_eq!(delta_dirs.len(), 2, "## Delta must have exactly 2 directives");

    let kinds: Vec<_> = delta_dirs.iter().map(|d| d.kind).collect();
    assert!(kinds.contains(&DirectiveKind::DerivedFrom));
    assert!(kinds.contains(&DirectiveKind::ConstrainedBy));
}

#[test]
fn multi_doc_a_beta_has_two_directives() {
    // a.md ## Beta also has 2 directives (constrained-by and blocked-by).
    let path = Path::new(MULTI).join("a.md");
    let directives = extract::extract_file(&path).unwrap();

    let beta_dirs: Vec<_> = directives
        .iter()
        .filter(|d| d.source_section.as_deref() == Some("beta"))
        .collect();
    assert_eq!(beta_dirs.len(), 2);

    let kinds: Vec<_> = beta_dirs.iter().map(|d| d.kind).collect();
    assert!(kinds.contains(&DirectiveKind::ConstrainedBy));
    assert!(kinds.contains(&DirectiveKind::BlockedBy));
}

#[test]
fn multi_doc_graph_edge_count() {
    let directives = collect_valid_multi();
    let graph = Graph::from_directives(&directives);
    assert_eq!(graph.edges.len(), 6);
}

#[test]
fn multi_doc_graph_node_count() {
    let directives = collect_valid_multi();
    let graph = Graph::from_directives(&directives);
    // Unique (file, section) pairs across all edges:
    // a#alpha, a#beta, b#gamma, b#delta, c(file), a#alpha(target of supersedes)
    // → a#alpha, a#beta, b#gamma, b#delta, c
    assert_eq!(graph.nodes.len(), 5);
}

#[test]
fn multi_doc_bidirectional_references_become_two_edges() {
    // a#alpha → b#gamma (derived-from) and b#gamma → a#alpha (supersedes)
    let directives = collect_valid_multi();
    let graph = Graph::from_directives(&directives);

    let a_to_b = graph.edges.iter().filter(|e| {
        e.source.section.as_deref() == Some("alpha")
            && e.target.section.as_deref() == Some("gamma")
    });
    let b_to_a = graph.edges.iter().filter(|e| {
        e.source.section.as_deref() == Some("gamma")
            && e.target.section.as_deref() == Some("alpha")
    });
    assert_eq!(a_to_b.count(), 1);
    assert_eq!(b_to_a.count(), 1);
}

#[test]
fn multi_doc_walker_finds_all_md_files() {
    let count = walker::markdown_files(Path::new(MULTI)).count();
    // a.md, b.md, c.md, broken.md
    assert_eq!(count, 4);
}

// ---------------------------------------------------------------------------
// validate — broken reference detection
// ---------------------------------------------------------------------------

#[test]
fn validate_detects_missing_file() {
    let path = Path::new(MULTI).join("broken.md");
    let directives = extract::extract_file(&path).unwrap();

    let missing_file = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::ConstrainedBy)
        .unwrap();

    let target = missing_file.target_file.as_ref().unwrap();
    let err = resolve::validate_target(target, missing_file.target_section.as_deref())
        .unwrap_err();
    assert!(
        matches!(err, ResolveError::MissingFile(_)),
        "expected MissingFile, got: {err}"
    );
}

#[test]
fn validate_detects_missing_section() {
    let path = Path::new(MULTI).join("broken.md");
    let directives = extract::extract_file(&path).unwrap();

    let missing_section = directives
        .iter()
        .find(|d| d.kind == DirectiveKind::DerivedFrom)
        .unwrap();

    let target = missing_section.target_file.as_ref().unwrap();
    let err = resolve::validate_target(target, missing_section.target_section.as_deref())
        .unwrap_err();
    assert!(
        matches!(err, ResolveError::MissingSection { .. }),
        "expected MissingSection, got: {err}"
    );
}

#[test]
fn validate_valid_cross_file_refs_pass() {
    let directives = collect_valid_multi();
    for d in &directives {
        if let Some(file) = &d.target_file {
            resolve::validate_target(file, d.target_section.as_deref())
                .unwrap_or_else(|e| panic!("unexpected validation error: {e}"));
        }
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn collect_all(root: &Path) -> Vec<md_depgraph::Directive> {
    walker::markdown_files(root)
        .flat_map(|f| extract::extract_file(&f).unwrap())
        .collect()
}
