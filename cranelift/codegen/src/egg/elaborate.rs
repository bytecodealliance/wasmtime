//! Elaboration phase: lowers EGraph back to sequences of operations
//! in CFG nodes.

/* TODO: extraction:

   /// Because the aegraph builds sets in a persistent-data-structure
   /// way, where a larger set is composed of a union of two smaller
   /// sets or else a single new node, we can do this in a single
   /// pass over the eclasses.
   ///
   /// The acyclic property of ENodes -- that is, the ENode in a
   /// given EClass can refer only to EClasses with a lower ID --
   /// means that this single pass can compute the cost of a new
   /// ENode just by looking up the lower-id EClasses and adding the
   /// cost of the op.


 - actually, we want to do this *during elaboration*, because the
   best extraction at a given point depends on what we already
   have. Walk the enode tree for an eclass; if we already have the
   value of one, that's it. Otherwise, find the min. Then memoize the
   result by elaborating. Don't memoize the selection because it
   depends on what's available in the scoped hashmap, which can
   "retract" as we pop scopes.
*/

use super::domtree::DomTreeWithChildren;
use super::node::{op_cost, Node, NodeCtx};
use super::Stats;
use crate::dominator_tree::DominatorTree;
use crate::ir::{Block, Function, Inst, SourceLoc, Type, Value, ValueList};
use crate::loop_analysis::LoopAnalysis;
use crate::scoped_hash_map::ScopedHashMap;
use cranelift_egraph::{EGraph, Id, Language};
use cranelift_entity::{packed_option::PackedOption, SecondaryMap};
use smallvec::{smallvec, SmallVec};

type LoopDepth = u32;

pub(crate) struct Elaborator<'a> {
    func: &'a mut Function,
    domtree: &'a DominatorTree,
    loop_analysis: &'a LoopAnalysis,
    node_ctx: &'a NodeCtx,
    egraph: &'a EGraph<NodeCtx>,
    id_to_value: ScopedHashMap<Id, IdValue>,
    id_to_best_cost_and_node: ScopedHashMap<Id, (usize, Id)>,
    /// Stack of blocks and loops in current elaboration path.
    loop_stack: SmallVec<[LoopStackEntry; 8]>,
    cur_block: Option<Block>,
    first_branch: SecondaryMap<Block, PackedOption<Inst>>,
    stats: &'a mut Stats,
}

#[derive(Clone, Debug)]
struct LoopStackEntry {
    /// The hoist point: a block that immediately dominates this
    /// loop. May not be an immediate predecessor, but will be a valid
    /// point to place all loop-invariant ops: they must depend only
    /// on inputs that dominate the loop, so are available at (the end
    /// of) this block.
    hoist_block: Block,
    /// The depth in the scope map.
    scope_depth: u32,
}

#[derive(Clone, Debug)]
enum IdValue {
    /// A single value.
    Value(LoopDepth, Value),
    /// Multiple results; indices in `node_args`.
    Values(LoopDepth, ValueList),
}

impl<'a> Elaborator<'a> {
    pub(crate) fn new(
        func: &'a mut Function,
        domtree: &'a DominatorTree,
        loop_analysis: &'a LoopAnalysis,
        egraph: &'a EGraph<NodeCtx>,
        node_ctx: &'a NodeCtx,
        stats: &'a mut Stats,
    ) -> Self {
        let num_blocks = func.dfg.num_blocks();
        Self {
            func,
            domtree,
            loop_analysis,
            egraph,
            node_ctx,
            id_to_value: ScopedHashMap::with_capacity(egraph.classes.len()),
            id_to_best_cost_and_node: ScopedHashMap::with_capacity(egraph.classes.len()),
            loop_stack: smallvec![],
            cur_block: None,
            first_branch: SecondaryMap::with_capacity(num_blocks),
            stats,
        }
    }

    fn cur_loop_depth(&self) -> LoopDepth {
        self.loop_stack.len() as LoopDepth
    }

