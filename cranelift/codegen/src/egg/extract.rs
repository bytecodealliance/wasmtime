//! Extraction phase: pick one enode per eclass, avoiding loops.

use super::node::Node;
use crate::fx::FxHashMap;
use egg::{EGraph, Id, Language};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EclassState {
    Visiting,
    Visited { cost: u32, node_idx: u32 },
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
    pub(crate) fn visit_eclass(&mut self, egraph: &EGraph<Node, ()>, id: Id) -> Option<u32> {
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
        for (i, node) in egraph[id].nodes.iter().enumerate() {
            let this_cost = self.visit_enode(egraph, node);
            best_cost_and_node = match (best_cost_and_node, this_cost) {
                (None, None) => None,
                (None, Some(c)) => Some((c, i)),
                (Some((c1, _)), Some(c2)) if c2 < c1 => Some((c2, i)),
                (Some((c1, i1)), _) => Some((c1, i1)),
            };
        }

        match best_cost_and_node {
            Some((cost, node_idx)) => {
                self.eclass_state.insert(
                    id,
                    EclassState::Visited {
                        cost,
                        node_idx: node_idx as u32,
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

    fn visit_enode(&mut self, egraph: &EGraph<Node, ()>, node: &Node) -> Option<u32> {
        let mut cost = node.cost() as u32;
        for &arg in node.children() {
            let arg_cost = self.visit_eclass(egraph, arg)?;
            cost += arg_cost;
        }
        Some(cost)
    }

    pub(crate) fn get_node<'a>(&self, egraph: &'a EGraph<Node, ()>, id: Id) -> Option<&'a Node> {
        match self.eclass_state.get(&id)? {
            EclassState::Visiting => unreachable!(),
            EclassState::Visited { node_idx, .. } => Some(&egraph[id].nodes[*node_idx as usize]),
            EclassState::Deleted => None,
        }
    }
}
