//! Stratification of call graphs for parallel bottom-up inlining.
//!
//! This module takes a call graph and constructs a strata, which is essentially
//! a parallel execution plan. A strata consists of an ordered sequence of
//! layers, and a layer of an unordered set of functions. The `i`th layer must
//! be processed before the `i + 1`th layer, but functions within the same layer
//! may be processed in any order (and in parallel).
//!
//! For example, when given the following tree-like call graph:
//!
//! ```text
//! +---+   +---+   +---+
//! | a |-->| b |-->| c |
//! +---+   +---+   +---+
//!   |       |
//!   |       |     +---+
//!   |       '---->| d |
//!   |             +---+
//!   |
//!   |     +---+   +---+
//!   '---->| e |-->| f |
//!         +---+   +---+
//!           |
//!           |     +---+
//!           '---->| g |
//!                 +---+
//! ```
//!
//! then stratification will produce these layers:
//!
//! ```text
//! [
//!     {c, d, f, g},
//!     {b, e},
//!     {a},
//! ]
//! ```
//!
//! Our goal in constructing the layers is to maximize potential parallelism at
//! each layer. Logically, we do this by finding the strongly-connected
//! components of the input call graph and peeling off all of the leaves of
//! SCCs' condensation (i.e. the DAG that the SCCs form; see the documentation
//! for the `StronglyConnectedComponents::evaporation` method for
//! details). These leaves become the strata's first layer. The layer's
//! components are removed from the condensation graph, and we repeat the
//! process, so that the condensation's new leaves become the strata's second
//! layer, and etc... until the condensation graph is empty and all components
//! have been processed. In practice we don't actually mutate the condensation
//! graph or remove its nodes but instead count how many unprocessed
//! dependencies each component has, and a component is ready for inclusion in a
//! layer once its unprocessed-dependencies count reaches zero.

use super::*;
use std::{fmt::Debug, ops::Range};
use wasmtime_environ::{
    EntityRef, SecondaryMap,
    graphs::{Graph, Scc, StronglyConnectedComponents},
};

/// A stratified call graph; essentially a parallel-execution plan for bottom-up
/// inlining.
///
/// See the module doc comment for more details.
pub struct Strata<Node> {
    layers: Vec<Range<u32>>,
    layer_elems: Vec<Node>,
}

impl<Node: Debug> Debug for Strata<Node> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct Layers<'a, Node>(&'a Strata<Node>);

        impl<'a, Node: Debug> Debug for Layers<'a, Node> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut f = f.debug_list();
                for layer in self.0.layers() {
                    f.entry(&layer);
                }
                f.finish()
            }
        }

        f.debug_struct("Strata")
            .field("layers", &Layers(self))
            .finish()
    }
}

