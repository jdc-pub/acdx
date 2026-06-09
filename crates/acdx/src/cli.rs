//! Command-line argument parser definitions.

use std::path::PathBuf;

use clap::Parser;

/// Run commands defined in an `AsciiDoc` file.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Path to the `AsciiDoc` file. When omitted, the working directory and its ancestors are
    /// searched for `COMMANDS`, `DEVELOP`, or `README` (`.adoc`/`.asciidoc`), stopping at the
    /// repository root.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Print each command's script instead of executing it.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// When listing commands, also print each command's script.
    #[arg(short, long)]
    pub verbose: bool,

    /// Commands to run. When empty, the available commands are listed instead of run.
    pub commands: Vec<String>,
}

impl Args {
    /// Parse [`std::env::args`], exiting with a usage message on failure.
    #[must_use]
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
