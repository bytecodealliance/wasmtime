//! Support for egraphs represented in the DataFlowGraph.

use crate::alias_analysis::{AliasAnalysis, LastStores};
use crate::ctxhash::{CtxEq, CtxHash, CtxHashMap};
use crate::cursor::{Cursor, CursorPosition, FuncCursor};
use crate::dominator_tree::{DominatorTree, DominatorTreePreorder};
use crate::egraph::elaborate::Elaborator;
use crate::fx::FxHashSet;
use crate::inst_predicates::{is_mergeable_for_egraph, is_pure_for_egraph};
use crate::ir::pcc::Fact;
use crate::ir::{
    Block, DataFlowGraph, Function, Inst, InstructionData, Opcode, Type, Value, ValueDef,
    ValueListPool,
};
use crate::loop_analysis::LoopAnalysis;
use crate::opts::IsleContext;
use crate::scoped_hash_map::{Entry as ScopedEntry, ScopedHashMap};
use crate::settings::Flags;
use crate::trace;
use crate::unionfind::UnionFind;
use core::cmp::Ordering;
use cranelift_control::ControlPlane;
use cranelift_entity::packed_option::ReservedValue;
use cranelift_entity::SecondaryMap;
use smallvec::SmallVec;
use std::hash::Hasher;

mod cost;
mod elaborate;

/// Pass over a Function that does the whole aegraph thing.
///
/// - Removes non-skeleton nodes from the Layout.
/// - Performs a GVN-and-rule-application pass over all Values
///   reachable from the skeleton, potentially creating new Union
///   nodes (i.e., an aegraph) so that some values have multiple
///   representations.
/// - Does "extraction" on the aegraph: selects the best value out of
///   the tree-of-Union nodes for each used value.
/// - Does "scoped elaboration" on the aegraph: chooses one or more
///   locations for pure nodes to become instructions again in the
///   layout, as forced by the skeleton.
///
/// At the beginning and end of this pass, the CLIF should be in a
/// state that passes the verifier and, additionally, has no Union
/// nodes. During the pass, Union nodes may exist, and instructions in
/// the layout may refer to results of instructions that are not
/// placed in the layout.
pub struct EgraphPass<'a> {
    /// The function we're operating on.
    func: &'a mut Function,
    /// Dominator tree for the CFG, used to visit blocks in pre-order
    /// so we see value definitions before their uses, and also used for
    /// O(1) dominance checks.
    domtree: DominatorTreePreorder,
    /// Alias analysis, used during optimization.
    alias_analysis: &'a mut AliasAnalysis<'a>,
    /// Loop analysis results, used for built-in LICM during
    /// elaboration.
    loop_analysis: &'a LoopAnalysis,
    /// Compiler flags.
    flags: &'a Flags,
    /// Chaos-mode control-plane so we can test that we still get
    /// correct results when our heuristics make bad decisions.
    ctrl_plane: &'a mut ControlPlane,
    /// Which canonical Values do we want to rematerialize in each
    /// block where they're used?
    ///
    /// (A canonical Value is the *oldest* Value in an eclass,
    /// i.e. tree of union value-nodes).
    remat_values: FxHashSet<Value>,
    /// Stats collected while we run this pass.
    pub(crate) stats: Stats,
    /// Union-find that maps all members of a Union tree (eclass) back
    /// to the *oldest* (lowest-numbered) `Value`.
    pub(crate) eclasses: UnionFind<Value>,
}

// The maximum number of rewrites we will take from a single call into ISLE.
const MATCHES_LIMIT: usize = 5;

