//! Constructing conventional SSA form.
//!
//! Conventional SSA form is a subset of SSA form where any (transitively) phi-related values do
//! not interfere. We construct CSSA by building virtual registers that are as large as possible
//! and inserting copies where necessary such that all argument values passed to an EBB parameter
//! will belong to the same virtual register as the EBB parameter value itself.

use cursor::{Cursor, EncCursor};
use dbg::DisplayList;
use dominator_tree::{DominatorTree, DominatorTreePreorder};
use flowgraph::ControlFlowGraph;
use ir::{self, InstBuilder};
use ir::{Function, Ebb, Inst, Value, ExpandedProgramPoint};
use regalloc::affinity::Affinity;
use regalloc::liveness::Liveness;
use regalloc::virtregs::{VirtReg, VirtRegs};
use std::fmt;
use isa::{TargetIsa, EncInfo};
use timing;

// # Implementation
//
// The coalescing algorithm implemented follows this paper fairly closely:
//
//     Budimlic, Z., Cooper, K. D., Harvey, T. J., et al. (2002). Fast copy coalescing and
//     live-range identification (Vol. 37, pp. 25–32). ACM. http://doi.org/10.1145/543552.512534
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
    forest: DomForest,
    preorder: DominatorTreePreorder,

    /// EBB parameter values present in the current virtual register.
    params: Vec<Value>,

    /// Worklist of virtual registers that need to be processed.
    worklist: Vec<VirtReg>,
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
    params: &'a mut Vec<Value>,
    worklist: &'a mut Vec<VirtReg>,
}

impl Coalescing {
    /// Create a new coalescing pass.
    pub fn new() -> Self {
        Self {
            forest: DomForest::new(),
            preorder: DominatorTreePreorder::new(),
            params: Vec::new(),
            worklist: Vec::new(),
        }

    }

    /// Clear all data structures in this coalescing pass.
    pub fn clear(&mut self) {
        self.forest.clear();
        self.params.clear();
        self.worklist.clear();
    }