    fn start_block(&mut self, idom: Option<Block>, block: Block, block_params: &[(Id, Type)]) {
        log::trace!(
            "start_block: block {:?} with idom {:?} at loop depth {} scope depth {}",
            block,
            idom,
            self.cur_loop_depth(),
            self.id_to_value.depth()
        );

        if let Some(idom) = idom {
            if self.loop_analysis.is_loop_header(block).is_some() {
                self.loop_stack.push(LoopStackEntry {
                    // Any code hoisted out of this loop will have code
                    // placed in `idom`, and will have def mappings
                    // inserted in to the scoped hashmap at that block's
                    // level.
                    hoist_block: idom,
                    scope_depth: (self.id_to_value.depth() - 1) as u32,
                });
                log::trace!(
                    " -> loop header, pushing; depth now {}",
                    self.loop_stack.len()
                );
            }
        }

        self.cur_block = Some(block);
        for &(id, ty) in block_params {
            let val = self.func.dfg.append_block_param(block, ty);
            log::trace!(" -> block param id {:?} value {:?}", id, val);
            self.id_to_value
                .insert_if_absent(id, IdValue::Value(self.cur_loop_depth(), val));
        }
    }

    fn add_node(&mut self, node: &Node, args: &[Value], to_block: Block) -> ValueList {
        let (instdata, result_tys, single_ty) = match node {
            Node::Pure { op, types, .. } | Node::Inst { op, types, .. } => (
                op.with_args(args, &mut self.func.dfg.value_lists),
                Some(types),
                None,
            ),
            Node::Load { op, ty, .. } => (
                op.with_args(args, &mut self.func.dfg.value_lists),
                None,
                Some(*ty),
            ),
            _ => panic!("Cannot `add_node()` on block param or projection"),
        };
        let srcloc = match node {
            Node::Inst { srcloc, .. } | Node::Load { srcloc, .. } => *srcloc,
            _ => SourceLoc::default(),
        };
        let is_term = instdata.opcode().is_branch() || instdata.opcode().is_return();
        let inst = self.func.dfg.make_inst(instdata);
        self.func.srclocs[inst] = srcloc;

        if let Some(result_tys) = result_tys {
            for &ty in result_tys.as_slice(&self.node_ctx.types) {
                self.func.dfg.append_result(inst, ty);
            }
        } else if let Some(ty) = single_ty {
            self.func.dfg.append_result(inst, ty);
        }

        if is_term {
            self.func.layout.append_inst(inst, to_block);
            if self.first_branch[to_block].is_none() {
                self.first_branch[to_block] = Some(inst).into();
            }
        } else if let Some(branch) = self.first_branch[to_block].into() {
            self.func.layout.insert_inst(inst, branch);
        } else {
            self.func.layout.append_inst(inst, to_block);
        }
        self.func.dfg.inst_results_list(inst)
    }

