//! Constructing Conventional SSA form.
//!
//! Conventional SSA (CSSA) form is a subset of SSA form where any (transitively) phi-related
//! values do not interfere. We construct CSSA by building virtual registers that are as large as
//! possible and inserting copies where necessary such that all argument values passed to an EBB
//! parameter will belong to the same virtual register as the EBB parameter value itself.

use crate::cursor::{Cursor, EncCursor};
use crate::dbg::DisplayList;
use crate::dominator_tree::{DominatorTree, DominatorTreePreorder};
use crate::flowgraph::{BasicBlock, ControlFlowGraph};
use crate::fx::FxHashMap;
use crate::ir::{self, InstBuilder, ProgramOrder};
use crate::ir::{Ebb, ExpandedProgramPoint, Function, Inst, Value};
use crate::isa::{EncInfo, TargetIsa};
use crate::regalloc::affinity::Affinity;
use crate::regalloc::liveness::Liveness;
use crate::regalloc::virtregs::{VirtReg, VirtRegs};
use crate::timing;
use core::cmp;
use core::fmt;
use core::iter;
use core::slice;
use log::debug;
use std::vec::Vec;

// # Implementation
//
// The coalescing algorithm implemented follows this paper fairly closely:
//
//     Budimlic, Z., Cooper, K. D., Harvey, T. J., et al. (2002). Fast copy coalescing and
//     live-range identification (Vol. 37, pp. 25–32). ACM. https://doi.org/10.1145/543552.512534
//
// We use a more efficient dominator forest representation (a linear stack) described here:
//
//     Boissinot, B., Darte, A., & Rastello, F. (2009). Revisiting out-of-SSA translation for
//     correctness, code quality and efficiency.
//
// The algorithm has two main phases:
//
// Phase 1: Union-find.
//
// We use the union-find support in `VirtRegs` to build virtual registers such that EBB parameter
// values always belong to the same virtual register as their corresponding EBB arguments at the
// predecessor branches. Trivial interferences between parameter and argument value live ranges are
// detected and resolved before unioning congruence classes, but non-trivial interferences between
// values that end up in the same congruence class are possible.
//
// Phase 2: Dominator forests.
//
// The virtual registers formed in phase 1 can contain interferences that we need to detect and
// eliminate. By ordering the values in a virtual register according to a dominator tree pre-order,
// we can identify all interferences in the virtual register in linear time.
//
// Interfering values are isolated and virtual registers rebuilt.

/// Data structures to be used by the coalescing pass.
pub struct Coalescing {
    preorder: DominatorTreePreorder,
    forest: DomForest,
    vcopies: VirtualCopies,
    values: Vec<Value>,
    predecessors: Vec<Inst>,
    backedges: Vec<Inst>,
}

/// One-shot context created once per invocation.
struct Context<'a> {
    isa: &'a TargetIsa,
    encinfo: EncInfo,

    func: &'a mut Function,
    cfg: &'a ControlFlowGraph,
    domtree: &'a DominatorTree,
    preorder: &'a DominatorTreePreorder,
    liveness: &'a mut Liveness,
    virtregs: &'a mut VirtRegs,

    forest: &'a mut DomForest,
    vcopies: &'a mut VirtualCopies,
    values: &'a mut Vec<Value>,
    predecessors: &'a mut Vec<Inst>,
    backedges: &'a mut Vec<Inst>,
}

impl Coalescing {
    /// Create a new coalescing pass.
    pub fn new() -> Self {
        Self {
            forest: DomForest::new(),
            preorder: DominatorTreePreorder::new(),
            vcopies: VirtualCopies::new(),
            values: Vec::new(),
            predecessors: Vec::new(),
            backedges: Vec::new(),
        }
    }

    /// Clear all data structures in this coalescing pass.
    pub fn clear(&mut self) {
        self.forest.clear();
        self.vcopies.clear();
        self.values.clear();
        self.predecessors.clear();
        self.backedges.clear();
    }

    /// Convert `func` to Conventional SSA form and build virtual registers in the process.
    pub fn conventional_ssa(
        &mut self,
        isa: &TargetIsa,
        func: &mut Function,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
        liveness: &mut Liveness,
        virtregs: &mut VirtRegs,
    ) {
        let _tt = timing::ra_cssa();
        debug!("Coalescing for:\n{}", func.display(isa));
        self.preorder.compute(domtree, &func.layout);
        let mut context = Context {
            isa,
            encinfo: isa.encoding_info(),
            func,
            cfg,
            domtree,
            preorder: &self.preorder,
            liveness,
            virtregs,
            forest: &mut self.forest,
            vcopies: &mut self.vcopies,
            values: &mut self.values,
            predecessors: &mut self.predecessors,
            backedges: &mut self.backedges,
        };

        // Run phase 1 (union-find) of the coalescing algorithm on the current function.
        for &ebb in domtree.cfg_postorder() {
            context.union_find_ebb(ebb);
        }
        context.finish_union_find();

        // Run phase 2 (dominator forests) on the current function.
        context.process_vregs();
    }
}

