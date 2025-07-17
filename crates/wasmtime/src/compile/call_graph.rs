//! Construction of the call-graph.

#![cfg_attr(not(test), expect(dead_code, reason = "used in upcoming PRs"))]

use super::*;
use core::{
    fmt::{self, Debug},
    ops::Range,
};
use wasmtime_environ::{EntityRef, SecondaryMap};

/// A call graph reified into a densely packed and quickly accessible
/// representation.
///
/// In a call graph, nodes are functions, and an edge `f --> g` means that the
/// function `f` calls the function `g`.
pub struct CallGraph<Node>
where
    Node: EntityRef + Debug,
{
    /// A map from each node to the subslice of `self.edge_elems` that are its
    /// edges.
    edges: SecondaryMap<Node, Range<u32>>,

    /// Densely packed edge elements for `self.edges`.
    edge_elems: Vec<Node>,
}

impl<Node> Debug for CallGraph<Node>
where
    Node: EntityRef + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Edges<'a, Node: EntityRef + Debug>(&'a CallGraph<Node>);

        impl<'a, Node: EntityRef + Debug> Debug for Edges<'a, Node> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(self.0.nodes().map(|n| (n, self.0.edges(n))))
                    .finish()
            }
        }

        f.debug_struct("CallGraph")
            .field("edges", &Edges(self))
            .finish()
    }
}

impl<Node> CallGraph<Node>
where
    Node: EntityRef + Debug,
{
    /// Construct a new call graph.
    ///
    /// `funcs` should be an iterator over all function nodes in this call
    /// graph's translation unit.
    ///
    /// The `get_calls` function should yield (by pushing onto the given `Vec`)
    /// all of the callee function nodes that the given caller function node
    /// calls.
    pub fn new(
        funcs: impl IntoIterator<Item = Node>,
        get_calls: impl Fn(Node, &mut Vec<Node>) -> Result<()>,
    ) -> Result<Self> {
        let funcs = funcs.into_iter();

        let (min, max) = funcs.size_hint();
        let capacity = max.unwrap_or_else(|| 2 * min);
        let mut edges = SecondaryMap::with_capacity(capacity);
        let mut edge_elems = vec![];

        let mut calls = vec![];
        for caller in funcs {
            calls.clear();
            get_calls(caller, &mut calls)?;

            let start = edge_elems.len();
            let start = u32::try_from(start).unwrap();
            edge_elems.extend_from_slice(&calls);
            let end = edge_elems.len();
            let end = u32::try_from(end).unwrap();

            debug_assert_eq!(edges[caller], Range::default());
            edges[caller] = start..end;
        }

        Ok(CallGraph { edges, edge_elems })
    }

    /// Get the function nodes in this call graph.
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = Node> {
        self.edges.keys()
    }

    /// Get the callee function nodes that the given caller function node calls.
    pub fn edges(&self, node: Node) -> &[Node] {
        let Range { start, end } = self.edges[node].clone();
        let start = usize::try_from(start).unwrap();
        let end = usize::try_from(end).unwrap();
        &self.edge_elems[start..end]
    }
}