    fn find_best_node(&mut self, id: Id) -> (usize, Id) {
        self.stats.elaborate_find_best_node += 1;
        log::trace!("find_best_node: {}", id);

        if self.id_to_value.get(&id).is_some() {
            self.stats.elaborate_find_best_node_existing_value += 1;
            log::trace!(" -> value already available; cost 0");
            return (0, id);
        }

        if let Some(&(cost, node)) = self.id_to_best_cost_and_node.get(&id) {
            self.stats.elaborate_find_best_node_memoize_hit += 1;
            log::trace!(" -> memoized to cost {} node {}", cost, node);
            return (cost, node);
        }
        self.stats.elaborate_find_best_node_memoize_miss += 1;

        let eclass = self.egraph.classes[id];
        let node = eclass.get_node();
        let parent1 = eclass
            .as_node_and_parent()
            .map(|(_, parent)| parent)
            .or(eclass.as_union().map(|(p1, _)| p1));
        let parent2 = eclass.as_union().map(|(_, p2)| p2);

        log::trace!(
            " -> id {} node expands to: node {:?} parent1 {:?} parent2 {:?}",
            id,
            node,
            parent1,
            parent2
        );

        let (mut best_cost, mut best_id) = if let Some(node) = node {
            let cost = match node.node::<NodeCtx>(&self.egraph.nodes) {
                Node::Param { .. } | Node::Inst { .. } | Node::Load { .. } => {
                    return (0, id);
                }
                Node::Result { value, .. } => {
                    return self.find_best_node(*value);
                }
                Node::Pure { op, .. } => op_cost(op),
            };
            log::trace!("  -> id {} has operand cost {}", id, cost);

            let mut children_cost = 0;
            let child_count = self
                .node_ctx
                .children(node.node::<NodeCtx>(&self.egraph.nodes))
                .len();
            for child_idx in 0..child_count {
                let child = self
                    .node_ctx
                    .children(node.node::<NodeCtx>(&self.egraph.nodes))[child_idx];
                assert!(child < id);
                log::trace!("  -> id {} child {}", id, child);
                self.stats.elaborate_find_best_node_arg_recurse += 1;
                let (child_cost, _) = self.find_best_node(child);
                children_cost += child_cost;
                log::trace!("  -> id {} child {} child cost {}", id, child, child_cost);
            }
            let node_cost = cost + children_cost;

            log::trace!(
                "  -> id {} total cost of operand plus args: {}",
                id,
                node_cost
            );
            (Some(node_cost), Some(id))
        } else {
            (None, None)
        };

        // Evaluate parents as options now, but only if we haven't
        // already found a "perfect" (zero-cost) option here. This
        // conditional lets us short-circuit cases where e.g. a
        // rewrite to a constant value occurs.
        if best_cost != Some(0) {
            for parent in parent1.into_iter().chain(parent2.into_iter()) {
                log::trace!(" -> id {} parent {}", id, parent);
                assert!(parent < id);
                self.stats.elaborate_find_best_node_parent_recurse += 1;
                let (parent_best_cost, parent_best_id) = self.find_best_node(parent);
                log::trace!(
                    " -> id {} parent {} has cost {} with best id {}",
                    id,
                    parent,
                    parent_best_cost,
                    parent_best_id
                );
                if best_cost.is_none() || parent_best_cost < best_cost.unwrap() {
                    self.stats.elaborate_find_best_node_parent_better += 1;
                    best_cost = Some(parent_best_cost);
                    best_id = Some(parent_best_id);
                }
            }
        }

        let best_id = best_id.expect("Must have at least one node");
        let best_cost = best_cost.expect("Must have at least one node");

        log::trace!(
            "-> for eclass {}, best node is in id {} with cost {}",
            id,
            best_id,
            best_cost
        );

        self.id_to_best_cost_and_node
            .insert_if_absent(id, (best_cost, best_id));

        (best_cost, best_id)
    }

