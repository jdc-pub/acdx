//! Command construction and execution.

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt;

use petgraph::acyclic::{Acyclic, AcyclicEdgeError};
use petgraph::data::Build;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, Reversed};

const DEFAULT_SHELL: &str = "sh";

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

/// A single command with its metadata.
#[derive(Clone, Debug)]
pub struct CommandBlock {
    /// The metadata associated with this command block.
    pub metadata: CommandMetadata,
    /// The script body.
    pub script: String,
}

impl CommandBlock {
    /// Construct a [`CommandBlock`], defaulting `shell` to `"sh"` when absent.
    #[must_use]
    pub fn new(id: CommandId, mut script: String, shell: Option<String>) -> Self {
        if !script.ends_with('\n') {
            script.push('\n');
        }
        Self {
            metadata: CommandMetadata {
                id,
                shell: shell.unwrap_or_else(|| DEFAULT_SHELL.to_string()),
            },
            script,
        }
    }

    /// Execute the script.
    ///
    /// # Errors
    ///
    /// TODO!
    #[allow(clippy::result_unit_err)]
    pub fn execute(self) -> Result<(), ()> {
        todo!()
    }
}

/// Error returned when building a [`CommandGraph`] from a [`CommandGraphBuilder`] fails.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Two commands share the same id.
    #[error("duplicate command id: {0}")]
    DuplicateId(CommandId),
    /// A declared dependency does not name any added command.
    #[error("unknown dependency: {0}")]
    UnknownDep(CommandId),
    /// A dependency edge would introduce a cycle.
    ///
    /// The edge runs from `dep` (the prerequisite) to `command` (the dependent); adding it would
    /// make `command` reachable from itself. A self-dependency has `dep == command`.
    #[error("dependency cycle: {dep} -> {command}")]
    Cycle {
        /// The dependent command.
        command: CommandId,
        /// The prerequisite that closes the cycle.
        dep: CommandId,
    },
}

/// Error returned when a command id is not found in a [`CommandGraph`].
#[derive(Debug, thiserror::Error)]
#[error("unknown command: {0}")]
pub struct UnknownCommand(pub CommandId);

/// A directed acyclic graph of commands, typically pulled from a single `AsciiDoc` file.
///
/// Edges run from prerequisite to dependent: an edge `A → B` means "A must run before B."
#[derive(Debug)]
pub struct CommandGraph {
    inner: Acyclic<DiGraph<CommandBlock, ()>>,
    index: HashMap<CommandId, NodeIndex>,
}

impl CommandGraph {
    fn collect_queue(&self, filter: &HashSet<NodeIndex>) -> CommandQueue {
        // Ordering is load-bearing: `Acyclic::nodes_iter` yields nodes in topological
        // order, and filtering by membership preserves it. Do not iterate `filter`.
        let graph: &DiGraph<CommandBlock, ()> = &self.inner;
        let queue: Vec<CommandBlock> = self
            .inner
            .nodes_iter()
            .filter(|idx| filter.contains(idx))
            .map(|idx| graph[idx].clone())
            .collect();
        CommandQueue(queue.into_iter())
    }

    /// Build a [`CommandQueue`] for the given commands and all of their transitive dependencies,
    /// in execution order.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownCommand`] if any id in `ids` is not in the graph.
    pub fn queue_for(&self, ids: &[CommandId]) -> Result<CommandQueue, UnknownCommand> {
        let rev = Reversed(&self.inner);
        let mut dfs = Dfs::empty(rev);
        let mut relevant: HashSet<NodeIndex> = HashSet::new();

        for id in ids {
            let &target = self
                .index
                .get(id)
                .ok_or_else(|| UnknownCommand(id.clone()))?;

            dfs.move_to(target);
            while let Some(node) = dfs.next(rev) {
                relevant.insert(node);
            }
        }

        Ok(self.collect_queue(&relevant))
    }
}

#[cfg(test)]
impl CommandGraph {
    /// Number of edges in the underlying graph; used to assert deps are deduplicated.
    fn edge_count(&self) -> usize {
        let graph: &DiGraph<CommandBlock, ()> = &self.inner;
        graph.edge_count()
    }
}

