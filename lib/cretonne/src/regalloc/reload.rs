//! Reload pass
//!
//! The reload pass runs between the spilling and coloring passes. Its primary responsibility is to
//! insert `spill` and `fill` instructions such that instruction operands expecting a register will
//! get a value with register affinity, and operands expecting a stack slot will get a value with
//! stack affinity.
//!
//! The secondary responsibility of the reload pass is to reuse values in registers as much as
//! possible to minimize the number of `fill` instructions needed. This must not cause the register
//! pressure limits to be exceeded.

use cursor::{Cursor, EncCursor};
use dominator_tree::DominatorTree;
use entity::{SparseMap, SparseMapValue};
use ir::{Ebb, Inst, Value, Function};
use ir::{InstBuilder, ArgumentType, ArgumentLoc};
use isa::RegClass;
use isa::{TargetIsa, Encoding, EncInfo, RecipeConstraints, ConstraintKind};
use regalloc::affinity::Affinity;
use regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use regalloc::liveness::Liveness;
use topo_order::TopoOrder;

/// Reusable data structures for the reload pass.
pub struct Reload {
    candidates: Vec<ReloadCandidate>,
    reloads: SparseMap<Value, ReloadedValue>,
}

/// Context data structure that gets instantiated once per pass.
struct Context<'a> {
    cur: EncCursor<'a>,

    // Cached ISA information.
    // We save it here to avoid frequent virtual function calls on the `TargetIsa` trait object.
    encinfo: EncInfo,

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,
    topo: &'a mut TopoOrder,

    candidates: &'a mut Vec<ReloadCandidate>,
    reloads: &'a mut SparseMap<Value, ReloadedValue>,
}

impl Reload {
    /// Create a new blank reload pass.
    pub fn new() -> Reload {
        Reload {
            candidates: Vec::new(),
            reloads: SparseMap::new(),
        }
    }

    /// Run the reload algorithm over `func`.
    pub fn run(
        &mut self,
        isa: &TargetIsa,
        func: &mut Function,
        domtree: &DominatorTree,
        liveness: &mut Liveness,
        topo: &mut TopoOrder,
        tracker: &mut LiveValueTracker,
    ) {
        dbg!("Reload for:\n{}", func.display(isa));
        let mut ctx = Context {
            cur: EncCursor::new(func, isa),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
            topo,
            candidates: &mut self.candidates,
            reloads: &mut self.reloads,
        };
        ctx.run(tracker)
    }
}

/// A reload candidate.
///
/// This represents a stack value that is used by the current instruction where a register is
/// needed.
struct ReloadCandidate {
    value: Value,
    regclass: RegClass,
}

/// A Reloaded value.
///
/// This represents a value that has been reloaded into a register value from the stack.
struct ReloadedValue {
    stack: Value,
    reg: Value,
}

impl SparseMapValue<Value> for ReloadedValue {
    fn key(&self) -> Value {
        self.stack
    }
}

impl<'a> Context<'a> {
    fn run(&mut self, tracker: &mut LiveValueTracker) {
        self.topo.reset(self.cur.func.layout.ebbs());
        while let Some(ebb) = self.topo.next(&self.cur.func.layout, self.domtree) {
            self.visit_ebb(ebb, tracker);
        }
    }

    fn visit_ebb(&mut self, ebb: Ebb, tracker: &mut LiveValueTracker) {
        dbg!("Reloading {}:", ebb);
        self.visit_ebb_header(ebb, tracker);
        tracker.drop_dead_args();

        // visit_ebb_header() places us at the first interesting instruction in the EBB.
        while let Some(inst) = self.cur.current_inst() {
            let encoding = self.cur.func.encodings[inst];
            if encoding.is_legal() {
                self.visit_inst(ebb, inst, encoding, tracker);
                tracker.drop_dead(inst);
            } else {
                self.cur.next_inst();
            }
        }
    }