    /// Convert `func` to conventional SSA form and build virtual registers in the process.
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
        dbg!("Coalescing for:\n{}", func.display(isa));
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
            params: &mut self.params,
            worklist: &mut self.worklist,
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
        for (pred_ebb, pred_inst) in self.cfg.pred_iter(ebb) {
            // The quick pre-order dominance check is accurate because the EBB parameter is defined
            // at the top of the EBB before any branches.
            if !self.preorder.dominates(ebb, pred_ebb) {
                continue;
            }

            dbg!(
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
                )
                {
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

        for (pred_ebb, pred_inst) in self.cfg.pred_iter(ebb) {
            let arg = self.func.dfg.inst_variable_args(pred_inst)[argnum];

            // Never coalesce incoming function parameters on the stack. These parameters are
            // pre-spilled, and the rest of the virtual register would be forced to spill to the
            // `incoming_arg` stack slot too.
            if let ir::ValueDef::Param(def_ebb, def_num) = self.func.dfg.value_def(arg) {
                if Some(def_ebb) == self.func.layout.entry_block() &&
                    self.func.signature.params[def_num].location.is_stack()
                {
                    dbg!("-> isolating function stack parameter {}", arg);
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
                assert!(
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
        pos.ins().with_result(param).copy(new_val);
        let inst = pos.built_inst();
        self.liveness.move_def_locally(param, inst);

        dbg!(
            "-> inserted {}, following {}({}: {})",
            pos.display_inst(inst),
            ebb,
            new_val,
            ty
        );

        // Create a live range for the new value.
        // TODO: Should we handle ghost values?
        let affinity = Affinity::new(
            &self.encinfo
                .operand_constraints(pos.func.encodings[inst])
                .expect("Bad copy encoding")
                .outs
                [0],
        );
        self.liveness.create_dead(new_val, ebb, affinity);
        self.liveness.extend_locally(
            new_val,
            ebb,
            inst,
            &pos.func.layout,
        );

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
        let copy = pos.ins().copy(pred_val);
        let inst = pos.built_inst();

        // Create a live range for the new value.
        // TODO: Handle affinity for ghost values.
        let affinity = Affinity::new(
            &self.encinfo
                .operand_constraints(pos.func.encodings[inst])
                .expect("Bad copy encoding")
                .outs
                [0],
        );
        self.liveness.create_dead(copy, inst, affinity);
        self.liveness.extend_locally(
            copy,
            pred_ebb,
            pred_inst,
            &pos.func.layout,
        );

        pos.func.dfg.inst_variable_args_mut(pred_inst)[argnum] = copy;

        dbg!(
            "-> inserted {}, before {}: {}",
            pos.display_inst(inst),
            pred_ebb,
            pos.display_inst(pred_inst)
        );

        copy
    }

    /// Finish the union-find part of the coalescing algorithm.
    ///     ///
    /// This builds the initial set of virtual registers as the transitive/reflexive/symmetric
    /// closure of the relation formed by EBB parameter-argument pairs found by `union_find_ebb()`.
    fn finish_union_find(&mut self) {
        self.virtregs.finish_union_find(None);
        dbg!("After union-find phase:{}", self.virtregs);
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
            while let Some(vr) = self.worklist.pop() {
                self.process_vreg(vr);
            }
        }
    }

    // Check `vreg` for interferences and fix conflicts.
    fn process_vreg(&mut self, vreg: VirtReg) {
        if self.analyze_vreg(vreg) {
            self.synthesize_vreg(vreg);
        }
    }

    // Check `vreg` for interferences and choose values to isolate.
    //
    // We use a Budimlic dominator forest to check for interferences between the values in `vreg`
    // and identify values that should be isolated.
    //
    // Returns true if `vreg` has conflicts that need to be fixed. Additionally leaves state in
    // member variables:
    //
    // - `self.params` contains all the EBB parameter values that were present in the virtual
    //    register.
    // - `self.forest` contains the set of values that should be isolated from the virtual register.
    fn analyze_vreg(&mut self, vreg: VirtReg) -> bool {
        // Order the values according to the dominator pre-order of their definition.
        let dfg = &self.func.dfg;
        let layout = &self.func.layout;
        let preorder = self.preorder;
        let values = self.virtregs.sort_values(vreg, |a, b| {
            let da = dfg.value_def(a);
            let db = dfg.value_def(b);
            preorder.pre_cmp(da, db, layout).then(
                da.num().cmp(&db.num()),
            )
        });
        dbg!("Analyzing {} = {}", vreg, DisplayList(values));

        // Now push the values in order to the dominator forest. This gives us the closest
        // dominating value def for each of the values.
        self.params.clear();
        self.forest.clear();
        for &value in values {
            let node = Node::new(value, self.func);

            // Remember the parameter values in case we need to re-synthesize virtual registers.
            if let ExpandedProgramPoint::Ebb(_) = node.def {
                self.params.push(value);
            }

            // Push this value and get the nearest dominating def back.
            let parent = match self.forest.push_value(
                node,
                self.func,
                self.domtree,
                self.preorder,
            ) {
                None => continue,
                Some(p) => p,
            };

            // Check for interference between `parent` and `value`. Since `parent` dominates
            // `value`, we only have to check if it overlaps the definition.
            let ctx = self.liveness.context(&self.func.layout);
            if !self.liveness[parent].overlaps_def(node.def, node.ebb, ctx) {
                // No interference, both values can stay in the virtual register.
                continue;
            }

            // The two values are interfering, so they can't both be in the same virtual register.
            // We need to pick one to isolate. It's hard to pick a heuristic that only looks at two
            // values since an optimal solution is a global problem involving all the values in the
            // virtual register.
            //
            // We choose to always isolate the dominating parent value for two reasons:
            //
            // 1. We avoid the case of a parent value with a very long live range pushing many
            //    following values out of the virtual register.
            //
            // 2. In the case of a value that is live across a branch to the definition of a
            //    parameter in the virtual register, our splitting method in `synthesize_vreg`
            //    doesn't actually resolve the interference unless we're trying to isolate the
            //    first value. This heuristic will at least pick the first value on a second
            //    attempt. This is actually a correctness issue - we could loop infinitely
            //    otherwise. See the `infinite-interference.cton` test case.
            dbg!("-> isolating {} which overlaps def of {}", parent, value);
            self.forest.drop_value(parent);
        }

        let dropped = self.forest.prepare_dropped();
        assert!(dropped < values.len());
        dropped != 0
    }

    /// Destroy and rebuild `vreg`.
    ///
    /// Use `self.params` to rebuild the virtual register, but this time making sure that dropped
    /// values in `self.forest` are isolated from non-dropped values. This may cause multiple new
    /// virtual registers to be formed.
    ///
    /// All new virtual registers are appended to `self.worklist`.
    fn synthesize_vreg(&mut self, vreg: VirtReg) {
        dbg!("Synthesizing {} from {}", vreg, DisplayList(self.params));
        self.virtregs.remove(vreg);

        while let Some(param) = self.params.pop() {
            let param_dropped = self.forest.is_dropped(param);
            let (ebb, argnum) = match self.func.dfg.value_def(param) {
                ir::ValueDef::Param(e, n) => (e, n),
                ir::ValueDef::Result(_, _) => panic!("{} expected to be EBB parameter"),
            };

            // Union the EBB parameter with corresponding arguments on the predecessor branches,
            // but make sure to isolate dropped values.
            //
            // Compare `union_pred_args()` which runs during phase 1. We don't need to check for
            // special cases here since they have already been eliminated during phase 1. We
            // already know that:
            //
            // 1. `arg` is not live-in to `ebb`.
            // 2. `arg` is not a function argument on the stack.
            for (pred_ebb, pred_inst) in self.cfg.pred_iter(ebb) {
                let arg = self.func.dfg.inst_variable_args(pred_inst)[argnum];
                let arg_dropped = self.forest.is_dropped(arg);

                // We don't want to union dropped values with each other because we can't ensure
                // that we are actually making progress -- the new virtual register of dropped
                // values may have its own interferences and so on.
                //
                // TODO: Maintain a secondary dominator forest to keep track of dropped values that
                // would be allowed to be unioned together.
                if param_dropped || arg_dropped {
                    dbg!(" - {}#{}: {} isolated from {}", ebb, argnum, param, arg);
                    let new_arg = self.isolate_arg(pred_ebb, pred_inst, argnum, arg);
                    self.virtregs.union(param, new_arg);
                } else {
                    self.virtregs.union(param, arg);
                }
            }
        }

        // TODO: Get back the new vregs so they can be re-checked.
        let old_len = self.worklist.len();
        self.virtregs.finish_union_find(Some(self.worklist));
        dbg!("-> new vregs {}", DisplayList(&self.worklist[old_len..]));
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
/// `push_value` will return the nearest previously pushed value that dominates the definition.
#[allow(dead_code)]
struct DomForest {
    // Stack representing the rightmost edge of the dominator forest so far, ending in the last
    // element of `values`.
    //
    // At all times, the EBB of each element in the stack dominates the EBB of the next one, and
    // all elements dominating the end of `values` are on the stack.
    stack: Vec<Node>,

    // The index into `stack` of the last dominating node returned by `push_value`.
    last_dom: Option<usize>,

    // List of values that have been dropped from the forest because they were interfering with
    // another member.
    //
    // This list is initially just appended to, then it sorted for quick member checks with
    // `is_dropped()`.
    dropped: Vec<Value>,
}

/// A node in the dominator forest.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Node {
    value: Value,
    /// The program point where `value` is defined.
    def: ExpandedProgramPoint,
    /// EBB containing `def`.
    ebb: Ebb,
}

impl Node {
    /// Create a node for `value`.
    pub fn new(value: Value, func: &Function) -> Node {
        let def = func.dfg.value_def(value).pp();
        let ebb = func.layout.pp_ebb(def);
        Node { value, def, ebb }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.value, self.ebb)
    }
}

impl DomForest {
    /// Create a new empty dominator forest.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            last_dom: None,
            dropped: Vec::new(),
        }
    }

    /// Clear all data structures in this dominator forest.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.last_dom = None;
        self.dropped.clear();
    }

    /// Add a single value to the forest.
    ///
    /// Update the stack so its dominance invariants are preserved. Detect a parent node on the
    /// stack which is the closest one dominating the new node and return it.
    fn push_value(
        &mut self,
        node: Node,
        func: &Function,
        domtree: &DominatorTree,
        preorder: &DominatorTreePreorder,
    ) -> Option<Value> {
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
                debug_assert!(domtree.dominates(top.ebb, node.def, &func.layout));

                // We do know, though, that if there is a nearest value dominating `node.def`, it
                // will be on the stack. We just need to find the last stack entry that actually
                // dominates.
                //
                // TODO: This search could be more efficient if we had access to
                // `domtree.last_dominator()`. Each call to `dominates()` here ends up walking up
                // the dominator tree starting from `node.ebb`.
                self.last_dom = self.stack[0..self.stack.len() - 1].iter().rposition(|n| {
                    domtree.dominates(n.def, node.def, &func.layout)
                });

                // If there is a dominating parent value, return it for interference checking.
                return self.last_dom.map(|pos| self.stack[pos].value);
            }
        }