/// Phase 1: Union-find.
///
/// The two entry points for phase 1 are `union_find_ebb()` and `finish_union_find`.
impl<'a> Context<'a> {
    /// Run the union-find algorithm on the parameter values on `ebb`.
    ///
    /// This ensure that all EBB parameters will belong to the same virtual register as their
    /// corresponding arguments at all predecessor branches.
    pub fn union_find_ebb(&mut self, ebb: Ebb) {
        let num_params = self.func.dfg.num_ebb_params(ebb);
        if num_params == 0 {
            return;
        }

        self.isolate_conflicting_params(ebb, num_params);

        for i in 0..num_params {
            self.union_pred_args(ebb, i);
        }
    }

    // Identify EBB parameter values that are live at one of the predecessor branches.
    //
    // Such a parameter value will conflict with any argument value at the predecessor branch, so
    // it must be isolated by inserting a copy.
    fn isolate_conflicting_params(&mut self, ebb: Ebb, num_params: usize) {
        debug_assert_eq!(num_params, self.func.dfg.num_ebb_params(ebb));
        // The only way a parameter value can interfere with a predecessor branch is if the EBB is
        // dominating the predecessor branch. That is, we are looking for loop back-edges.
        for BasicBlock {
            ebb: pred_ebb,
            inst: pred_inst,
        } in self.cfg.pred_iter(ebb)
        {
            // The quick pre-order dominance check is accurate because the EBB parameter is defined
            // at the top of the EBB before any branches.
            if !self.preorder.dominates(ebb, pred_ebb) {
                continue;
            }

            debug!(
                " - checking {} params at back-edge {}: {}",
                num_params,
                pred_ebb,
                self.func.dfg.display_inst(pred_inst, self.isa)
            );

            // Now `pred_inst` is known to be a back-edge, so it is possible for parameter values
            // to be live at the use.
            for i in 0..num_params {
                let param = self.func.dfg.ebb_params(ebb)[i];
                if self.liveness[param].reaches_use(
                    pred_inst,
                    pred_ebb,
                    self.liveness.context(&self.func.layout),
                ) {
                    self.isolate_param(ebb, param);
                }
            }
        }
    }

    // Union EBB parameter value `num` with the corresponding EBB arguments on the predecessor
    // branches.
    //
    // Detect cases where the argument value is live-in to `ebb` so it conflicts with any EBB
    // parameter. Isolate the argument in those cases before unioning it with the parameter value.
    fn union_pred_args(&mut self, ebb: Ebb, argnum: usize) {
        let param = self.func.dfg.ebb_params(ebb)[argnum];

        for BasicBlock {
            ebb: pred_ebb,
            inst: pred_inst,
        } in self.cfg.pred_iter(ebb)
        {
            let arg = self.func.dfg.inst_variable_args(pred_inst)[argnum];

            // Never coalesce incoming function parameters on the stack. These parameters are
            // pre-spilled, and the rest of the virtual register would be forced to spill to the
            // `incoming_arg` stack slot too.
            if let ir::ValueDef::Param(def_ebb, def_num) = self.func.dfg.value_def(arg) {
                if Some(def_ebb) == self.func.layout.entry_block()
                    && self.func.signature.params[def_num].location.is_stack()
                {
                    debug!("-> isolating function stack parameter {}", arg);
                    let new_arg = self.isolate_arg(pred_ebb, pred_inst, argnum, arg);
                    self.virtregs.union(param, new_arg);
                    continue;
                }
            }

            // Check for basic interference: If `arg` overlaps a value defined at the entry to
            // `ebb`, it can never be used as an EBB argument.
            let interference = {
                let lr = &self.liveness[arg];
                let ctx = self.liveness.context(&self.func.layout);

                // There are two ways the argument value can interfere with `ebb`:
                //
                // 1. It is defined in a dominating EBB and live-in to `ebb`.
                // 2. If is itself a parameter value for `ebb`. This case should already have been
                //    eliminated by `isolate_conflicting_params()`.
                debug_assert!(
                    lr.def() != ebb.into(),
                    "{} parameter {} was missed by isolate_conflicting_params()",
                    ebb,
                    arg
                );

                // The only other possibility is that `arg` is live-in to `ebb`.
                lr.is_livein(ebb, ctx)
            };

            if interference {
                let new_arg = self.isolate_arg(pred_ebb, pred_inst, argnum, arg);
                self.virtregs.union(param, new_arg);
            } else {
                self.virtregs.union(param, arg);
            }
        }
    }

