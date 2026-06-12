//! Command-line argument parser definitions.

use std::path::PathBuf;

use facet::Facet;
use figue as args;
use figue::FigueBuiltins;

/// Run commands defined in an `AsciiDoc` file.
#[derive(Debug, Facet)]
pub struct Args {
    /// Path to the `AsciiDoc` file. When omitted, the working directory and its ancestors are
    /// searched for `COMMANDS`, `DEVELOP`, or `README` (`.adoc`/`.asciidoc`), stopping at the
    /// repository root.
    #[facet(args::named, args::short = 'f')]
    pub file: Option<PathBuf>,

    /// Print each command's script instead of executing it.
    #[facet(args::named, args::short = 'n', default)]
    pub dry_run: bool,

    /// When listing commands, also print each command's script.
    #[facet(args::named, args::short = 'v')]
    pub verbose: bool,

    /// Commands to run. When empty, the available commands are listed instead of run.
    #[facet(args::positional, default)]
    pub commands: Vec<String>,

    #[facet(flatten)]
    builtins: FigueBuiltins,
}

impl Args {
    /// Parse [`std::env::args`], exiting with a usage message on failure.
    #[must_use]
    pub fn parse() -> Self {
        args::from_std_args().unwrap()
    }
}