impl<Node> Strata<Node> {
    /// Stratify the given call graph, yielding a `Strata` parallel-execution
    /// plan.
    pub fn new<G>(call_graph: &G) -> Self
    where
        Node: EntityRef + Debug,
        G: Debug + Graph<Node>,
    {
        log::trace!("Stratifying {call_graph:#?}");

        let components = StronglyConnectedComponents::new(call_graph);
        let evaporation = components.evaporation(call_graph);

        // A map from each component to the count of how many call-graph
        // dependencies to other components it has that have not been fulfilled
        // yet. These counts are decremented as we assign a component's dependencies
        // to layers.
        let mut unfulfilled_deps_count = SecondaryMap::<Scc, u32>::with_capacity(components.len());
        for to_component in components.keys() {
            for from_component in evaporation.reverse_edges(to_component) {
                unfulfilled_deps_count[*from_component] += 1;
            }
        }

        // Build the strata.
        //
        // The first layer is formed by searching through all components for those
        // that have a zero unfulfilled-deps count. When we finish a layer, we
        // iterate over each of component in that layer and decrement the
        // unfulfilled-deps count of every other component that depends on the
        // newly-assigned-to-a-layer component. Any component that then reaches a
        // zero unfulfilled-dep count is added to the next layer. This proceeds to a
        // fixed point, similarly to GC tracing and ref-count decrementing.

        let mut layers: Vec<Range<u32>> = vec![];
        let (min, max) = call_graph.nodes().size_hint();
        let cap = max.unwrap_or(min);
        let mut layer_elems: Vec<Node> = Vec::with_capacity(cap);

        let mut current_layer: Vec<Scc> = components
            .keys()
            .filter(|scc| unfulfilled_deps_count[*scc] == 0)
            .collect();
        debug_assert!(
            !current_layer.is_empty() || call_graph.nodes().next().is_none(),
            "the first layer can only be empty when the call graph itself is empty"
        );

        let mut next_layer = vec![];

        while !current_layer.is_empty() {
            debug_assert!(next_layer.is_empty());

            for dependee in &current_layer {
                for depender in evaporation.reverse_edges(*dependee) {
                    debug_assert!(unfulfilled_deps_count[*depender] > 0);
                    unfulfilled_deps_count[*depender] -= 1;
                    if unfulfilled_deps_count[*depender] == 0 {
                        next_layer.push(*depender);
                    }
                }
            }

            layers.push(extend_with_range(
                &mut layer_elems,
                current_layer
                    .drain(..)
                    .flat_map(|scc| components.nodes(scc).iter().copied()),
            ));

            std::mem::swap(&mut next_layer, &mut current_layer);
        }

        debug_assert!(
            unfulfilled_deps_count.values().all(|c| *c == 0),
            "after every component is assigned to a layer, all dependencies should be fulfilled"
        );

        let result = Strata {
            layers,
            layer_elems,
        };
        log::trace!("  -> {result:#?}");
        result
    }