    // Isolate EBB parameter value `param` on `ebb`.
    //
    // When `param=v10`:
    //
    //     ebb1(v10: i32):
    //         foo
    //
    // becomes:
    //
    //     ebb1(v11: i32):
    //         v10 = copy v11
    //         foo
    //
    // This function inserts the copy and updates the live ranges of the old and new parameter
    // values. Returns the new parameter value.
    fn isolate_param(&mut self, ebb: Ebb, param: Value) -> Value {
        debug_assert_eq!(
            self.func.dfg.value_def(param).pp(),
            ExpandedProgramPoint::Ebb(ebb)
        );
        let ty = self.func.dfg.value_type(param);
        let new_val = self.func.dfg.replace_ebb_param(param, ty);

        // Insert a copy instruction at the top of `ebb`.
        let mut pos = EncCursor::new(self.func, self.isa).at_first_inst(ebb);
        if let Some(inst) = pos.current_inst() {
            pos.use_srcloc(inst);
        }
        pos.ins().with_result(param).copy(new_val);
        let inst = pos.built_inst();
        self.liveness.move_def_locally(param, inst);

        debug!(
            "-> inserted {}, following {}({}: {})",
            pos.display_inst(inst),
            ebb,
            new_val,
            ty
        );

        // Create a live range for the new value.
        // TODO: Should we handle ghost values?
        let affinity = Affinity::new(
            &self
                .encinfo
                .operand_constraints(pos.func.encodings[inst])
                .expect("Bad copy encoding")
                .outs[0],
        );
        self.liveness.create_dead(new_val, ebb, affinity);
        self.liveness
            .extend_locally(new_val, ebb, inst, &pos.func.layout);

        new_val
    }

    // Isolate the EBB argument `pred_val` from the predecessor `(pred_ebb, pred_inst)`.
    //
    // It is assumed that `pred_inst` is a branch instruction in `pred_ebb` whose `argnum`'th EBB
    // argument is `pred_val`. Since the argument value interferes with the corresponding EBB
    // parameter at the destination, a copy is used instead:
    //
    //     brnz v1, ebb2(v10)
    //
    // Becomes:
    //
    //     v11 = copy v10
    //     brnz v1, ebb2(v11)
    //
    // This way the interference with the EBB parameter is avoided.
    //
    // A live range for the new value is created while the live range for `pred_val` is left
    // unaltered.
    //
    // The new argument value is returned.
    fn isolate_arg(
        &mut self,
        pred_ebb: Ebb,
        pred_inst: Inst,
        argnum: usize,
        pred_val: Value,
    ) -> Value {
        let mut pos = EncCursor::new(self.func, self.isa).at_inst(pred_inst);
        pos.use_srcloc(pred_inst);
        let copy = pos.ins().copy(pred_val);
        let inst = pos.built_inst();

        // Create a live range for the new value.
        // TODO: Handle affinity for ghost values.
        let affinity = Affinity::new(
            &self
                .encinfo
                .operand_constraints(pos.func.encodings[inst])
                .expect("Bad copy encoding")
                .outs[0],
        );
        self.liveness.create_dead(copy, inst, affinity);
        self.liveness
            .extend_locally(copy, pred_ebb, pred_inst, &pos.func.layout);

        pos.func.dfg.inst_variable_args_mut(pred_inst)[argnum] = copy;

        debug!(
            "-> inserted {}, before {}: {}",
            pos.display_inst(inst),
            pred_ebb,
            pos.display_inst(pred_inst)
        );

        copy
    }

    /// Finish the union-find part of the coalescing algorithm.
    ///
    /// This builds the initial set of virtual registers as the transitive/reflexive/symmetric
    /// closure of the relation formed by EBB parameter-argument pairs found by `union_find_ebb()`.
    fn finish_union_find(&mut self) {
        self.virtregs.finish_union_find(None);
        debug!("After union-find phase:{}", self.virtregs);
    }
}

/// Phase 2: Dominator forests.
///
/// The main entry point is `process_vregs()`.
impl<'a> Context<'a> {
    /// Check al virtual registers for interference and fix conflicts.
    pub fn process_vregs(&mut self) {
        for vreg in self.virtregs.all_virtregs() {
            self.process_vreg(vreg);
        }
    }

    // Check `vreg` for interferences and fix conflicts.
    fn process_vreg(&mut self, vreg: VirtReg) {
        if !self.check_vreg(vreg) {
            self.synthesize_vreg(vreg);
        }
    }