/// Context passed through node insertion and optimization.
pub(crate) struct OptimizeCtx<'opt, 'analysis>
where
    'analysis: 'opt,
{
    // Borrowed from EgraphPass:
    pub(crate) func: &'opt mut Function,
    pub(crate) value_to_opt_value: &'opt mut SecondaryMap<Value, Value>,
    pub(crate) gvn_map: &'opt mut CtxHashMap<(Type, InstructionData), Value>,
    pub(crate) effectful_gvn_map: &'opt mut ScopedHashMap<(Type, InstructionData), Value>,
    available_block: &'opt mut SecondaryMap<Value, Block>,
    pub(crate) eclasses: &'opt mut UnionFind<Value>,
    pub(crate) remat_values: &'opt mut FxHashSet<Value>,
    pub(crate) stats: &'opt mut Stats,
    domtree: &'opt DominatorTreePreorder,
    pub(crate) alias_analysis: &'opt mut AliasAnalysis<'analysis>,
    pub(crate) alias_analysis_state: &'opt mut LastStores,
    flags: &'opt Flags,
    ctrl_plane: &'opt mut ControlPlane,
    // Held locally during optimization of one node (recursively):
    pub(crate) rewrite_depth: usize,
    pub(crate) subsume_values: FxHashSet<Value>,
    optimized_values: SmallVec<[Value; MATCHES_LIMIT]>,
}

/// For passing to `insert_pure_enode`. Sometimes the enode already
/// exists as an Inst (from the original CLIF), and sometimes we're in
/// the middle of creating it and want to avoid inserting it if
/// possible until we know we need it.
pub(crate) enum NewOrExistingInst {
    New(InstructionData, Type),
    Existing(Inst),
}

impl NewOrExistingInst {
    fn get_inst_key<'a>(&'a self, dfg: &'a DataFlowGraph) -> (Type, InstructionData) {
        match self {
            NewOrExistingInst::New(data, ty) => (*ty, *data),
            NewOrExistingInst::Existing(inst) => {
                let ty = dfg.ctrl_typevar(*inst);
                (ty, dfg.insts[*inst])
            }
        }
    }
}

