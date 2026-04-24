# tree-sitter-markdown (fork) + md-depgraph

tree-sitter-markdown の [フォーク](https://github.com/tree-sitter-grammars/tree-sitter-markdown) に**依存関係ディレクティブ**を追加し、Markdown 文書間・章間の意味的な依存関係を HTML コメントで記述・抽出・グラフ化するツールセットです。

## ディレクティブ書式

Markdown の HTML コメントとして記述するため、通常のレンダラでは非表示になります。

```markdown
<!-- constrained-by path/to/doc.md#section name -->
<!-- blocked-by    path/to/doc.md#section name -->
<!-- supersedes    path/to/doc.md#section name -->
<!-- derived-from  path/to/doc.md#section name -->

<!-- constrained-by #section name -->   <!-- 同一ファイル内の章 -->
<!-- constrained-by path/to/doc.md -->  <!-- 文書全体への依存 -->
```

### ディレクティブの意味

| 種別 | 意味 |
|------|------|
| `constrained-by` | この章の内容は対象章の制約を受ける |
| `blocked-by` | 対象章が完了するまでこの章を進められない |
| `supersedes` | この章は対象章を置き換える |
| `derived-from` | この章の内容は対象章から派生している |

### target の書式

| 書式 | 意味 |
|------|------|
| `path/to/doc.md#section name` | 他文書の特定章 |
| `path/to/doc.md` | 他文書全体 |
| `#section name` | 同一ファイル内の章 |

### source 側のアンカー推論

ディレクティブの「発信元章」は、コメント直前の最も近い見出しから自動推論されます。見出しがない場合は文書全体への依存として扱われます。

### 同名セクションの扱い

同一ファイル内に同じ名前の見出しが複数あるとき、GitHub の README アンカー規則に準拠して 2 回目以降に連番サフィックスを付与します。

```markdown
## Usage        → slug: usage
## Usage        → slug: usage-1
## Usage        → slug: usage-2
```

target 側も `#usage-1` のように明示的に指定します。

## インストール

```bash
# devbox 環境のセットアップ（Nix ベース）
devbox shell

# ビルド
cargo build --release
```

## CLI 使い方

```bash
# ディレクティブ抽出（JSON）
md-depgraph extract path/to/docs

# 壊れた参照を検出（exit code 1 + stderr）
md-depgraph validate path/to/docs

# 依存グラフ出力
md-depgraph graph path/to/docs --format dot | dot -Tpng -o graph.png
md-depgraph graph path/to/docs --format json
```

### 出力例

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

## アーキテクチャ

```
vendor/tree-sitter-markdown/   ← upstream フォーク (git subtree)
  tree-sitter-markdown/
    grammar.js                 ← directive_comment ルール追加
    src/scanner.c              ← <!-- 判定分岐追加

crates/md-depgraph/src/
  walker.rs    ← .md ファイル再帰 walk
  extract.rs   ← tree-sitter parse → Directive 構造体
  anchor.rs    ← source 章推論 + slug 化
  resolve.rs   ← target path/anchor 解決・検証
  graph.rs     ← Graph { nodes, edges } + JSON/DOT 出力
  bin/         ← CLI (clap)
```

## 開発

```bash
# 文法生成 + テスト
devbox run generate
devbox run test-grammar

# Rust テスト
devbox run test

# 実際のファイルで動作確認
devbox run extract -- crates/md-depgraph/tests/fixtures/project
devbox run validate -- crates/md-depgraph/tests/fixtures/project
devbox run graph -- crates/md-depgraph/tests/fixtures/project --format dot
```

### upstream との同期

```bash
git fetch upstream-ts-md
git subtree pull --prefix=vendor/tree-sitter-markdown upstream-ts-md HEAD --squash
```

## 将来の展望

- `code-review-graph` MCP との統合（Markdown 文書の依存グラフをコードグラフに合流）
- inline コメント対応（段落内ディレクティブ）
- `--slug-style` オプション（GitLab / Pandoc 準拠）