    // Check `vreg` for interferences.
    //
    // We use a Budimlic dominator forest to check for interferences between the values in `vreg`
    // and identify values that should be isolated.
    //
    // Returns true if `vreg` is free of interference.
    fn check_vreg(&mut self, vreg: VirtReg) -> bool {
        // Order the values according to the dominator pre-order of their definition.
        let values = self.virtregs.sort_values(vreg, self.func, self.preorder);
        debug!("Checking {} = {}", vreg, DisplayList(values));

        // Now push the values in order to the dominator forest.
        // This gives us the closest dominating value def for each of the values.
        self.forest.clear();
        for &value in values {
            let node = Node::value(value, 0, self.func);

            // Push this value and get the nearest dominating def back.
            let parent = match self
                .forest
                .push_node(node, self.func, self.domtree, self.preorder)
            {
                None => continue,
                Some(n) => n,
            };

            // Check for interference between `parent` and `value`. Since `parent` dominates
            // `value`, we only have to check if it overlaps the definition.
            let ctx = self.liveness.context(&self.func.layout);
            if self.liveness[parent.value].overlaps_def(node.def, node.ebb, ctx) {
                // The two values are interfering, so they can't be in the same virtual register.
                debug!("-> interference: {} overlaps def of {}", parent, value);
                return false;
            }
        }

        // No interference found.
        true
    }

    /// Destroy and rebuild `vreg` by iterative coalescing.
    ///
    /// When detecting that a virtual register formed in phase 1 contains interference, we have to
    /// start over in a more careful way. We'll split the vreg into individual values and then
    /// reassemble virtual registers using an iterative algorithm of pairwise merging.
    ///
    /// It is possible to recover multiple large virtual registers this way while still avoiding
    /// a lot of copies.
    fn synthesize_vreg(&mut self, vreg: VirtReg) {
        self.vcopies.initialize(
            self.virtregs.values(vreg),
            self.func,
            self.cfg,
            self.preorder,
        );
        debug!(
            "Synthesizing {} from {} branches and params {}",
            vreg,
            self.vcopies.branches.len(),
            DisplayList(&self.vcopies.params)
        );
        self.virtregs.remove(vreg);

        while let Some(param) = self.vcopies.next_param() {
            self.merge_param(param);
            self.vcopies.merged_param(param, self.func);
        }
    }

    /// Merge EBB parameter value `param` with virtual registers at its predecessors.
    fn merge_param(&mut self, param: Value) {
        let (ebb, argnum) = match self.func.dfg.value_def(param) {
            ir::ValueDef::Param(e, n) => (e, n),
            ir::ValueDef::Result(_, _) => panic!("Expected parameter"),
        };

        // Collect all the predecessors and rearrange them.
        //
        // The order we process the predecessors matters because once one predecessor's virtual
        // register is merged, it can cause interference with following merges. This means that the
        // first predecessors processed are more likely to be copy-free. We want an ordering that
        // is a) good for performance and b) as stable as possible. The pred_iter() iterator uses
        // instruction numbers which is not great for reproducible test cases.
        //
        // First merge loop back-edges in layout order, on the theory that shorter back-edges are
        // more sensitive to inserted copies.
        //
        // Second everything else in reverse layout order. Again, short forward branches get merged
        // first. There can also be backwards branches mixed in here, though, as long as they are
        // not loop backedges.
        debug_assert!(self.predecessors.is_empty());
        debug_assert!(self.backedges.is_empty());
        for BasicBlock {
            ebb: pred_ebb,
            inst: pred_inst,
        } in self.cfg.pred_iter(ebb)
        {
            if self.preorder.dominates(ebb, pred_ebb) {
                self.backedges.push(pred_inst);
            } else {
                self.predecessors.push(pred_inst);
            }
        }
        // Order instructions in reverse order so we can pop them off the back.
        {
            let l = &self.func.layout;
            self.backedges.sort_unstable_by(|&a, &b| l.cmp(b, a));
            self.predecessors.sort_unstable_by(|&a, &b| l.cmp(a, b));
            self.predecessors.extend_from_slice(&self.backedges);
            self.backedges.clear();
        }

        while let Some(pred_inst) = self.predecessors.pop() {
            let arg = self.func.dfg.inst_variable_args(pred_inst)[argnum];

            // We want to merge the vreg containing `param` with the vreg containing `arg`.
            if self.try_merge_vregs(param, arg) {
                continue;
            }

            // Can't merge because of interference. Insert a copy instead.
            let pred_ebb = self.func.layout.pp_ebb(pred_inst);
            let new_arg = self.isolate_arg(pred_ebb, pred_inst, argnum, arg);
            self.virtregs
                .insert_single(param, new_arg, self.func, self.preorder);
        }
    }