/// Builds a [`CommandGraph`] from commands added in any order.
///
/// Commands may declare dependencies on ids that have not been added yet; all ids are resolved
/// once, at [`build`](CommandGraphBuilder::build). This lifts the ordering constraint that direct
/// insertion would impose, at the cost of deferring every validation error to build time.
#[derive(Debug, Default)]
pub struct CommandGraphBuilder {
    pending: Vec<(CommandBlock, Vec<CommandId>)>,
}

impl CommandGraphBuilder {
    /// Create an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command and the ids of its prerequisites.
    ///
    /// Ids are not validated here; duplicates, unknown deps, and cycles are all reported by
    /// [`build`](CommandGraphBuilder::build).
    pub fn add(&mut self, block: CommandBlock, deps: Vec<CommandId>) {
        self.pending.push((block, deps));
    }

    /// Resolve all commands into a [`CommandGraph`].
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::DuplicateId`] if two commands share an id, [`BuildError::UnknownDep`]
    /// if a dep names no added command, or [`BuildError::Cycle`] if the dependencies are cyclic.
    pub fn build(self) -> Result<CommandGraph, BuildError> {
        let mut graph = CommandGraph {
            inner: Acyclic::default(),
            index: HashMap::new(),
        };

        // Pass 1: add every node and index its id, so deps can be resolved in any order.
        let mut edges: Vec<(NodeIndex, Vec<CommandId>)> = Vec::with_capacity(self.pending.len());
        for (block, deps) in self.pending {
            let id = block.metadata.id.clone();
            if graph.index.contains_key::<str>(id.borrow()) {
                return Err(BuildError::DuplicateId(id));
            }
            let node = graph.inner.add_node(block);
            graph.index.insert(id, node);
            edges.push((node, deps));
        }

        // Pass 2: wire edges. Cycles are genuinely possible now, so check them.
        for (node, deps) in edges {
            let mut seen: HashSet<NodeIndex> = HashSet::new();
            for dep in deps {
                let dep_node = *graph
                    .index
                    .get::<str>(dep.borrow())
                    .ok_or_else(|| BuildError::UnknownDep(dep.clone()))?;
                // A dep listed twice would add a parallel edge; skip the repeat.
                if !seen.insert(dep_node) {
                    continue;
                }
                match graph.inner.try_add_edge(dep_node, node, ()) {
                    Ok(_) => {}
                    Err(AcyclicEdgeError::Cycle(_) | AcyclicEdgeError::SelfLoop) => {
                        let command = graph.inner[node].metadata.id.clone();
                        return Err(BuildError::Cycle { command, dep });
                    }
                    Err(AcyclicEdgeError::InvalidEdge) => {
                        // See Pass 1.
                        unreachable!("both endpoints were added to the graph");
                    }
                }
            }
        }

        Ok(graph)
    }
}

impl IntoIterator for CommandGraph {
    type Item = CommandBlock;
    type IntoIter = CommandQueue;

    fn into_iter(self) -> Self::IntoIter {
        // Owned path: move blocks out instead of cloning. `nodes_iter` gives the
        // topological order; `into_nodes_edges` gives the blocks in node-index order
        // (contiguous, since `inner` is a `DiGraph`), which we re-sequence by taking
        // each out exactly once.
        let order: Vec<NodeIndex> = self.inner.nodes_iter().collect();
        let (nodes, _edges) = self.inner.into_inner().into_nodes_edges();
        let mut blocks: Vec<Option<CommandBlock>> =
            nodes.into_iter().map(|n| Some(n.weight)).collect();
        let queue: Vec<CommandBlock> = order
            .into_iter()
            .map(|idx| blocks[idx.index()].take().expect("each node taken once"))
            .collect();
        CommandQueue(queue.into_iter())
    }
}

/// A queue of commands in execution order, produced by [`CommandGraph::queue_for`] or by
/// iterating a [`CommandGraph`].
#[derive(Debug)]
pub struct CommandQueue(std::vec::IntoIter<CommandBlock>);

impl Iterator for CommandQueue {
    type Item = CommandBlock;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for CommandQueue {}

#[cfg(test)]
mod tests;
