# tree-sitter-markdown (fork) + md-depgraph

[![CI](https://github.com/wataru-manji/treesitter-markdown/actions/workflows/ci.yml/badge.svg)](https://github.com/wataru-manji/treesitter-markdown/actions/workflows/ci.yml)

A [fork](https://github.com/tree-sitter-grammars/tree-sitter-markdown) of tree-sitter-markdown with **dependency directives** added, plus a toolset to describe, extract, and graph semantic dependencies between Markdown documents and sections using HTML comments.

## Directive Syntax

Directives are written as HTML comments, so they remain invisible in standard renderers.

```markdown
<!-- constrained-by path/to/doc.md#section name -->
<!-- blocked-by    path/to/doc.md#section name -->
<!-- supersedes    path/to/doc.md#section name -->
<!-- derived-from  path/to/doc.md#section name -->

<!-- constrained-by #section name -->   <!-- section within the same file -->
<!-- constrained-by path/to/doc.md -->  <!-- dependency on an entire document -->
```

### Directive Kinds

| Kind | Meaning |
|------|---------|
| `constrained-by` | This section is constrained by the target section |
| `blocked-by` | This section cannot proceed until the target section is complete |
| `supersedes` | This section replaces the target section |
| `derived-from` | This section's content is derived from the target section |

### Target Format

| Format | Meaning |
|--------|---------|
| `path/to/doc.md#section name` | A specific section in another document |
| `path/to/doc.md` | An entire other document |
| `#section name` | A section within the same file |

### Source Section Inference

The "source section" of a directive is automatically inferred from the nearest heading preceding the comment. If no heading exists, the directive is treated as a document-level dependency.

### Duplicate Section Names

When the same heading name appears multiple times in a file, a numeric suffix is appended to the second and subsequent occurrences, following GitHub's anchor rules.

```markdown
## Usage        → slug: usage
## Usage        → slug: usage-1
## Usage        → slug: usage-2
```

The target side must also be specified explicitly, e.g. `#usage-1`.

## Installation

```bash
# Set up the devbox environment (Nix-based)
devbox shell

# Build
cargo build --release
```

## CLI Usage

```bash
# Extract directives (JSON output)
md-depgraph extract path/to/docs

# Detect broken references (exit code 1 + stderr)
md-depgraph validate path/to/docs

# Output dependency graph
md-depgraph graph path/to/docs --format dot | dot -Tpng -o graph.png
md-depgraph graph path/to/docs --format json
```

### Example Output

```json
{
  "nodes": [
    {"file": "spec.md", "section": "rationale"},
    {"file": "impl.md", "section": "implementation"}
  ],
  "edges": [
    {
      "source": {"file": "impl.md", "section": "implementation"},
      "target": {"file": "spec.md", "section": "rationale"},
      "kind": "derived-from"
    }
  ]
}
```

## Architecture

```
vendor/tree-sitter-markdown/   ← upstream fork (git subtree)
  tree-sitter-markdown/
    grammar.js                 ← directive_comment rule added
    src/scanner.c              ← <!-- detection branch added

crates/md-depgraph/src/
  walker.rs    ← recursive .md file walk
  extract.rs   ← tree-sitter parse → Directive structs
  anchor.rs    ← source section inference + slugification
  resolve.rs   ← target path/anchor resolution and validation
  graph.rs     ← Graph { nodes, edges } + JSON/DOT output
  bin/         ← CLI (clap)
```

## Development

```bash
# Generate grammar + run grammar tests
devbox run generate
devbox run test-grammar

# Run Rust tests
devbox run test

# Smoke-test with real files
devbox run extract -- crates/md-depgraph/tests/fixtures/project
devbox run validate -- crates/md-depgraph/tests/fixtures/project
devbox run graph -- crates/md-depgraph/tests/fixtures/project --format dot
```

### Syncing with Upstream

```bash
git fetch upstream-ts-md
git subtree pull --prefix=vendor/tree-sitter-markdown upstream-ts-md HEAD --squash
```

### Editing the Grammar

`src/parser.c` is checked in — `cargo build` works without running `tree-sitter generate`.
If you modify `grammar.js` or `src/scanner.c`, regenerate and commit the parser:

```bash
# Inside devbox shell, from vendor/tree-sitter-markdown/tree-sitter-markdown/
devbox run generate   # runs tree-sitter generate
git add vendor/tree-sitter-markdown
git commit -m "chore: regenerate parser after grammar change"
```

## Roadmap

- Integration with `code-review-graph` MCP (merge Markdown dependency graph into the code graph)
- Inline comment support (directives inside paragraphs)
- `--slug-style` option (GitLab / Pandoc compatibility)

## License

MIT — see [LICENSE](LICENSE)