impl<'opt, 'analysis> OptimizeCtx<'opt, 'analysis>
where
    'analysis: 'opt,
{
    /// Optimization of a single instruction.
    ///
    /// This does a few things:
    /// - Looks up the instruction in the GVN deduplication map. If we
    ///   already have the same instruction somewhere else, with the
    ///   same args, then we can alias the original instruction's
    ///   results and omit this instruction entirely.
    ///   - Note that we do this canonicalization based on the
    ///     instruction with its arguments as *canonical* eclass IDs,
    ///     that is, the oldest (smallest index) `Value` reachable in
    ///     the tree-of-unions (whole eclass). This ensures that we
    ///     properly canonicalize newer nodes that use newer "versions"
    ///     of a value that are still equal to the older versions.
    /// - If the instruction is "new" (not deduplicated), then apply
    ///   optimization rules:
    ///   - All of the mid-end rules written in ISLE.
    ///   - Store-to-load forwarding.
    /// - Update the value-to-opt-value map, and update the eclass
    ///   union-find, if we rewrote the value to different form(s).
    pub(crate) fn insert_pure_enode(&mut self, inst: NewOrExistingInst) -> Value {
        // Create the external context for looking up and updating the
        // GVN map. This is necessary so that instructions themselves
        // do not have to carry all the references or data for a full
        // `Eq` or `Hash` impl.
        let gvn_context = GVNContext {
            union_find: self.eclasses,
            value_lists: &self.func.dfg.value_lists,
        };

        self.stats.pure_inst += 1;
        if let NewOrExistingInst::New(..) = inst {
            self.stats.new_inst += 1;
        }

        // Does this instruction already exist? If so, add entries to
        // the value-map to rewrite uses of its results to the results
        // of the original (existing) instruction. If not, optimize
        // the new instruction.
        if let Some(&orig_result) = self
            .gvn_map
            .get(&inst.get_inst_key(&self.func.dfg), &gvn_context)
        {
            self.stats.pure_inst_deduped += 1;
            if let NewOrExistingInst::Existing(inst) = inst {
                debug_assert_eq!(self.func.dfg.inst_results(inst).len(), 1);
                let result = self.func.dfg.first_result(inst);
                debug_assert!(
                    self.domtree.dominates(
                        self.available_block[orig_result],
                        self.get_available_block(inst)
                    ),
                    "GVN shouldn't replace {result} (available in {}) with non-dominating {orig_result} (available in {})",
                    self.get_available_block(inst),
                    self.available_block[orig_result],
                );
                self.value_to_opt_value[result] = orig_result;
                self.func.dfg.merge_facts(result, orig_result);
            }
            orig_result
        } else {
            // Now actually insert the InstructionData and attach
            // result value (exactly one).
            let (inst, result, ty) = match inst {
                NewOrExistingInst::New(data, typevar) => {
                    let inst = self.func.dfg.make_inst(data);
                    // TODO: reuse return value?
                    self.func.dfg.make_inst_results(inst, typevar);
                    let result = self.func.dfg.first_result(inst);
                    // Add to eclass unionfind.
                    self.eclasses.add(result);
                    // New inst. We need to do the analysis of its result.
                    (inst, result, typevar)
                }
                NewOrExistingInst::Existing(inst) => {
                    let result = self.func.dfg.first_result(inst);
                    let ty = self.func.dfg.ctrl_typevar(inst);
                    (inst, result, ty)
                }
            };

            self.attach_constant_fact(inst, result, ty);

            self.available_block[result] = self.get_available_block(inst);
            let opt_value = self.optimize_pure_enode(inst);

            for &argument in self.func.dfg.inst_args(inst) {
                self.eclasses.pin_index(argument);
            }

            let gvn_context = GVNContext {
                union_find: self.eclasses,
                value_lists: &self.func.dfg.value_lists,
            };
            self.gvn_map
                .insert((ty, self.func.dfg.insts[inst]), opt_value, &gvn_context);
            self.value_to_opt_value[result] = opt_value;
            opt_value
        }
    }

    /// Find the block where a pure instruction first becomes available,
    /// defined as the block that is closest to the root where all of
    /// its arguments are available. In the unusual case where a pure
    /// instruction has no arguments (e.g. get_return_address), we can
    /// place it anywhere, so it is available in the entry block.
    ///
    /// This function does not compute available blocks recursively.
    /// All of the instruction's arguments must have had their available
    /// blocks assigned already.
    fn get_available_block(&self, inst: Inst) -> Block {
        // Side-effecting instructions have different rules for where
        // they become available, so this function does not apply.
        debug_assert!(is_pure_for_egraph(self.func, inst));

        // Note that the def-point of all arguments to an instruction
        // in SSA lie on a line of direct ancestors in the domtree, and
        // so do their available-blocks. This means that for any pair of
        // arguments, their available blocks are either the same or one
        // strictly dominates the other. We just need to find any argument
        // whose available block is deepest in the domtree.
        self.func.dfg.insts[inst]
            .arguments(&self.func.dfg.value_lists)
            .iter()
            .map(|&v| {
                let block = self.available_block[v];
                debug_assert!(!block.is_reserved_value());
                block
            })
            .max_by(|&x, &y| {
                if self.domtree.dominates(x, y) {
                    Ordering::Less
                } else {
                    debug_assert!(self.domtree.dominates(y, x));
                    Ordering::Greater
                }
            })
            .unwrap_or(self.func.layout.entry_block().unwrap())
    }

    /// Optimizes an enode by applying any matching mid-end rewrite
    /// rules (or store-to-load forwarding, which is a special case),
    /// unioning together all possible optimized (or rewritten) forms
    /// of this expression into an eclass and returning the `Value`
    /// that represents that eclass.
    fn optimize_pure_enode(&mut self, inst: Inst) -> Value {
        // A pure node always has exactly one result.
        let orig_value = self.func.dfg.first_result(inst);

        let mut optimized_values = std::mem::take(&mut self.optimized_values);

        // Limit rewrite depth. When we apply optimization rules, they
        // may create new nodes (values) and those are, recursively,
        // optimized eagerly as soon as they are created. So we may
        // have more than one ISLE invocation on the stack. (This is
        // necessary so that as the toplevel builds the
        // right-hand-side expression bottom-up, it uses the "latest"
        // optimized values for all the constituent parts.) To avoid
        // infinite or problematic recursion, we bound the rewrite
        // depth to a small constant here.
        const REWRITE_LIMIT: usize = 5;
        if self.rewrite_depth > REWRITE_LIMIT {
            self.stats.rewrite_depth_limit += 1;
            return orig_value;
        }
        self.rewrite_depth += 1;
        trace!("Incrementing rewrite depth; now {}", self.rewrite_depth);

        // Invoke the ISLE toplevel constructor, getting all new
        // values produced as equivalents to this value.
        trace!("Calling into ISLE with original value {}", orig_value);
        self.stats.rewrite_rule_invoked += 1;
        debug_assert!(optimized_values.is_empty());
        crate::opts::generated_code::constructor_simplify(
            &mut IsleContext { ctx: self },
            orig_value,
            &mut optimized_values,
        );

        optimized_values.push(orig_value);

        // Remove any values from optimized_values that do not have
        // the highest possible available block in the domtree, in
        // O(n) time. This loop scans in reverse, establishing the
        // loop invariant that all values at indices >= idx have the
        // same available block, which is the best available block
        // seen so far. Note that orig_value must also be removed if
        // it isn't in the best block, so we push it above, which means
        // optimized_values is never empty: there's always at least one
        // value in best_block.
        let mut best_block = self.available_block[*optimized_values.last().unwrap()];
        for idx in (0..optimized_values.len() - 1).rev() {
            // At the beginning of each iteration, there is a non-empty
            // collection of values after idx, which are all available
            // at best_block.
            let this_block = self.available_block[optimized_values[idx]];
            if this_block != best_block {
                if self.domtree.dominates(this_block, best_block) {
                    // If the available block for this value dominates
                    // the best block we've seen so far, discard all
                    // the values we already checked and leave only this
                    // value in the tail of the vector.
                    optimized_values.truncate(idx + 1);
                    best_block = this_block;
                } else {
                    // Otherwise the tail of the vector contains values
                    // which are all better than this value, so we can
                    // swap any of them in place of this value to delete
                    // this one in O(1) time.
                    debug_assert!(self.domtree.dominates(best_block, this_block));
                    optimized_values.swap_remove(idx);
                    debug_assert!(optimized_values.len() > idx);
                }
            }
        }

        // It's not supposed to matter what order `simplify` returns values in.
        self.ctrl_plane.shuffle(&mut optimized_values);

        let num_matches = optimized_values.len();
        if num_matches > MATCHES_LIMIT {
            trace!(
                "Reached maximum matches limit; too many optimized values \
                 ({num_matches} > {MATCHES_LIMIT}); ignoring rest.",
            );
            optimized_values.truncate(MATCHES_LIMIT);
        }

        trace!("  -> returned from ISLE: {orig_value} -> {optimized_values:?}");

        // Create a union of all new values with the original (or
        // maybe just one new value marked as "subsuming" the
        // original, if present.)
        let mut union_value = optimized_values.pop().unwrap();
        for optimized_value in optimized_values.drain(..) {
            trace!(
                "Returned from ISLE for {}, got {:?}",
                orig_value,
                optimized_value
            );
            if optimized_value == orig_value {
                trace!(" -> same as orig value; skipping");
                continue;
            }
            if self.subsume_values.contains(&optimized_value) {
                // Merge in the unionfind so canonicalization
                // still works, but take *only* the subsuming
                // value, and break now.
                self.eclasses.union(optimized_value, union_value);
                self.func.dfg.merge_facts(optimized_value, union_value);
                union_value = optimized_value;
                break;
            }

            let old_union_value = union_value;
            union_value = self.func.dfg.union(old_union_value, optimized_value);
            self.available_block[union_value] = best_block;
            self.stats.union += 1;
            trace!(" -> union: now {}", union_value);
            self.eclasses.add(union_value);
            self.eclasses.union(old_union_value, optimized_value);
            self.func.dfg.merge_facts(old_union_value, optimized_value);
            self.eclasses.union(old_union_value, union_value);
        }

        self.rewrite_depth -= 1;
        trace!("Decrementing rewrite depth; now {}", self.rewrite_depth);

        debug_assert!(self.optimized_values.is_empty());
        self.optimized_values = optimized_values;

        union_value
    }

    /// Optimize a "skeleton" instruction, possibly removing
    /// it. Returns `true` if the instruction should be removed from
    /// the layout.
    fn optimize_skeleton_inst(&mut self, inst: Inst) -> bool {
        self.stats.skeleton_inst += 1;

        for &result in self.func.dfg.inst_results(inst) {
            self.available_block[result] = self.func.layout.inst_block(inst).unwrap();
        }

        // First, can we try to deduplicate? We need to keep some copy
        // of the instruction around because it's side-effecting, but
        // we may be able to reuse an earlier instance of it.
        if is_mergeable_for_egraph(self.func, inst) {
            let result = self.func.dfg.inst_results(inst)[0];
            trace!(" -> mergeable side-effecting op {}", inst);

            // Does this instruction already exist? If so, add entries to
            // the value-map to rewrite uses of its results to the results
            // of the original (existing) instruction. If not, optimize
            // the new instruction.
            //
            // Note that we use the "effectful GVN map", which is
            // scoped: because effectful ops are not removed from the
            // skeleton (`Layout`), we need to be mindful of whether
            // our current position is dominated by an instance of the
            // instruction. (See #5796 for details.)
            let ty = self.func.dfg.ctrl_typevar(inst);
            match self
                .effectful_gvn_map
                .entry((ty, self.func.dfg.insts[inst].clone()))
            {
                ScopedEntry::Occupied(o) => {
                    let orig_result = *o.get();
                    // Hit in GVN map -- reuse value.
                    self.value_to_opt_value[result] = orig_result;
                    trace!(" -> merges result {} to {}", result, orig_result);
                    true
                }
                ScopedEntry::Vacant(v) => {
                    // Otherwise, insert it into the value-map.
                    self.value_to_opt_value[result] = result;
                    v.insert(result);
                    trace!(" -> inserts as new (no GVN)");
                    false
                }
            }
        }
        // Otherwise, if a load or store, process it with the alias
        // analysis to see if we can optimize it (rewrite in terms of
        // an earlier load or stored value).
        else if let Some(new_result) =
            self.alias_analysis
                .process_inst(self.func, self.alias_analysis_state, inst)
        {
            self.stats.alias_analysis_removed += 1;
            let result = self.func.dfg.first_result(inst);
            trace!(
                " -> inst {} has result {} replaced with {}",
                inst,
                result,
                new_result
            );
            self.value_to_opt_value[result] = new_result;
            self.func.dfg.merge_facts(result, new_result);
            true
        }
        // Otherwise, generic side-effecting op -- always keep it, and
        // set its results to identity-map to original values.
        else {
            // Set all results to identity-map to themselves
            // in the value-to-opt-value map.
            for &result in self.func.dfg.inst_results(inst) {
                self.value_to_opt_value[result] = result;
                self.eclasses.add(result);
            }
            false
        }
    }

    /// Helper to propagate facts on constant values: if PCC is
    /// enabled, then unconditionally add a fact attesting to the
    /// Value's concrete value.
    fn attach_constant_fact(&mut self, inst: Inst, value: Value, ty: Type) {
        if self.flags.enable_pcc() {
            if let InstructionData::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } = self.func.dfg.insts[inst]
            {
                let imm: i64 = imm.into();
                self.func.dfg.facts[value] =
                    Some(Fact::constant(ty.bits().try_into().unwrap(), imm as u64));
            }
        }
    }
}

