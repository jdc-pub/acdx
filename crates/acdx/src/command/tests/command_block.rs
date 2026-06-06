//! Unit tests for [`CommandBlock`].

use super::super::*;
use proptest::prelude::*;
use rstest::rstest;

// --------------------------------------------------------------------------
// `CommandBlock::new` — field access
// --------------------------------------------------------------------------

#[test]
fn new_stores_id_script_and_default_shell() {
    let id = CommandId::new("build").unwrap();
    let block = CommandBlock::new(id.clone(), "cargo build".into(), None);
    assert_eq!(block.metadata.id, id);
    assert_eq!(block.metadata.shell, "sh");
    assert_eq!(block.script, "cargo build\n");
}

#[test]
fn new_stores_explicit_shell() {
    let id = CommandId::new("test").unwrap();
    let block = CommandBlock::new(id, String::new(), Some("bash".into()));
    assert_eq!(block.metadata.shell, "bash");
}

#[rstest]
#[case("bash")]
#[case("python3")]
#[case("sh")]
#[case("zsh")]
fn new_stores_any_shell_string(#[case] shell: &str) {
    let id = CommandId::new("run").unwrap();
    let block = CommandBlock::new(id, String::new(), Some(shell.into()));
    assert_eq!(block.metadata.shell, shell);
}

// --------------------------------------------------------------------------
// Trailing newline invariant
// --------------------------------------------------------------------------

// TODO investigate platform limitations with `\n`

#[test]
fn new_appends_newline_when_missing() {
    let id = CommandId::new("build").unwrap();
    let block = CommandBlock::new(id, "cargo build".into(), None);
    assert!(block.script.ends_with('\n'));
    assert_eq!(block.script, "cargo build\n");
}

#[test]
fn new_does_not_double_newline() {
    let id = CommandId::new("build").unwrap();
    let block = CommandBlock::new(id, "cargo build\n".into(), None);
    assert_eq!(block.script, "cargo build\n");
}

#[test]
fn new_empty_script_gets_newline() {
    let id = CommandId::new("noop").unwrap();
    let block = CommandBlock::new(id, String::new(), None);
    assert_eq!(block.script, "\n");
}

// --------------------------------------------------------------------------
// Multi-line scripts
// --------------------------------------------------------------------------

#[test]
fn new_preserves_multiline_script() {
    let id = CommandId::new("build").unwrap();
    let script = "set -e\ncargo build\necho done\n".to_string();
    let block = CommandBlock::new(id, script.clone(), None);
    assert_eq!(block.script, script);
}

// --------------------------------------------------------------------------
// Clone
// --------------------------------------------------------------------------

// TODO this is likely unnecessary; investigate

#[test]
fn clone_is_independent() {
    let id = CommandId::new("deploy").unwrap();
    let block = CommandBlock::new(id, "echo deploy".into(), Some("bash".into()));
    let mut cloned = block.clone();
    cloned.script.push_str("echo done\n");
    assert_eq!(block.script, "echo deploy\n");
    assert_eq!(cloned.script, "echo deploy\necho done\n");
}

// --------------------------------------------------------------------------
// Property-based tests
// --------------------------------------------------------------------------

proptest! {
    #[test]
    fn new_preserves_id(raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*") {
        let id = CommandId::new(raw.clone()).unwrap();
        let block = CommandBlock::new(id, String::new(), None);
        prop_assert_eq!(block.metadata.id.as_str(), &raw);
    }

    #[test]
    fn new_uses_sh_when_shell_is_none(raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*") {
        let id = CommandId::new(raw).unwrap();
        let block = CommandBlock::new(id, String::new(), None);
        prop_assert_eq!(block.metadata.shell, "sh");
    }

    #[test]
    fn script_always_ends_with_newline(
        raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*",
        script in ".*",
    ) {
        let id = CommandId::new(raw).unwrap();
        let block = CommandBlock::new(id, script, None);
        prop_assert!(block.script.ends_with('\n'));
    }
}
