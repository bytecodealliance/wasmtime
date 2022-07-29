//! Elaboration phase: lowers EGraph back to sequences of operations
//! in CFG nodes.

use super::domtree::DomTreeWithChildren;
use super::extract::Extractor;
use super::node::{Node, NodeCtx};
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
    extractor: &'a Extractor,
    id_to_value: ScopedHashMap<Id, IdValue>,
    /// Stack of blocks and loops in current elaboration path.
    loop_stack: SmallVec<[LoopStackEntry; 8]>,
    cur_block: Option<Block>,
    first_branch: SecondaryMap<Block, PackedOption<Inst>>,
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
        extractor: &'a Extractor,
    ) -> Self {
        let num_blocks = func.dfg.num_blocks();
        Self {
            func,
            domtree,
            loop_analysis,
            egraph,
            node_ctx,
            extractor,
            id_to_value: ScopedHashMap::new(),
            loop_stack: smallvec![],
            cur_block: None,
            first_branch: SecondaryMap::with_capacity(num_blocks),
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
        let (instdata, result_tys) = match node {
            Node::Pure { op, types, .. } | Node::Inst { op, types, .. } => {
                (op.with_args(args, &mut self.func.dfg.value_lists), types)
            }
            _ => panic!("Cannot `add_node()` on block param or projection"),
        };
        let srcloc = match node {
            Node::Inst { srcloc, .. } => *srcloc,
            _ => SourceLoc::default(),
        };
        let is_term = instdata.opcode().is_branch() || instdata.opcode().is_return();
        let inst = self.func.dfg.make_inst(instdata);
        self.func.srclocs[inst] = srcloc;
        for &ty in result_tys.as_slice(&self.node_ctx.types) {
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

    fn elaborate_eclass_use(&mut self, id: Id) -> IdValue {
        if let Some(val) = self.id_to_value.get(&id) {
            return val.clone();
        }

        let node = self
            .extractor
            .get_node(self.egraph, id)
            .expect("Should have extracted node for eclass that is used");

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
            .children(node)
            .iter()
            .map(|&id| self.elaborate_eclass_use(id))
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
        let (loop_depth, scope_depth, block) = if let Node::Inst { .. } = node {
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

        let blockparam_ids_tys = (block_params_fn)(block);
        self.start_block(idom, block, blockparam_ids_tys);
        for &id in (block_roots_fn)(block) {
            self.elaborate_eclass_use(id);
        }

        for child in domtree.children(block) {
            self.elaborate_block(Some(block), child, block_params_fn, block_roots_fn, domtree);
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
        block_params_fn: PF,
        block_roots_fn: RF,
    ) {
        let domtree = DomTreeWithChildren::new(self.func, self.domtree);
        let root = domtree.root();
        self.clear_func_body();
        self.elaborate_block(None, root, &block_params_fn, &block_roots_fn, &domtree);
    }
}
