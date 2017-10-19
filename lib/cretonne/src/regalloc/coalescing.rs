//! Constructing conventional SSA form.
//!
//! Conventional SSA form is a subset of SSA form where any (transitively) phi-related values do
//! not interfere. We construct CSSA by building virtual registers that are as large as possible
//! and inserting copies where necessary such that all argument values passed to an EBB parameter
//! will belong to the same virtual register as the EBB parameter value itself.

use cursor::{Cursor, EncCursor};
use dbg::DisplayList;
use dominator_tree::DominatorTree;
use flowgraph::{ControlFlowGraph, BasicBlock};
use ir::{DataFlowGraph, Layout, InstBuilder, ValueDef};
use ir::{Function, Ebb, Inst, Value, ExpandedProgramPoint};
use regalloc::affinity::Affinity;
use regalloc::liveness::Liveness;
use regalloc::virtregs::VirtRegs;
use std::cmp::Ordering;
use std::iter::Peekable;
use std::mem;
use isa::{TargetIsa, EncInfo};

/// Dominator forest.
///
/// This is a utility type used for merging virtual registers, where each virtual register is a
/// list of values ordered according to `DomTree::rpo_cmp`.
///
/// A `DomForest` object is used as a buffer for building virtual registers. It lets you merge two
/// sorted lists of values while checking for interference only whee necessary.
///
/// The idea of a dominator forest was introduced here:
///
/// Budimlic, Z., Budimlic, Z., Cooper, K. D., Cooper, K. D., Harvey, T. J., Harvey, T. J., et al.
/// (2002). Fast copy coalescing and live-range identification (Vol. 37, pp. 25–32). ACM.
/// http://doi.org/10.1145/543552.512534
///
/// The linear stack representation here:
///
/// Boissinot, B., Darte, A., & Rastello, F. (2009). Revisiting out-of-SSA translation for
/// correctness, code quality and efficiency. Presented at the Proceedings of the 7th  ….
struct DomForest {
    // The sequence of values that have been merged so far. In RPO order of their defs.
    values: Vec<Value>,

    // Stack representing the rightmost edge of the dominator forest so far, ending in the last
    // element of `values`. At all times, each element in the stack dominates the next one, and all
    // elements dominating the end of `values` are on the stack.
    stack: Vec<Node>,
}

/// A node in the dominator forest.
#[derive(Clone, Copy, Debug)]
struct Node {
    value: Value,
    /// Set identifier. Values in the same set are assumed to be non-interfering.
    set: u8,
    /// The program point where `value` is defined.
    def: ExpandedProgramPoint,
}

impl Node {
    /// Create a node for `value`.
    pub fn new(value: Value, set: u8, dfg: &DataFlowGraph) -> Node {
        Node {
            value,
            set,
            def: dfg.value_def(value).into(),
        }
    }
}

/// Push a node to `stack` and update `stack` so it contains all dominator forest ancestors of
/// the pushed value.
///

