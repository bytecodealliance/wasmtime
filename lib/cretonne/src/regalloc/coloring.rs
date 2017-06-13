//! Register allocator coloring pass.
//!
//! The coloring pass assigns a physical register to every SSA value with a register affinity,
//! under the assumption that the register pressure has been lowered sufficiently by spilling and
//! splitting.
//!
//! # Preconditions
//!
//! The coloring pass doesn't work on arbitrary code. Certain preconditions must be satisfied:
//!
//! 1. All instructions must be legalized and assigned an encoding. The encoding recipe guides the
//!    register assignments and provides exact constraints.
//!
//! 2. Instructions with tied operands must be in a coloring-friendly state. Specifically, the
//!    values used by the tied operands must be killed by the instruction. This can be achieved by
//!    inserting a `copy` to a new value immediately before the two-address instruction.
//!
//! 3. If a value is bound to more than one operand on the same instruction, the operand
//!    constraints must be compatible. This can also be achieved by inserting copies so the
//!    incompatible operands get different values.
//!
//! 4. The register pressure must be lowered sufficiently by inserting spill code. Register
//!    operands are allowed to read spilled values, but each such instance must be counted as using
//!    a register.
//!
//! # Iteration order
//!
//! The SSA property guarantees that whenever the live range of two values overlap, one of the
//! values will be live at the definition point of the other value. If we visit the instructions in
//! a topological order relative to the dominance relation, we can assign colors to the values
//! defined by the instruction and only consider the colors of other values that are live at the
//! instruction.

use entity_map::EntityMap;
use dominator_tree::DominatorTree;
use ir::{Ebb, Inst, Value, Function, Cursor, ValueLoc, DataFlowGraph};
use ir::{InstBuilder, Signature, ArgumentType, ArgumentLoc};
use isa::{TargetIsa, Encoding, EncInfo, OperandConstraint, ConstraintKind};
use isa::{RegUnit, RegClass, RegInfo, regs_overlap};
use regalloc::affinity::Affinity;
use regalloc::allocatable_set::AllocatableSet;
use regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use regalloc::liveness::Liveness;
use regalloc::solver::Solver;
use regalloc::RegDiversions;
use topo_order::TopoOrder;


/// Data structures for the coloring pass.
///
/// These are scratch space data structures that can be reused between invocations.
pub struct Coloring {
    divert: RegDiversions,
    solver: Solver,
}

/// Bundle of references that the coloring algorithm needs.
///
/// Some of the needed mutable references are passed around as explicit function arguments so we
/// can avoid many fights with the borrow checker over mutable borrows of `self`. This includes the
/// `Function` and `LiveValueTracker` references.
///
/// Immutable context information and mutable references that don't need to be borrowed across
/// method calls should go in this struct.
struct Context<'a> {
    // Cached ISA information.
    // We save it here to avoid frequent virtual function calls on the `TargetIsa` trait object.
    reginfo: RegInfo,
    encinfo: EncInfo,

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,

    // References to working set data structures.
    // If we need to borrow out of a data structure across a method call, it must be passed as a
    // function argument instead, see the `LiveValueTracker` arguments.
    topo: &'a mut TopoOrder,
    divert: &'a mut RegDiversions,
    solver: &'a mut Solver,

    // Pristine set of registers that the allocator can use.
    // This set remains immutable, we make clones.
    usable_regs: AllocatableSet,
}

impl Coloring {
    /// Allocate scratch space data structures for the coloring pass.
    pub fn new() -> Coloring {
        Coloring {
            divert: RegDiversions::new(),
            solver: Solver::new(),
        }
    }

    /// Run the coloring algorithm over `func`.
    pub fn run(&mut self,
               isa: &TargetIsa,
               func: &mut Function,
               domtree: &DominatorTree,
               liveness: &mut Liveness,
               topo: &mut TopoOrder,
               tracker: &mut LiveValueTracker) {
        dbg!("Coloring for:\n{}", func.display(isa));
        let mut ctx = Context {
            reginfo: isa.register_info(),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
            topo,
            divert: &mut self.divert,
            solver: &mut self.solver,
            usable_regs: isa.allocatable_registers(func),
        };
        ctx.run(func, tracker)
    }
}