    /// Merge the virtual registers containing `param` and `arg` if possible.
    ///
    /// Use self.vcopies to check for virtual copy interference too.
    ///
    /// Returns true if the virtual registers are successfully merged.
    fn try_merge_vregs(&mut self, param: Value, arg: Value) -> bool {
        if self.virtregs.same_class(param, arg) {
            return true;
        }

        if !self.can_merge_vregs(param, arg) {
            return false;
        }

        let _vreg = self.virtregs.unify(self.values);
        debug!("-> merged into {} = {}", _vreg, DisplayList(self.values));
        true
    }

    /// Check if it is possible to merge two virtual registers.
    ///
    /// Also leave `self.values` with the ordered list of values in the merged vreg.
    fn can_merge_vregs(&mut self, param: Value, arg: Value) -> bool {
        // We only need an immutable function reference.
        let func = &*self.func;
        let domtree = self.domtree;
        let preorder = self.preorder;

        // Restrict the virtual copy nodes we look at and key the `set_id` and `value` properties
        // of the nodes. Set_id 0 will be `param` and set_id 1 will be `arg`.
        self.vcopies
            .set_filter([param, arg], func, self.virtregs, preorder);

        // Now create an ordered sequence of dom-forest nodes from three sources: The two virtual
        // registers and the filtered virtual copies.
        let v0 = self.virtregs.congruence_class(&param);
        let v1 = self.virtregs.congruence_class(&arg);
        debug!(
            " - set 0: {}\n - set 1: {}",
            DisplayList(v0),
            DisplayList(v1)
        );
        let nodes = MergeNodes::new(
            func,
            preorder,
            MergeNodes::new(
                func,
                preorder,
                v0.iter().map(|&value| Node::value(value, 0, func)),
                v1.iter().map(|&value| Node::value(value, 1, func)),
            ),
            self.vcopies.iter(func),
        );

        // Now push the values in order to the dominator forest.
        // This gives us the closest dominating value def for each of the values.
        self.forest.clear();
        self.values.clear();
        let ctx = self.liveness.context(&self.func.layout);
        for node in nodes {
            // Accumulate ordered values for the new vreg.
            if node.is_value() {
                self.values.push(node.value);
            }

            // Push this value and get the nearest dominating def back.
            let parent = match self.forest.push_node(node, func, domtree, preorder) {
                None => {
                    if node.is_vcopy {
                        self.forest.pop_last();
                    }
                    continue;
                }
                Some(n) => n,
            };

            if node.is_vcopy {
                // Vcopy nodes don't represent interference if they are copies of the parent value.
                // In that case, the node must be removed because the parent value can still be
                // live belong the vcopy.
                if parent.is_vcopy || node.value == parent.value {
                    self.forest.pop_last();
                    continue;
                }

                // Check if the parent value interferes with the virtual copy.
                let inst = node.def.unwrap_inst();
                if node.set_id != parent.set_id
                    && self.liveness[parent.value].reaches_use(inst, node.ebb, ctx)
                {
                    debug!(
                        " - interference: {} overlaps vcopy at {}:{}",
                        parent,
                        node.ebb,
                        self.func.dfg.display_inst(inst, self.isa)
                    );
                    return false;
                }

                // Keep this vcopy on the stack. It will save us a few interference checks.
                continue;
            }

            // Parent vcopies never represent any interference. We only keep them on the stack to
            // avoid an interference check against a value higher up.
            if parent.is_vcopy {
                continue;
            }

            // Both node and parent are values, so check for interference.
            debug_assert!(node.is_value() && parent.is_value());
            if node.set_id != parent.set_id
                && self.liveness[parent.value].overlaps_def(node.def, node.ebb, ctx)
            {
                // The two values are interfering.
                debug!(" - interference: {} overlaps def of {}", parent, node.value);
                return false;
            }
        }

        // The values vector should receive all values.
        debug_assert_eq!(v0.len() + v1.len(), self.values.len());

        // No interference found.
        true
    }
}

/// Dominator forest.
///
/// This is a utility type used for detecting interference in virtual registers, where each virtual
/// register is a list of values ordered according to the dominator tree pre-order.
///
/// The idea of a dominator forest was introduced on the Budimlic paper and the linear stack
/// representation in the Boissinot paper. Our version of the linear stack is slightly modified
/// because we have a pre-order of the dominator tree at the EBB granularity, not basic block
/// granularity.
///
/// Values are pushed in dominator tree pre-order of their definitions, and for each value pushed,
/// `push_node` will return the nearest previously pushed value that dominates the definition.
#[allow(dead_code)]
struct DomForest {
    // Stack representing the rightmost edge of the dominator forest so far, ending in the last
    // element of `values`.
    //
    // At all times, the EBB of each element in the stack dominates the EBB of the next one.
    stack: Vec<Node>,
}