        // No dominators, start a new tree in the forest.
        self.stack.push(node);
        None
    }

    /// Drop `value` from the forest and add it to the `dropped` list.
    ///
    /// The value must be either the last value passed to `push_value` or the dominating value
    /// returned from the call.
    pub fn drop_value(&mut self, value: Value) {
        self.dropped.push(value);

        // Are they dropping the last value pushed?
        if self.stack.last().expect("Nothing pushed").value == value {
            self.stack.pop();
        } else {
            // Otherwise, they must be dropping the last dominator.
            let pos = self.last_dom.take().expect("No last dominator");
            let node = self.stack.remove(pos);
            assert_eq!(node.value, value, "Inconsistent value to drop_value");
        }
    }

    /// Prepare the set of dropped values to be queried with `is_dropped()`.
    ///
    /// Returns the number of dropped values.
    pub fn prepare_dropped(&mut self) -> usize {
        self.stack.clear();
        if !self.dropped.is_empty() {
            self.dropped.sort_unstable();
            dbg!("-> dropped {}", DisplayList(&self.dropped));
        }
        self.dropped.len()
    }

    /// Check if `value` was dropped.
    pub fn is_dropped(&self, value: Value) -> bool {
        debug_assert!(self.stack.is_empty(), "Call prepare_dropped first");
        self.dropped.binary_search(&value).is_ok()
    }
}
