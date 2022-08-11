//! Elaboration phase: lowers EGraph back to sequences of operations
//! in CFG nodes.

use super::domtree::DomTreeWithChildren;
use super::node::{op_cost, Node, NodeCtx};
use super::Stats;
use crate::dominator_tree::DominatorTree;
use crate::fx::FxHashSet;
use crate::ir::{Block, Function, Inst, SourceLoc, Type, Value, ValueList};
use crate::loop_analysis::{LoopAnalysis, LoopLevel};
use crate::scoped_hash_map::ScopedHashMap;
use cranelift_egraph::{EGraph, Id, Language};
use cranelift_entity::{packed_option::PackedOption, SecondaryMap};
use smallvec::{smallvec, SmallVec};

type LoopDepth = u32;

/// Borrows of "input data". Held separately so that we can keep
/// borrows open while calling `&mut self` methods on `Elaborator`
/// itself.
pub(crate) struct ElaboratorCtx<'a> {
    pub(crate) domtree: &'a DominatorTree,
    pub(crate) loop_analysis: &'a LoopAnalysis,
    pub(crate) node_ctx: &'a NodeCtx,
    pub(crate) egraph: &'a EGraph<NodeCtx>,
    pub(crate) loop_levels: &'a SecondaryMap<Id, LoopLevel>,
    pub(crate) remat_ids: &'a FxHashSet<Id>,
}

pub(crate) struct Elaborator<'a> {
    func: &'a mut Function,
    id_to_value: ScopedHashMap<Id, IdValue>,
    id_to_best_cost_and_node: SecondaryMap<Id, (usize, Id)>,
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
    Value {
        depth: LoopDepth,
        block: Block,
        value: Value,
    },
    /// Multiple results; indices in `node_args`.
    Values {
        depth: LoopDepth,
        block: Block,
        values: ValueList,
    },
}

impl IdValue {
    fn block(&self) -> Block {
        match self {
            IdValue::Value { block, .. } | IdValue::Values { block, .. } => *block,
        }
    }
}

impl<'a> Elaborator<'a> {
    pub(crate) fn new(
        func: &'a mut Function,
        ctx: &ElaboratorCtx<'_>,
        stats: &'a mut Stats,
    ) -> Self {
        let num_blocks = func.dfg.num_blocks();
        let mut id_to_best_cost_and_node = SecondaryMap::with_default((0, Id::invalid()));
        id_to_best_cost_and_node.resize(ctx.egraph.classes.len());
        Self {
            func,
            id_to_value: ScopedHashMap::with_capacity(ctx.egraph.classes.len()),
            id_to_best_cost_and_node,
            loop_stack: smallvec![],
            cur_block: None,
            first_branch: SecondaryMap::with_capacity(num_blocks),
            stats,
        }
    }

    fn cur_loop_depth(&self) -> LoopDepth {
        self.loop_stack.len() as LoopDepth
    }