/// A node in the dominator forest.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Node {
    /// The program point where the live range is defined.
    def: ExpandedProgramPoint,
    /// EBB containing `def`.
    ebb: Ebb,
    /// Is this a virtual copy or a value?
    is_vcopy: bool,
    /// Set identifier.
    set_id: u8,
    /// For a value node: The value defined at `def`.
    /// For a vcopy node: The relevant branch argument at `def`.
    value: Value,
}

impl Node {
    /// Create a node representing `value`.
    pub fn value(value: Value, set_id: u8, func: &Function) -> Self {
        let def = func.dfg.value_def(value).pp();
        let ebb = func.layout.pp_ebb(def);
        Self {
            def,
            ebb,
            is_vcopy: false,
            set_id,
            value,
        }
    }

    /// Create a node representing a virtual copy.
    pub fn vcopy(branch: Inst, value: Value, set_id: u8, func: &Function) -> Self {
        let def = branch.into();
        let ebb = func.layout.pp_ebb(def);
        Self {
            def,
            ebb,
            is_vcopy: true,
            set_id,
            value,
        }
    }

    /// IF this a value node?
    pub fn is_value(&self) -> bool {
        !self.is_vcopy
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_vcopy {
            write!(f, "{}:vcopy({})@{}", self.set_id, self.value, self.ebb)
        } else {
            write!(f, "{}:{}@{}", self.set_id, self.value, self.ebb)
        }
    }
}

impl DomForest {
    /// Create a new empty dominator forest.
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Clear all data structures in this dominator forest.
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Add a single node to the forest.
    ///
    /// Update the stack so its dominance invariants are preserved. Detect a parent node on the
    /// stack which is the closest one dominating the new node and return it.
    fn push_node(
        &mut self,
        node: Node,
        func: &Function,
        domtree: &DominatorTree,
        preorder: &DominatorTreePreorder,
    ) -> Option<Node> {
        // The stack contains the current sequence of dominating defs. Pop elements until we
        // find one whose EBB dominates `node.ebb`.
        while let Some(top) = self.stack.pop() {
            if preorder.dominates(top.ebb, node.ebb) {
                // This is the right insertion spot for `node`.
                self.stack.push(top);
                self.stack.push(node);

                // We know here that `top.ebb` dominates `node.ebb`, and thus `node.def`. This does
                // not necessarily mean that `top.def` dominates `node.def`, though. The `top.def`
                // program point may be below the last branch in `top.ebb` that dominates
                // `node.def`.
                //
                // We do know, though, that if there is a nearest value dominating `node.def`, it
                // will be on the stack. We just need to find the last stack entry that actually
                // dominates.
                let mut last_dom = node.def;
                for &n in self.stack.iter().rev().skip(1) {
                    // If the node is defined at the EBB header, it does in fact dominate
                    // everything else pushed on the stack.
                    let def_inst = match n.def {
                        ExpandedProgramPoint::Ebb(_) => return Some(n),
                        ExpandedProgramPoint::Inst(i) => i,
                    };

                    // We need to find the last program point in `n.ebb` to dominate `node.def`.
                    last_dom = match domtree.last_dominator(n.ebb, last_dom, &func.layout) {
                        None => n.ebb.into(),
                        Some(inst) => {
                            if func.layout.cmp(def_inst, inst) != cmp::Ordering::Greater {
                                return Some(n);
                            }
                            inst.into()
                        }
                    };
                }

                // No real dominator found on the stack.
                return None;
            }
        }

        // No dominators, start a new tree in the forest.
        self.stack.push(node);
        None
    }

    pub fn pop_last(&mut self) {
        self.stack.pop().expect("Stack is empty");
    }
}

