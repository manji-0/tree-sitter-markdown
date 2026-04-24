/// Compute the GitHub-compatible slug for a heading text.
/// Rules: lowercase, spaces → `-`, keep alphanumeric and `-_`, strip the rest.
pub fn slugify(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() {
                vec![c.to_ascii_lowercase()]
            } else if c == ' ' || c == '-' {
                vec!['-']
            } else if c == '_' {
                vec!['_']
            } else {
                vec![]
            }
        })
        .collect()
}

/// Extract all heading texts from a parsed tree-sitter tree in document order.
/// Returns (byte_start_of_heading_node, slug) pairs.
/// Duplicate heading texts receive GitHub-style numeric suffixes on 2nd+ occurrence:
/// first → "usage", second → "usage-1", third → "usage-2", etc.
pub fn collect_headings(
    source: &[u8],
    tree: &tree_sitter::Tree,
) -> Vec<(usize, String)> {
    let mut raw: Vec<(usize, String)> = Vec::new();
    let mut cursor = tree.walk();
    collect_headings_recursive(&mut cursor, source, &mut raw);

    // Assign GitHub-compatible unique slugs. Track (base → count_so_far) and
    // the full set of already-assigned slugs so we can skip collisions even
    // when a literal heading like "Usage 1" occupies "usage-1" early.
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();

    raw.into_iter()
        .map(|(start, base)| {
            let n = counts.entry(base.clone()).or_insert(0);
            let slug = if *n == 0 {
                if assigned.contains(&base) {
                    // Literal collision: find next available number.
                    let mut k = 1usize;
                    loop {
                        let candidate = format!("{base}-{k}");
                        if !assigned.contains(&candidate) {
                            break candidate;
                        }
                        k += 1;
                    }
                } else {
                    base.clone()
                }
            } else {
                // 2nd+ occurrence: find next available number.
                let mut k = *n;
                loop {
                    let candidate = format!("{base}-{k}");
                    if !assigned.contains(&candidate) {
                        break candidate;
                    }
                    k += 1;
                }
            };
            *n += 1;
            assigned.insert(slug.clone());
            (start, slug)
        })
        .collect()
}

fn collect_headings_recursive(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    source: &[u8],
    out: &mut Vec<(usize, String)>,
) {
    let node = cursor.node();
    let kind = node.kind();

    if kind == "atx_heading" || kind == "setext_heading" {
        let text = heading_text(node, source);
        out.push((node.start_byte(), slugify(&text)));
    }

    if cursor.goto_first_child() {
        loop {
            collect_headings_recursive(cursor, source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Extract the plain text of a heading node (skipping the marker tokens).
fn heading_text(node: tree_sitter::Node<'_>, source: &[u8]) -> String {
    // Children of atx_heading: marker, optional inline content
    // Children of setext_heading: paragraph, underline
    let mut parts = Vec::new();
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let k = child.kind();
            if k == "atx_h1_marker"
                || k == "atx_h2_marker"
                || k == "atx_h3_marker"
                || k == "atx_h4_marker"
                || k == "atx_h5_marker"
                || k == "atx_h6_marker"
                || k == "setext_h1_underline"
                || k == "setext_h2_underline"
            {
                // skip
            } else if let Ok(t) = std::str::from_utf8(&source[child.start_byte()..child.end_byte()])
            {
                parts.push(t.trim().to_string());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    parts.join(" ").trim().to_string()
}

/// Given the list of (heading_byte_start, slug) pairs and the byte position of
/// a directive, return the slug of the immediately preceding heading (if any).
pub fn source_section_for(
    headings: &[(usize, String)],
    directive_start: usize,
) -> Option<String> {
    headings
        .iter()
        .filter(|(hstart, _)| *hstart < directive_start)
        .last()
        .map(|(_, slug)| slug.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("My Section"), "my-section");
        assert_eq!(slugify("API_v2"), "api_v2");
        // '&' is stripped, adjacent spaces each become '-'
        assert_eq!(slugify("foo & bar!"), "foo--bar");
    }

    #[test]
    fn duplicate_headings_get_github_suffixes() {
        // Verify the suffix logic directly without requiring tree-sitter.
        // We simulate what collect_headings produces for raw slugs.
        let raw = vec!["usage", "api", "usage", "usage", "api"];
        let mut counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut assigned: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let result: Vec<String> = raw
            .into_iter()
            .map(|base| {
                let base = base.to_string();
                let n = counts.entry(base.clone()).or_insert(0);
                let slug = if *n == 0 {
                    if assigned.contains(&base) {
                        let mut k = 1usize;
                        loop {
                            let c = format!("{base}-{k}");
                            if !assigned.contains(&c) {
                                break c;
                            }
                            k += 1;
                        }
                    } else {
                        base.clone()
                    }
                } else {
                    let mut k = *n;
                    loop {
                        let c = format!("{base}-{k}");
                        if !assigned.contains(&c) {
                            break c;
                        }
                        k += 1;
                    }
                };
                *n += 1;
                assigned.insert(slug.clone());
                slug
            })
            .collect();
        assert_eq!(
            result,
            vec!["usage", "api", "usage-1", "usage-2", "api-1"]
        );
    }

    #[test]
    fn source_section_before_directive() {
        let headings = vec![
            (0, "intro".to_string()),
            (100, "impl".to_string()),
            (200, "outro".to_string()),
        ];
        assert_eq!(
            source_section_for(&headings, 150),
            Some("impl".to_string())
        );
        assert_eq!(source_section_for(&headings, 50), Some("intro".to_string()));
        // heading at byte 0 < directive at byte 5 → "intro" is the preceding heading
        assert_eq!(source_section_for(&headings, 5), Some("intro".to_string()));
    }
}
