//! Strongly-connected components (Tarjan, iterative).
//!
//! Same engineering as Wasmtime's inliner SCC:
//! - O(V+E)
//! - iterative (no recursion)
//! - components stored as `Vec<Range<u32>>` into a flat node buffer
//! - deterministic via ordered containers (BTreeMap/BTreeSet)
//! This is a modified version of Wasmtime's inliner SCC.
//! Please see: https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasmtime/src/compile/scc.rs

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use crate::generators::gc_ops::types::RecGroupId;

/// SCC results: `components` maps each SCC to a slice range in `component_nodes`.
pub struct StronglyConnectedComponents {
    components: Vec<Range<u32>>,
    component_nodes: Vec<RecGroupId>,
}

impl StronglyConnectedComponents {
    /// Find SCCs in the given graph.
    pub fn new<I, F, S>(nodes: I, successors: F) -> Self
    where
        I: IntoIterator<Item = RecGroupId>,
        F: Fn(RecGroupId) -> S,
        S: Iterator<Item = RecGroupId>,
    {
        let nodes = nodes.into_iter();

        // The resulting components and their nodes.
        let mut component_nodes: Vec<RecGroupId> = vec![];
        let mut components: Vec<Range<u32>> = vec![];

        // The DFS index counter.
        let mut index = NonMaxU32::default();

        // DFS index and lowlink for each RecGroupId.
        // Because RecGroupId is not dense, we use BTreeMap.
        let mut indices: BTreeMap<RecGroupId, NonMaxU32> = BTreeMap::new();
        let mut lowlinks: BTreeMap<RecGroupId, NonMaxU32> = BTreeMap::new();

        // SCC stack and membership.
        let mut stack: Vec<RecGroupId> = vec![];
        let mut on_stack: BTreeSet<RecGroupId> = BTreeSet::new();

        let mut dfs = Dfs::new(nodes);
        while let Some(event) = dfs.next(
            &successors,
            // seen?
            |node| indices.contains_key(&node),
        ) {
            match event {
                DfsEvent::Pre(node) => {
                    debug_assert!(!indices.contains_key(&node));
                    debug_assert!(!lowlinks.contains_key(&node));

                    indices.insert(node, index);
                    lowlinks.insert(node, index);

                    index = NonMaxU32::new(index.get() + 1).unwrap();

                    stack.push(node);
                    let inserted = on_stack.insert(node);
                    debug_assert!(inserted);
                }

                DfsEvent::AfterEdge(node, succ) => {
                    let node_idx = indices[&node];
                    let node_low = lowlinks[&node];
                    let succ_idx = indices[&succ];
                    let succ_low = lowlinks[&succ];

                    debug_assert!(node_low <= node_idx);
                    debug_assert!(succ_low <= succ_idx);

                    if on_stack.contains(&succ) {
                        let new_low = std::cmp::min(node_low, succ_low);
                        lowlinks.insert(node, new_low);
                    }
                }

                DfsEvent::Post(node) => {
                    let node_idx = indices[&node];
                    let node_low = lowlinks[&node];

                    if node_idx == node_low {
                        // Node is SCC root. Pop until node.
                        let start = u32::try_from(component_nodes.len()).unwrap();

                        loop {
                            let v = stack.pop().unwrap();
                            let removed = on_stack.remove(&v);
                            debug_assert!(removed);

                            component_nodes.push(v);

                            if v == node {
                                break;
                            }
                        }

                        let end = u32::try_from(component_nodes.len()).unwrap();
                        components.push(start..end);
                    }
                }
            }
        }

        Self {
            components,
            component_nodes,
        }
    }

    fn node_range(&self, range: Range<u32>) -> &[RecGroupId] {
        let start = usize::try_from(range.start).unwrap();
        let end = usize::try_from(range.end).unwrap();
        &self.component_nodes[start..end]
    }

    /// Iterate SCCs.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &[RecGroupId]> + '_ {
        self.components.iter().map(|r| self.node_range(r.clone()))
    }
}

/// An iterative depth-first traversal.
struct Dfs {
    stack: Vec<DfsEvent>,
}

impl Dfs {
    fn new(roots: impl IntoIterator<Item = RecGroupId>) -> Self {
        Self {
            stack: roots.into_iter().map(DfsEvent::Pre).collect(),
        }
    }

    fn next<S>(
        &mut self,
        successors: impl Fn(RecGroupId) -> S,
        seen: impl Fn(RecGroupId) -> bool,
    ) -> Option<DfsEvent>
    where
        S: Iterator<Item = RecGroupId>,
    {
        loop {
            let event = self.stack.pop()?;

            if let DfsEvent::Pre(node) = event {
                if seen(node) {
                    continue;
                }

                let succs = successors(node);
                let (min, max) = succs.size_hint();
                let est = max.unwrap_or_else(|| 2 * min);

                self.stack.reserve(2 * est + 1);

                self.stack.push(DfsEvent::Post(node));
                for succ in succs {
                    self.stack.push(DfsEvent::AfterEdge(node, succ));
                    if !seen(succ) {
                        self.stack.push(DfsEvent::Pre(succ));
                    }
                }
            }

            return Some(event);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DfsEvent {
    Pre(RecGroupId),
    AfterEdge(RecGroupId, RecGroupId),
    Post(RecGroupId),
}

mod non_max {
    use std::num::NonZeroU32;

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonMaxU32(NonZeroU32);

    impl Default for NonMaxU32 {
        fn default() -> Self {
            Self::new(0).unwrap()
        }
    }

    impl NonMaxU32 {
        pub fn new(x: u32) -> Option<Self> {
            if x == u32::MAX {
                None
            } else {
                Some(Self(unsafe { NonZeroU32::new_unchecked(x + 1) }))
            }
        }

        pub fn get(&self) -> u32 {
            self.0.get() - 1
        }
    }
}
use non_max::NonMaxU32;
