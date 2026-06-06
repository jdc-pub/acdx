//! Unit tests for [`CommandId`].

use super::super::*;
use proptest::prelude::*;
use rstest::rstest;

// --------------------------------------------------------------------------
// `CommandId` public members
// --------------------------------------------------------------------------

#[test]
fn command_metadata_holds_id_and_shell() {
    let meta = CommandMetadata {
        id: CommandId::new("build").unwrap(),
        shell: "bash".to_string(),
    };
    assert_eq!(meta.id.as_str(), "build");
    assert_eq!(meta.shell, "bash");
}

// --------------------------------------------------------------------------
// `CommandId::new` validation
// --------------------------------------------------------------------------

#[rstest]
#[case("foo")]
#[case("bar")]
#[case("some-command")]
#[case("cmd_123")]
#[case("a")]
#[case("A")]
#[case("_")]
#[case("_hidden")]
#[case("123")]
#[case("a-b_c-1")]
fn new_accepts_valid_ids(#[case] raw: &str) {
    let id = CommandId::new(raw).expect("id should be valid");
    assert_eq!(id.as_str(), raw);
}

#[test]
fn new_rejects_empty() {
    assert!(matches!(CommandId::new(""), Err(InvalidCommandId::Empty)));
}

#[rstest]
#[case("-foo")]
#[case("-")]
#[case("--bar")]
fn new_rejects_leading_dash(#[case] raw: &str) {
    match CommandId::new(raw) {
        Err(InvalidCommandId::LeadingDash { id }) => assert_eq!(id, raw),
        other => panic!("expected LeadingDash, got {other:?}"),
    }
}

#[rstest]
#[case(" ", ' ')]
#[case(" foo", ' ')]
#[case("foo ", ' ')]
#[case("foo bar", ' ')]
#[case("foo.bar", '.')]
#[case("foo/bar", '/')]
#[case("foo!", '!')]
#[case("héllo", 'é')]
fn new_rejects_illegal_char(#[case] raw: &str, #[case] expected: char) {
    match CommandId::new(raw) {
        Err(InvalidCommandId::IllegalChar { id, ch }) => {
            assert_eq!(id, raw);
            assert_eq!(ch, expected);
        }
        other => panic!("expected IllegalChar, got {other:?}"),
    }
}

// --------------------------------------------------------------------------
// Trait implementations
// --------------------------------------------------------------------------

#[test]
fn from_str_matches_new() {
    let parsed: CommandId = "build".parse().expect("should parse");
    assert_eq!(parsed, CommandId::new("build").unwrap());
}

#[test]
fn from_str_propagates_error() {
    let result: Result<CommandId, _> = "".parse::<CommandId>();
    assert!(matches!(result, Err(InvalidCommandId::Empty)));
}

#[test]
fn borrow_as_str_enables_str_lookup() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(CommandId::new("build").unwrap());

    assert!(set.contains("build"));
}

#[test]
fn display_renders_inner_string() {
    let id = CommandId::new("deploy").unwrap();
    assert_eq!(id.to_string(), "deploy");
    assert_eq!(format!("{id}"), "deploy");
}

// --------------------------------------------------------------------------
// Property-based tests
// --------------------------------------------------------------------------

proptest! {
    #[test]
    fn new_accepts_all_legal_identifiers(raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*") {
        let id = CommandId::new(raw.clone()).expect("legal id should be accepted");
        prop_assert_eq!(id.as_str(), &raw);
    }

    #[test]
    fn new_then_parse_roundtrips(raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*") {
        let id = CommandId::new(raw.clone()).unwrap();
        let reparsed: CommandId = id.as_str().parse().unwrap();
        prop_assert_eq!(id, reparsed);
    }

    #[test]
    fn new_is_deterministic(raw in "[a-zA-Z0-9_][a-zA-Z0-9_-]*") {
        let a = CommandId::new(raw.clone()).unwrap();
        let b = CommandId::new(raw).unwrap();
        prop_assert_eq!(a, b);
    }
}