impl<'a> Context<'a> {
    /// Run the coloring algorithm.
    fn run(&mut self, func: &mut Function, tracker: &mut LiveValueTracker) {
        // Just visit blocks in layout order, letting `self.topo` enforce a topological ordering.
        // TODO: Once we have a loop tree, we could visit hot blocks first.
        self.topo.reset(func.layout.ebbs());
        while let Some(ebb) = self.topo.next(&func.layout, self.domtree) {
            self.visit_ebb(ebb, func, tracker);
        }
    }

    /// Visit `ebb`, assuming that the immediate dominator has already been visited.
    fn visit_ebb(&mut self, ebb: Ebb, func: &mut Function, tracker: &mut LiveValueTracker) {
        dbg!("Coloring {}:", ebb);
        let mut regs = self.visit_ebb_header(ebb, func, tracker);
        tracker.drop_dead_args();
        self.divert.clear();

        // Now go through the instructions in `ebb` and color the values they define.
        let mut pos = Cursor::new(&mut func.layout);
        pos.goto_top(ebb);
        while let Some(inst) = pos.next_inst() {
            let encoding = func.encodings[inst];
            assert!(encoding.is_legal(), "Illegal: {}", func.dfg[inst].opcode());
            self.visit_inst(inst,
                            encoding,
                            &mut pos,
                            &mut func.dfg,
                            tracker,
                            &mut regs,
                            &mut func.locations,
                            &func.signature);
            tracker.drop_dead(inst);
        }

    }

    /// Visit the `ebb` header.
    ///
    /// Initialize the set of live registers and color the arguments to `ebb`.
    fn visit_ebb_header(&self,
                        ebb: Ebb,
                        func: &mut Function,
                        tracker: &mut LiveValueTracker)
                        -> AllocatableSet {
        // Reposition the live value tracker and deal with the EBB arguments.
        let (liveins, args) =
            tracker.ebb_top(ebb, &func.dfg, self.liveness, &func.layout, self.domtree);

        // Arguments to the entry block have ABI constraints.
        if func.layout.entry_block() == Some(ebb) {
            assert_eq!(liveins.len(), 0);
            self.color_entry_args(&func.signature, args, &mut func.locations)
        } else {
            // The live-ins have already been assigned a register. Reconstruct the allocatable set.
            let regs = self.livein_regs(liveins, func);
            self.color_args(args, regs, &mut func.locations)
        }
    }

    /// Initialize a set of allocatable registers from the values that are live-in to a block.
    /// These values must already be colored when the dominating blocks were processed.
    fn livein_regs(&self, liveins: &[LiveValue], func: &Function) -> AllocatableSet {
        // Start from the registers that are actually usable. We don't want to include any reserved
        // registers in the set.
        let mut regs = self.usable_regs.clone();

        for lv in liveins {
            let value = lv.value;
            let affinity = self.liveness
                .get(value)
                .expect("No live range for live-in")
                .affinity;
            if let Affinity::Reg(rci) = affinity {
                let rc = self.reginfo.rc(rci);
                let loc = func.locations[value];
                dbg!("Live-in: {}:{} in {}",
                     lv.value,
                     rc,
                     loc.display(&self.reginfo));
                match loc {
                    ValueLoc::Reg(reg) => regs.take(rc, reg),
                    ValueLoc::Unassigned => panic!("Live-in {} wasn't assigned", value),
                    ValueLoc::Stack(ss) => {
                        panic!("Live-in {} is in {}, should be register", value, ss)
                    }
                }
            }
        }

        regs
    }