    /// Iterate over the layers of this `Strata`.
    ///
    /// The `i`th layer must be processed before the `i + 1`th layer, but the
    /// functions within a layer may be processed in any order and in parallel.
    pub fn layers(&self) -> impl ExactSizeIterator<Item = &[Node]> {
        self.layers.iter().map(|range| {
            let start = usize::try_from(range.start).unwrap();
            let end = usize::try_from(range.end).unwrap();
            &self.layer_elems[start..end]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime_environ::graphs::Graph;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Function(u32);
    wasmtime_environ::entity_impl!(Function);

    #[derive(Debug)]
    struct Functions {
        calls: SecondaryMap<Function, Vec<Function>>,
    }

    impl Default for Functions {
        fn default() -> Self {
            let _ = env_logger::try_init();
            Self {
                calls: Default::default(),
            }
        }
    }

    impl Graph<Function> for Functions {
        type NodesIter<'a>
            = wasmtime_environ::Keys<Function>
        where
            Self: 'a;

        fn nodes(&self) -> Self::NodesIter<'_> {
            self.calls.keys()
        }

        type SuccessorsIter<'a>
            = core::iter::Copied<core::slice::Iter<'a, Function>>
        where
            Self: 'a;

        fn successors(&self, f: Function) -> Self::SuccessorsIter<'_> {
            self.calls[f].iter().copied()
        }
    }

    impl Functions {
        fn define_func(&mut self, f: u32) -> &mut Self {
            let f = Function::from_u32(f);
            if self.calls.get(f).is_none() {
                self.calls[f] = vec![];
            }
            self
        }

        fn define_call(&mut self, caller: u32, callee: u32) -> &mut Self {
            self.define_func(caller);
            self.define_func(callee);
            let caller = Function::from_u32(caller);
            let callee = Function::from_u32(callee);
            self.calls[caller].push(callee);
            self
        }

        fn define_calls(
            &mut self,
            caller: u32,
            callees: impl IntoIterator<Item = u32>,
        ) -> &mut Self {
            for callee in callees {
                self.define_call(caller, callee);
            }
            self
        }

        fn stratify(&self) -> Strata<Function> {
            Strata::new(self)
        }

        fn assert_stratification(&self, mut expected: Vec<Vec<u32>>) {
            for layer in &mut expected {
                layer.sort();
            }
            log::trace!("expected stratification = {expected:?}");

            let actual = self
                .stratify()
                .layers()
                .map(|layer| {
                    let mut layer = layer.iter().map(|f| f.as_u32()).collect::<Vec<_>>();
                    layer.sort();
                    layer
                })
                .collect::<Vec<_>>();
            log::trace!("actual stratification = {actual:?}");

            assert_eq!(expected.len(), actual.iter().len());
            for (expected, actual) in expected.into_iter().zip(actual) {
                log::trace!("expected layer = {expected:?}");
                log::trace!("  actual layer = {expected:?}");

                assert_eq!(expected.len(), actual.len());
                for (expected, actual) in expected.into_iter().zip(actual) {
                    assert_eq!(expected, actual);
                }
            }
        }
    }

    #[test]
    fn test_disconnected_functions() {
        // +---+   +---+   +---+
        // | 0 |   | 1 |   | 2 |
        // +---+   +---+   +---+
        Functions::default()
            .define_func(0)
            .define_func(1)
            .define_func(2)
            .assert_stratification(vec![vec![0, 1, 2]]);
    }

    #[test]
    fn test_chained_functions() {
        // +---+   +---+   +---+
        // | 0 |-->| 1 |-->| 2 |
        // +---+   +---+   +---+
        Functions::default()
            .define_call(0, 1)
            .define_call(1, 2)
            .assert_stratification(vec![vec![2], vec![1], vec![0]]);
    }

    #[test]
    fn test_cycle() {
        //   ,---------------.
        //   V               |
        // +---+   +---+   +---+
        // | 0 |-->| 1 |-->| 2 |
        // +---+   +---+   +---+
        Functions::default()
            .define_call(0, 1)
            .define_call(1, 2)
            .define_call(2, 0)
            .assert_stratification(vec![vec![0, 1, 2]]);
    }

    #[test]
    fn test_tree() {
        // +---+   +---+   +---+
        // | 0 |-->| 1 |-->| 2 |
        // +---+   +---+   +---+
        //   |       |
        //   |       |     +---+
        //   |       '---->| 3 |
        //   |             +---+
        //   |
        //   |     +---+   +---+
        //   '---->| 4 |-->| 5 |
        //         +---+   +---+
        //           |
        //           |     +---+
        //           '---->| 6 |
        //                 +---+
        Functions::default()
            .define_calls(0, [1, 4])
            .define_calls(1, [2, 3])
            .define_calls(4, [5, 6])
            .assert_stratification(vec![vec![2, 3, 5, 6], vec![1, 4], vec![0]]);
    }

    #[test]
    fn test_chain_of_cycles() {
        //   ,-----.
        //   |     |
        //   V     |
        // +---+   |
        // | 0 |---'
        // +---+
        //   |
        //   V
        // +---+    +---+
        // | 1 |<-->| 2 |
        // +---+    +---+
        //  |
        //  | ,----------------.
        //  | |                |
        //  V |                V
        // +---+    +---+    +---+
        // | 3 |<---| 4 |<---| 5 |
        // +---+    +---+    +---+
        Functions::default()
            .define_calls(0, [0, 1])
            .define_calls(1, [2, 3])
            .define_calls(2, [1])
            .define_calls(3, [5])
            .define_calls(4, [3])
            .define_calls(5, [4])
            .assert_stratification(vec![vec![3, 4, 5], vec![1, 2], vec![0]]);
    }

    #[test]
    fn test_multiple_edges_to_same_component() {
        // +---+           +---+
        // | 0 |           | 1 |
        // +---+           +---+
        //   ^               ^
        //   |               |
        //   V               V
        // +---+           +---+
        // | 2 |           | 3 |
        // +---+           +---+
        //   |               |
        //   `------. ,------'
        //          | |
        //          V V
        //         +---+
        //         | 4 |
        //         +---+
        //           ^
        //           |
        //           V
        //         +---+
        //         | 5 |
        //         +---+
        Functions::default()
            .define_calls(0, [2])
            .define_calls(1, [3])
            .define_calls(2, [0, 4])
            .define_calls(3, [1, 4])
            .define_calls(4, [5])
            .define_calls(5, [4])
            .assert_stratification(vec![vec![4, 5], vec![0, 1, 2, 3]]);
    }
}