impl<'a> EgraphPass<'a> {
    /// Create a new EgraphPass.
    pub fn new(
        func: &'a mut Function,
        raw_domtree: &'a DominatorTree,
        loop_analysis: &'a LoopAnalysis,
        alias_analysis: &'a mut AliasAnalysis<'a>,
        flags: &'a Flags,
        ctrl_plane: &'a mut ControlPlane,
    ) -> Self {
        let num_values = func.dfg.num_values();
        let mut domtree = DominatorTreePreorder::new();
        domtree.compute(raw_domtree, &func.layout);
        Self {
            func,
            domtree,
            loop_analysis,
            alias_analysis,
            flags,
            ctrl_plane,
            stats: Stats::default(),
            eclasses: UnionFind::with_capacity(num_values),
            remat_values: FxHashSet::default(),
        }
    }

    /// Run the process.
    pub fn run(&mut self) {
        self.remove_pure_and_optimize();

        trace!("egraph built:\n{}\n", self.func.display());
        if cfg!(feature = "trace-log") {
            for (value, def) in self.func.dfg.values_and_defs() {
                trace!(" -> {} = {:?}", value, def);
                match def {
                    ValueDef::Result(i, 0) => {
                        trace!("  -> {} = {:?}", i, self.func.dfg.insts[i]);
                    }
                    _ => {}
                }
            }
        }
        trace!("stats: {:#?}", self.stats);
        trace!("pinned_union_count: {}", self.eclasses.pinned_union_count);
        self.elaborate();
    }

