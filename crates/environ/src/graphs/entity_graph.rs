use super::*;
use crate::{EntityRef, SecondaryMap, prelude::*};
use core::{
    fmt::{self, Debug},
    iter,
    ops::Range,
};

/// A graph of `EntityRef` nodes reified into a densely packed representation.
pub struct EntityGraph<Node>
where
    Node: EntityRef,
{
    /// A map from each node to the subslice of `self.edge_elems` that are its
    /// edges.
    edges: SecondaryMap<Node, Range<u32>>,

    /// Densely packed edge elements for `self.edges`.
    edge_elems: Vec<Node>,
}

impl<Node> Debug for EntityGraph<Node>
where
    Node: EntityRef + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Edges<'a, Node: EntityRef + Debug>(&'a EntityGraph<Node>);

        impl<'a, Node: EntityRef + Debug> Debug for Edges<'a, Node> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(
                        self.0
                            .nodes()
                            .map(|n| (n, self.0.successors(n).collect::<Box<[_]>>())),
                    )
                    .finish()
            }
        }

        f.debug_struct("Graph")
            .field("edges", &Edges(self))
            .finish()
    }
}

impl<Node> EntityGraph<Node>
where
    Node: EntityRef + Debug,
{
    /// Construct a new, concrete `EntityGraph`.
    pub fn new<E>(
        nodes: impl IntoIterator<Item = Node>,
        mut successors: impl FnMut(Node, &mut Vec<Node>) -> Result<(), E>,
    ) -> Result<Self, E> {
        let nodes = nodes.into_iter();

        let (min, max) = nodes.size_hint();
        let capacity = max.unwrap_or_else(|| 2 * min);

        let mut edges = SecondaryMap::with_capacity(capacity);
        let mut edge_elems = vec![];

        let mut succs = vec![];
        for v in nodes {
            debug_assert!(succs.is_empty());
            successors(v, &mut succs)?;

            debug_assert_eq!(edges[v], Range::default());
            edges[v] = extend_with_range(&mut edge_elems, succs.drain(..));
        }

        Ok(EntityGraph { edges, edge_elems })
    }
}

impl<Node> Graph<Node> for EntityGraph<Node>
where
    Node: EntityRef,
{
    type NodesIter<'a>
        = cranelift_entity::Keys<Node>
    where
        Self: 'a;

    #[inline]
    fn nodes(&self) -> Self::NodesIter<'_> {
        self.edges.keys()
    }

    type SuccessorsIter<'a>
        = iter::Copied<core::slice::Iter<'a, Node>>
    where
        Self: 'a;

    fn successors(&self, node: Node) -> Self::SuccessorsIter<'_> {
        let Range { start, end } = self.edges[node].clone();
        let start = usize::try_from(start).unwrap();
        let end = usize::try_from(end).unwrap();
        self.edge_elems[start..end].iter().copied()
    }
}