    fn start_block(
        &mut self,
        ctx: &ElaboratorCtx<'_>,
        idom: Option<Block>,
        block: Block,
        block_params: &[(Id, Type)],
    ) {
        log::trace!(
            "start_block: block {:?} with idom {:?} at loop depth {} scope depth {}",
            block,
            idom,
            self.cur_loop_depth(),
            self.id_to_value.depth()
        );

        if let Some(idom) = idom {
            if ctx.loop_analysis.is_loop_header(block).is_some() {
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
            let value = self.func.dfg.append_block_param(block, ty);
            log::trace!(" -> block param id {:?} value {:?}", id, value);
            self.id_to_value.insert_if_absent(
                id,
                IdValue::Value {
                    depth: self.cur_loop_depth(),
                    block,
                    value,
                },
            );
        }
    }

    fn add_node(
        &mut self,
        ctx: &ElaboratorCtx<'_>,
        node: &Node,
        args: &[Value],
        to_block: Block,
    ) -> ValueList {
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
            for &ty in result_tys.as_slice(&ctx.node_ctx.types) {
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

    fn find_best_node(
        &mut self,
        ctx: &ElaboratorCtx<'_>,
        id: Id,
        mut upper_bound: Option<usize>,
    ) -> Option<(usize, Id)> {
        self.stats.elaborate_find_best_node += 1;
        log::trace!("find_best_node: {} upper_bound {:?}", id, upper_bound);

        let (cost, node) = self.id_to_best_cost_and_node[id];
        if node != Id::invalid() {
            self.stats.elaborate_find_best_node_memoize_hit += 1;
            log::trace!(" -> memoized to cost {} node {}", cost, node);
            if upper_bound.is_none() || cost <= upper_bound.unwrap() {
                return Some((cost, node));
            } else {
                return None;
            }
        }
        self.stats.elaborate_find_best_node_memoize_miss += 1;

        let eclass = ctx.egraph.classes[id];
        let node = eclass.get_node();
        let parent1 = eclass.parent1();
        let parent2 = eclass.parent2();

        log::trace!(
            " -> id {} node expands to: node {:?} parent1 {:?} parent2 {:?}",
            id,
            node,
            parent1,
            parent2
        );

        let (mut best_cost, mut best_id) = if let Some(node) = node {
            let cost = match node.node::<NodeCtx>(&ctx.egraph.nodes) {
                Node::Param { .. } | Node::Inst { .. } | Node::Load { .. } => {
                    return Some((0, id));
                }
                Node::Result { value, .. } => {
                    return self.find_best_node(ctx, *value, upper_bound);
                }
                Node::Pure { op, .. } => op_cost(op),
            };
            let level = ctx.loop_levels[id].level() as u32;
            let cost = cost * (1 << (10 * level));
            log::trace!("  -> id {} has operand cost {}", id, cost);

            let mut children_cost = 0;
            let mut exceeded = false;
            for &child in ctx
                .node_ctx
                .children(node.node::<NodeCtx>(&ctx.egraph.nodes))
            {
                if upper_bound.is_some() && cost + children_cost > upper_bound.unwrap() {
                    exceeded = true;
                    break;
                }
                assert!(child < id);
                log::trace!("  -> id {} child {}", id, child);
                self.stats.elaborate_find_best_node_arg_recurse += 1;
                let child_upper_bound = upper_bound.map(|u| u - (cost + children_cost));
                if let Some((child_cost, _)) = self.find_best_node(ctx, child, child_upper_bound) {
                    children_cost += child_cost;
                    log::trace!("  -> id {} child {} child cost {}", id, child, child_cost);
                } else {
                    exceeded = true;
                    break;
                }
            }

            if exceeded {
                (None, None)
            } else {
                let node_cost = cost + children_cost;

                log::trace!(
                    "  -> id {} total cost of operand plus args: {}",
                    id,
                    node_cost
                );
                (Some(node_cost), Some(id))
            }
        } else {
            (None, None)
        };

        if best_cost.is_some()
            && (upper_bound.is_none() || best_cost.unwrap() < upper_bound.unwrap())
        {
            upper_bound = best_cost;
        }

        // Evaluate parents as options now, but only if we haven't
        // already found a "perfect" (zero-cost) option here. This
        // conditional lets us short-circuit cases where e.g. a
        // rewrite to a constant value occurs.
        if best_cost != Some(0) {
            for parent in parent1.into_iter().chain(parent2.into_iter()) {
                log::trace!(" -> id {} parent {}", id, parent);
                assert!(parent < id);
                self.stats.elaborate_find_best_node_parent_recurse += 1;
                if let Some((parent_best_cost, parent_best_id)) =
                    self.find_best_node(ctx, parent, upper_bound)
                {
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
                    if upper_bound.is_none() || parent_best_cost < upper_bound.unwrap() {
                        upper_bound = Some(parent_best_cost);
                    }
                } else {
                    log::trace!(
                        " -> id {} parent {} nothing below upper bound {:?}",
                        id,
                        parent,
                        upper_bound
                    );
                }
            }
        }

        if let (Some(best_id), Some(best_cost)) = (best_id, best_cost) {
            log::trace!(
                "-> for eclass {}, best node is in id {} with cost {}",
                id,
                best_id,
                best_cost
            );

            self.id_to_best_cost_and_node[id] = (best_cost, best_id);

            Some((best_cost, best_id))
        } else {
            log::trace!(
                "-> for eclass {}, nothing below upper bound {:?}",
                id,
                upper_bound
            );
            None
        }
    }

    fn elaborate_eclass_use(&mut self, ctx: &ElaboratorCtx<'_>, id: Id) -> IdValue {
        self.stats.elaborate_visit_node += 1;
        let canonical = ctx.egraph.canonical_id(id);
        log::trace!("elaborate: id {}", id);

        let remat = if let Some(val) = self.id_to_value.get(&canonical) {
            // Look at the defined block, and determine whether this
            // node kind allows rematerialization if the value comes
            // from another block. If so, ignore the hit and recompute
            // below.
            let remat =
                val.block() != self.cur_block.unwrap() && ctx.remat_ids.contains(&canonical);
            if !remat {
                log::trace!("elaborate: id {} -> {:?}", id, val);
                self.stats.elaborate_memoize_hit += 1;
                return val.clone();
            }
            log::trace!("elaborate: id {} -> remat", id);
            self.stats.elaborate_memoize_miss_remat += 1;
            self.id_to_value.remove(&canonical);
            true
        } else {
            ctx.remat_ids.contains(&canonical)
        };
        self.stats.elaborate_memoize_miss += 1;

        let (_, best_node_eclass) = self
            .find_best_node(ctx, id, None)
            .expect("Must have some option with unlimited upper bound");

        log::trace!(
            "elaborate: id {} -> best {} -> eclass node {:?}",
            id,
            best_node_eclass,
            ctx.egraph.classes[best_node_eclass]
        );
        let node_key = ctx.egraph.classes[best_node_eclass].get_node().unwrap();
        let node = node_key.node::<NodeCtx>(&ctx.egraph.nodes);

        // Is the node a block param? We should never get here if so
        // (they are inserted when first visiting the block).
        if matches!(node, Node::Param { .. }) {
            unreachable!("Param nodes should already be inserted");
        }

        // Is the node a result projection? If so, at this point we
        // have everything we need; no need to allocate a new Value
        // for the result.
        if let Node::Result { value, result, .. } = node {
            let value = self.elaborate_eclass_use(ctx, *value);
            let (depth, block, values) = match value {
                IdValue::Values {
                    depth,
                    block,
                    values,
                    ..
                } => (depth, block, values),
                IdValue::Value { .. } => {
                    unreachable!("Projection nodes should not be used on single results");
                }
            };
            let values = values.as_slice(&self.func.dfg.value_lists);
            let value = IdValue::Value {
                depth,
                block,
                value: values[*result],
            };
            self.id_to_value.insert_if_absent(canonical, value.clone());
            return value;
        }

        // We're going to need to emit this operator. First, elaborate
        // all args, recursively. Also track maximum loop depth while we're here.
        let mut max_loop_depth = 0;
        let args: SmallVec<[Value; 8]> = ctx
            .node_ctx
            .children(&node)
            .iter()
            .map(|&id| {
                self.stats.elaborate_visit_node_recurse += 1;
                self.elaborate_eclass_use(ctx, id)
            })
            .map(|idvalue| match idvalue {
                IdValue::Value { depth, value, .. } => {
                    max_loop_depth = std::cmp::max(max_loop_depth, depth);
                    value
                }
                IdValue::Values { .. } => panic!("enode depends directly on multi-value result"),
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
        } else if max_loop_depth == self.cur_loop_depth() || remat {
            // Pure op, but depends on some value at the current loop depth, or remat forces it here: as above.
            (
                self.cur_loop_depth(),
                self.id_to_value.depth(),
                self.cur_block.unwrap(),
            )
        } else {
            // Pure op, and does not depend on any args at current loop depth: hoist out of loop.
            self.stats.elaborate_licm_hoist += 1;
            let data = &self.loop_stack[max_loop_depth as usize];
            (max_loop_depth, data.scope_depth as usize, data.hoist_block)
        };

        // This is an actual operation; emit the node in sequence now.
        let results = self.add_node(ctx, node, &args[..], block);
        let results_slice = results.as_slice(&self.func.dfg.value_lists);

        // Build the result and memoize in the id-to-value map.
        let result = if results_slice.len() == 1 {
            IdValue::Value {
                depth: loop_depth,
                block,
                value: results_slice[0],
            }
        } else {
            IdValue::Values {
                depth: loop_depth,
                block,
                values: results,
            }
        };

        self.id_to_value
            .insert_if_absent_with_depth(canonical, result.clone(), scope_depth);
        result
    }

    fn elaborate_block<'b, PF: Fn(Block) -> &'b [(Id, Type)], RF: Fn(Block) -> &'b [Id]>(
        &mut self,
        ctx: &ElaboratorCtx<'_>,
        idom: Option<Block>,
        block: Block,
        block_params_fn: &PF,
        block_roots_fn: &RF,
        domtree: &DomTreeWithChildren,
    ) {
        self.id_to_value.increment_depth();

        let blockparam_ids_tys = (block_params_fn)(block);
        self.start_block(ctx, idom, block, blockparam_ids_tys);
        for &id in (block_roots_fn)(block) {
            self.id_to_best_cost_and_node[id] = (0, id);
            self.elaborate_eclass_use(ctx, id);
        }

        for child in domtree.children(block) {
            self.elaborate_block(
                ctx,
                Some(block),
                child,
                block_params_fn,
                block_roots_fn,
                domtree,
            );
        }

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
        ctx: &ElaboratorCtx<'_>,
        block_params_fn: PF,
        block_roots_fn: RF,
    ) {
        let domtree = DomTreeWithChildren::new(self.func, ctx.domtree);
        let root = domtree.root();
        self.stats.elaborate_func += 1;
        self.stats.elaborate_func_pre_insts += self.func.dfg.num_insts() as u64;
        self.clear_func_body();
        self.elaborate_block(ctx, None, root, &block_params_fn, &block_roots_fn, &domtree);
        self.stats.elaborate_func_post_insts += self.func.dfg.num_insts() as u64;
    }
}