    fn elaborate_eclass_use(&mut self, id: Id) -> IdValue {
        self.stats.elaborate_visit_node += 1;
        let (_, best_node_eclass) = self.find_best_node(id);

        if let Some(val) = self.id_to_value.get(&best_node_eclass) {
            self.stats.elaborate_memoize_hit += 1;
            return val.clone();
        }
        self.stats.elaborate_memoize_miss += 1;

        let node_key = self.egraph.classes[best_node_eclass].get_node().unwrap();
        let node = node_key.node::<NodeCtx>(&self.egraph.nodes);

        // Is the node a block param? We should never get here if so
        // (they are inserted when first visiting the block).
        if matches!(node, Node::Param { .. }) {
            unreachable!("Param nodes should already be inserted");
        }

        // Is the node a result projection? If so, at this point we
        // have everything we need; no need to allocate a new Value
        // for the result.
        if let Node::Result { value, result, .. } = node {
            let value = self.elaborate_eclass_use(*value);
            let (depth, range) = match value {
                IdValue::Values(depth, range) => (depth, range),
                IdValue::Value(..) => {
                    unreachable!("Projection nodes should not be used on single results");
                }
            };
            let values = range.as_slice(&self.func.dfg.value_lists);
            let value = IdValue::Value(depth, values[*result]);
            self.id_to_value.insert_if_absent(id, value.clone());
            return value;
        }

        // We're going to need to emit this operator. First, elaborate
        // all args, recursively. Also track maximum loop depth while we're here.
        let mut max_loop_depth = 0;
        let args: SmallVec<[Value; 8]> = self
            .node_ctx
            .children(&node)
            .iter()
            .map(|&id| {
                self.stats.elaborate_visit_node_recurse += 1;
                self.elaborate_eclass_use(id)
            })
            .map(|idvalue| match idvalue {
                IdValue::Value(depth, value) => {
                    max_loop_depth = std::cmp::max(max_loop_depth, depth);
                    value
                }
                IdValue::Values(..) => panic!("enode depends directly on multi-value result"),
            })
            .collect();

        // Determine the location at which we emit it. This is the
        // current block *unless* we hoist above a loop when all args
        // are loop-invariant (and this op is pure).
        let (loop_depth, scope_depth, block) = if node.is_non_pure() {
            // Non-pure op: always at the current location.
            (
                self.cur_loop_depth(),
                self.id_to_value.depth(),
                self.cur_block.unwrap(),
            )
        } else if max_loop_depth == self.cur_loop_depth() {
            // Pure op, but depends on some value at the current loop depth: as above.
            (
                self.cur_loop_depth(),
                self.id_to_value.depth(),
                self.cur_block.unwrap(),
            )
        } else {
            // Pure op, and does not depend on any args at current loop depth: hoist out of loop.
            let data = &self.loop_stack[max_loop_depth as usize];
            (max_loop_depth, data.scope_depth as usize, data.hoist_block)
        };

        // This is an actual operation; emit the node in sequence now.
        let results = self.add_node(node, &args[..], block);
        let results_slice = results.as_slice(&self.func.dfg.value_lists);

        // Build the result and memoize in the id-to-value map.
        let result = if results_slice.len() == 1 {
            IdValue::Value(loop_depth, results_slice[0])
        } else {
            IdValue::Values(loop_depth, results)
        };

        self.id_to_value
            .insert_if_absent_with_depth(id, result.clone(), scope_depth);
        result
    }

    fn elaborate_block<'b, PF: Fn(Block) -> &'b [(Id, Type)], RF: Fn(Block) -> &'b [Id]>(
        &mut self,
        idom: Option<Block>,
        block: Block,
        block_params_fn: &PF,
        block_roots_fn: &RF,
        domtree: &DomTreeWithChildren,
    ) {
        self.id_to_value.increment_depth();
        self.id_to_best_cost_and_node.increment_depth();

        let blockparam_ids_tys = (block_params_fn)(block);
        self.start_block(idom, block, blockparam_ids_tys);
        for &id in (block_roots_fn)(block) {
            self.elaborate_eclass_use(id);
        }

        for child in domtree.children(block) {
            self.elaborate_block(Some(block), child, block_params_fn, block_roots_fn, domtree);
        }

        self.id_to_best_cost_and_node.decrement_depth();
        self.id_to_value.decrement_depth();
        if let Some(innermost_loop) = self.loop_stack.last() {
            if innermost_loop.scope_depth as usize == self.id_to_value.depth() {
                self.loop_stack.pop();
            }
        }
    }

    fn clear_func_body(&mut self) {
        // Clear all instructions and args/results from the DFG. We
        // rebuild them entirely during elaboration. (TODO: reuse the
        // existing inst for the *first* copy of a given node.)
        self.func.dfg.clear_insts();
        // Clear the instructions in every block, but leave the list
        // of blocks and their layout unmodified.
        self.func.layout.clear_insts();
        self.func.srclocs.clear();
    }

    pub(crate) fn elaborate<'b, PF: Fn(Block) -> &'b [(Id, Type)], RF: Fn(Block) -> &'b [Id]>(
        &mut self,
        block_params_fn: PF,
        block_roots_fn: RF,
    ) {
        let domtree = DomTreeWithChildren::new(self.func, self.domtree);
        let root = domtree.root();
        self.stats.elaborate_func += 1;
        self.stats.elaborate_func_pre_insts += self.func.dfg.num_insts() as u64;
        self.clear_func_body();
        self.elaborate_block(None, root, &block_params_fn, &block_roots_fn, &domtree);
        self.stats.elaborate_func_post_insts += self.func.dfg.num_insts() as u64;
    }
}
