//! Command construction and execution.

use std::borrow::Borrow;
use std::fmt;

/// A unique identifier for a command.
///
/// Ids are restricted to ASCII alphanumerics, `-`, and `_`, so they are safe to use unquoted on
/// the command line and as `AsciiDoc` element ids. Construct via [`CommandId::new`] or
/// [`str::parse`]; the inner string is validated on construction and never exposed for mutation.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CommandId(String);

impl CommandId {
    /// Validate `id` and wrap it.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCommandId`] if `id` is empty, starts with `-`, or contains whitespace.
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidCommandId> {
        let id: String = id.into();
        if id.is_empty() {
            return Err(InvalidCommandId::Empty);
        }
        if id.starts_with('-') {
            return Err(InvalidCommandId::LeadingDash { id });
        }
        if let Some(ch) = id.chars().find(|&c| !Self::is_legal(c)) {
            return Err(InvalidCommandId::IllegalChar { id, ch });
        }
        Ok(Self(id))
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_legal(c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
    }
}

impl std::str::FromStr for CommandId {
    type Err = InvalidCommandId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Borrow<str> for CommandId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error returned when a string is not a valid [`CommandId`].
#[derive(Debug, thiserror::Error)]
pub enum InvalidCommandId {
    /// The id was empty.
    #[error("command id must not be empty")]
    Empty,
    /// The id started with `-`, which would collide with command-line flag parsing.
    #[error("command id {id:?} must not start with '-'")]
    LeadingDash {
        /// The rejected id.
        id: String,
    },
    /// The id contained a character outside the allowed set.
    #[error("command id {id:?} contains illegal character {ch:?}")]
    IllegalChar {
        /// The rejected id.
        id: String,
        /// The first offending character.
        ch: char,
    },
}

/// Metadata for a command block.
#[derive(Clone, Debug)]
pub struct CommandMetadata {
    /// The identifier for the command, e.g. `build` or `test`.
    pub id: CommandId,
    /// The shell or runtime to use to execute the command.
    ///
    /// This value is what might appear *last* in the *shebang* of a script, e.g. `python3` for
    /// `#!/usr/bin/env python3`, or `bash` for `#!/usr/bin/env bash`.
    pub shell: String,
}

#[cfg(test)]
mod tests;
