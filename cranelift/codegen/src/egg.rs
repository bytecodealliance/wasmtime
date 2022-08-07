//! Egraph-based mid-end optimization framework.

use crate::dominator_tree::DominatorTree;
use crate::flowgraph::ControlFlowGraph;
use crate::loop_analysis::LoopAnalysis;
use crate::{
    fx::FxHashMap,
    inst_predicates::has_side_effect,
    ir::{Block, Function, Inst, InstructionImms, Opcode, Type},
};
use alloc::vec::Vec;
use core::ops::Range;
use cranelift_egraph::{EGraph, Id, Language, NewOrExisting};
use cranelift_entity::EntityList;
use cranelift_entity::SecondaryMap;

mod domtree;
mod elaborate;
mod node;
mod stores;

use elaborate::Elaborator;
pub use node::{Node, NodeCtx};
pub use stores::{AliasAnalysis, MemoryState};

pub struct FuncEGraph<'a> {
    /// Dominator tree, used for elaboration pass.
    domtree: &'a DominatorTree,
    /// Loop analysis results, used for built-in LICM during elaboration.
    loop_analysis: &'a LoopAnalysis,
    /// Last-store tracker for integrated alias analysis during egraph build.
    alias_analysis: AliasAnalysis,
    /// The egraph itself.
    pub(crate) egraph: EGraph<NodeCtx>,
    /// "node context", containing arenas for node data.
    pub(crate) node_ctx: NodeCtx,
    /// Ranges in `side_effect_ids` for sequences of side-effecting
    /// eclasses per block.
    side_effects: SecondaryMap<Block, Range<u32>>,
    side_effect_ids: Vec<Id>,
    /// Map from store instructions to their nodes; used for store-to-load forwarding.
    pub(crate) store_nodes: FxHashMap<Inst, (Type, Id)>,
    /// Ranges in `blockparam_ids_tys` for sequences of blockparam
    /// eclass IDs and types per block.
    blockparams: SecondaryMap<Block, Range<u32>>,
    blockparam_ids_tys: Vec<(Id, Type)>,
    /// Statistics recorded during the process of building,
    /// optimizing, and lowering out of this egraph.
    pub(crate) stats: Stats,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    pub(crate) node_created: u64,
    pub(crate) node_param: u64,
    pub(crate) node_result: u64,
    pub(crate) node_pure: u64,
    pub(crate) node_inst: u64,
    pub(crate) node_load: u64,
    pub(crate) node_dedup_query: u64,
    pub(crate) node_dedup_hit: u64,
    pub(crate) node_dedup_miss: u64,
    pub(crate) node_ctor_created: u64,
    pub(crate) node_ctor_deduped: u64,
    pub(crate) node_union: u64,
    pub(crate) store_map_insert: u64,
    pub(crate) side_effect_nodes: u64,
    pub(crate) rewrite_rule_return_count_sum: u64,
    pub(crate) rewrite_rule_invoked: u64,
    pub(crate) store_to_load_forward: u64,
    pub(crate) elaborate_use_eclass: u64,
    pub(crate) elaborate_find_best_node: u64,
    pub(crate) elaborate_find_best_node_existing_value: u64,
    pub(crate) elaborate_find_best_node_memoize_hit: u64,
    pub(crate) elaborate_find_best_node_memoize_miss: u64,
    pub(crate) elaborate_find_best_node_arg_recurse: u64,
    pub(crate) elaborate_find_best_node_parent_recurse: u64,
    pub(crate) elaborate_find_best_node_parent_better: u64,
    pub(crate) elaborate_visit_node: u64,
    pub(crate) elaborate_visit_node_recurse: u64,
    pub(crate) elaborate_memoize_hit: u64,
    pub(crate) elaborate_memoize_miss: u64,
    pub(crate) elaborate_licm_hoist: u64,
    pub(crate) elaborate_func: u64,
    pub(crate) elaborate_func_pre_insts: u64,
    pub(crate) elaborate_func_post_insts: u64,
}

