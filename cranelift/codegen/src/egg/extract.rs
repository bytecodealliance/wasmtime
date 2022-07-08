//! Extraction phase: pick one enode per eclass, avoiding loops.

use super::node::Node;
use crate::fx::FxHashMap;
use cranelift_egraph::{EGraph, Id, Language, NodeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EclassState {
    Visiting,
    Visited { cost: u32, node: NodeId },
    Deleted,
}

#[derive(Clone, Debug)]
pub(crate) struct Extractor {
    eclass_state: FxHashMap<Id, EclassState>,
}

impl Extractor {
    pub(crate) fn new() -> Self {
        Self {
            eclass_state: FxHashMap::default(),
        }
    }

    /// Visit an eclass. Return `None` if deleted, or Some(cost) if
    /// present.
    pub(crate) fn visit_eclass(&mut self, egraph: &EGraph<Node>, id: Id) -> Option<u32> {
        if let Some(state) = self.eclass_state.get(&id) {
            match state {
                EclassState::Visiting => {
                    // Found a cycle!
                    return None;
                }
                EclassState::Visited { cost, .. } => {
                    return Some(*cost);
                }
                EclassState::Deleted => {
                    return None;
                }
            }
        }
        self.eclass_state.insert(id, EclassState::Visiting);

        let mut best_cost_and_node = None;
        for (node_id, node) in egraph.enodes(id) {
            let this_cost = self.visit_enode(egraph, node);
            best_cost_and_node = match (best_cost_and_node, this_cost) {
                (None, None) => None,
                (None, Some(c)) => Some((c, node_id)),
                (Some((c1, _)), Some(c2)) if c2 < c1 => Some((c2, node_id)),
                (Some((c1, node_id1)), _) => Some((c1, node_id1)),
            };
        }

        match best_cost_and_node {
            Some((cost, node_id)) => {
                self.eclass_state.insert(
                    id,
                    EclassState::Visited {
                        cost,
                        node: node_id,
                    },
                );
                Some(cost)
            }
            None => {
                self.eclass_state.insert(id, EclassState::Deleted);
                None
            }
        }
    }

    fn visit_enode(&mut self, egraph: &EGraph<Node<'_>>, node: &Node) -> Option<u32> {
        let mut cost = node.cost() as u32;
        for &arg in node.children() {
            let arg_cost = self.visit_eclass(egraph, arg)?;
            cost += arg_cost;
        }
        Some(cost)
    }

    pub(crate) fn get_node<'a>(&'a self, egraph: &'a EGraph<Node<'a>>, id: Id) -> Option<&'a Node> {
        match self.eclass_state.get(&id)? {
            &EclassState::Visiting => unreachable!(),
            &EclassState::Visited { node, .. } => Some(egraph.enode(node)),
            &EclassState::Deleted => None,
        }
    }
}
