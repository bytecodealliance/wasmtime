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
//! 5. The code must be in conventional SSA form. Among other things, this means that values passed
//!    as arguments when branching to an EBB must belong to the same virtual register as the
//!    corresponding EBB argument value.
//!
//! # Iteration order
//!
//! The SSA property guarantees that whenever the live range of two values overlap, one of the
//! values will be live at the definition point of the other value. If we visit the instructions in
//! a topological order relative to the dominance relation, we can assign colors to the values
//! defined by the instruction and only consider the colors of other values that are live at the
//! instruction.
//!
//! The first time we see a branch to an EBB, the EBB's argument values are colored to match the
//! registers currently holding branch argument values passed to the predecessor branch. By
//! visiting EBBs in a CFG topological order, we guarantee that at least one predecessor branch has
//! been visited before the destination EBB. Therefore, the EBB's arguments are already colored.
//!
//! The exception is the entry block whose arguments are colored from the ABI requirements.

use dominator_tree::DominatorTree;
use ir::{Ebb, Inst, Value, Function, Cursor, ValueLoc, DataFlowGraph, Layout, ValueLocations};
use ir::{InstBuilder, Signature, ArgumentType, ArgumentLoc};
use isa::{RegUnit, RegClass, RegInfo, regs_overlap};
use isa::{TargetIsa, EncInfo, RecipeConstraints, OperandConstraint, ConstraintKind};
use regalloc::RegDiversions;
use regalloc::affinity::Affinity;
use regalloc::allocatable_set::AllocatableSet;
use regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use regalloc::liveness::Liveness;
use regalloc::liverange::LiveRange;
use regalloc::solver::Solver;


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
               tracker: &mut LiveValueTracker) {
        dbg!("Coloring for:\n{}", func.display(isa));
        let mut ctx = Context {
            reginfo: isa.register_info(),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
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
        func.locations.resize(func.dfg.num_values());

        // Visit blocks in reverse post-order. We need to ensure that at least one predecessor has
        // been visited before each EBB. That guarantees that the EBB arguments have been colored.
        for &ebb in self.domtree.cfg_postorder().iter().rev() {
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
            if let Some(constraints) = self.encinfo.operand_constraints(func.encodings[inst]) {
                self.visit_inst(inst,
                                constraints,
                                &mut pos,
                                &mut func.dfg,
                                tracker,
                                &mut regs,
                                &mut func.locations,
                                &func.signature);
            } else {
                let (_throughs, kills) = tracker.process_ghost(inst);
                self.process_ghost_kills(kills, &mut regs, &func.locations);
            }
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
        tracker.ebb_top(ebb, &func.dfg, self.liveness, &func.layout, self.domtree);

        if func.layout.entry_block() == Some(ebb) {
            // Arguments to the entry block have ABI constraints.
            self.color_entry_args(&func.signature, tracker.live(), &mut func.locations)
        } else {
            // The live-ins and arguments to a non-entry EBB have already been assigned a register.
            // Reconstruct the allocatable set.
            self.livein_regs(tracker.live(), func)
        }
    }

    /// Initialize a set of allocatable registers from the values that are live-in to a block.
    /// These values must already be colored when the dominating blocks were processed.
    ///
    /// Also process the EBB arguments which were colored when the first predecessor branch was
    /// encountered.
    fn livein_regs(&self, live: &[LiveValue], func: &Function) -> AllocatableSet {
        // Start from the registers that are actually usable. We don't want to include any reserved
        // registers in the set.
        let mut regs = self.usable_regs.clone();

        for lv in live.iter().filter(|lv| !lv.is_dead) {
            let value = lv.value;
            let affinity = self.liveness
                .get(value)
                .expect("No live range for live-in")
                .affinity;
            dbg!("Live-in: {}:{} in {}",
                 value,
                 affinity.display(&self.reginfo),
                 func.locations[value].display(&self.reginfo));
            if let Affinity::Reg(rci) = affinity {
                let rc = self.reginfo.rc(rci);
                let loc = func.locations[value];
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
                        locations: &mut ValueLocations)
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
                        locations[lv.value] = ValueLoc::Reg(reg);
                    } else {
                        // This should have been fixed by the reload pass.
                        panic!("Entry arg {} has {} affinity, but ABI {}",
                               lv.value,
                               lv.affinity.display(&self.reginfo),
                               abi.display(&self.reginfo));
                    }

                }
                // The spiller will have assigned an incoming stack slot already.
                Affinity::Stack => assert!(abi.location.is_stack()),
                // This is a ghost value, unused in the function. Don't assign it to a location
                // either.
                Affinity::None => {}
            }
        }

        regs
    }

    /// Color the values defined by `inst` and insert any necessary shuffle code to satisfy
    /// instruction constraints.
    ///
    /// Update `regs` to reflect the allocated registers after `inst`, including removing any dead
    /// or killed values from the set.
    fn visit_inst(&mut self,
                  inst: Inst,
                  constraints: &RecipeConstraints,
                  pos: &mut Cursor,
                  dfg: &mut DataFlowGraph,
                  tracker: &mut LiveValueTracker,
                  regs: &mut AllocatableSet,
                  locations: &mut ValueLocations,
                  func_signature: &Signature) {
        dbg!("Coloring {}", dfg.display_inst(inst));

        // EBB whose arguments should be colored to match the current branch instruction's
        // arguments.
        let mut color_dest_args = None;

        // Program the solver with register constraints for the input side.
        self.solver.reset(regs);
        self.program_input_constraints(inst, constraints.ins, dfg, locations);
        let call_sig = dfg.call_signature(inst);
        if let Some(sig) = call_sig {
            self.program_input_abi(inst, &dfg.signatures[sig].argument_types, dfg, locations);
        } else if dfg[inst].opcode().is_return() {
            self.program_input_abi(inst, &func_signature.return_types, dfg, locations);
        } else if dfg[inst].opcode().is_branch() {
            // This is a branch, so we need to make sure that globally live values are in their
            // global registers. For EBBs that take arguments, we also need to place the argument
            // values in the expected registers.
            if let Some(dest) = dfg[inst].branch_destination() {
                if self.program_ebb_arguments(inst, dest, dfg, pos.layout, locations) {
                    color_dest_args = Some(dest);
                }
            } else {
                // This is a multi-way branch like `br_table`. We only support arguments on
                // single-destination branches.
                assert_eq!(dfg.inst_variable_args(inst).len(),
                           0,
                           "Can't handle EBB arguments: {}",
                           dfg.display_inst(inst));
                self.undivert_regs(|lr| !lr.is_local());
            }
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

        // If this is the first time we branch to `dest`, color its arguments to match the current
        // register state.
        if let Some(dest) = color_dest_args {
            self.color_ebb_arguments(inst, dest, dfg, locations);
        }

        // Apply the solution to the defs.
        for v in self.solver.vars().iter().filter(|&v| v.is_define()) {
            locations[v.value] = ValueLoc::Reg(v.solution);
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

        self.forget_diverted(kills);

        *regs = output_regs;
    }

    /// Program the input-side constraints for `inst` into the constraint solver.
    fn program_input_constraints(&mut self,
                                 inst: Inst,
                                 constraints: &[OperandConstraint],
                                 dfg: &DataFlowGraph,
                                 locations: &ValueLocations) {
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
                         locations: &ValueLocations) {
        for (abi, &value) in abi_types.iter().zip(dfg.inst_variable_args(inst)) {
            if let ArgumentLoc::Reg(reg) = abi.location {
                if let Affinity::Reg(rci) =
                    self.liveness
                        .get(value)
                        .expect("ABI register must have live range")
                        .affinity {
                    let rc = self.reginfo.rc(rci);
                    let cur_reg = self.divert.reg(value, locations);
                    self.solver.reassign_in(value, rc, cur_reg, reg);
                } else {
                    panic!("ABI argument {} should be in a register", value);
                }
            }
        }
    }

    /// Prepare for a branch to `dest`.
    ///
    /// 1. Any values that are live-in to `dest` must be un-diverted so they live in their globally
    ///    assigned register.
    /// 2. If the `dest` EBB takes arguments, reassign the branch argument values to the matching
    ///    registers.
    ///
    /// Returns true if this is the first time a branch to `dest` is seen, so the `dest` argument
    /// values should be colored after `shuffle_inputs`.
    fn program_ebb_arguments(&mut self,
                             inst: Inst,
                             dest: Ebb,
                             dfg: &DataFlowGraph,
                             layout: &Layout,
                             locations: &ValueLocations)
                             -> bool {
        // Find diverted registers that are live-in to `dest` and reassign them to their global
        // home.
        //
        // Values with a global live range that are not live in to `dest` could appear as branch
        // arguments, so they can't always be un-diverted.
        self.undivert_regs(|lr| lr.livein_local_end(dest, layout).is_some());

        // Now handle the EBB arguments.
        let br_args = dfg.inst_variable_args(inst);
        let dest_args = dfg.ebb_args(dest);
        assert_eq!(br_args.len(), dest_args.len());
        for (&dest_arg, &br_arg) in dest_args.iter().zip(br_args) {
            // The first time we encounter a branch to `dest`, we get to pick the location. The
            // following times we see a branch to `dest`, we must follow suit.
            match locations[dest_arg] {
                ValueLoc::Unassigned => {
                    // This is the first branch to `dest`, so we should color `dest_arg` instead of
                    // `br_arg`. However, we don't know where `br_arg` will end up until
                    // after `shuffle_inputs`. See `color_ebb_arguments` below.
                    //
                    // It is possible for `dest_arg` to have no affinity, and then it should simply
                    // be ignored.
                    if self.liveness[dest_arg].affinity.is_reg() {
                        return true;
                    }
                }
                ValueLoc::Reg(dest_reg) => {
                    // We've branched to `dest` before. Make sure we use the correct argument
                    // registers by reassigning `br_arg`.
                    if let Affinity::Reg(rci) = self.liveness[br_arg].affinity {
                        let rc = self.reginfo.rc(rci);
                        let br_reg = self.divert.reg(br_arg, locations);
                        self.solver.reassign_in(br_arg, rc, br_reg, dest_reg);
                    } else {
                        panic!("Branch argument {} is not in a register", br_arg);
                    }
                }
                ValueLoc::Stack(ss) => {
                    // The spiller should already have given us identical stack slots.
                    debug_assert_eq!(ValueLoc::Stack(ss), locations[br_arg]);
                }
            }
        }

        // No `dest` arguments need coloring.
        false
    }

    /// Knowing that we've never seen a branch to `dest` before, color its arguments to match our
    /// register state.
    ///
    /// This function is only called when `program_ebb_arguments()` returned `true`.
    fn color_ebb_arguments(&mut self,
                           inst: Inst,
                           dest: Ebb,
                           dfg: &DataFlowGraph,
                           locations: &mut ValueLocations) {
        let br_args = dfg.inst_variable_args(inst);
        let dest_args = dfg.ebb_args(dest);
        assert_eq!(br_args.len(), dest_args.len());
        for (&dest_arg, &br_arg) in dest_args.iter().zip(br_args) {
            match locations[dest_arg] {
                ValueLoc::Unassigned => {
                    if self.liveness[dest_arg].affinity.is_reg() {
                        let br_reg = self.divert.reg(br_arg, locations);
                        locations[dest_arg] = ValueLoc::Reg(br_reg);
                    }
                }
                ValueLoc::Reg(_) => panic!("{} arg {} already colored", dest, dest_arg),
                // Spilled value consistency is verified by `program_ebb_arguments()` above.
                ValueLoc::Stack(_) => {}
            }
        }
    }

    /// Find all diverted registers where `pred` returns `true` and undo their diversion so they
    /// are reallocated to their global register assignments.
    fn undivert_regs<Pred>(&mut self, mut pred: Pred)
        where Pred: FnMut(&LiveRange) -> bool
    {
        for rdiv in self.divert.all() {
            let lr = self.liveness
                .get(rdiv.value)
                .expect("Missing live range for diverted register");
            if pred(lr) {
                if let Affinity::Reg(rci) = lr.affinity {
                    let rc = self.reginfo.rc(rci);
                    self.solver.reassign_in(rdiv.value, rc, rdiv.to, rdiv.from);
                } else {
                    panic!("Diverted register {} with {} affinity",
                           rdiv.value,
                           lr.affinity.display(&self.reginfo));
                }
            }
        }
    }

    // Find existing live values that conflict with the fixed input register constraints programmed
    // into the constraint solver. Convert them to solver variables so they can be diverted.
    fn divert_fixed_input_conflicts(&mut self,
                                    live: &[LiveValue],
                                    locations: &mut ValueLocations) {
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
                             locations: &mut ValueLocations) {
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
                          locations: &mut ValueLocations) {
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
                        locations: &mut ValueLocations) {
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
        locations[value] = ValueLoc::Reg(reg);
    }

    /// Program the output-side constraints for `inst` into the constraint solver.
    ///
    /// It is assumed that all fixed outputs have already been handled.
    fn program_output_constraints(&mut self,
                                  _inst: Inst,
                                  constraints: &[OperandConstraint],
                                  defs: &[LiveValue],
                                  _dfg: &mut DataFlowGraph,
                                  _locations: &mut ValueLocations) {
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

    /// Forget about any register diversions in `kills`.
    fn forget_diverted(&mut self, kills: &[LiveValue]) {
        if self.divert.is_empty() {
            return;
        }

        for lv in kills {
            if lv.affinity.is_reg() {
                self.divert.remove(lv.value);
            }
        }
    }

    /// Process kills on a ghost instruction.
    /// - Forget diversions.
    /// - Free killed registers.
    fn process_ghost_kills(&mut self,
                           kills: &[LiveValue],
                           regs: &mut AllocatableSet,
                           locations: &ValueLocations) {
        for lv in kills {
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let reg = match self.divert.remove(lv.value) {
                    Some(r) => r,
                    None => locations[lv.value].unwrap_reg(),
                };
                regs.free(rc, reg);
            }
        }
    }
}
