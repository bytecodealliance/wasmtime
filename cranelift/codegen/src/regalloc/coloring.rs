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
//! 5. The code must be in Conventional SSA form. Among other things, this means that values passed
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

use crate::cursor::{Cursor, EncCursor};
use crate::dominator_tree::DominatorTree;
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{ArgumentLoc, InstBuilder, ValueDef};
use crate::ir::{Ebb, Function, Inst, InstructionData, Layout, Opcode, SigRef, Value, ValueLoc};
use crate::isa::{regs_overlap, RegClass, RegInfo, RegUnit};
use crate::isa::{ConstraintKind, EncInfo, OperandConstraint, RecipeConstraints, TargetIsa};
use crate::packed_option::PackedOption;
use crate::regalloc::affinity::Affinity;
use crate::regalloc::diversion::RegDiversions;
use crate::regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use crate::regalloc::liveness::Liveness;
use crate::regalloc::liverange::LiveRange;
use crate::regalloc::register_set::RegisterSet;
use crate::regalloc::solver::{Solver, SolverError};
use crate::timing;
use core::mem;
use log::debug;

/// Data structures for the coloring pass.
///
/// These are scratch space data structures that can be reused between invocations.
pub struct Coloring {
    divert: RegDiversions,
    solver: Solver,
}

/// Kinds of ABI parameters.
enum AbiParams {
    Parameters(SigRef),
    Returns,
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
    // Current instruction as well as reference to function and ISA.
    cur: EncCursor<'a>,

    // Cached ISA information.
    // We save it here to avoid frequent virtual function calls on the `TargetIsa` trait object.
    reginfo: RegInfo,
    encinfo: EncInfo,

    // References to contextual data structures we need.
    cfg: &'a ControlFlowGraph,
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,

    // References to working set data structures.
    // If we need to borrow out of a data structure across a method call, it must be passed as a
    // function argument instead, see the `LiveValueTracker` arguments.
    divert: &'a mut RegDiversions,
    solver: &'a mut Solver,

    // Pristine set of registers that the allocator can use.
    // This set remains immutable, we make clones.
    usable_regs: RegisterSet,

    uses_pinned_reg: bool,
}

impl Coloring {
    /// Allocate scratch space data structures for the coloring pass.
    pub fn new() -> Self {
        Self {
            divert: RegDiversions::new(),
            solver: Solver::new(),
        }
    }

    /// Clear all data structures in this coloring pass.
    pub fn clear(&mut self) {
        self.divert.clear();
        self.solver.clear();
    }

    /// Run the coloring algorithm over `func`.
    pub fn run(
        &mut self,
        isa: &dyn TargetIsa,
        func: &mut Function,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
        liveness: &mut Liveness,
        tracker: &mut LiveValueTracker,
    ) {
        let _tt = timing::ra_coloring();
        debug!("Coloring for:\n{}", func.display(isa));
        let mut ctx = Context {
            usable_regs: isa.allocatable_registers(func),
            uses_pinned_reg: isa.flags().enable_pinned_reg(),
            cur: EncCursor::new(func, isa),
            reginfo: isa.register_info(),
            encinfo: isa.encoding_info(),
            cfg,
            domtree,
            liveness,
            divert: &mut self.divert,
            solver: &mut self.solver,
        };
        ctx.run(tracker)
    }
}

impl<'a> Context<'a> {
    /// Is the pinned register usage enabled, and is this register the pinned register?
    #[inline]
    fn is_pinned_reg(&self, rc: RegClass, reg: RegUnit) -> bool {
        rc.is_pinned_reg(self.uses_pinned_reg, reg)
    }

    /// Run the coloring algorithm.
    fn run(&mut self, tracker: &mut LiveValueTracker) {
        self.cur
            .func
            .locations
            .resize(self.cur.func.dfg.num_values());

        // Visit blocks in reverse post-order. We need to ensure that at least one predecessor has
        // been visited before each EBB. That guarantees that the EBB arguments have been colored.
        for &ebb in self.domtree.cfg_postorder().iter().rev() {
            self.visit_ebb(ebb, tracker);
        }
    }

