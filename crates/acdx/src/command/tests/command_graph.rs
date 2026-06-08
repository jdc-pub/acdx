//! Unit tests for [`CommandGraph`], [`CommandGraphBuilder`], and [`CommandQueue`].

use super::super::*;
use proptest::prelude::*;
use rstest::rstest;

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

/// Exclusive upper bound on node count for the random-DAG property test.
const N_MAX: usize = 12;

fn id(s: &str) -> CommandId {
    CommandId::new(s).expect("test id should be valid")
}

fn block(name: &str) -> CommandBlock {
    CommandBlock::new(id(name), format!("echo \"{name}\""), None)
}

/// Build a graph from `(command, [deps])` specs, expecting success.
fn graph(specs: &[(&str, &[&str])]) -> CommandGraph {
    build(specs).expect("graph should build")
}

/// Attempt to build a graph from `(command, [deps])` specs.
fn build(specs: &[(&str, &[&str])]) -> Result<CommandGraph, BuildError> {
    let mut builder = CommandGraphBuilder::new();
    for (name, deps) in specs {
        builder.add(block(name), deps.iter().map(|d| id(d)).collect());
    }
    builder.build()
}

/// Collect a queue's command ids, in order.
fn order(queue: CommandQueue) -> Vec<String> {
    queue.map(|b| b.metadata.id.as_str().to_string()).collect()
}

/// Index of `name` within an ordered id list.
fn pos(ids: &[String], name: &str) -> usize {
    ids.iter()
        .position(|s| s == name)
        .unwrap_or_else(|| panic!("{name:?} missing from {ids:?}"))
}

// --------------------------------------------------------------------------
// `CommandGraphBuilder::build` — success
// --------------------------------------------------------------------------

#[test]
fn build_empty_yields_empty_graph() {
    let graph = CommandGraphBuilder::new().build().unwrap();
    assert_eq!(graph.into_iter().count(), 0);
}

#[test]
fn build_single_command() {
    let graph = graph(&[("build", &[])]);
    assert_eq!(order(graph.into_iter()), ["build"]);
}