impl DomForest {
    /// Create a new empty dominator forest.
    pub fn new() -> DomForest {
        DomForest {
            values: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Swap the merged list with `buffer`, leaving the dominator forest empty.
    ///
    /// This is typically called after a successful merge to extract the merged value list.
    pub fn swap(&mut self, buffer: &mut Vec<Value>) {
        buffer.clear();
        mem::swap(&mut self.values, buffer);
    }

    /// Add a single node to the forest.
    ///
    /// Update the stack so its dominance invariants are preserved. Detect a parent node on the
    /// stack which is the closest one dominating the new node.
    ///
    /// If the pushed node's parent in the dominator forest belongs to a different set, returns
    /// `Some(parent)`.
    fn push_node(&mut self, node: Node, layout: &Layout, domtree: &DominatorTree) -> Option<Value> {
        self.values.push(node.value);

        // The stack contains the current sequence of dominating defs. Pop elements until we
        // find one that dominates `node`.
        while let Some(top) = self.stack.pop() {
            if domtree.dominates(top.def, node.def, layout) {
                // This is the right insertion spot for `node`.
                self.stack.push(top);
                self.stack.push(node);
                // If the parent value comes from a different set, return it for interference
                // checking. If the sets are equal, assume that interference is already handled.
                if top.set != node.set {
                    return Some(top.value);
                } else {
                    return None;
                }
            }
        }

        // No dominators, start a new tree in the forest.
        self.stack.push(node);
        None
    }

    /// Try to merge two sorted sets of values. Each slice must already be sorted and free of any
    /// interference.
    ///
    /// It is permitted for a value to appear in both lists. The merged sequence will only have one
    /// copy of the value.
    ///
    /// If an interference is detected, returns `Err((a, b))` with the two conflicting values form
    /// `va` and `vb` respectively.
    ///
    /// If the merge succeeds, returns `Ok(())`. The merged sequence can be extracted with
    /// `swap()`.
    pub fn try_merge(
        &mut self,
        va: &[Value],
        vb: &[Value],
        dfg: &DataFlowGraph,
        layout: &Layout,
        domtree: &DominatorTree,
        liveness: &Liveness,
    ) -> Result<(), (Value, Value)> {
        self.stack.clear();
        self.values.clear();
        self.values.reserve(va.len() + vb.len());

        // Convert the two value lists into a merged sequence of nodes.
        let merged = MergedNodes {
            a: va.iter().map(|&value| Node::new(value, 0, dfg)).peekable(),
            b: vb.iter().map(|&value| Node::new(value, 1, dfg)).peekable(),
            layout,
            domtree,
        };
        for node in merged {
            if let Some(parent) = self.push_node(node, layout, domtree) {
                // Check if `parent` live range contains `node.def`.
                let lr = liveness.get(parent).expect(
                    "No live range for parent value",
                );
                if lr.overlaps_def(node.def, layout.pp_ebb(node.def), layout) {
                    // Interference detected. Get the `(a, b)` order right in the error.
                    return Err(if node.set == 0 {
                        (node.value, parent)
                    } else {
                        (parent, node.value)
                    });
                }
            }
        }

        Ok(())
    }
}

/// Node-merging iterator.
///
/// Given two ordered sequences of nodes, yield an ordered sequence containing all of them.
/// Duplicates are removed.
struct MergedNodes<'a, IA, IB>
where
    IA: Iterator<Item = Node>,
    IB: Iterator<Item = Node>,
{
    a: Peekable<IA>,
    b: Peekable<IB>,
    layout: &'a Layout,
    domtree: &'a DominatorTree,
}

impl<'a, IA, IB> Iterator for MergedNodes<'a, IA, IB>
where
    IA: Iterator<Item = Node>,
    IB: Iterator<Item = Node>,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        let ord = match (self.a.peek(), self.b.peek()) {
            (Some(a), Some(b)) => {
                // If the two values are defined at the same point, compare value numbers instead
                // this is going to cause an interference conflict unless its actually the same
                // value appearing in both streams.
                self.domtree.rpo_cmp(a.def, b.def, self.layout).then(
                    Ord::cmp(
                        &a.value,
                        &b.value,
                    ),
                )
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => return None,
        };
        match ord {
            Ordering::Equal => {
                // The two iterators produced the same value. Just return the first one.
                self.b.next();
                self.a.next()
            }
            Ordering::Less => self.a.next(),
            Ordering::Greater => self.b.next(),
        }
    }
}

/// Data structures to be used by the coalescing pass.
pub struct Coalescing {
    forest: DomForest,

    // Current set of coalesced values. Kept sorted and interference free.
    values: Vec<Value>,

    // New values that were created when splitting interferences.
    split_values: Vec<Value>,
}

/// One-shot context created once per invocation.
struct Context<'a> {
    isa: &'a TargetIsa,
    encinfo: EncInfo,

    func: &'a mut Function,
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,
    virtregs: &'a mut VirtRegs,

    forest: &'a mut DomForest,
    values: &'a mut Vec<Value>,
    split_values: &'a mut Vec<Value>,
}

impl Coalescing {
    /// Create a new coalescing pass.
    pub fn new() -> Coalescing {
        Coalescing {
            forest: DomForest::new(),
            values: Vec::new(),
            split_values: Vec::new(),
        }

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
        dbg!("Coalescing for:\n{}", func.display(isa));
        let mut context = Context {
            isa,
            encinfo: isa.encoding_info(),
            func,
            domtree,
            liveness,
            virtregs,
            forest: &mut self.forest,
            values: &mut self.values,
            split_values: &mut self.split_values,
        };

        // TODO: The iteration order matters here. We should coalesce in the most important blocks
        // first, so they get first pick at forming virtual registers.
        for &ebb in domtree.cfg_postorder() {
            let preds = cfg.get_predecessors(ebb);
            if !preds.is_empty() {
                for argnum in 0..context.func.dfg.num_ebb_params(ebb) {
                    context.coalesce_ebb_param(ebb, argnum, preds)
                }
            }
        }
    }
}