    /// Remove pure nodes from the `Layout` of the function, ensuring
    /// that only the "side-effect skeleton" remains, and also
    /// optimize the pure nodes. This is the first step of
    /// egraph-based processing and turns the pure CFG-based CLIF into
    /// a CFG skeleton with a sea of (optimized) nodes tying it
    /// together.
    ///
    /// As we walk through the code, we eagerly apply optimization
    /// rules; at any given point we have a "latest version" of an
    /// eclass of possible representations for a `Value` in the
    /// original program, which is itself a `Value` at the root of a
    /// union-tree. We keep a map from the original values to these
    /// optimized values. When we encounter any instruction (pure or
    /// side-effecting skeleton) we rewrite its arguments to capture
    /// the "latest" optimized forms of these values. (We need to do
    /// this as part of this pass, and not later using a finished map,
    /// because the eclass can continue to be updated and we need to
    /// only refer to its subset that exists at this stage, to
    /// maintain acyclicity.)
    fn remove_pure_and_optimize(&mut self) {
        // This pass relies on every value having a unique name, so first
        // eliminate any value aliases.
        self.func.dfg.resolve_all_aliases();

        let mut cursor = FuncCursor::new(self.func);
        let mut value_to_opt_value: SecondaryMap<Value, Value> =
            SecondaryMap::with_default(Value::reserved_value());
        // Map from instruction to value for hash-consing of pure ops
        // into the egraph. This can be a standard (non-scoped)
        // hashmap because pure ops have no location: they are
        // "outside of" control flow.
        //
        // Note also that we keep the controlling typevar (the `Type`
        // in the tuple below) because it may disambiguate
        // instructions that are identical except for type.
        let mut gvn_map: CtxHashMap<(Type, InstructionData), Value> =
            CtxHashMap::with_capacity(cursor.func.dfg.num_values());
        // Map from instruction to value for GVN'ing of effectful but
        // idempotent ops, which remain in the side-effecting
        // skeleton. This needs to be scoped because we cannot
        // deduplicate one instruction to another that is in a
        // non-dominating block.
        //
        // Note that we can use a ScopedHashMap here without the
        // "context" (as needed by CtxHashMap) because in practice the
        // ops we want to GVN have all their args inline. Equality on
        // the InstructionData itself is conservative: two insts whose
        // struct contents compare shallowly equal are definitely
        // identical, but identical insts in a deep-equality sense may
        // not compare shallowly equal, due to list indirection. This
        // is fine for GVN, because it is still sound to skip any
        // given GVN opportunity (and keep the original instructions).
        //
        // As above, we keep the controlling typevar here as part of
        // the key: effectful instructions may (as for pure
        // instructions) be differentiated only on the type.
        let mut effectful_gvn_map: ScopedHashMap<(Type, InstructionData), Value> =
            ScopedHashMap::new();

        // We assign an "available block" to every value. Values tied to
        // the side-effecting skeleton are available in the block where
        // they're defined. Results from pure instructions could legally
        // float up the domtree so they are available as soon as all
        // their arguments are available. Values which identify union
        // nodes are available in the same block as all values in the
        // eclass, enforced by optimize_pure_enode.
        let mut available_block: SecondaryMap<Value, Block> =
            SecondaryMap::with_default(Block::reserved_value());
        // This is an initial guess at the size we'll need, but we add
        // more values as we build simplified alternative expressions so
        // this is likely to realloc again later.
        available_block.resize(cursor.func.dfg.num_values());

        // In domtree preorder, visit blocks. (TODO: factor out an
        // iterator from this and elaborator.)
        let root = cursor.layout().entry_block().unwrap();
        enum StackEntry {
            Visit(Block),
            Pop,
        }
        let mut block_stack = vec![StackEntry::Visit(root)];
        while let Some(entry) = block_stack.pop() {
            match entry {
                StackEntry::Visit(block) => {
                    // We popped this block; push children
                    // immediately, then process this block.
                    block_stack.push(StackEntry::Pop);
                    block_stack.extend(
                        self.ctrl_plane
                            .shuffled(self.domtree.children(block))
                            .map(StackEntry::Visit),
                    );
                    effectful_gvn_map.increment_depth();

                    trace!("Processing block {}", block);
                    cursor.set_position(CursorPosition::Before(block));

                    let mut alias_analysis_state = self.alias_analysis.block_starting_state(block);

                    for &param in cursor.func.dfg.block_params(block) {
                        trace!("creating initial singleton eclass for blockparam {}", param);
                        self.eclasses.add(param);
                        value_to_opt_value[param] = param;
                        available_block[param] = block;
                    }
                    while let Some(inst) = cursor.next_inst() {
                        trace!("Processing inst {}", inst);

                        // While we're passing over all insts, create initial
                        // singleton eclasses for all result and blockparam
                        // values.  Also do initial analysis of all inst
                        // results.
                        for &result in cursor.func.dfg.inst_results(inst) {
                            trace!("creating initial singleton eclass for {}", result);
                            self.eclasses.add(result);
                        }

                        // Rewrite args of *all* instructions using the
                        // value-to-opt-value map.
                        cursor.func.dfg.map_inst_values(inst, |arg| {
                            let new_value = value_to_opt_value[arg];
                            trace!("rewriting arg {} of inst {} to {}", arg, inst, new_value);
                            debug_assert_ne!(new_value, Value::reserved_value());
                            new_value
                        });

                        // Build a context for optimization, with borrows of
                        // state. We can't invoke a method on `self` because
                        // we've borrowed `self.func` mutably (as
                        // `cursor.func`) so we pull apart the pieces instead
                        // here.
                        let mut ctx = OptimizeCtx {
                            func: cursor.func,
                            value_to_opt_value: &mut value_to_opt_value,
                            gvn_map: &mut gvn_map,
                            effectful_gvn_map: &mut effectful_gvn_map,
                            available_block: &mut available_block,
                            eclasses: &mut self.eclasses,
                            rewrite_depth: 0,
                            subsume_values: FxHashSet::default(),
                            remat_values: &mut self.remat_values,
                            stats: &mut self.stats,
                            domtree: &self.domtree,
                            alias_analysis: self.alias_analysis,
                            alias_analysis_state: &mut alias_analysis_state,
                            flags: self.flags,
                            ctrl_plane: self.ctrl_plane,
                            optimized_values: Default::default(),
                        };

                        if is_pure_for_egraph(ctx.func, inst) {
                            // Insert into GVN map and optimize any new nodes
                            // inserted (recursively performing this work for
                            // any nodes the optimization rules produce).
                            let inst = NewOrExistingInst::Existing(inst);
                            ctx.insert_pure_enode(inst);
                            // We've now rewritten all uses, or will when we
                            // see them, and the instruction exists as a pure
                            // enode in the eclass, so we can remove it.
                            cursor.remove_inst_and_step_back();
                        } else {
                            if ctx.optimize_skeleton_inst(inst) {
                                cursor.remove_inst_and_step_back();
                            }
                        }
                    }
                }
                StackEntry::Pop => {
                    effectful_gvn_map.decrement_depth();
                }
            }
        }
    }