#[test]
fn build_independent_commands() {
    let graph = graph(&[("a", &[]), ("b", &[]), ("c", &[])]);
    let mut ids = order(graph.into_iter());
    ids.sort();
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn build_resolves_forward_referenced_dep() {
    // `build` depends on `gen`, which is added *after* it.
    let graph = graph(&[("build", &["gen"]), ("gen", &[])]);
    let ids = order(graph.into_iter());
    assert!(pos(&ids, "gen") < pos(&ids, "build"));
}

#[test]
fn build_orders_chain_dependencies() {
    let graph = graph(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
    assert_eq!(order(graph.into_iter()), ["a", "b", "c"]);
}

#[test]
fn build_orders_diamond_dependencies() {
    let graph = graph(&[("d", &["b", "c"]), ("b", &["a"]), ("c", &["a"]), ("a", &[])]);
    let ids = order(graph.into_iter());
    assert!(pos(&ids, "a") < pos(&ids, "b"));
    assert!(pos(&ids, "a") < pos(&ids, "c"));
    assert!(pos(&ids, "b") < pos(&ids, "d"));
    assert!(pos(&ids, "c") < pos(&ids, "d"));
}

#[test]
fn build_dedupes_duplicate_dep() {
    let graph = graph(&[("b", &["a", "a"]), ("a", &[])]);
    assert_eq!(
        graph.edge_count(),
        1,
        "duplicate dep must not add a parallel edge"
    );
    assert_eq!(order(graph.into_iter()), ["a", "b"]);
}

// --------------------------------------------------------------------------
// `CommandGraphBuilder::build` — errors
// --------------------------------------------------------------------------

#[test]
fn build_rejects_duplicate_id() {
    match build(&[("build", &[]), ("build", &[])]) {
        Err(BuildError::DuplicateId(got)) => assert_eq!(got, id("build")),
        other => panic!("expected DuplicateId, got {other:?}"),
    }
}

#[test]
fn build_rejects_unknown_dep() {
    match build(&[("build", &["missing"])]) {
        Err(BuildError::UnknownDep(got)) => assert_eq!(got, id("missing")),
        other => panic!("expected UnknownDep, got {other:?}"),
    }
}

#[test]
fn build_rejects_self_dependency() {
    match build(&[("a", &["a"])]) {
        Err(BuildError::Cycle { command, dep }) => {
            assert_eq!(command, id("a"));
            assert_eq!(dep, id("a"));
        }
        other => panic!("expected Cycle, got {other:?}"),
    }
}

#[test]
fn build_rejects_two_node_cycle() {
    match build(&[("a", &["b"]), ("b", &["a"])]) {
        Err(BuildError::Cycle { command, dep }) => {
            assert_eq!(command, id("b"));
            assert_eq!(dep, id("a"));
        }
        other => panic!("expected Cycle, got {other:?}"),
    }
}

#[test]
fn build_rejects_longer_cycle() {
    assert!(matches!(
        build(&[("a", &["c"]), ("b", &["a"]), ("c", &["b"])]),
        Err(BuildError::Cycle { .. })
    ));
}

// --------------------------------------------------------------------------
// `CommandGraph::queue_for`
// --------------------------------------------------------------------------

#[test]
fn queue_for_unknown_id_errors() {
    let graph = graph(&[("a", &[])]);
    match graph.queue_for(&[id("nope")]) {
        Err(UnknownCommand(got)) => assert_eq!(got, id("nope")),
        other => panic!("expected UnknownCommand, got {other:?}"),
    }
}

#[test]
fn queue_for_empty_ids_is_empty() {
    let graph = graph(&[("a", &[]), ("b", &[])]);
    let queue = graph.queue_for(&[]).unwrap();
    assert_eq!(queue.len(), 0);
}

#[test]
fn queue_for_single_command_without_deps() {
    let graph = graph(&[("a", &[]), ("b", &[])]);
    let queue = graph.queue_for(&[id("a")]).unwrap();
    assert_eq!(order(queue), ["a"]);
}

#[test]
fn queue_for_includes_transitive_deps_in_order() {
    let graph = graph(&[("c", &["b"]), ("b", &["a"]), ("a", &[]), ("unused", &[])]);
    let ids = order(graph.queue_for(&[id("c")]).unwrap());
    assert_eq!(ids, ["a", "b", "c"]);
    assert!(!ids.contains(&"unused".to_string()));
}

#[test]
fn queue_for_diamond_dedupes_shared_ancestor() {
    let graph = graph(&[("d", &["b", "c"]), ("b", &["a"]), ("c", &["a"]), ("a", &[])]);
    let ids = order(graph.queue_for(&[id("d")]).unwrap());
    assert_eq!(
        ids.len(),
        4,
        "shared ancestor `a` must appear once: {ids:?}"
    );
    assert!(pos(&ids, "a") < pos(&ids, "b"));
    assert!(pos(&ids, "a") < pos(&ids, "c"));
    assert!(pos(&ids, "b") < pos(&ids, "d"));
    assert!(pos(&ids, "c") < pos(&ids, "d"));
}

#[test]
fn queue_for_duplicate_input_ids_dedupes() {
    let graph = graph(&[("a", &[]), ("b", &["a"])]);
    let ids = order(graph.queue_for(&[id("b"), id("b")]).unwrap());
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn queue_for_multiple_targets_dedupes() {
    // `b` and `c` are independent targets that both depend on `a`.
    let graph = graph(&[("a", &[]), ("b", &["a"]), ("c", &["a"])]);
    let mut ids = order(graph.queue_for(&[id("b"), id("c")]).unwrap());
    ids.sort();
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn queue_for_target_that_is_ancestor_of_another_target() {
    // `a` is itself a prerequisite of `c`; requesting both must not duplicate `a`.
    let graph = graph(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
    let ids = order(graph.queue_for(&[id("c"), id("a")]).unwrap());
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn queue_for_unknown_id_after_valid_ids_still_errors() {
    let graph = graph(&[("a", &[])]);
    assert!(graph.queue_for(&[id("a"), id("nope")]).is_err());
}

// --------------------------------------------------------------------------
// `IntoIterator` / `CommandQueue`
// --------------------------------------------------------------------------

#[test]
fn into_iter_yields_every_command() {
    let graph = graph(&[("a", &[]), ("b", &["a"]), ("c", &[])]);
    let mut ids = order(graph.into_iter());
    ids.sort();
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn into_iter_is_topologically_ordered() {
    let graph = graph(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
    assert_eq!(order(graph.into_iter()), ["a", "b", "c"]);
}

#[test]
fn queue_reports_exact_len() {
    let graph = graph(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
    let mut queue = graph.queue_for(&[id("c")]).unwrap();
    assert_eq!(queue.len(), 3);
    assert_eq!(queue.size_hint(), (3, Some(3)));
    queue.next();
    assert_eq!(queue.len(), 2);
}

// --------------------------------------------------------------------------
// Error `Display`
// --------------------------------------------------------------------------

#[rstest]
#[case(BuildError::DuplicateId(id("build")), "duplicate command id: build")]
#[case(BuildError::UnknownDep(id("gen")), "unknown dependency: gen")]
#[case(
    BuildError::Cycle { command: id("b"), dep: id("a") },
    "dependency cycle: a -> b"
)]
fn build_error_display(#[case] err: BuildError, #[case] expected: &str) {
    assert_eq!(err.to_string(), expected);
}

#[test]
fn unknown_command_display() {
    assert_eq!(UnknownCommand(id("x")).to_string(), "unknown command: x");
}

// --------------------------------------------------------------------------
// Property-based tests
// --------------------------------------------------------------------------

/// Assert that `ids` lists every command in `names` exactly once, with each
/// dependency in `deps` appearing before the command that requires it.
fn assert_topo(ids: &[String], deps: &[Vec<usize>], names: &[String]) {
    assert_eq!(ids.len(), names.len(), "every command once: {ids:?}");
    for (i, deps_i) in deps.iter().enumerate() {
        for &j in deps_i {
            assert!(
                pos(ids, &names[j]) < pos(ids, &names[i]),
                "{} must precede {} in {ids:?}",
                names[j],
                names[i],
            );
        }
    }
}

proptest! {
    /// A random DAG — node `i` may depend only on lower-indexed nodes, so it is
    /// acyclic by construction — builds regardless of insertion order, and both
    /// `queue_for` (over every id) and `into_iter` yield a valid topological
    /// order: every dependency precedes its dependent, with no duplicates.
    #[test]
    fn random_dag_orders_dependencies_before_dependents(
        n in 1usize..N_MAX,
        // Candidate edges as `(a, b)` pairs, plus a sort key per node used to shuffle
        // insertion order.
        pairs in prop::collection::vec((0usize..N_MAX, 0usize..N_MAX), 0..N_MAX * N_MAX),
        keys in prop::collection::vec(any::<u64>(), N_MAX),
    ) {
        let names: Vec<String> = (0..n).map(|i| format!("c{i}")).collect();

        // Edge `(a, b)` means `b` depends on `a`. Keeping only `a < b < n` makes every
        // edge point low -> high, so the graph is acyclic by construction.
        let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (a, b) in pairs {
            if a < b && b < n {
                deps[b].push(a);
            }
        }

        // Insert in a shuffled order to exercise forward-referenced deps: sort the
        // node indices by their random keys.
        let mut insertion: Vec<usize> = (0..n).collect();
        insertion.sort_by_key(|&i| keys[i]);

        let mut builder = CommandGraphBuilder::new();
        for &i in &insertion {
            let dep_ids = deps[i].iter().map(|&j| id(&names[j])).collect();
            builder.add(block(&names[i]), dep_ids);
        }
        let graph = builder.build().expect("acyclic graph should build");

        let all_ids: Vec<CommandId> = names.iter().map(|s| id(s)).collect();
        assert_topo(&order(graph.queue_for(&all_ids).unwrap()), &deps, &names);
        assert_topo(&order(graph.into_iter()), &deps, &names);
    }
}