impl<'a> Context<'a> {
    /// Coalesce the `argnum`'th parameter on `ebb`.
    fn coalesce_ebb_param(&mut self, ebb: Ebb, argnum: usize, preds: &[BasicBlock]) {
        self.split_values.clear();
        let mut succ_val = self.func.dfg.ebb_params(ebb)[argnum];
        dbg!("Processing {}/{}: {}", ebb, argnum, succ_val);

        // We want to merge the virtual register for `succ_val` with the virtual registers for
        // the branch arguments in the predecessors. This may not be possible if any live
        // ranges interfere, so we can insert copies to break interferences:
        //
        // pred:
        //     jump ebb1(v1)
        //
        // ebb1(v10: i32):
        //      ...
        //
        // In the predecessor:
        //
        //     v2 = copy v1
        //     jump ebb(v2)
        //
        // A predecessor copy is always required if the branch argument virtual register is
        // live into the successor.
        //
        // In the successor:
        //
        // ebb1(v11: i32):
        //     v10 = copy v11
        //
        // A successor copy is always required if the `succ_val` virtual register is live at
        // any predecessor branch.

        while let Some(bad_value) = self.try_coalesce(argnum, succ_val, preds) {
            dbg!("Isolating interfering value {}", bad_value);
            // The bad value has some conflict that can only be reconciled by excluding its
            // congruence class from the new virtual register.
            //
            // Try to catch infinite splitting loops. The values created by splitting should never
            // have irreconcilable interferences.
            assert!(
                !self.split_values.contains(&bad_value),
                "{} was already isolated",
                bad_value
            );
            let split_len = self.split_values.len();

            // The bad value can be both the successor value and a predecessor value at the same
            // time.
            if self.virtregs.same_class(bad_value, succ_val) {
                succ_val = self.split_succ(ebb, succ_val);
            }

            // Check the predecessors.
            for &(pred_ebb, pred_inst) in preds {
                let pred_val = self.func.dfg.inst_variable_args(pred_inst)[argnum];
                if self.virtregs.same_class(bad_value, pred_val) {
                    self.split_pred(pred_inst, pred_ebb, argnum, pred_val);
                }
            }

            // Second loop check.
            assert_ne!(
                split_len,
                self.split_values.len(),
                "Couldn't isolate {}",
                bad_value
            );
        }

        let vreg = self.virtregs.unify(self.values);
        dbg!(
            "Coalesced {} arg {} into {} = {}",
            ebb,
            argnum,
            vreg,
            DisplayList(self.virtregs.values(vreg))
        );
    }

    /// Reset `self.values` to just the set of split values.
    fn reset_values(&mut self) {
        self.values.clear();
        self.values.extend_from_slice(self.split_values);
        let domtree = &self.domtree;
        let func = &self.func;
        self.values.sort_by(|&a, &b| {
            domtree.rpo_cmp(func.dfg.value_def(a), func.dfg.value_def(b), &func.layout)
        });
    }

