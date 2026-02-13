//! Generic graph traits, traversals, and data structures for use in Wasmtime.

mod dfs;
mod entity_graph;
mod scc;
use core::{fmt, iter};

pub use dfs::*;
pub use entity_graph::*;
pub use scc::*;

use crate::prelude::*;

/// A trait for any kind of graph data structure.
pub trait Graph<Node> {
    /// The iterator type returned by `Nodes::nodes`.
    type NodesIter<'a>: Iterator<Item = Node>
    where
        Self: 'a;

    /// Iterate over the nodes in this graph.
    fn nodes(&self) -> Self::NodesIter<'_>;

    /// The iterator type returned by `Successors::successors`.
    type SuccessorsIter<'a>: Iterator<Item = Node>
    where
        Self: 'a;

    /// Iterate over the successors of the given `node`.
    fn successors(&self, node: Node) -> Self::SuccessorsIter<'_>;

    // Provided Methods.

    /// Like `Iterator::by_ref` but for `Graph`.
    fn by_ref(&self) -> &Self {
        self
    }

    /// Use the given predicate to filter out certain nodes from the graph.
    fn filter_nodes<F>(self, predicate: F) -> FilterNodes<Self, F>
    where
        Self: Sized,
        F: Fn(&Node) -> bool,
    {
        FilterNodes {
            graph: self,
            predicate,
        }
    }
}

impl<T, Node> Graph<Node> for &'_ T
where
    T: ?Sized + Graph<Node>,
{
    type NodesIter<'a>
        = T::NodesIter<'a>
    where
        Self: 'a;

    fn nodes(&self) -> Self::NodesIter<'_> {
        (*self).nodes()
    }

    type SuccessorsIter<'a>
        = T::SuccessorsIter<'a>
    where
        Self: 'a;

    fn successors(&self, node: Node) -> Self::SuccessorsIter<'_> {
        (*self).successors(node)
    }
}

/// A graph whose nodes are being filtered by the predicate `F`.
///
/// Created by the `Graph::filter_nodes` trait method.
pub struct FilterNodes<G, F> {
    graph: G,
    predicate: F,
}

impl<G, F> fmt::Debug for FilterNodes<G, F>
where
    G: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter")
            .field("graph", &self.graph)
            .field("predicate", &"..")
            .finish()
    }
}

impl<G, F, Node> Graph<Node> for FilterNodes<G, F>
where
    G: Graph<Node>,
    F: Fn(&Node) -> bool,
{
    type NodesIter<'a>
        = iter::Filter<G::NodesIter<'a>, &'a F>
    where
        Self: 'a;

    fn nodes(&self) -> Self::NodesIter<'_> {
        self.graph.nodes().filter(&self.predicate)
    }

    type SuccessorsIter<'a>
        = iter::Filter<G::SuccessorsIter<'a>, &'a F>
    where
        Self: 'a;

    fn successors(&self, node: Node) -> Self::SuccessorsIter<'_> {
        self.graph.successors(node).filter(&self.predicate)
    }
}

/// Extend `dest` with `items` and return the range of indices in `dest` where
/// they ended up.
fn extend_with_range<T>(
    dest: &mut Vec<T>,
    items: impl IntoIterator<Item = T>,
) -> core::ops::Range<u32> {
    let start = dest.len();
    let start = u32::try_from(start).unwrap();

    dest.extend(items);

    let end = dest.len();
    let end = u32::try_from(end).unwrap();

    start..end
}
