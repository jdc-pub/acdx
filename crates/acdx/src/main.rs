#![allow(missing_docs)]

use std::path::PathBuf;
use std::process;

use acdc_parser as acdc;
use acdx::adoc;
use acdx::cli::Args;
use acdx::command::{CommandGraph, CommandId, UnknownCommand};
use owo_colors::OwoColorize as _;

/// Basenames searched, in precedence order, when no file is given.
const CANDIDATES: [&str; 6] = [
    "COMMANDS.adoc",
    "COMMANDS.asciidoc",
    "DEVELOP.adoc",
    "DEVELOP.asciidoc",
    "README.adoc",
    "README.asciidoc",
];

/// Search for a candidate file, starting in the current directory and walking up toward the
/// filesystem root. The repo root — the first ancestor containing `.git` — is the last directory
/// searched: a project keeps its commands at or below its root, so ascending past it would reach
/// into unrelated parents.
fn discover_file() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        for name in CANDIDATES {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if dir.join(".git").exists() {
            return None;
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn main() {
    let args = Args::parse();

    let file = args.file.unwrap_or_else(|| {
        discover_file().unwrap_or_else(|| {
            eprintln!(
                "acdx: no file given and none of COMMANDS, DEVELOP, README (.adoc/.asciidoc) found"
            );
            process::exit(1);
        })
    });

    let result = acdc::parse_file(&file, &acdc::Options::new()).unwrap_or_else(|e| {
        eprintln!("acdx: {}: {e}", file.display());
        process::exit(1);
    });

    let graph: CommandGraph = result
        .document()
        .try_into()
        .unwrap_or_else(|e: adoc::Error| {
            eprintln!("acdx: {e}");
            process::exit(1);
        });

    if args.commands.is_empty() {
        println!("ACDX Command Runner\n");
        for block in graph {
            println!("{}", block.metadata.id.bold().cyan());
            if args.verbose {
                println!();
                for line in block.script.lines() {
                    println!("  {}", line.yellow());
                }
            }
            println!();
        }
        return;
    }

    let ids: Vec<CommandId> = args
        .commands
        .iter()
        .map(|s| {
            s.parse().unwrap_or_else(|e| {
                eprintln!("acdx: invalid command id {s:?}: {e}");
                process::exit(2);
            })
        })
        .collect();
    let queue = graph.queue_for(&ids).unwrap_or_else(|UnknownCommand(id)| {
        eprintln!("acdx: unknown command: {id}");
        process::exit(1);
    });

    for block in queue {
        if args.dry_run {
            print!("{}", block.script);
        } else {
            block.execute().unwrap_or_else(|e| {
                eprintln!("acdx: {e}");
                process::exit(1);
            });
        }
    }
}