/// Virtual copies.
///
/// When building a full virtual register at once, like phase 1 does with union-find, it is good
/// enough to check for interference between the values in the full virtual register like
/// `check_vreg()` does. However, in phase 2 we are doing pairwise merges of partial virtual
/// registers that don't represent the full transitive closure of the EBB argument-parameter
/// relation. This means that just checking for interference between values is inadequate.
///
/// Example:
///
///   v1 = iconst.i32 1
///   brnz v10, ebb1(v1)
///   v2 = iconst.i32 2
///   brnz v11, ebb1(v2)
///   return v1
///
/// ebb1(v3: i32):
///   v4 = iadd v3, v1
///
/// With just value interference checking, we could build the virtual register [v3, v1] since those
/// two values don't interfere. We can't merge v2 into this virtual register because v1 and v2
/// interfere. However, we can't resolve that interference either by inserting a copy:
///
///   v1 = iconst.i32 1
///   brnz v10, ebb1(v1)
///   v2 = iconst.i32 2
///   v20 = copy v2          <-- new value
///   brnz v11, ebb1(v20)
///   return v1
///
/// ebb1(v3: i32):
///   v4 = iadd v3, v1
///
/// The new value v20 still interferes with v1 because v1 is live across the "brnz v11" branch. We
/// shouldn't have placed v1 and v3 in the same virtual register to begin with.
///
/// LLVM detects this form of interference by inserting copies in the predecessors of all phi
/// instructions, then attempting to delete the copies. This is quite expensive because it involves
/// creating a large number of copies and value.
///
/// We'll detect this form of interference with *virtual copies*: Each EBB parameter value that
/// hasn't yet been fully merged with its EBB argument values is given a set of virtual copies at
/// the predecessors. Any candidate value to be merged is checked for interference against both the
/// virtual register and the virtual copies.
///
/// In the general case, we're checking if two virtual registers can be merged, and both can
/// contain incomplete EBB parameter values with associated virtual copies.
///
/// The `VirtualCopies` struct represents a set of incomplete parameters and their associated
/// virtual copies. Given two virtual registers, it can produce an ordered sequence of nodes
/// representing the virtual copies in both vregs.
struct VirtualCopies {
    // Incomplete EBB parameters. These don't need to belong to the same virtual register.
    params: Vec<Value>,

    // Set of `(branch, destination)` pairs. These are all the predecessor branches for the EBBs
    // whose parameters can be found in `params`.
    //
    // Ordered by dominator tree pre-order of the branch instructions.
    branches: Vec<(Inst, Ebb)>,

    // Filter for the currently active node iterator.
    //
    // An ebb => (set_id, num) entry means that branches to `ebb` are active in `set_id` with
    // branch argument number `num`.
    filter: FxHashMap<Ebb, (u8, usize)>,
}

impl VirtualCopies {
    /// Create an empty VirtualCopies struct.
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            branches: Vec::new(),
            filter: FxHashMap(),
        }
    }

    /// Clear all state.
    pub fn clear(&mut self) {
        self.params.clear();
        self.branches.clear();
        self.filter.clear();
    }

    /// Initialize virtual copies from the (interfering) values in a union-find virtual register
    /// that is going to be broken up and reassembled iteratively.
    ///
    /// The values are assumed to be in domtree pre-order.
    ///
    /// This will extract the EBB parameter values and associate virtual copies all of them.
    pub fn initialize(
        &mut self,
        values: &[Value],
        func: &Function,
        cfg: &ControlFlowGraph,
        preorder: &DominatorTreePreorder,
    ) {
        self.clear();

        let mut last_ebb = None;
        for &val in values {
            if let ir::ValueDef::Param(ebb, _) = func.dfg.value_def(val) {
                self.params.push(val);

                // We may have multiple parameters from the same EBB, but we only need to collect
                // predecessors once. Also verify the ordering of values.
                if let Some(last) = last_ebb {
                    match preorder.pre_cmp_ebb(last, ebb) {
                        cmp::Ordering::Less => {}
                        cmp::Ordering::Equal => continue,
                        cmp::Ordering::Greater => panic!("values in wrong order"),
                    }
                }

                // This EBB hasn't been seen before.
                for BasicBlock {
                    inst: pred_inst, ..
                } in cfg.pred_iter(ebb)
                {
                    self.branches.push((pred_inst, ebb));
                }
                last_ebb = Some(ebb);
            }
        }

        // Reorder the predecessor branches as required by the dominator forest.
        self.branches
            .sort_unstable_by(|&(a, _), &(b, _)| preorder.pre_cmp(a, b, &func.layout));
    }

    /// Get the next unmerged parameter value.
    pub fn next_param(&self) -> Option<Value> {
        self.params.last().cloned()
    }

    /// Indicate that `param` is now fully merged.
    pub fn merged_param(&mut self, param: Value, func: &Function) {
        let popped = self.params.pop();
        debug_assert_eq!(popped, Some(param));

        // The domtree pre-order in `self.params` guarantees that all parameters defined at the
        // same EBB will be adjacent. This means we can see when all parameters at an EBB have been
        // merged.
        //
        // We don't care about the last parameter - when that is merged we are done.
        let last = match self.params.last() {
            None => return,
            Some(x) => *x,
        };
        let ebb = func.dfg.value_def(param).unwrap_ebb();
        if func.dfg.value_def(last).unwrap_ebb() == ebb {
            // We're not done with `ebb` parameters yet.
            return;
        }

        // Alright, we know there are no remaining `ebb` parameters in `self.params`. This means we
        // can get rid of the `ebb` predecessors in `self.branches`. We don't have to, the
        // `VCopyIter` will just skip them, but this reduces its workload.
        self.branches.retain(|&(_, dest)| dest != ebb);
    }

    /// Set a filter for the virtual copy nodes we're generating.
    ///
    /// Only generate nodes for parameter values that are in the same congruence class as `reprs`.
    /// Assign a set_id to each node corresponding to the index into `reprs` of the parameter's
    /// congruence class.
    pub fn set_filter(
        &mut self,
        reprs: [Value; 2],
        func: &Function,
        virtregs: &VirtRegs,
        preorder: &DominatorTreePreorder,
    ) {
        self.filter.clear();

        // Parameters in `self.params` are ordered according to the domtree per-order, and they are
        // removed from the back once they are fully merged. This means we can stop looking for
        // parameters once we're beyond the last one.
        let last_param = *self.params.last().expect("No more parameters");
        let limit = func.dfg.value_def(last_param).unwrap_ebb();

        for (set_id, repr) in reprs.iter().enumerate() {
            let set_id = set_id as u8;
            for &value in virtregs.congruence_class(repr) {
                if let ir::ValueDef::Param(ebb, num) = func.dfg.value_def(value) {
                    if preorder.pre_cmp_ebb(ebb, limit) == cmp::Ordering::Greater {
                        // Stop once we're outside the bounds of `self.params`.
                        break;
                    }
                    self.filter.insert(ebb, (set_id, num));
                }
            }
        }
    }

    /// Look up the set_id and argument number for `ebb` in the current filter.
    ///
    /// Returns `None` if none of the currently active parameters are defined at `ebb`. Otherwise
    /// returns `(set_id, argnum)` for an active parameter defined at `ebb`.
    fn lookup(&self, ebb: Ebb) -> Option<(u8, usize)> {
        self.filter.get(&ebb).cloned()
    }

    /// Get an iterator of dom-forest nodes corresponding to the current filter.
    pub fn iter<'a>(&'a self, func: &'a Function) -> VCopyIter {
        VCopyIter {
            func,
            vcopies: self,
            branches: self.branches.iter(),
        }
    }
}

