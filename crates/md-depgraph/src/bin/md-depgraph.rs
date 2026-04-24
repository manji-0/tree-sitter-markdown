use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};

use md_depgraph::{extract, graph::Graph, resolve, walker};

#[derive(Parser)]
#[command(
    name = "md-depgraph",
    version,
    about = "Extract dependency directives from Markdown"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract all directives from Markdown files and print them.
    Extract {
        /// Path to a file or directory.
        path: PathBuf,
        #[arg(long, default_value = "json")]
        format: ExtractFormat,
    },
    /// Validate that all directive targets exist and resolve correctly.
    Validate {
        /// Path to a file or directory.
        path: PathBuf,
    },
    /// Output the dependency graph.
    Graph {
        /// Path to a file or directory.
        path: PathBuf,
        #[arg(long, default_value = "json")]
        format: GraphFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum ExtractFormat {
    Json,
    Jsonl,
}

#[derive(Clone, ValueEnum)]
enum GraphFormat {
    Json,
    Dot,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract { path, format } => {
            let directives = collect_directives(&path)?;
            match format {
                ExtractFormat::Json => println!("{}", serde_json::to_string_pretty(&directives)?),
                ExtractFormat::Jsonl => {
                    for d in &directives {
                        println!("{}", serde_json::to_string(d)?);
                    }
                }
            }
        }

        Command::Validate { path } => {
            let directives = collect_directives(&path)?;
            let mut errors = 0usize;
            for d in &directives {
                // Validate same-file anchor references.
                if d.target_file.is_none() {
                    if let Some(ref section) = d.target_section {
                        if let Err(e) =
                            resolve::validate_target(&d.source_file, Some(section.as_str()))
                        {
                            eprintln!("error: {e}");
                            errors += 1;
                        }
                    }
                    continue;
                }
                let file = d.target_file.as_ref().unwrap();
                let section = d.target_section.as_deref();
                if let Err(e) = resolve::validate_target(file, section) {
                    eprintln!("error: {e}");
                    errors += 1;
                }
            }
            if errors > 0 {
                std::process::exit(1);
            }
            eprintln!("ok: {} directive(s) validated, 0 errors", directives.len());
        }

        Command::Graph { path, format } => {
            let directives = collect_directives(&path)?;
            let graph = Graph::from_directives(&directives);
            match format {
                GraphFormat::Json => println!("{}", graph.to_json()?),
                GraphFormat::Dot => println!("{}", graph.to_dot()),
            }
        }
    }

    Ok(())
}

fn collect_directives(path: &std::path::Path) -> anyhow::Result<Vec<md_depgraph::Directive>> {
    let mut directives = Vec::new();
    if path.is_file() {
        directives.extend(
            extract::extract_file(path)
                .with_context(|| format!("extracting {}", path.display()))?,
        );
    } else {
        for file in walker::markdown_files(path) {
            directives.extend(
                extract::extract_file(&file)
                    .with_context(|| format!("extracting {}", file.display()))?,
            );
        }
    }
    Ok(directives)
}
