//! Unit tests for `command.rs`.

use super::*;
mod command_block;
mod command_graph;
mod command_id;

// --------------------------------------------------------------------------
// Global validation
// --------------------------------------------------------------------------

#[test]
fn default_shell_is_sh() {
    assert!(DEFAULT_SHELL == "sh");
}