impl<'a> FuncEGraph<'a> {
    /// Create a new EGraph for the given function. Requires the
    /// domtree to be precomputed as well; the domtree is used for
    /// scheduling when lowering out of the egraph.
    pub fn new(
        func: &Function,
        domtree: &'a DominatorTree,
        loop_analysis: &'a LoopAnalysis,
        cfg: &ControlFlowGraph,
    ) -> FuncEGraph<'a> {
        let node_count_estimate = func.dfg.num_values() * 2;
        let alias_analysis = AliasAnalysis::new(func, cfg);
        let mut this = Self {
            domtree,
            loop_analysis,
            alias_analysis,
            egraph: EGraph::with_capacity(node_count_estimate),
            node_ctx: NodeCtx::with_capacity(
                func.dfg.num_values(),
                func.dfg.value_lists.capacity(),
            ),
            side_effects: SecondaryMap::with_default(0..0),
            side_effect_ids: vec![],
            store_nodes: FxHashMap::default(),
            blockparams: SecondaryMap::with_default(0..0),
            blockparam_ids_tys: vec![],
            stats: Default::default(),
        };
        this.build(func);
        this
    }

    fn build(&mut self, func: &Function) {
        // Mapping of SSA `Value` to eclass ID.
        let mut value_to_id = FxHashMap::default();

        // For each block in RPO, create an enode for block entry, for
        // each block param, and for each instruction.
        for &block in self.domtree.cfg_postorder().iter().rev() {
            let blockparam_start = self.blockparam_ids_tys.len() as u32;
            for (i, &value) in func.dfg.block_params(block).iter().enumerate() {
                let ty = func.dfg.value_type(value);
                let param = self
                    .egraph
                    .add(
                        Node::Param {
                            block,
                            index: i as u32,
                            ty,
                        },
                        &mut self.node_ctx,
                    )
                    .get();
                value_to_id.insert(value, param);
                self.blockparam_ids_tys.push((param, ty));
                self.stats.node_created += 1;
                self.stats.node_param += 1;
            }
            let blockparam_end = self.blockparam_ids_tys.len() as u32;
            self.blockparams[block] = blockparam_start..blockparam_end;

            let side_effect_start = self.side_effect_ids.len() as u32;
            for inst in func.layout.block_insts(block) {
                let side_effect = has_side_effect(func, inst)
                    || (func.dfg[inst].opcode().can_load()
                        && !func.dfg[inst].memflags().unwrap().readonly())
                    || func.dfg[inst].opcode().can_store();

                // Build args from SSA values.
                let args = EntityList::from_iter(
                    func.dfg.inst_args(inst).iter().map(|&arg| {
                        let arg = func.dfg.resolve_aliases(arg);
                        *value_to_id
                            .get(&arg)
                            .expect("Must have seen def before this use")
                    }),
                    &mut self.node_ctx.args,
                );

                let results = func.dfg.inst_results(inst);

                let types = self
                    .node_ctx
                    .types
                    .from_iter(results.iter().map(|&val| func.dfg.value_type(val)));
                let types = types.freeze(&mut self.node_ctx.types);

                let mem_state = self.alias_analysis.get_state_for_load(inst);
                let is_readonly_load = func.dfg[inst].opcode() == Opcode::Load && {
                    let memflags = func.dfg[inst].memflags().unwrap();
                    memflags.readonly() && memflags.notrap()
                };

                // Create the egraph node.
                let op = InstructionImms::from(&func.dfg[inst]);
                let opcode = op.opcode();
                let srcloc = func.srclocs[inst];

                let node = if is_readonly_load {
                    self.stats.node_created += 1;
                    self.stats.node_pure += 1;
                    Node::Pure { op, args, types }
                } else if let Some(mem_state) = mem_state {
                    let addr = args.as_slice(&self.node_ctx.args)[0];
                    let ty = types.as_slice(&self.node_ctx.types)[0];
                    log::trace!("load at inst {} has mem state {:?}", inst, mem_state);
                    self.stats.node_created += 1;
                    self.stats.node_load += 1;
                    Node::Load {
                        op,
                        ty,
                        inst,
                        addr,
                        mem_state,
                        srcloc,
                    }
                } else if side_effect {
                    self.stats.node_created += 1;
                    self.stats.node_inst += 1;
                    Node::Inst {
                        op,
                        inst,
                        args,
                        types,
                        srcloc,
                    }
                } else {
                    self.stats.node_created += 1;
                    self.stats.node_pure += 1;
                    Node::Pure { op, args, types }
                };
                let dedup_needed = self.node_ctx.needs_dedup(&node);

                let id = self.egraph.add(node, &mut self.node_ctx);

                if dedup_needed {
                    self.stats.node_dedup_query += 1;
                    match id {
                        NewOrExisting::New(_) => {
                            self.stats.node_dedup_miss += 1;
                        }
                        NewOrExisting::Existing(_) => {
                            self.stats.node_dedup_hit += 1;
                        }
                    }
                }

                if opcode == Opcode::Store {
                    let store_data_ty = func.dfg.value_type(func.dfg.inst_args(inst)[0]);
                    self.store_nodes.insert(inst, (store_data_ty, id.get()));
                    self.stats.store_map_insert += 1;
                }

                let id = match (side_effect, mem_state, id) {
                    (_, Some(..), NewOrExisting::New(id)) => {
                        // Loads: still optimize, but otherwise add to side-effecting roots.
                        let id = crate::opts::optimize_eclass(id, self);
                        self.side_effect_ids.push(id);
                        self.stats.side_effect_nodes += 1;
                        id
                    }
                    (true, _, id) => {
                        let id = id.get();
                        self.side_effect_ids.push(id);
                        self.stats.side_effect_nodes += 1;
                        id
                    }
                    (false, _, NewOrExisting::New(id)) => {
                        // Apply all optimization rules immediately; the
                        // aegraph (acyclic egraph) works best when we do
                        // this so all uses pick up the eclass with all
                        // possible enodes.
                        crate::opts::optimize_eclass(id, self)
                    }
                    (false, _, NewOrExisting::Existing(id)) => id,
                };

                // Create results and save in Value->Id map.
                match results {
                    &[] => {}
                    &[one_result] => {
                        value_to_id.insert(one_result, id);
                    }
                    many_results => {
                        debug_assert!(many_results.len() > 1);
                        for (i, &result) in many_results.iter().enumerate() {
                            let ty = func.dfg.value_type(result);
                            let projection = self
                                .egraph
                                .add(
                                    Node::Result {
                                        value: id,
                                        result: i,
                                        ty,
                                    },
                                    &mut self.node_ctx,
                                )
                                .get();
                            self.stats.node_created += 1;
                            self.stats.node_result += 1;
                            value_to_id.insert(result, projection);
                        }
                    }
                }
            }

            let side_effect_end = self.side_effect_ids.len() as u32;
            let side_effect_range = side_effect_start..side_effect_end;
            self.side_effects[block] = side_effect_range;
        }
    }

    /// Scoped elaboration: compute a final ordering of op computation
    /// for each block and replace the given Func body.
    ///
    /// This works in concert with the domtree. We do a preorder
    /// traversal of the domtree, tracking a scoped map from Id to
    /// (new) Value. The map's scopes correspond to levels in the
    /// domtree.
    ///
    /// At each block, we iterate forward over the side-effecting
    /// eclasses, and recursively generate their arg eclasses, then
    /// emit the ops themselves.
    ///
    /// To use an eclass in a given block, we first look it up in the
    /// scoped map, and get the Value if already present. If not, we
    /// need to generate it. We emit the extracted enode for this
    /// eclass after recursively generating its args. Eclasses are
    /// thus computed "as late as possible", but then memoized into
    /// the Id-to-Value map and available to all dominated blocks and
    /// for the rest of this block. (This subsumes GVN.)
    pub fn elaborate(&mut self, func: &mut Function) {
        let mut elab = Elaborator::new(
            func,
            self.domtree,
            self.loop_analysis,
            &self.egraph,
            &self.node_ctx,
            &mut self.stats,
        );
        elab.elaborate(
            |block| {
                let blockparam_range = self.blockparams[block].clone();
                &self.blockparam_ids_tys
                    [blockparam_range.start as usize..blockparam_range.end as usize]
            },
            |block| {
                let side_effect_range = self.side_effects[block].clone();
                &self.side_effect_ids
                    [side_effect_range.start as usize..side_effect_range.end as usize]
            },
        );
    }
}