    /// Try coalescing predecessors with `succ_val`.
    ///
    /// Returns a value from a congruence class that needs to be split before starting over, or
    /// `None` if everything was successfully coalesced into `self.values`.
    fn try_coalesce(
        &mut self,
        argnum: usize,
        succ_val: Value,
        preds: &[BasicBlock],
    ) -> Option<Value> {
        // Initialize the value list with the split values. These are guaranteed to be
        // interference free, and anything that interferes with them must be split away.
        self.reset_values();
        dbg!("Trying {} with split values: {:?}", succ_val, self.values);

        // Start by adding `succ_val` so we can determine if it interferes with any of the new
        // split values. If it does, we must split it.
        if self.add_class(succ_val).is_err() {
            return Some(succ_val);
        }

        for &(pred_ebb, pred_inst) in preds {
            let pred_val = self.func.dfg.inst_variable_args(pred_inst)[argnum];
            dbg!(
                "Checking {}: {}: {}",
                pred_val,
                pred_ebb,
                self.func.dfg.display_inst(pred_inst, self.isa)
            );

            // Never coalesce incoming function arguments on the stack. These arguments are
            // pre-spilled, and the rest of the virtual register would be forced to spill to the
            // `incoming_arg` stack slot too.
            if let ValueDef::Param(def_ebb, def_num) = self.func.dfg.value_def(pred_val) {
                if Some(def_ebb) == self.func.layout.entry_block() &&
                    self.func.signature.argument_types[def_num]
                        .location
                        .is_stack()
                {
                    dbg!("Isolating incoming stack parameter {}", pred_val);
                    let new_val = self.split_pred(pred_inst, pred_ebb, argnum, pred_val);
                    assert!(self.add_class(new_val).is_ok());
                    continue;
                }
            }

            if let Err((a, b)) = self.add_class(pred_val) {
                dbg!("Found conflict between {} and {}", a, b);
                // We have a conflict between the already merged value `a` and one of the new
                // values `b`.
                //
                // Check if the `a` live range is fundamentally incompatible with `pred_inst`.
                if self.liveness
                    .get(a)
                    .expect("No live range for interfering value")
                    .reaches_use(pred_inst, pred_ebb, &self.func.layout)
                {
                    // Splitting at `pred_inst` wouldn't resolve the interference, so we need to
                    // start over.
                    return Some(a);
                }

                // The local conflict could likely be avoided by splitting at this predecessor, so
                // try that. This split is not necessarily required, but it allows us to make
                // progress.
                let new_val = self.split_pred(pred_inst, pred_ebb, argnum, pred_val);

                // If this tiny new live range can't be merged, there is something in the already
                // merged values that is fundamentally incompatible with `pred_inst`, and we need
                // to start over after removing that value.
                // TODO: It is unfortunate that we discover this *after* splitting. It would have
                // been better if we could detect and isolate `merged` before splitting.
                if let Err((merged, _)) = self.add_class(new_val) {
                    dbg!("Splitting didn't help: {} interferes", merged);
                    // We need to start over, isolating the bad value.
                    return Some(merged);
                }
            }
        }

        None
    }

    /// Try merging the congruence class for `value` into `self.values`.
    ///
    /// Leave `self.values` unchanged on failure.
    fn add_class(&mut self, value: Value) -> Result<(), (Value, Value)> {
        self.forest.try_merge(
            self.values,
            self.virtregs.congruence_class(&value),
            &self.func.dfg,
            &self.func.layout,
            self.domtree,
            self.liveness,
        )?;
        self.forest.swap(&mut self.values);
        Ok(())
    }

    /// Split the congruence class for the `argnum` argument to `pred_inst` by inserting a copy.
    fn split_pred(
        &mut self,
        pred_inst: Inst,
        pred_ebb: Ebb,
        argnum: usize,
        pred_val: Value,
    ) -> Value {
        let mut pos = EncCursor::new(self.func, self.isa).at_inst(pred_inst);
        let copy = pos.ins().copy(pred_val);
        let inst = pos.built_inst();

        dbg!(
            "Inserted {}, before {}: {}",
            pos.display_inst(inst),
            pred_ebb,
            pos.display_inst(pred_inst)
        );

        // Create a live range for the new value.
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
        self.split_values.push(copy);
        copy
    }

    /// Split the congruence class for the successor EBB value itself.
    fn split_succ(&mut self, ebb: Ebb, succ_val: Value) -> Value {
        let ty = self.func.dfg.value_type(succ_val);
        let new_val = self.func.dfg.replace_ebb_param(succ_val, ty);

        // Insert a copy instruction at the top of ebb.
        let mut pos = EncCursor::new(self.func, self.isa).at_first_inst(ebb);
        pos.ins().with_result(succ_val).copy(new_val);
        let inst = pos.built_inst();
        self.liveness.move_def_locally(succ_val, inst);

        dbg!(
            "Inserted {}, following {}({}: {})",
            pos.display_inst(inst),
            ebb,
            new_val,
            ty
        );

        // Create a live range for the new value.
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

        self.split_values.push(new_val);
        new_val
    }
}
