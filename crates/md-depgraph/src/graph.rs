use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Directive, DirectiveKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    pub file: PathBuf,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub kind: DirectiveKind,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<NodeId>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn from_directives(directives: &[Directive]) -> Self {
        let mut node_set: HashMap<NodeId, ()> = HashMap::new();
        let mut edges = Vec::new();

        for d in directives {
            let source = NodeId {
                file: d.source_file.clone(),
                section: d.source_section.clone(),
            };
            let target = NodeId {
                file: d
                    .target_file
                    .clone()
                    .unwrap_or_else(|| d.source_file.clone()),
                section: d.target_section.clone(),
            };

            node_set.insert(source.clone(), ());
            node_set.insert(target.clone(), ());
            edges.push(Edge {
                source,
                target,
                kind: d.kind,
            });
        }

        let mut nodes: Vec<NodeId> = node_set.into_keys().collect();
        nodes.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.section.cmp(&b.section)));

        Graph { nodes, edges }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_dot(&self) -> String {
        let mut out = String::from("digraph {\n  rankdir=LR;\n  node [shape=box];\n");

        // Assign stable IDs to nodes.
        let id_of: HashMap<&NodeId, usize> =
            self.nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();

        for (i, node) in self.nodes.iter().enumerate() {
            let label = match &node.section {
                Some(s) => format!("{}#{}", node.file.display(), s),
                None => node.file.display().to_string(),
            };
            out.push_str(&format!(
                "  n{i} [label=\"{}\"];\n",
                label.replace('"', "\\\"")
            ));
        }

        for edge in &self.edges {
            let src = id_of[&edge.source];
            let tgt = id_of[&edge.target];
            let (color, style) = edge_style(edge.kind);
            out.push_str(&format!(
                "  n{src} -> n{tgt} [label=\"{}\", color=\"{color}\", style=\"{style}\"];\n",
                edge.kind
            ));
        }

        out.push('}');
        out
    }
}

fn edge_style(kind: DirectiveKind) -> (&'static str, &'static str) {
    match kind {
        DirectiveKind::ConstrainedBy => ("blue", "solid"),
        DirectiveKind::BlockedBy => ("red", "solid"),
        DirectiveKind::Supersedes => ("orange", "dashed"),
        DirectiveKind::DerivedFrom => ("green", "solid"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Directive, DirectiveKind};

    fn dir(
        kind: DirectiveKind,
        src: &str,
        src_sec: Option<&str>,
        tgt: Option<&str>,
        tgt_sec: Option<&str>,
    ) -> Directive {
        Directive {
            kind,
            source_file: PathBuf::from(src),
            source_section: src_sec.map(str::to_string),
            target_file: tgt.map(PathBuf::from),
            target_section: tgt_sec.map(str::to_string),
            span: (0, 0),
        }
    }

    #[test]
    fn empty_graph_dot_has_no_nodes_or_edges() {
        let dot = Graph::default().to_dot();
        assert!(dot.contains("digraph {"));
        assert!(!dot.contains("n0"));
    }

    #[test]
    fn dot_label_escapes_double_quotes() {
        let d = dir(
            DirectiveKind::DerivedFrom,
            "a.md",
            Some("say-\"hello\""),
            Some("b.md"),
            None,
        );
        let dot = Graph::from_directives(&[d]).to_dot();
        // The literal characters \" must appear inside the DOT label.
        assert!(
            dot.contains("\\\""),
            "double-quote not escaped in DOT:\n{dot}"
        );
    }

    #[test]
    fn all_four_edge_styles_appear_in_dot() {
        let directives = vec![
            dir(
                DirectiveKind::ConstrainedBy,
                "a.md",
                None,
                Some("b.md"),
                None,
            ),
            dir(DirectiveKind::BlockedBy, "c.md", None, Some("d.md"), None),
            dir(DirectiveKind::Supersedes, "e.md", None, Some("f.md"), None),
            dir(DirectiveKind::DerivedFrom, "g.md", None, Some("h.md"), None),
        ];
        let dot = Graph::from_directives(&directives).to_dot();
        assert!(dot.contains("color=\"blue\""));
        assert!(dot.contains("color=\"red\""));
        assert!(dot.contains("color=\"orange\""));
        assert!(dot.contains("color=\"green\""));
        assert!(dot.contains("style=\"solid\""));
        assert!(dot.contains("style=\"dashed\""));
    }

    #[test]
    fn to_dot_is_deterministic() {
        let directives = vec![
            dir(
                DirectiveKind::ConstrainedBy,
                "z.md",
                None,
                Some("a.md"),
                None,
            ),
            dir(DirectiveKind::BlockedBy, "m.md", None, Some("b.md"), None),
        ];
        let g = Graph::from_directives(&directives);
        assert_eq!(g.to_dot(), g.to_dot());
    }

    #[test]
    fn to_json_produces_valid_json_with_nodes_and_edges() {
        let d = dir(DirectiveKind::DerivedFrom, "a.md", None, Some("b.md"), None);
        let json = Graph::from_directives(&[d]).to_json().unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(v["edges"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn nodes_are_sorted_by_file_then_section() {
        let directives = vec![
            dir(
                DirectiveKind::DerivedFrom,
                "b.md",
                Some("z"),
                Some("a.md"),
                Some("a"),
            ),
            dir(
                DirectiveKind::DerivedFrom,
                "a.md",
                Some("m"),
                Some("a.md"),
                Some("a"),
            ),
        ];
        let g = Graph::from_directives(&directives);
        let files: Vec<_> = g.nodes.iter().map(|n| n.file.to_str().unwrap()).collect();
        let sorted = {
            let mut f = files.clone();
            f.sort();
            f
        };
        assert_eq!(files, sorted);
    }
}