    /// Process the EBB parameters. Move to the next instruction in the EBB to be processed
    fn visit_ebb_header(&mut self, ebb: Ebb, tracker: &mut LiveValueTracker) {
        let (liveins, args) = tracker.ebb_top(
            ebb,
            &self.cur.func.dfg,
            self.liveness,
            &self.cur.func.layout,
            self.domtree,
        );

        if self.cur.func.layout.entry_block() == Some(ebb) {
            assert_eq!(liveins.len(), 0);
            self.visit_entry_args(ebb, args);
        } else {
            self.visit_ebb_args(ebb, args);
        }
    }

    /// Visit the arguments to the entry block.
    /// These values have ABI constraints from the function signature.
    fn visit_entry_args(&mut self, ebb: Ebb, args: &[LiveValue]) {
        assert_eq!(self.cur.func.signature.argument_types.len(), args.len());
        self.cur.goto_first_inst(ebb);

        for (arg_idx, arg) in args.iter().enumerate() {
            let abi = self.cur.func.signature.argument_types[arg_idx];
            match abi.location {
                ArgumentLoc::Reg(_) => {
                    if arg.affinity.is_stack() {
                        // An incoming register parameter was spilled. Replace the parameter value
                        // with a temporary register value that is immediately spilled.
                        let reg = self.cur.func.dfg.replace_ebb_arg(arg.value, abi.value_type);
                        let affinity = Affinity::abi(&abi, self.cur.isa);
                        self.liveness.create_dead(reg, ebb, affinity);
                        self.insert_spill(ebb, arg.value, reg);
                    }
                }
                ArgumentLoc::Stack(_) => {
                    assert!(arg.affinity.is_stack());
                }
                ArgumentLoc::Unassigned => panic!("Unexpected ABI location"),
            }
        }
    }

    fn visit_ebb_args(&mut self, ebb: Ebb, _args: &[LiveValue]) {
        self.cur.goto_first_inst(ebb);
    }

    /// Process the instruction pointed to by `pos`, and advance the cursor to the next instruction
    /// that needs processing.
    fn visit_inst(
        &mut self,
        ebb: Ebb,
        inst: Inst,
        encoding: Encoding,
        tracker: &mut LiveValueTracker,
    ) {
        self.cur.use_srcloc(inst);

        // Get the operand constraints for `inst` that we are trying to satisfy.
        let constraints = self.encinfo.operand_constraints(encoding).expect(
            "Missing instruction encoding",
        );

        // Identify reload candidates.
        assert!(self.candidates.is_empty());
        self.find_candidates(inst, constraints);

        // Insert fill instructions before `inst`.
        while let Some(cand) = self.candidates.pop() {
            if let Some(_reload) = self.reloads.get_mut(cand.value) {
                continue;
            }

            let reg = self.cur.ins().fill(cand.value);
            let fill = self.cur.built_inst();

            self.reloads.insert(ReloadedValue {
                stack: cand.value,
                reg: reg,
            });

            // Create a live range for the new reload.
            let affinity = Affinity::Reg(cand.regclass.into());
            self.liveness.create_dead(reg, fill, affinity);
            self.liveness.extend_locally(
                reg,
                ebb,
                inst,
                &self.cur.func.layout,
            );
        }

        // Rewrite arguments.
        for arg in self.cur.func.dfg.inst_args_mut(inst) {
            if let Some(reload) = self.reloads.get(*arg) {
                *arg = reload.reg;
            }
        }

        // TODO: Reuse reloads for future instructions.
        self.reloads.clear();

        let (_throughs, _kills, defs) =
            tracker.process_inst(inst, &self.cur.func.dfg, self.liveness);

        // Advance to the next instruction so we can insert any spills after the instruction.
        self.cur.next_inst();

        // Rewrite register defs that need to be spilled.
        //
        // Change:
        //
        // v2 = inst ...
        //
        // Into:
        //
        // v7 = inst ...
        // v2 = spill v7
        //
        // That way, we don't need to rewrite all future uses of v2.
        for (lv, op) in defs.iter().zip(constraints.outs) {
            if lv.affinity.is_stack() && op.kind != ConstraintKind::Stack {
                let value_type = self.cur.func.dfg.value_type(lv.value);
                let reg = self.cur.func.dfg.replace_result(lv.value, value_type);
                self.liveness.create_dead(reg, inst, Affinity::new(op));
                self.insert_spill(ebb, lv.value, reg);
            }
        }

        // Same thing for spilled call return values.
        let retvals = &defs[constraints.outs.len()..];
        if !retvals.is_empty() {
            let sig = self.cur.func.dfg.call_signature(inst).expect(
                "Extra results on non-call instruction",
            );
            for (i, lv) in retvals.iter().enumerate() {
                let abi = self.cur.func.dfg.signatures[sig].return_types[i];
                debug_assert!(abi.location.is_reg());
                if lv.affinity.is_stack() {
                    let reg = self.cur.func.dfg.replace_result(lv.value, abi.value_type);
                    self.liveness.create_dead(
                        reg,
                        inst,
                        Affinity::abi(&abi, self.cur.isa),
                    );
                    self.insert_spill(ebb, lv.value, reg);
                }
            }
        }
    }

