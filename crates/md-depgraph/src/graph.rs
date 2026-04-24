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
        nodes.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.section.cmp(&b.section))
        });

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
            out.push_str(&format!("  n{i} [label=\"{}\"];\n", label.replace('"', "\\\"")));
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