    /// Visit `ebb`, assuming that the immediate dominator has already been visited.
    fn visit_ebb(&mut self, ebb: Ebb, tracker: &mut LiveValueTracker) {
        debug!("Coloring {}:", ebb);
        let mut regs = self.visit_ebb_header(ebb, tracker);
        tracker.drop_dead_params();

        // Now go through the instructions in `ebb` and color the values they define.
        self.cur.goto_top(ebb);
        while let Some(inst) = self.cur.next_inst() {
            self.cur.use_srcloc(inst);
            let opcode = self.cur.func.dfg[inst].opcode();
            if !opcode.is_ghost() {
                // This is an instruction which either has an encoding or carries ABI-related
                // register allocation constraints.
                let enc = self.cur.func.encodings[inst];
                let constraints = self.encinfo.operand_constraints(enc);
                if self.visit_inst(inst, constraints, tracker, &mut regs) {
                    self.replace_global_defines(inst, tracker);
                    // Restore cursor location after `replace_global_defines` moves it.
                    // We want to revisit the copy instructions it inserted.
                    self.cur.goto_inst(inst);
                }
            } else {
                // This is a ghost instruction with no encoding and no extra constraints.
                let (_throughs, kills) = tracker.process_ghost(inst);
                self.process_ghost_kills(kills, &mut regs);
            }
            tracker.drop_dead(inst);

            // We are not able to insert any regmove for diversion or un-diversion after the first
            // branch. Instead, we record the diversion to be restored at the entry of the next EBB,
            // which should have a single predecessor.
            if opcode.is_branch() {
                // The next instruction is necessarily an unconditional branch.
                if let Some(branch) = self.cur.next_inst() {
                    debug!(
                        "Skip coloring {}\n    from {}\n    with diversions {}",
                        self.cur.display_inst(branch),
                        regs.input.display(&self.reginfo),
                        self.divert.display(&self.reginfo)
                    );
                    use crate::ir::instructions::BranchInfo::*;
                    let target = match self.cur.func.dfg.analyze_branch(branch) {
                        NotABranch | Table(_, _) => panic!(
                            "unexpected instruction {} after a conditional branch",
                            self.cur.display_inst(branch)
                        ),
                        SingleDest(ebb, _) => ebb,
                    };

                    // We have a single branch with a single target, and an EBB with a single
                    // predecessor. Thus we can forward the diversion set to the next EBB.
                    if self.cfg.pred_iter(target).count() == 1 {
                        // Transfer the diversion to the next EBB.
                        self.divert
                            .save_for_ebb(&mut self.cur.func.entry_diversions, target);
                        debug!(
                            "Set entry-diversion for {} to\n      {}",
                            target,
                            self.divert.display(&self.reginfo)
                        );
                    } else {
                        debug_assert!(
                            self.divert.is_empty(),
                            "Divert set is non-empty after the terminator."
                        );
                    }
                    assert_eq!(
                        self.cur.next_inst(),
                        None,
                        "Unexpected instruction after a branch group."
                    );
                } else {
                    assert!(opcode.is_terminator());
                }
            }
        }
    }

    /// Visit the `ebb` header.
    ///
    /// Initialize the set of live registers and color the arguments to `ebb`.
    fn visit_ebb_header(&mut self, ebb: Ebb, tracker: &mut LiveValueTracker) -> AvailableRegs {
        // Reposition the live value tracker and deal with the EBB arguments.
        tracker.ebb_top(
            ebb,
            &self.cur.func.dfg,
            self.liveness,
            &self.cur.func.layout,
            self.domtree,
        );

        // Copy the content of the registered diversions to be reused at the
        // entry of this basic block.
        self.divert.at_ebb(&self.cur.func.entry_diversions, ebb);
        debug!(
            "Start {} with entry-diversion set to\n      {}",
            ebb,
            self.divert.display(&self.reginfo)
        );

        if self.cur.func.layout.entry_block() == Some(ebb) {
            // Parameters on the entry block have ABI constraints.
            self.color_entry_params(tracker.live())
        } else {
            // The live-ins and parameters of a non-entry EBB have already been assigned a register.
            // Reconstruct the allocatable set.
            self.livein_regs(tracker.live())
        }
    }

    /// Initialize a set of allocatable registers from the values that are live-in to a block.
    /// These values must already be colored when the dominating blocks were processed.
    ///
    /// Also process the EBB arguments which were colored when the first predecessor branch was
    /// encountered.
    fn livein_regs(&self, live: &[LiveValue]) -> AvailableRegs {
        // Start from the registers that are actually usable. We don't want to include any reserved
        // registers in the set.
        let mut regs = AvailableRegs::new(&self.usable_regs);

        for lv in live.iter().filter(|lv| !lv.is_dead) {
            debug!(
                "Live-in: {}:{} in {}",
                lv.value,
                lv.affinity.display(&self.reginfo),
                self.divert
                    .get(lv.value, &self.cur.func.locations)
                    .display(&self.reginfo)
            );
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let loc = self.cur.func.locations[lv.value];
                let reg = match loc {
                    ValueLoc::Reg(reg) => reg,
                    ValueLoc::Unassigned => panic!("Live-in {} wasn't assigned", lv.value),
                    ValueLoc::Stack(ss) => {
                        panic!("Live-in {} is in {}, should be register", lv.value, ss)
                    }
                };
                if lv.is_local {
                    regs.take(rc, reg, lv.is_local);
                } else {
                    let loc = self.divert.get(lv.value, &self.cur.func.locations);
                    let reg_divert = match loc {
                        ValueLoc::Reg(reg) => reg,
                        ValueLoc::Unassigned => {
                            panic!("Diversion: Live-in {} wasn't assigned", lv.value)
                        }
                        ValueLoc::Stack(ss) => panic!(
                            "Diversion: Live-in {} is in {}, should be register",
                            lv.value, ss
                        ),
                    };
                    regs.take_divert(rc, reg, reg_divert);
                }
            }
        }