/// Virtual copy iterator.
///
/// This iterator produces dom-forest nodes corresponding to the current filter in the virtual
/// copies container.
struct VCopyIter<'a> {
    func: &'a Function,
    vcopies: &'a VirtualCopies,
    branches: slice::Iter<'a, (Inst, Ebb)>,
}

impl<'a> Iterator for VCopyIter<'a> {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        while let Some(&(branch, dest)) = self.branches.next() {
            if let Some((set_id, argnum)) = self.vcopies.lookup(dest) {
                let arg = self.func.dfg.inst_variable_args(branch)[argnum];
                return Some(Node::vcopy(branch, arg, set_id, self.func));
            }
        }
        None
    }
}

/// Node-merging iterator.
///
/// Given two ordered sequences of nodes, yield an ordered sequence containing all of them.
struct MergeNodes<'a, IA, IB>
where
    IA: Iterator<Item = Node>,
    IB: Iterator<Item = Node>,
{
    a: iter::Peekable<IA>,
    b: iter::Peekable<IB>,
    layout: &'a ir::Layout,
    preorder: &'a DominatorTreePreorder,
}

impl<'a, IA, IB> MergeNodes<'a, IA, IB>
where
    IA: Iterator<Item = Node>,
    IB: Iterator<Item = Node>,
{
    pub fn new(func: &'a Function, preorder: &'a DominatorTreePreorder, a: IA, b: IB) -> Self {
        MergeNodes {
            a: a.peekable(),
            b: b.peekable(),
            layout: &func.layout,
            preorder,
        }
    }
}

impl<'a, IA, IB> Iterator for MergeNodes<'a, IA, IB>
where
    IA: Iterator<Item = Node>,
    IB: Iterator<Item = Node>,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        let ord = match (self.a.peek(), self.b.peek()) {
            (Some(a), Some(b)) => {
                let layout = self.layout;
                self.preorder
                    .pre_cmp_ebb(a.ebb, b.ebb)
                    .then_with(|| layout.cmp(a.def, b.def))
            }
            (Some(_), None) => cmp::Ordering::Less,
            (None, Some(_)) => cmp::Ordering::Greater,
            (None, None) => return None,
        };
        // When the nodes compare equal, prefer the `a` side.
        if ord != cmp::Ordering::Greater {
            self.a.next()
        } else {
            self.b.next()
        }
    }
}