    /// Color the arguments to the entry block.
    ///
    /// These are function arguments that should already have assigned register units in the
    /// function signature.
    ///
    /// Return the set of remaining allocatable registers after filtering out the dead arguments.
    fn color_entry_args(&self,
                        sig: &Signature,
                        args: &[LiveValue],
                        locations: &mut EntityMap<Value, ValueLoc>)
                        -> AllocatableSet {
        assert_eq!(sig.argument_types.len(), args.len());

        let mut regs = self.usable_regs.clone();

        for (lv, abi) in args.iter().zip(&sig.argument_types) {
            match lv.affinity {
                Affinity::Reg(rci) => {
                    let rc = self.reginfo.rc(rci);
                    if let ArgumentLoc::Reg(reg) = abi.location {
                        if !lv.is_dead {
                            regs.take(rc, reg);
                        }
                        *locations.ensure(lv.value) = ValueLoc::Reg(reg);
                    } else {
                        // This should have been fixed by the reload pass.
                        panic!("Entry arg {} has {} affinity, but ABI {}",
                               lv.value,
                               lv.affinity.display(&self.reginfo),
                               abi.display(&self.reginfo));
                    }

                }
                Affinity::Stack => {
                    if let ArgumentLoc::Stack(_offset) = abi.location {
                        // TODO: Allocate a stack slot at incoming offset and assign it.
                        panic!("Unimplemented {}: {} stack allocation",
                               lv.value,
                               abi.display(&self.reginfo));
                    } else {
                        // This should have been fixed by the reload pass.
                        panic!("Entry arg {} has stack affinity, but ABI {}",
                               lv.value,
                               abi.display(&self.reginfo));
                    }
                }
                // This is a ghost value, unused in the function. Don't assign it to a location
                // either.
                Affinity::None => {}
            }
        }

        regs
    }

    /// Color the live arguments to the current block.
    ///
    /// It is assumed that any live-in register values have already been taken out of the register
    /// set.
    fn color_args(&self,
                  args: &[LiveValue],
                  mut regs: AllocatableSet,
                  locations: &mut EntityMap<Value, ValueLoc>)
                  -> AllocatableSet {
        // Available registers *after* filtering out the dead arguments.
        let mut live_regs = regs.clone();

        for lv in args {
            // Only look at the register arguments.
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                // TODO: Fall back to a top-level super-class. Sub-classes are only hints.
                let reg = regs.iter(rc)
                    .next()
                    .expect("Out of registers for arguments");
                regs.take(rc, reg);
                if !lv.is_dead {
                    live_regs.take(rc, reg);
                }
                *locations.ensure(lv.value) = ValueLoc::Reg(reg);
            }
        }