        regs
    }

    /// Color the parameters on the entry block.
    ///
    /// These are function parameters that should already have assigned register units in the
    /// function signature.
    ///
    /// Return the set of remaining allocatable registers after filtering out the dead arguments.
    fn color_entry_params(&mut self, args: &[LiveValue]) -> AvailableRegs {
        let sig = &self.cur.func.signature;
        debug_assert_eq!(sig.params.len(), args.len());

        let mut regs = AvailableRegs::new(&self.usable_regs);

        for (lv, abi) in args.iter().zip(&sig.params) {
            match lv.affinity {
                Affinity::Reg(rci) => {
                    let rc = self.reginfo.rc(rci);
                    if let ArgumentLoc::Reg(reg) = abi.location {
                        if !lv.is_dead {
                            regs.take(rc, reg, lv.is_local);
                        }
                        self.cur.func.locations[lv.value] = ValueLoc::Reg(reg);
                    } else {
                        // This should have been fixed by the reload pass.
                        panic!(
                            "Entry arg {} has {} affinity, but ABI {}",
                            lv.value,
                            lv.affinity.display(&self.reginfo),
                            abi.display(&self.reginfo)
                        );
                    }
                }
                // The spiller will have assigned an incoming stack slot already.
                Affinity::Stack => debug_assert!(abi.location.is_stack()),
                // This is a ghost value, unused in the function. Don't assign it to a location
                // either.
                Affinity::Unassigned => {}
            }
        }

        regs
    }

    /// Program the input-side ABI constraints for `inst` into the constraint solver.
    ///
    /// ABI constraints are the fixed register assignments useds for calls and returns.
    fn program_input_abi(&mut self, inst: Inst, abi_params: AbiParams) {
        let abi_types = match abi_params {
            AbiParams::Parameters(sig) => &self.cur.func.dfg.signatures[sig].params,
            AbiParams::Returns => &self.cur.func.signature.returns,
        };

        for (abi, &value) in abi_types
            .iter()
            .zip(self.cur.func.dfg.inst_variable_args(inst))
        {
            if let ArgumentLoc::Reg(reg) = abi.location {
                if let Affinity::Reg(rci) = self
                    .liveness
                    .get(value)
                    .expect("ABI register must have live range")
                    .affinity
                {
                    let rc = self.reginfo.rc(rci);
                    let cur_reg = self.divert.reg(value, &self.cur.func.locations);
                    self.solver.reassign_in(value, rc, cur_reg, reg);
                } else {
                    panic!("ABI argument {} should be in a register", value);
                }
            }
        }
    }

    /// Color the values defined by `inst` and insert any necessary shuffle code to satisfy
    /// instruction constraints.
    ///
    /// Update `regs` to reflect the allocated registers after `inst`, including removing any dead
    /// or killed values from the set.
    ///
    /// Returns true when the global values defined by `inst` must be replaced by local values.
    fn visit_inst(
        &mut self,
        inst: Inst,
        constraints: Option<&RecipeConstraints>,
        tracker: &mut LiveValueTracker,
        regs: &mut AvailableRegs,
    ) -> bool {
        debug!(
            "Coloring {}\n    from {}",
            self.cur.display_inst(inst),
            regs.input.display(&self.reginfo),
        );

        // EBB whose arguments should be colored to match the current branch instruction's
        // arguments.
        let mut color_dest_args = None;

        // Program the solver with register constraints for the input side.
        self.solver.reset(&regs.input);

        if let Some(constraints) = constraints {
            self.program_input_constraints(inst, constraints.ins);
        }

        let call_sig = self.cur.func.dfg.call_signature(inst);
        if let Some(sig) = call_sig {
            self.program_input_abi(inst, AbiParams::Parameters(sig));
        } else if self.cur.func.dfg[inst].opcode().is_return() {
            self.program_input_abi(inst, AbiParams::Returns);
        } else if self.cur.func.dfg[inst].opcode().is_branch() {
            // This is a branch, so we need to make sure that globally live values are in their
            // global registers. For EBBs that take arguments, we also need to place the argument
            // values in the expected registers.
            if let Some(dest) = self.cur.func.dfg[inst].branch_destination() {
                if self.program_ebb_arguments(inst, dest) {
                    color_dest_args = Some(dest);
                }
            } else {
                // This is a multi-way branch like `br_table`. We only support arguments on
                // single-destination branches.
                debug_assert_eq!(
                    self.cur.func.dfg.inst_variable_args(inst).len(),
                    0,
                    "Can't handle EBB arguments: {}",
                    self.cur.display_inst(inst)
                );
                self.undivert_regs(|lr, _| !lr.is_local());
            }
        }

        if self.solver.has_fixed_input_conflicts() {
            self.divert_fixed_input_conflicts(tracker.live());
        }

        self.solver.inputs_done();

        // Update the live value tracker with this instruction.
        let (throughs, kills, defs) = tracker.process_inst(inst, &self.cur.func.dfg, self.liveness);

        // Get rid of the killed values.
        for lv in kills {
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let reg = self.divert.reg(lv.value, &self.cur.func.locations);

                if self.is_pinned_reg(rc, reg) {
                    // Don't kill the pinned reg, either in the local or global register sets.
                    debug_assert!(lv.is_local, "pinned register SSA value can't be global");
                    continue;
                }

                debug!(
                    "    kill {} in {} ({} {})",
                    lv.value,
                    self.reginfo.display_regunit(reg),
                    if lv.is_local { "local" } else { "global" },
                    rc
                );
                self.solver.add_kill(lv.value, rc, reg);

                // Update the global register set which has no diversions.
                if !lv.is_local {
                    regs.global
                        .free(rc, self.cur.func.locations[lv.value].unwrap_reg());
                }
            }
        }

        // This aligns with the "    from" line at the top of the function.
        debug!("    glob {}", regs.global.display(&self.reginfo));

        // This flag is set when the solver failed to find a solution for the global defines that
        // doesn't interfere with `regs.global`. We need to rewrite all of `inst`s global defines
        // as local defines followed by copies.
        let mut replace_global_defines = false;

        // Program the fixed output constraints before the general defines. This allows us to
        // detect conflicts between fixed outputs and tied operands where the input value hasn't
        // been converted to a solver variable.
        if let Some(constraints) = constraints {
            if constraints.fixed_outs {
                self.program_fixed_outputs(
                    constraints.outs,
                    defs,
                    throughs,
                    &mut replace_global_defines,
                    &regs.global,
                );
            }
        }

        if let Some(sig) = call_sig {
            self.program_output_abi(
                sig,
                defs,
                throughs,
                &mut replace_global_defines,
                &regs.global,
            );
        }

        if let Some(constraints) = constraints {
            self.program_output_constraints(
                inst,
                constraints.outs,
                defs,
                &mut replace_global_defines,
                &regs.global,
            );
        }

        // Finally, we've fully programmed the constraint solver.
        // We expect a quick solution in most cases.
        let is_reload = match &self.cur.func.dfg[inst] {
            InstructionData::Unary {
                opcode: Opcode::Fill,
                ..
            } => true,
            _ => false,
        };

        let output_regs = self
            .solver
            .quick_solve(&regs.global, is_reload)
            .unwrap_or_else(|_| {
                debug!("quick_solve failed for {}", self.solver);
                self.iterate_solution(
                    throughs,
                    &regs.global,
                    &mut replace_global_defines,
                    is_reload,
                )
            });

        // The solution and/or fixed input constraints may require us to shuffle the set of live
        // registers around.
        self.shuffle_inputs(&mut regs.input);

        // If this is the first time we branch to `dest`, color its arguments to match the current
        // register state.
        if let Some(dest) = color_dest_args {
            self.color_ebb_params(inst, dest);
        }

        // Apply the solution to the defs.
        for v in self.solver.vars().iter().filter(|&v| v.is_define()) {
            self.cur.func.locations[v.value] = ValueLoc::Reg(v.solution);
        }

        // Tied defs are not part of the solution above.
        // Copy register assignments from tied inputs to tied outputs.
        if let Some(constraints) = constraints {
            if constraints.tied_ops {
                for (constraint, lv) in constraints.outs.iter().zip(defs) {
                    if let ConstraintKind::Tied(num) = constraint.kind {
                        let arg = self.cur.func.dfg.inst_args(inst)[num as usize];
                        let reg = self.divert.reg(arg, &self.cur.func.locations);
                        self.cur.func.locations[lv.value] = ValueLoc::Reg(reg);
                    }
                }
            }
        }

        // Update `regs` for the next instruction.
        regs.input = output_regs;
        for lv in defs {
            let loc = self.cur.func.locations[lv.value];
            debug!(
                "    color {} -> {}{}",
                lv.value,
                loc.display(&self.reginfo),
                if lv.is_local {
                    ""
                } else if replace_global_defines {
                    " (global to be replaced)"
                } else {
                    " (global)"
                }
            );

            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let reg = loc.unwrap_reg();

                debug_assert!(
                    !self.is_pinned_reg(rc, reg)
                        || self.cur.func.dfg[inst].opcode() == Opcode::GetPinnedReg,
                    "pinned register may not be part of outputs for '{}'.",
                    self.cur.func.dfg[inst].opcode()
                );

                if self.is_pinned_reg(rc, reg) {
                    continue;
                }

                // Remove the dead defs.
                if lv.endpoint == inst {
                    regs.input.free(rc, reg);
                    debug_assert!(lv.is_local);
                }

                // Track globals in their undiverted locations.
                if !lv.is_local && !replace_global_defines {
                    regs.global.take(rc, reg);
                }
            }
        }

        self.forget_diverted(kills);

        replace_global_defines
    }

    /// Program the input-side constraints for `inst` into the constraint solver.
    fn program_input_constraints(&mut self, inst: Inst, constraints: &[OperandConstraint]) {
        for (constraint, &arg_val) in constraints
            .iter()
            .zip(self.cur.func.dfg.inst_args(inst))
            .filter(|&(constraint, _)| constraint.kind != ConstraintKind::Stack)
        {
            // Reload pass is supposed to ensure that all arguments to register operands are
            // already in a register.
            let cur_reg = self.divert.reg(arg_val, &self.cur.func.locations);
            match constraint.kind {
                ConstraintKind::FixedReg(regunit) => {
                    // Add the fixed constraint even if `cur_reg == regunit`.
                    // It is possible that we will want to convert the value to a variable later,
                    // and this identity assignment prevents that from happening.
                    self.solver
                        .reassign_in(arg_val, constraint.regclass, cur_reg, regunit);
                }
                ConstraintKind::FixedTied(regunit) => {
                    // The pinned register may not be part of a fixed tied requirement. If this
                    // becomes the case, then it must be changed to a different register.
                    debug_assert!(
                        !self.is_pinned_reg(constraint.regclass, regunit),
                        "see comment above"
                    );
                    // See comment right above.
                    self.solver
                        .reassign_in(arg_val, constraint.regclass, cur_reg, regunit);
                }
                ConstraintKind::Tied(_) => {
                    if self.is_pinned_reg(constraint.regclass, cur_reg) {
                        // Divert the pinned register; it shouldn't be reused for a tied input.
                        if self.solver.can_add_var(constraint.regclass, cur_reg) {
                            self.solver.add_var(arg_val, constraint.regclass, cur_reg);
                        }
                    } else if !constraint.regclass.contains(cur_reg) {
                        self.solver.add_var(arg_val, constraint.regclass, cur_reg);
                    }
                }
                ConstraintKind::Reg => {
                    if !constraint.regclass.contains(cur_reg) {
                        self.solver.add_var(arg_val, constraint.regclass, cur_reg);
                    }
                }
                ConstraintKind::Stack => unreachable!(),
            }
        }
    }

    /// Program the complete set of input constraints into the solver.
    ///
    /// The `program_input_constraints()` function above will not tell the solver about any values
    /// that are already assigned to appropriate registers. This is normally fine, but if we want
    /// to add additional variables to help the solver, we need to make sure that they are
    /// constrained properly.
    ///
    /// This function completes the work of `program_input_constraints()` by calling `add_var` for
    /// all values used by the instruction.
    fn program_complete_input_constraints(&mut self) {
        let inst = self.cur.current_inst().expect("Not on an instruction");
        let constraints = self
            .encinfo
            .operand_constraints(self.cur.func.encodings[inst])
            .expect("Current instruction not encoded")
            .ins;

        for (constraint, &arg_val) in constraints.iter().zip(self.cur.func.dfg.inst_args(inst)) {
            match constraint.kind {
                ConstraintKind::Reg | ConstraintKind::Tied(_) => {
                    let cur_reg = self.divert.reg(arg_val, &self.cur.func.locations);

                    // This is the opposite condition of `program_input_constraints()`. The pinned
                    // register mustn't be added back as a variable.
                    if constraint.regclass.contains(cur_reg)
                        && !self.is_pinned_reg(constraint.regclass, cur_reg)
                    {
                        // This code runs after calling `solver.inputs_done()` so we must identify
                        // the new variable as killed or live-through.
                        let layout = &self.cur.func.layout;
                        if self.liveness[arg_val].killed_at(inst, layout.pp_ebb(inst), layout) {
                            self.solver
                                .add_killed_var(arg_val, constraint.regclass, cur_reg);
                        } else {
                            self.solver
                                .add_through_var(arg_val, constraint.regclass, cur_reg);
                        }
                    }
                }
                ConstraintKind::FixedReg(_)
                | ConstraintKind::FixedTied(_)
                | ConstraintKind::Stack => {}
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
    fn program_ebb_arguments(&mut self, inst: Inst, dest: Ebb) -> bool {
        // Find diverted registers that are live-in to `dest` and reassign them to their global
        // home.
        //
        // Values with a global live range that are not live in to `dest` could appear as branch
        // arguments, so they can't always be un-diverted.
        self.undivert_regs(|lr, layout| lr.is_livein(dest, layout));

        // Now handle the EBB arguments.
        let br_args = self.cur.func.dfg.inst_variable_args(inst);
        let dest_args = self.cur.func.dfg.ebb_params(dest);
        debug_assert_eq!(br_args.len(), dest_args.len());
        for (&dest_arg, &br_arg) in dest_args.iter().zip(br_args) {
            // The first time we encounter a branch to `dest`, we get to pick the location. The
            // following times we see a branch to `dest`, we must follow suit.
            match self.cur.func.locations[dest_arg] {
                ValueLoc::Unassigned => {
                    // This is the first branch to `dest`, so we should color `dest_arg` instead of
                    // `br_arg`. However, we don't know where `br_arg` will end up until
                    // after `shuffle_inputs`. See `color_ebb_params` below.
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
                        let br_reg = self.divert.reg(br_arg, &self.cur.func.locations);
                        self.solver.reassign_in(br_arg, rc, br_reg, dest_reg);
                    } else {
                        panic!("Branch argument {} is not in a register", br_arg);
                    }
                }
                ValueLoc::Stack(ss) => {
                    // The spiller should already have given us identical stack slots.
                    debug_assert_eq!(ValueLoc::Stack(ss), self.cur.func.locations[br_arg]);
                }
            }
        }

        // No `dest` arguments need coloring.
        false
    }

    /// Knowing that we've never seen a branch to `dest` before, color its parameters to match our
    /// register state.
    ///
    /// This function is only called when `program_ebb_arguments()` returned `true`.
    fn color_ebb_params(&mut self, inst: Inst, dest: Ebb) {
        let br_args = self.cur.func.dfg.inst_variable_args(inst);
        let dest_args = self.cur.func.dfg.ebb_params(dest);
        debug_assert_eq!(br_args.len(), dest_args.len());
        for (&dest_arg, &br_arg) in dest_args.iter().zip(br_args) {
            match self.cur.func.locations[dest_arg] {
                ValueLoc::Unassigned => {
                    if self.liveness[dest_arg].affinity.is_reg() {
                        let br_reg = self.divert.reg(br_arg, &self.cur.func.locations);
                        self.cur.func.locations[dest_arg] = ValueLoc::Reg(br_reg);
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
    where
        Pred: FnMut(&LiveRange, &Layout) -> bool,
    {
        for (&value, rdiv) in self.divert.iter() {
            let lr = self
                .liveness
                .get(value)
                .expect("Missing live range for diverted register");
            if pred(lr, &self.cur.func.layout) {
                if let Affinity::Reg(rci) = lr.affinity {
                    let rc = self.reginfo.rc(rci);
                    // Stack diversions should not be possible here. They only live transiently
                    // during `shuffle_inputs()`.
                    self.solver.reassign_in(
                        value,
                        rc,
                        rdiv.to.unwrap_reg(),
                        rdiv.from.unwrap_reg(),
                    );
                } else {
                    panic!(
                        "Diverted register {} with {} affinity",
                        value,
                        lr.affinity.display(&self.reginfo)
                    );
                }
            }
        }
    }

    /// Find existing live values that conflict with the fixed input register constraints programmed
    /// into the constraint solver. Convert them to solver variables so they can be diverted.
    fn divert_fixed_input_conflicts(&mut self, live: &[LiveValue]) {
        for lv in live {
            if let Affinity::Reg(rci) = lv.affinity {
                let toprc = self.reginfo.toprc(rci);
                let reg = self.divert.reg(lv.value, &self.cur.func.locations);
                if self.solver.is_fixed_input_conflict(toprc, reg) {
                    debug!(
                        "adding var to divert fixed input conflict for {}",
                        toprc.info.display_regunit(reg)
                    );
                    self.solver.add_var(lv.value, toprc, reg);
                }
            }
        }
    }

    /// Program any fixed-register output constraints into the solver. This may also detect
    /// conflicts between live-through registers and fixed output registers. These live-through
    /// values need to be turned into solver variables so they can be reassigned.
    fn program_fixed_outputs(
        &mut self,
        constraints: &[OperandConstraint],
        defs: &[LiveValue],
        throughs: &[LiveValue],
        replace_global_defines: &mut bool,
        global_regs: &RegisterSet,
    ) {
        for (constraint, lv) in constraints.iter().zip(defs) {
            match constraint.kind {
                ConstraintKind::FixedReg(reg) | ConstraintKind::FixedTied(reg) => {
                    self.add_fixed_output(lv.value, constraint.regclass, reg, throughs);
                    if !lv.is_local && !global_regs.is_avail(constraint.regclass, reg) {
                        debug!(
                            "Fixed output {} in {}:{} is not available in global regs",
                            lv.value,
                            constraint.regclass,
                            self.reginfo.display_regunit(reg)
                        );
                        *replace_global_defines = true;
                    }
                }
                ConstraintKind::Reg | ConstraintKind::Tied(_) | ConstraintKind::Stack => {}
            }
        }
    }

    /// Program the output-side ABI constraints for `inst` into the constraint solver.
    ///
    /// That means return values for a call instruction.
    fn program_output_abi(
        &mut self,
        sig: SigRef,
        defs: &[LiveValue],
        throughs: &[LiveValue],
        replace_global_defines: &mut bool,
        global_regs: &RegisterSet,
    ) {
        // It's technically possible for a call instruction to have fixed results before the
        // variable list of results, but we have no known instances of that.
        // Just assume all results are variable return values.
        debug_assert_eq!(defs.len(), self.cur.func.dfg.signatures[sig].returns.len());
        for (i, lv) in defs.iter().enumerate() {
            let abi = self.cur.func.dfg.signatures[sig].returns[i];
            if let ArgumentLoc::Reg(reg) = abi.location {
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc = self.reginfo.rc(rci);
                    self.add_fixed_output(lv.value, rc, reg, throughs);
                    if !lv.is_local && !global_regs.is_avail(rc, reg) {
                        debug!(
                            "ABI output {} in {}:{} is not available in global regs",
                            lv.value,
                            rc,
                            self.reginfo.display_regunit(reg)
                        );
                        *replace_global_defines = true;
                    }
                } else {
                    panic!("ABI argument {} should be in a register", lv.value);
                }
            }
        }
    }

    /// Add a single fixed output value to the solver.
    fn add_fixed_output(
        &mut self,
        value: Value,
        rc: RegClass,
        reg: RegUnit,
        throughs: &[LiveValue],
    ) {
        // Pinned register is already unavailable in the solver, since it is copied in the
        // available registers on entry.
        if !self.is_pinned_reg(rc, reg) && !self.solver.add_fixed_output(rc, reg) {
            // The fixed output conflicts with some of the live-through registers.
            for lv in throughs {
                if let Affinity::Reg(rci) = lv.affinity {
                    let toprc2 = self.reginfo.toprc(rci);
                    let reg2 = self.divert.reg(lv.value, &self.cur.func.locations);
                    if regs_overlap(rc, reg, toprc2, reg2) {
                        // This live-through value is interfering with the fixed output assignment.
                        // Convert it to a solver variable.
                        self.solver.add_through_var(lv.value, toprc2, reg2);
                    }
                }
            }

            let ok = self.solver.add_fixed_output(rc, reg);
            debug_assert!(ok, "Couldn't clear fixed output interference for {}", value);
        }
        self.cur.func.locations[value] = ValueLoc::Reg(reg);
    }

    /// Program the output-side constraints for `inst` into the constraint solver.
    ///
    /// It is assumed that all fixed outputs have already been handled.
    fn program_output_constraints(
        &mut self,
        inst: Inst,
        constraints: &[OperandConstraint],
        defs: &[LiveValue],
        replace_global_defines: &mut bool,
        global_regs: &RegisterSet,
    ) {
        for (constraint, lv) in constraints.iter().zip(defs) {
            match constraint.kind {
                ConstraintKind::FixedReg(_)
                | ConstraintKind::FixedTied(_)
                | ConstraintKind::Stack => continue,
                ConstraintKind::Reg => {
                    self.solver
                        .add_def(lv.value, constraint.regclass, !lv.is_local);
                }
                ConstraintKind::Tied(num) => {
                    // Find the input operand we're tied to.
                    // The solver doesn't care about the output value.
                    let arg = self.cur.func.dfg.inst_args(inst)[num as usize];
                    let reg = self.divert.reg(arg, &self.cur.func.locations);

                    if let Some(reg) =
                        self.solver
                            .add_tied_input(arg, constraint.regclass, reg, !lv.is_local)
                    {
                        // The value we're tied to has been assigned to a fixed register.
                        // We need to make sure that fixed output register is compatible with the
                        // global register set.
                        if !lv.is_local && !global_regs.is_avail(constraint.regclass, reg) {
                            debug!(
                                "Tied output {} in {}:{} is not available in global regs",
                                lv.value,
                                constraint.regclass,
                                self.reginfo.display_regunit(reg)
                            );
                            *replace_global_defines = true;
                        }
                    }
                }
            }
        }
    }

    /// Try harder to find a solution to the constraint problem since `quick_solve()` failed.
    ///
    /// We may need to move more registers around before a solution is possible. Use an iterative
    /// algorithm that adds one more variable until a solution can be found.
    fn iterate_solution(
        &mut self,
        throughs: &[LiveValue],
        global_regs: &RegisterSet,
        replace_global_defines: &mut bool,
        is_reload: bool,
    ) -> RegisterSet {
        // Make sure `try_add_var()` below doesn't create a variable with too loose constraints.
        self.program_complete_input_constraints();

        loop {
            match self.solver.real_solve(global_regs, is_reload) {
                Ok(regs) => return regs,
                Err(SolverError::Divert(rc)) => {
                    // Do we have any live-through `rc` registers that are not already variables?
                    let added = self.try_add_var(rc, throughs);
                    debug_assert!(added, "Ran out of registers in {}", rc);
                }
                Err(SolverError::Global(_value)) => {
                    debug!(
                        "Not enough global registers for {}, trying as local",
                        _value
                    );
                    // We'll clear the `is_global` flag on all solver variables and instead make a
                    // note to replace all global defines with local defines followed by a copy.
                    *replace_global_defines = true;
                    self.solver.clear_all_global_flags();
                }
            };
        }
    }

    /// Try to add an `rc` variable to the solver from the `throughs` set.
    fn try_add_var(&mut self, rc: RegClass, throughs: &[LiveValue]) -> bool {
        debug!("Trying to add a {} reg from {} values", rc, throughs.len());

        for lv in throughs {
            if let Affinity::Reg(rci) = lv.affinity {
                // The new variable gets to roam the whole top-level register class because it is
                // not actually constrained by the instruction. We just want it out of the way.
                let toprc2 = self.reginfo.toprc(rci);
                let reg2 = self.divert.reg(lv.value, &self.cur.func.locations);
                if rc.contains(reg2)
                    && self.solver.can_add_var(toprc2, reg2)
                    && !self.is_live_on_outgoing_edge(lv.value)
                {
                    self.solver.add_through_var(lv.value, toprc2, reg2);
                    return true;
                }
            }
        }

        false
    }

    /// Determine if `value` is live on a CFG edge from the current instruction.
    ///
    /// This means that the current instruction is a branch and `value` is live in to one of the
    /// branch destinations. Branch arguments and EBB parameters are not considered live on the
    /// edge.
    fn is_live_on_outgoing_edge(&self, value: Value) -> bool {
        use crate::ir::instructions::BranchInfo::*;

        let inst = self.cur.current_inst().expect("Not on an instruction");
        let layout = &self.cur.func.layout;
        match self.cur.func.dfg.analyze_branch(inst) {
            NotABranch => false,
            SingleDest(ebb, _) => {
                let lr = &self.liveness[value];
                lr.is_livein(ebb, layout)
            }
            Table(jt, ebb) => {
                let lr = &self.liveness[value];
                !lr.is_local()
                    && (ebb.map_or(false, |ebb| lr.is_livein(ebb, layout))
                        || self.cur.func.jump_tables[jt]
                            .iter()
                            .any(|ebb| lr.is_livein(*ebb, layout)))
            }
        }
    }

    /// Emit `regmove` instructions as needed to move the live registers into place before the
    /// instruction. Also update `self.divert` accordingly.
    ///
    /// The `self.cur` cursor is expected to point at the instruction. The register moves are
    /// inserted before.
    ///
    /// The solver needs to be reminded of the available registers before any moves are inserted.
    fn shuffle_inputs(&mut self, regs: &mut RegisterSet) {
        use crate::regalloc::solver::Move::*;

        let spills = self.solver.schedule_moves(regs);

        // The move operations returned by `schedule_moves` refer to emergency spill slots by
        // consecutive indexes starting from 0. Map these to real stack slots.
        // It is very unlikely (impossible?) that we would need more than one spill per top-level
        // register class, so avoid allocation by using a fixed array here.
        let mut slot = [PackedOption::default(); 8];
        debug_assert!(spills <= slot.len(), "Too many spills ({})", spills);

        for m in self.solver.moves() {
            match *m {
                Reg {
                    value,
                    from,
                    to,
                    rc,
                } => {
                    debug_assert!(
                        !self.is_pinned_reg(rc, to),
                        "pinned register used in a regmove"
                    );
                    self.divert.regmove(value, from, to);
                    self.cur.ins().regmove(value, from, to);
                }
                Spill {
                    value,
                    from,
                    to_slot,
                    ..
                } => {
                    debug_assert_eq!(slot[to_slot].expand(), None, "Overwriting slot in use");
                    let ss = self
                        .cur
                        .func
                        .stack_slots
                        .get_emergency_slot(self.cur.func.dfg.value_type(value), &slot[0..spills]);
                    slot[to_slot] = ss.into();
                    self.divert.regspill(value, from, ss);
                    self.cur.ins().regspill(value, from, ss);
                }
                Fill {
                    value,
                    from_slot,
                    to,
                    rc,
                } => {
                    debug_assert!(
                        !self.is_pinned_reg(rc, to),
                        "pinned register used in a regfill"
                    );
                    // These slots are single use, so mark `ss` as available again.
                    let ss = slot[from_slot].take().expect("Using unallocated slot");
                    self.divert.regfill(value, ss, to);
                    self.cur.ins().regfill(value, ss, to);
                }
            }
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

    /// Replace all global values defined by `inst` with local values that are then copied into the
    /// global value:
    ///
    ///   v1 = foo
    ///
    /// becomes:
    ///
    ///   v20 = foo
    ///   v1 = copy v20
    ///
    /// This is sometimes necessary when there are no global registers available that can satisfy
    /// the constraints on the instruction operands.
    ///
    fn replace_global_defines(&mut self, inst: Inst, tracker: &mut LiveValueTracker) {
        debug!("Replacing global defs on {}", self.cur.display_inst(inst));

        // We'll insert copies *after `inst`. Our caller will move the cursor back.
        self.cur.next_inst();

        // The tracker keeps the defs from `inst` at the end. Any dead defs have already been
        // removed, so it's not obvious how many defs to process
        for lv in tracker.live_mut().iter_mut().rev() {
            // Keep going until we reach a value that is not defined by `inst`.
            if match self.cur.func.dfg.value_def(lv.value) {
                ValueDef::Result(i, _) => i != inst,
                _ => true,
            } {
                break;
            }
            if lv.is_local || !lv.affinity.is_reg() {
                continue;
            }

            // Now `lv.value` is globally live and defined by `inst`. Replace it with a local live
            // range that is copied after `inst`.
            let ty = self.cur.func.dfg.value_type(lv.value);
            let local = self.cur.func.dfg.replace_result(lv.value, ty);
            self.cur.ins().with_result(lv.value).copy(local);
            let copy = self.cur.built_inst();

            // Create a live range for `local: inst -> copy`.
            self.liveness.create_dead(local, inst, lv.affinity);
            self.liveness.extend_locally(
                local,
                self.cur.func.layout.pp_ebb(inst),
                copy,
                &self.cur.func.layout,
            );

            // Move the definition of the global `lv.value`.
            self.liveness.move_def_locally(lv.value, copy);

            // Transfer the register coloring to `local`.
            let loc = mem::replace(&mut self.cur.func.locations[lv.value], ValueLoc::default());
            self.cur.func.locations[local] = loc;

            // Update `lv` to reflect the new `local` live range.
            lv.value = local;
            lv.endpoint = copy;
            lv.is_local = true;

            debug!(
                "  + {} with {} in {}",
                self.cur.display_inst(copy),
                local,
                loc.display(&self.reginfo)
            );
        }
        debug!("Done: {}", self.cur.display_inst(inst));
    }

    /// Process kills on a ghost instruction.
    /// - Forget diversions.
    /// - Free killed registers.
    fn process_ghost_kills(&mut self, kills: &[LiveValue], regs: &mut AvailableRegs) {
        for lv in kills {
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                let loc = match self.divert.remove(lv.value) {
                    Some(loc) => loc,
                    None => self.cur.func.locations[lv.value],
                };
                regs.input.free(rc, loc.unwrap_reg());
                if !lv.is_local {
                    regs.global
                        .free(rc, self.cur.func.locations[lv.value].unwrap_reg());
                }
            }
        }
    }
}

/// Keep track of the set of available registers in two interference domains: all registers
/// considering diversions and global registers not considering diversions.
struct AvailableRegs {
    /// The exact set of registers available on the input side of the current instruction. This
    /// takes into account register diversions, and it includes both local and global live ranges.
    input: RegisterSet,

    /// Registers available for allocating globally live values. This set ignores any local values,
    /// and it does not account for register diversions.
    ///
    /// Global values must be allocated out of this set because conflicts with other global values
    /// can't be resolved with local diversions.
    global: RegisterSet,
}

impl AvailableRegs {
    /// Initialize both the input and global sets from `regs`.
    pub fn new(regs: &RegisterSet) -> Self {
        Self {
            input: regs.clone(),
            global: regs.clone(),
        }
    }

    /// Take an un-diverted register from one or both sets.
    pub fn take(&mut self, rc: RegClass, reg: RegUnit, is_local: bool) {
        self.input.take(rc, reg);
        if !is_local {
            self.global.take(rc, reg);
        }
    }

    /// Take a diverted register from both sets for a non-local allocation.
    pub fn take_divert(&mut self, rc: RegClass, reg: RegUnit, reg_divert: RegUnit) {
        self.input.take(rc, reg_divert);
        self.global.take(rc, reg);
    }
}
