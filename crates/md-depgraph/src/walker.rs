use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

/// Walk `root` recursively and yield all Markdown file paths.
/// Respects .gitignore and other ignore files.
pub fn markdown_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e, "md" | "markdown"))
                .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
}
