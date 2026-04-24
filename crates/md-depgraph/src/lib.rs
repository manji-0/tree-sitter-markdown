pub mod anchor;
pub mod extract;
pub mod graph;
pub mod resolve;
pub mod walker;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectiveKind {
    ConstrainedBy,
    BlockedBy,
    Supersedes,
    DerivedFrom,
}

impl DirectiveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DirectiveKind::ConstrainedBy => "constrained-by",
            DirectiveKind::BlockedBy => "blocked-by",
            DirectiveKind::Supersedes => "supersedes",
            DirectiveKind::DerivedFrom => "derived-from",
        }
    }
}

impl std::fmt::Display for DirectiveKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single dependency directive extracted from a Markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directive {
    pub kind: DirectiveKind,
    pub source_file: PathBuf,
    /// Slug of the heading that immediately precedes this directive.
    /// None means the directive is at document scope.
    pub source_section: Option<String>,
    /// Resolved target file. None when the directive references a section in
    /// the same file (`<!-- kind #anchor -->`).
    pub target_file: Option<PathBuf>,
    pub target_section: Option<String>,
    /// Byte offset span [start, end) of the directive comment in source_file.
    pub span: (usize, usize),
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("malformed target: {0}")]
    MalformedTarget(String),
    #[error("file not found: {}", .0.display())]
    MissingFile(PathBuf),
    #[error("section '#{section}' not found in {}", .file.display())]
    MissingSection { file: PathBuf, section: String },
    #[error("absolute paths are not allowed in targets: {0}")]
    AbsolutePath(String),
}
