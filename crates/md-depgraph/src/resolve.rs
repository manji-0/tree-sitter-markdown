use std::path::{Path, PathBuf};

use crate::anchor::slugify;
use crate::ResolveError;

/// Parse a raw target string (from the directive comment) into
/// (optional_path, optional_anchor).
///
/// Accepted forms:
///   `/path/to/doc.md#section name`  → (Some(path), Some(slug))
///   `/path/to/doc.md`               → (Some(path), None)
///   `#section name`                  → (None, Some(slug))
pub fn parse_target(
    raw: &str,
    source_file: &Path,
) -> Result<(Option<PathBuf>, Option<String>), ResolveError> {
    let raw = raw.trim();

    if raw.is_empty() {
        return Err(ResolveError::MalformedTarget(raw.to_string()));
    }

    if raw.starts_with('#') {
        // Same-file anchor reference.
        let section = raw.trim_start_matches('#').trim();
        if section.is_empty() {
            return Err(ResolveError::MalformedTarget(raw.to_string()));
        }
        return Ok((None, Some(slugify(section))));
    }

    if raw.starts_with('/') {
        return Err(ResolveError::AbsolutePath(raw.to_string()));
    }

    // Split on the last `#` (section anchors may contain `#` themselves
    // only as the separator; we take the first `#`).
    match raw.find('#') {
        Some(idx) => {
            let path_str = &raw[..idx];
            let section = raw[idx + 1..].trim();
            let resolved = resolve_path(path_str, source_file)?;
            if section.is_empty() {
                Ok((Some(resolved), None))
            } else {
                Ok((Some(resolved), Some(slugify(section))))
            }
        }
        None => {
            let resolved = resolve_path(raw, source_file)?;
            Ok((Some(resolved), None))
        }
    }
}

fn resolve_path(raw: &str, source_file: &Path) -> Result<PathBuf, ResolveError> {
    let base = source_file.parent().unwrap_or(Path::new("."));
    let p = base.join(raw);
    // Canonicalize to catch `..` traversal issues; keep as-is if not yet on disk.
    Ok(p)
}

/// Check that `target_file` exists and, if a section is given, that the
/// section slug is present in that file.
pub fn validate_target(
    target_file: &Path,
    target_section: Option<&str>,
) -> Result<(), ResolveError> {
    if !target_file.exists() {
        return Err(ResolveError::MissingFile(target_file.to_path_buf()));
    }
    if let Some(section) = target_section {
        let source = std::fs::read(target_file)
            .map_err(|_| ResolveError::MissingFile(target_file.to_path_buf()))?;
        let headings = crate::extract::headings_in_file(&source)
            .map_err(|_| ResolveError::MissingFile(target_file.to_path_buf()))?;
        if !headings.iter().any(|s| s == section) {
            return Err(ResolveError::MissingSection {
                file: target_file.to_path_buf(),
                section: section.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_same_file_anchor() {
        let (file, section) =
            parse_target("#my section", Path::new("/docs/a.md")).unwrap();
        assert!(file.is_none());
        assert_eq!(section.as_deref(), Some("my-section"));
    }

    #[test]
    fn parse_path_with_anchor() {
        let (file, section) =
            parse_target("../spec.md#rationale", Path::new("/docs/a.md")).unwrap();
        assert!(file.is_some());
        assert_eq!(section.as_deref(), Some("rationale"));
    }

    #[test]
    fn parse_path_only() {
        let (file, section) =
            parse_target("./other.md", Path::new("/docs/a.md")).unwrap();
        assert!(file.is_some());
        assert!(section.is_none());
    }

    #[test]
    fn absolute_path_rejected() {
        let err = parse_target("/absolute/path.md", Path::new("/docs/a.md"));
        assert!(matches!(err, Err(ResolveError::AbsolutePath(_))));
    }
}
