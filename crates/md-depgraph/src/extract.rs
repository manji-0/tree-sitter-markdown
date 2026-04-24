use std::path::Path;

use anyhow::Context;
use tree_sitter::Parser;

use crate::anchor::{collect_headings, source_section_for};
use crate::resolve::parse_target;
use crate::{Directive, DirectiveKind};

/// Parse a markdown source buffer and return headings slugs (for use in
/// cross-file anchor validation).
pub fn headings_in_file(source: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut parser = make_parser()?;
    let tree = parser
        .parse(source, None)
        .context("tree-sitter parse returned None")?;
    let headings = collect_headings(source, &tree);
    Ok(headings.into_iter().map(|(_, slug)| slug).collect())
}

/// Extract all directive comments from a single Markdown file.
pub fn extract_file(path: &Path) -> anyhow::Result<Vec<Directive>> {
    let source = std::fs::read(path)
        .with_context(|| format!("reading {}", path.display()))?;
    extract_bytes(&source, path)
}

/// Extract directive comments from a byte slice (useful for tests).
pub fn extract_bytes(source: &[u8], path: &Path) -> anyhow::Result<Vec<Directive>> {
    let mut parser = make_parser()?;
    let tree = parser
        .parse(source, None)
        .context("tree-sitter parse returned None")?;

    let headings = collect_headings(source, &tree);

    let mut directives = Vec::new();
    collect_directives(&tree.root_node(), source, path, &headings, &mut directives)?;
    Ok(directives)
}

fn collect_directives(
    node: &tree_sitter::Node<'_>,
    source: &[u8],
    path: &Path,
    headings: &[(usize, String)],
    out: &mut Vec<Directive>,
) -> anyhow::Result<()> {
    if node.kind() == "directive_comment" {
        if let Some(d) = parse_directive_node(node, source, path, headings)? {
            out.push(d);
        }
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_directives(&cursor.node(), source, path, headings, out)?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    Ok(())
}

fn parse_directive_node(
    node: &tree_sitter::Node<'_>,
    source: &[u8],
    path: &Path,
    headings: &[(usize, String)],
) -> anyhow::Result<Option<Directive>> {
    let text = std::str::from_utf8(&source[node.start_byte()..node.end_byte()])
        .context("directive_comment is not valid UTF-8")?
        .trim();

    // text = "<!-- kind target -->" (possibly with trailing newline stripped by trim)
    let inner = text
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
        .map(str::trim);

    let inner = match inner {
        Some(i) => i,
        None => return Ok(None),
    };

    let (kind_str, rest) = inner
        .split_once(char::is_whitespace)
        .map(|(k, r)| (k.trim(), r.trim()))
        .unwrap_or((inner, ""));

    let kind = match kind_str {
        "constrained-by" => DirectiveKind::ConstrainedBy,
        "blocked-by" => DirectiveKind::BlockedBy,
        "supersedes" => DirectiveKind::Supersedes,
        "derived-from" => DirectiveKind::DerivedFrom,
        _ => return Ok(None),
    };

    let (target_file, target_section) = match parse_target(rest, path) {
        Ok(pair) => pair,
        Err(_) => return Ok(None),
    };

    let source_section = source_section_for(headings, node.start_byte());

    Ok(Some(Directive {
        kind,
        source_file: path.to_path_buf(),
        source_section,
        target_file,
        target_section,
        span: (node.start_byte(), node.end_byte()),
    }))
}

fn make_parser() -> anyhow::Result<Parser> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_md::LANGUAGE.into())
        .context("failed to set tree-sitter-md language")?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn extract(src: &str) -> Vec<Directive> {
        extract_bytes(src.as_bytes(), Path::new("test.md")).unwrap()
    }

    #[test]
    fn extracts_constrained_by() {
        let dirs = extract("<!-- constrained-by ./spec.md#rationale -->\n");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].kind, DirectiveKind::ConstrainedBy);
        assert_eq!(dirs[0].target_section.as_deref(), Some("rationale"));
    }

    #[test]
    fn extracts_same_file_anchor() {
        let dirs = extract("<!-- derived-from #design -->\n");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].kind, DirectiveKind::DerivedFrom);
        assert!(dirs[0].target_file.is_none());
        assert_eq!(dirs[0].target_section.as_deref(), Some("design"));
    }

    #[test]
    fn detects_source_section() {
        let src = "## My Heading\n\n<!-- blocked-by #other -->\n";
        let dirs = extract(src);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].source_section.as_deref(), Some("my-heading"));
    }

    #[test]
    fn ignores_plain_html_comment() {
        let dirs = extract("<!-- just a comment -->\n");
        assert_eq!(dirs.len(), 0);
    }

    #[test]
    fn multiple_directives() {
        let src = concat!(
            "<!-- constrained-by ./a.md#sec -->\n",
            "<!-- supersedes ./b.md -->\n",
        );
        let dirs = extract(src);
        assert_eq!(dirs.len(), 2);
    }
}