        // All arguments are accounted for in `regs`. We don't care about the dead arguments now
        // that we have made sure they don't interfere.
        live_regs
    }

    /// Color the values defined by `inst` and insert any necessary shuffle code to satisfy
    /// instruction constraints.
    ///
    /// Update `regs` to reflect the allocated registers after `inst`, including removing any dead
    /// or killed values from the set.
    fn visit_inst(&mut self,
                  inst: Inst,
                  encoding: Encoding,
                  pos: &mut Cursor,
                  dfg: &mut DataFlowGraph,
                  tracker: &mut LiveValueTracker,
                  regs: &mut AllocatableSet,
                  locations: &mut EntityMap<Value, ValueLoc>,
                  func_signature: &Signature) {
        dbg!("Coloring [{}] {}",
             self.encinfo.display(encoding),
             dfg.display_inst(inst));

        // Get the operand constraints for `inst` that we are trying to satisfy.
        let constraints = self.encinfo
            .operand_constraints(encoding)
            .expect("Missing instruction encoding");

        // Program the solver with register constraints for the input side.
        self.solver.reset(regs);
        self.program_input_constraints(inst, constraints.ins, dfg, locations);
        let call_sig = dfg.call_signature(inst);
        if let Some(sig) = call_sig {
            self.program_input_abi(inst, &dfg.signatures[sig].argument_types, dfg, locations);
        } else if dfg[inst].opcode().is_return() {
            self.program_input_abi(inst, &func_signature.return_types, dfg, locations);
        }
        if self.solver.has_fixed_input_conflicts() {
            self.divert_fixed_input_conflicts(tracker.live(), locations);
        }
        self.solver.inputs_done();

        // Update the live value tracker with this instruction.
        let (throughs, kills, defs) = tracker.process_inst(inst, dfg, self.liveness);

        // Get rid of the killed values.
        for lv in kills {
            if let Affinity::Reg(rci) = lv.affinity {
                self.solver
                    .add_kill(lv.value,
                              self.reginfo.rc(rci),
                              self.divert.reg(lv.value, locations));
            }
        }

        // Program the fixed output constraints before the general defines. This allows us to
        // detect conflicts between fixed outputs and tied operands where the input value hasn't
        // been converted to a solver variable.
        if constraints.fixed_outs {
            self.program_fixed_outputs(constraints.outs, defs, throughs, locations);
        }
        if let Some(sig) = call_sig {
            let abi = &dfg.signatures[sig].return_types;
            self.program_output_abi(abi, defs, throughs, locations);
        }
        self.program_output_constraints(inst, constraints.outs, defs, dfg, locations);

        // Finally, we've fully programmed the constraint solver.
        // We expect a quick solution in most cases.
        let mut output_regs = self.solver
            .quick_solve()
            .unwrap_or_else(|_| self.iterate_solution());


        // The solution and/or fixed input constraints may require us to shuffle the set of live
        // registers around.
        self.shuffle_inputs(pos, dfg, regs);

        // Apply the solution to the defs.
        for v in self.solver.vars().iter().filter(|&v| v.is_define()) {
            *locations.ensure(v.value) = ValueLoc::Reg(v.solution);
        }

        // Update `regs` for the next instruction, remove the dead defs.
        for lv in defs {
            if lv.endpoint == inst {
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc = self.reginfo.rc(rci);
                    let reg = self.divert.reg(lv.value, locations);
                    output_regs.free(rc, reg);
                }
            }
        }
        *regs = output_regs;
    }

    /// Program the input-side constraints for `inst` into the constraint solver.
    fn program_input_constraints(&mut self,
                                 inst: Inst,
                                 constraints: &[OperandConstraint],
                                 dfg: &DataFlowGraph,
                                 locations: &EntityMap<Value, ValueLoc>) {
        for (op, &value) in constraints
                .iter()
                .zip(dfg.inst_args(inst))
                .filter(|&(op, _)| op.kind != ConstraintKind::Stack) {
            // Reload pass is supposed to ensure that all arguments to register operands are
            // already in a register.
            let cur_reg = self.divert.reg(value, locations);
            match op.kind {
                ConstraintKind::FixedReg(regunit) => {
                    if regunit != cur_reg {
                        self.solver
                            .reassign_in(value, op.regclass, cur_reg, regunit);
                    }
                }
                ConstraintKind::Reg |
                ConstraintKind::Tied(_) => {
                    if !op.regclass.contains(cur_reg) {
                        self.solver
                            .add_var(value, op.regclass, cur_reg, &self.reginfo);
                    }
                }
                ConstraintKind::Stack => unreachable!(),
            }
        }
    }

    /// Program the input-side ABI constraints for `inst` into the constraint solver.
    ///
    /// ABI constraints are the fixed register assignments used for calls and returns.
    fn program_input_abi(&mut self,
                         inst: Inst,
                         abi_types: &[ArgumentType],
                         dfg: &DataFlowGraph,
                         locations: &EntityMap<Value, ValueLoc>) {
        for (abi, &value) in abi_types.iter().zip(dfg.inst_variable_args(inst)) {
            if let ArgumentLoc::Reg(reg) = abi.location {
                let cur_reg = self.divert.reg(value, locations);
                if reg != cur_reg {
                    if let Affinity::Reg(rci) =
                        self.liveness
                            .get(value)
                            .expect("ABI register must have live range")
                            .affinity {
                        let rc = self.reginfo.rc(rci);
                        self.solver.reassign_in(value, rc, cur_reg, reg);
                    } else {
                        panic!("ABI argument {} should be in a register", value);
                    }
                }
            }
        }
    }

    // Find existing live values that conflict with the fixed input register constraints programmed
    // into the constraint solver. Convert them to solver variables so they can be diverted.
    fn divert_fixed_input_conflicts(&mut self,
                                    live: &[LiveValue],
                                    locations: &mut EntityMap<Value, ValueLoc>) {
        for lv in live {
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let reg = self.divert.reg(lv.value, locations);
                if self.solver.is_fixed_input_conflict(rc, reg) {
                    self.solver.add_var(lv.value, rc, reg, &self.reginfo);
                }
            }
        }
    }

    /// Program any fixed-register output constraints into the solver. This may also detect
    /// conflicts between live-through registers and fixed output registers. These live-through
    /// values need to be turned into solver variables so they can be reassigned.
    fn program_fixed_outputs(&mut self,
                             constraints: &[OperandConstraint],
                             defs: &[LiveValue],
                             throughs: &[LiveValue],
                             locations: &mut EntityMap<Value, ValueLoc>) {
        for (op, lv) in constraints.iter().zip(defs) {
            if let ConstraintKind::FixedReg(reg) = op.kind {
                self.add_fixed_output(lv.value, op.regclass, reg, throughs, locations);
            }
        }
    }

    /// Program the output-side ABI constraints for `inst` into the constraint solver.
    ///
    /// That means return values for a call instruction.
    fn program_output_abi(&mut self,
                          abi_types: &[ArgumentType],
                          defs: &[LiveValue],
                          throughs: &[LiveValue],
                          locations: &mut EntityMap<Value, ValueLoc>) {
        // It's technically possible for a call instruction to have fixed results before the
        // variable list of results, but we have no known instances of that.
        // Just assume all results are variable return values.
        assert_eq!(defs.len(), abi_types.len());
        for (abi, lv) in abi_types.iter().zip(defs) {
            if let ArgumentLoc::Reg(reg) = abi.location {
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc = self.reginfo.rc(rci);
                    self.add_fixed_output(lv.value, rc, reg, throughs, locations);
                } else {
                    panic!("ABI argument {} should be in a register", lv.value);
                }
            }
        }
    }

    /// Add a single fixed output value to the solver.
    fn add_fixed_output(&mut self,
                        value: Value,
                        rc: RegClass,
                        reg: RegUnit,
                        throughs: &[LiveValue],
                        locations: &mut EntityMap<Value, ValueLoc>) {
        if !self.solver.add_fixed_output(rc, reg) {
            // The fixed output conflicts with some of the live-through registers.
            for lv in throughs {
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc2 = self.reginfo.rc(rci);
                    let reg2 = self.divert.reg(lv.value, locations);
                    if regs_overlap(rc, reg, rc2, reg2) {
                        // This live-through value is interfering with the fixed output assignment.
                        // Convert it to a solver variable.
                        // TODO: Use a looser constraint than the affinity hint. Any allocatable
                        // register in the top-level register class would be OK. Maybe `add_var`
                        // should take both a preferred class and a required constraint class.
                        self.solver.add_var(lv.value, rc2, reg2, &self.reginfo);
                    }
                }
            }

            let ok = self.solver.add_fixed_output(rc, reg);
            assert!(ok, "Couldn't clear fixed output interference for {}", value);
        }
        *locations.ensure(value) = ValueLoc::Reg(reg);
    }

    /// Program the output-side constraints for `inst` into the constraint solver.
    ///
    /// It is assumed that all fixed outputs have already been handled.
    fn program_output_constraints(&mut self,
                                  _inst: Inst,
                                  constraints: &[OperandConstraint],
                                  defs: &[LiveValue],
                                  _dfg: &mut DataFlowGraph,
                                  _locations: &mut EntityMap<Value, ValueLoc>) {
        for (op, lv) in constraints.iter().zip(defs) {
            match op.kind {
                ConstraintKind::FixedReg(_) |
                ConstraintKind::Stack => continue,
                ConstraintKind::Reg => {
                    self.solver.add_def(lv.value, op.regclass);
                }
                ConstraintKind::Tied(_) => unimplemented!(),
            }
        }
    }

    /// Try harder to find a solution to the constraint problem since `quick_solve()` failed.
    ///
    /// We may need to move more registers around before a solution is possible. Use an iterative
    /// algorithm that adds one more variable until a solution can be found.
    fn iterate_solution(&self) -> AllocatableSet {
        unimplemented!();
    }

    /// Emit `regmove` instructions as needed to move the live registers into place before the
    /// instruction. Also update `self.divert` accordingly.
    ///
    /// The `pos` cursor is expected to point at the instruction. The register moves are inserted
    /// before.
    ///
    /// The solver needs to be reminded of the available registers before any moves are inserted.
    fn shuffle_inputs(&mut self,
                      pos: &mut Cursor,
                      dfg: &mut DataFlowGraph,
                      regs: &mut AllocatableSet) {
        self.solver.schedule_moves(regs);

        for m in self.solver.moves() {
            self.divert.regmove(m.value, m.from, m.to);
            dfg.ins(pos).regmove(m.value, m.from, m.to);
        }
    }
}
