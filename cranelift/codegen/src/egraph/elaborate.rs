//! Elaboration phase: lowers EGraph back to sequences of operations
//! in CFG nodes.

use super::domtree::DomTreeWithChildren;
use super::node::{op_cost, Cost, Node, NodeCtx};
use super::Stats;
use crate::dominator_tree::DominatorTree;
use crate::fx::FxHashSet;
use crate::ir::{Block, Function, Inst, RelSourceLoc, Type, Value, ValueList};
use crate::loop_analysis::{LoopAnalysis, LoopLevel};
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
    loop_levels: &'a SecondaryMap<Id, LoopLevel>,
    id_to_value: ScopedHashMap<Id, IdValue>,
    id_to_best_cost_and_node: SecondaryMap<Id, (Cost, Id)>,
    /// Stack of blocks and loops in current elaboration path.
    loop_stack: SmallVec<[LoopStackEntry; 8]>,
    cur_block: Option<Block>,
    first_branch: SecondaryMap<Block, PackedOption<Inst>>,
    remat_ids: &'a FxHashSet<Id>,
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
        domtree: &'a DominatorTree,
        loop_analysis: &'a LoopAnalysis,
        egraph: &'a EGraph<NodeCtx>,
        node_ctx: &'a NodeCtx,
        loop_levels: &'a SecondaryMap<Id, LoopLevel>,
        remat_ids: &'a FxHashSet<Id>,
        stats: &'a mut Stats,
    ) -> Self {
        let num_blocks = func.dfg.num_blocks();
        let mut id_to_best_cost_and_node =
            SecondaryMap::with_default((Cost::infinity(), Id::invalid()));
        id_to_best_cost_and_node.resize(egraph.classes.len());
        Self {
            func,
            domtree,
            loop_analysis,
            egraph,
            node_ctx,
            loop_levels,
            id_to_value: ScopedHashMap::with_capacity(egraph.classes.len()),
            id_to_best_cost_and_node,
            loop_stack: smallvec![],
            cur_block: None,
            first_branch: SecondaryMap::with_capacity(num_blocks),
            remat_ids,
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

        // Note that if the *entry* block is a loop header, we will
        // not make note of the loop here because it will not have an
        // immediate dominator. We must disallow this case because we
        // will skip adding the `LoopStackEntry` here but our
        // `LoopAnalysis` will otherwise still make note of this loop
        // and loop depths will not match.
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
        } else {
            debug_assert!(
                self.loop_analysis.is_loop_header(block).is_none(),
                "Entry block (domtree root) cannot be a loop header!"
            );
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
            _ => RelSourceLoc::default(),
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

    fn compute_best_nodes(&mut self) {
        let best = &mut self.id_to_best_cost_and_node;
        for (eclass_id, eclass) in &self.egraph.classes {
            log::trace!("computing best for eclass {:?}", eclass_id);
            if let Some(child1) = eclass.child1() {
                log::trace!(" -> child {:?}", child1);
                best[eclass_id] = best[child1];
            }
            if let Some(child2) = eclass.child2() {
                log::trace!(" -> child {:?}", child2);
                if best[child2].0 < best[eclass_id].0 {
                    best[eclass_id] = best[child2];
                }
            }
            if let Some(node_key) = eclass.get_node() {
                let node = node_key.node::<NodeCtx>(&self.egraph.nodes);
                log::trace!(" -> eclass {:?}: node {:?}", eclass_id, node);
                let (cost, id) = match node {
                    Node::Param { .. } | Node::Inst { .. } | Node::Load { .. } => {
                        (Cost::zero(), eclass_id)
                    }
                    Node::Result { value, .. } => best[*value],
                    Node::Pure { op, .. } => {
                        let args_cost = self
                            .node_ctx
                            .children(node)
                            .iter()
                            .map(|&arg_id| {
                                log::trace!("  -> arg {:?}", arg_id);
                                best[arg_id].0
                            })
                            // Can't use `.sum()` for `Cost` types; do
                            // an explicit reduce instead.
                            .reduce(|a, b| a + b)
                            .unwrap_or(Cost::zero());
                        let level = self.loop_levels[eclass_id];
                        let cost = op_cost(op).at_level(level) + args_cost;
                        (cost, eclass_id)
                    }
                };

                if cost < best[eclass_id].0 {
                    best[eclass_id] = (cost, id);
                }
            }
            debug_assert_ne!(best[eclass_id].0, Cost::infinity());
            debug_assert_ne!(best[eclass_id].1, Id::invalid());
            log::trace!("best for eclass {:?}: {:?}", eclass_id, best[eclass_id]);
        }
    }

    fn elaborate_eclass_use(&mut self, id: Id) -> IdValue {
        self.stats.elaborate_visit_node += 1;
        let canonical = self.egraph.canonical_id(id);
        log::trace!("elaborate: id {}", id);

        let remat = if let Some(val) = self.id_to_value.get(&canonical) {
            // Look at the defined block, and determine whether this
            // node kind allows rematerialization if the value comes
            // from another block. If so, ignore the hit and recompute
            // below.
            let remat =
                val.block() != self.cur_block.unwrap() && self.remat_ids.contains(&canonical);
            if !remat {
                log::trace!("elaborate: id {} -> {:?}", id, val);
                self.stats.elaborate_memoize_hit += 1;
                return val.clone();
            }
            log::trace!("elaborate: id {} -> remat", id);
            self.stats.elaborate_memoize_miss_remat += 1;
            // The op is pure at this point, so it is always valid to
            // remove from this map.
            self.id_to_value.remove(&canonical);
            true
        } else {
            self.remat_ids.contains(&canonical)
        };
        self.stats.elaborate_memoize_miss += 1;

        // Get the best option; we use `id` (latest id) here so we
        // have a full view of the eclass.
        let (_, best_node_eclass) = self.id_to_best_cost_and_node[id];
        debug_assert_ne!(best_node_eclass, Id::invalid());

        log::trace!(
            "elaborate: id {} -> best {} -> eclass node {:?}",
            id,
            best_node_eclass,
            self.egraph.classes[best_node_eclass]
        );
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
            // Recurse here: this recursion is safe because we only
            // ever have one level of `Result` node (we don't have
            // nested tuples).
            let value = self.elaborate_eclass_use(*value);
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
        let args: SmallVec<[Value; 8]> = self
            .node_ctx
            .children(&node)
            .iter()
            .map(|&id| {
                self.stats.elaborate_visit_node_recurse += 1;
                self.elaborate_eclass_use(id)
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
            // Pure op, but depends on some value at the current loop
            // depth, or remat forces it here: as above.
            (
                self.cur_loop_depth(),
                self.id_to_value.depth(),
                self.cur_block.unwrap(),
            )
        } else {
            // Pure op, and does not depend on any args at current
            // loop depth: hoist out of loop.
            self.stats.elaborate_licm_hoist += 1;
            let data = &self.loop_stack[max_loop_depth as usize];
            (max_loop_depth, data.scope_depth as usize, data.hoist_block)
        };
        // Loop scopes are a subset of all scopes.
        debug_assert!(scope_depth >= loop_depth as usize);

        // This is an actual operation; emit the node in sequence now.
        let results = self.add_node(node, &args[..], block);
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

    fn elaborate_block<'b, PF: Fn(Block) -> &'b [(Id, Type)], SEF: Fn(Block) -> &'b [Id]>(
        &mut self,
        idom: Option<Block>,
        block: Block,
        block_params_fn: &PF,
        block_side_effects_fn: &SEF,
        domtree: &DomTreeWithChildren,
    ) {
        self.id_to_value.increment_depth();

        let blockparam_ids_tys = (block_params_fn)(block);
        self.start_block(idom, block, blockparam_ids_tys);
        for &id in (block_side_effects_fn)(block) {
            self.elaborate_eclass_use(id);
        }

        for child in domtree.children(block) {
            self.elaborate_block(
                Some(block),
                child,
                block_params_fn,
                block_side_effects_fn,
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

    pub(crate) fn elaborate<'b, PF: Fn(Block) -> &'b [(Id, Type)], SEF: Fn(Block) -> &'b [Id]>(
        &mut self,
        block_params_fn: PF,
        block_side_effects_fn: SEF,
    ) {
        let domtree = DomTreeWithChildren::new(self.func, self.domtree);
        let root = domtree.root();
        self.stats.elaborate_func += 1;
        self.stats.elaborate_func_pre_insts += self.func.dfg.num_insts() as u64;
        self.clear_func_body();
        self.compute_best_nodes();
        self.elaborate_block(
            None,
            root,
            &block_params_fn,
            &block_side_effects_fn,
            &domtree,
        );
        self.stats.elaborate_func_post_insts += self.func.dfg.num_insts() as u64;
    }
}