    /// Scoped elaboration: compute a final ordering of op computation
    /// for each block and update the given Func body. After this
    /// runs, the function body is back into the state where every
    /// Inst with an used result is placed in the layout (possibly
    /// duplicated, if our code-motion logic decides this is the best
    /// option).
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
    fn elaborate(&mut self) {
        let mut elaborator = Elaborator::new(
            self.func,
            &self.domtree,
            self.loop_analysis,
            &mut self.remat_values,
            &mut self.stats,
            self.ctrl_plane,
        );
        elaborator.elaborate();

        self.check_post_egraph();
    }

    #[cfg(debug_assertions)]
    fn check_post_egraph(&self) {
        // Verify that no union nodes are reachable from inst args,
        // and that all inst args' defining instructions are in the
        // layout.
        for block in self.func.layout.blocks() {
            for inst in self.func.layout.block_insts(block) {
                self.func
                    .dfg
                    .inst_values(inst)
                    .for_each(|arg| match self.func.dfg.value_def(arg) {
                        ValueDef::Result(i, _) => {
                            debug_assert!(self.func.layout.inst_block(i).is_some());
                        }
                        ValueDef::Union(..) => {
                            panic!("egraph union node {} still reachable at {}!", arg, inst);
                        }
                        _ => {}
                    })
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn check_post_egraph(&self) {}
}

/// Implementation of external-context equality and hashing on
/// InstructionData. This allows us to deduplicate instructions given
/// some context that lets us see its value lists and the mapping from
/// any value to "canonical value" (in an eclass).
struct GVNContext<'a> {
    value_lists: &'a ValueListPool,
    union_find: &'a UnionFind<Value>,
}

impl<'a> CtxEq<(Type, InstructionData), (Type, InstructionData)> for GVNContext<'a> {
    fn ctx_eq(
        &self,
        (a_ty, a_inst): &(Type, InstructionData),
        (b_ty, b_inst): &(Type, InstructionData),
    ) -> bool {
        a_ty == b_ty
            && a_inst.eq(b_inst, self.value_lists, |value| {
                self.union_find.find(value)
            })
    }
}

impl<'a> CtxHash<(Type, InstructionData)> for GVNContext<'a> {
    fn ctx_hash<H: Hasher>(&self, state: &mut H, (ty, inst): &(Type, InstructionData)) {
        std::hash::Hash::hash(&ty, state);
        inst.hash(state, self.value_lists, |value| self.union_find.find(value));
    }
}

/// Statistics collected during egraph-based processing.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    pub(crate) pure_inst: u64,
    pub(crate) pure_inst_deduped: u64,
    pub(crate) skeleton_inst: u64,
    pub(crate) alias_analysis_removed: u64,
    pub(crate) new_inst: u64,
    pub(crate) union: u64,
    pub(crate) subsume: u64,
    pub(crate) remat: u64,
    pub(crate) rewrite_rule_invoked: u64,
    pub(crate) rewrite_depth_limit: u64,
    pub(crate) elaborate_visit_node: u64,
    pub(crate) elaborate_memoize_hit: u64,
    pub(crate) elaborate_memoize_miss: u64,
    pub(crate) elaborate_remat: u64,
    pub(crate) elaborate_licm_hoist: u64,
    pub(crate) elaborate_func: u64,
    pub(crate) elaborate_func_pre_insts: u64,
    pub(crate) elaborate_func_post_insts: u64,
    pub(crate) elaborate_best_cost_fixpoint_iters: u64,
}