    // Find reload candidates for `inst` and add them to `self.condidates`.
    //
    // These are uses of spilled values where the operand constraint requires a register.
    fn find_candidates(&mut self, inst: Inst, constraints: &RecipeConstraints) {
        let args = self.cur.func.dfg.inst_args(inst);

        for (op, &arg) in constraints.ins.iter().zip(args) {
            if op.kind != ConstraintKind::Stack {
                if self.liveness[arg].affinity.is_stack() {
                    self.candidates.push(ReloadCandidate {
                        value: arg,
                        regclass: op.regclass,
                    })
                }
            }
        }

        // If we only have the fixed arguments, we're done now.
        if args.len() == constraints.ins.len() {
            return;
        }
        let var_args = &args[constraints.ins.len()..];

        // Handle ABI arguments.
        if let Some(sig) = self.cur.func.dfg.call_signature(inst) {
            handle_abi_args(
                self.candidates,
                &self.cur.func.dfg.signatures[sig].argument_types,
                var_args,
                self.cur.isa,
                self.liveness,
            );
        } else if self.cur.func.dfg[inst].opcode().is_return() {
            handle_abi_args(
                self.candidates,
                &self.cur.func.signature.return_types,
                var_args,
                self.cur.isa,
                self.liveness,
            );
        }
    }

    /// Insert a spill at `pos` and update data structures.
    ///
    /// - Insert `stack = spill reg` at `pos`, and assign an encoding.
    /// - Move the `stack` live range starting point to the new instruction.
    /// - Extend the `reg` live range to reach the new instruction.
    fn insert_spill(&mut self, ebb: Ebb, stack: Value, reg: Value) {
        self.cur.ins().with_result(stack).spill(reg);
        let inst = self.cur.built_inst();

        // Update live ranges.
        self.liveness.move_def_locally(stack, inst);
        self.liveness.extend_locally(
            reg,
            ebb,
            inst,
            &self.cur.func.layout,
        );
    }
}

/// Find reload candidates in the instruction's ABI variable arguments. This handles both
/// return values and call arguments.
fn handle_abi_args(
    candidates: &mut Vec<ReloadCandidate>,
    abi_types: &[ArgumentType],
    var_args: &[Value],
    isa: &TargetIsa,
    liveness: &Liveness,
) {
    assert_eq!(abi_types.len(), var_args.len());
    for (abi, &arg) in abi_types.iter().zip(var_args) {
        if abi.location.is_reg() {
            let lv = liveness.get(arg).expect("Missing live range for ABI arg");
            if lv.affinity.is_stack() {
                candidates.push(ReloadCandidate {
                    value: arg,
                    regclass: isa.regclass_for_abi_type(abi.value_type),
                });
            }
        }
    }
}
