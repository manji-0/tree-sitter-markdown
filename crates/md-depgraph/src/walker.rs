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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn sorted(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files: Vec<_> = markdown_files(root).collect();
        files.sort();
        files
    }

    #[test]
    fn finds_md_and_markdown_extensions() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.markdown"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();
        let files = sorted(dir.path());
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.file_name().unwrap() == "a.md"));
        assert!(files.iter().any(|p| p.file_name().unwrap() == "b.markdown"));
    }

    #[test]
    fn ignores_non_markdown_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();
        fs::write(dir.path().join("c.html"), "").unwrap();
        assert!(sorted(dir.path()).is_empty());
    }

    #[test]
    fn recurses_into_nested_subdirectories() {
        let dir = TempDir::new().unwrap();
        let deep = dir.path().join("one").join("two");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.md"), "").unwrap();
        let files = sorted(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("deep.md"));
    }

    #[test]
    fn respects_gitignore() {
        let dir = TempDir::new().unwrap();
        // The ignore crate only activates gitignore processing when the walk
        // root is inside a git repository; create a minimal .git dir.
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(dir.path().join("ignored.md"), "").unwrap();
        fs::write(dir.path().join("kept.md"), "").unwrap();
        let files = sorted(dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "kept.md");
    }

    #[test]
    fn skips_hidden_directories() {
        let dir = TempDir::new().unwrap();
        let hidden = dir.path().join(".hidden");
        fs::create_dir_all(&hidden).unwrap();
        fs::write(hidden.join("secret.md"), "").unwrap();
        fs::write(dir.path().join("visible.md"), "").unwrap();
        let files = sorted(dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "visible.md");
    }
}
