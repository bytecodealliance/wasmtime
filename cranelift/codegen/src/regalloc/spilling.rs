//! Spilling pass.
//!
//! The spilling pass is the first to run after the liveness analysis. Its primary function is to
//! ensure that the register pressure never exceeds the number of available registers by moving
//! some SSA values to spill slots on the stack. This is encoded in the affinity of the value's
//! live range.
//!
//! Some instruction operand constraints may require additional registers to resolve. Since this
//! can cause spilling, the spilling pass is also responsible for resolving those constraints by
//! inserting copies. The extra constraints are:
//!
//! 1. A value used by a tied operand must be killed by the instruction. This is resolved by
//!    inserting a copy to a temporary value when necessary.
//! 2. When the same value is used more than once by an instruction, the operand constraints must
//!    be compatible. Otherwise, the value must be copied into a new register for some of the
//!    operands.

use crate::cursor::{Cursor, EncCursor};
use crate::dominator_tree::DominatorTree;
use crate::ir::{ArgumentLoc, Block, Function, Inst, InstBuilder, SigRef, Value, ValueLoc};
use crate::isa::registers::{RegClass, RegClassIndex, RegClassMask, RegUnit};
use crate::isa::{ConstraintKind, EncInfo, RecipeConstraints, RegInfo, TargetIsa};
use crate::regalloc::affinity::Affinity;
use crate::regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use crate::regalloc::liveness::Liveness;
use crate::regalloc::pressure::Pressure;
use crate::regalloc::virtregs::VirtRegs;
use crate::timing;
use crate::topo_order::TopoOrder;
use alloc::vec::Vec;
use core::fmt;

/// Return a top-level register class which contains `unit`.
fn toprc_containing_regunit(unit: RegUnit, reginfo: &RegInfo) -> RegClass {
    let bank = reginfo.bank_containing_regunit(unit).unwrap();
    reginfo.classes[bank.first_toprc..(bank.first_toprc + bank.num_toprcs)]
        .iter()
        .find(|&rc| rc.contains(unit))
        .expect("reg unit should be in a toprc")
}

/// Persistent data structures for the spilling pass.
pub struct Spilling {
    spills: Vec<Value>,
    reg_uses: Vec<RegUse>,
}

/// Context data structure that gets instantiated once per pass.
struct Context<'a> {
    // Current instruction as well as reference to function and ISA.
    cur: EncCursor<'a>,

    // Cached ISA information.
    reginfo: RegInfo,
    encinfo: EncInfo,

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,
    virtregs: &'a VirtRegs,
    topo: &'a mut TopoOrder,

    // Current register pressure.
    pressure: Pressure,

    // Values spilled for the current instruction. These values have already been removed from the
    // pressure tracker, but they are still present in the live value tracker and their affinity
    // hasn't been changed yet.
    spills: &'a mut Vec<Value>,

    // Uses of register values in the current instruction.
    reg_uses: &'a mut Vec<RegUse>,
}

impl Spilling {
    /// Create a new spilling data structure.
    pub fn new() -> Self {
        Self {
            spills: Vec::new(),
            reg_uses: Vec::new(),
        }
    }

    /// Clear all data structures in this spilling pass.
    pub fn clear(&mut self) {
        self.spills.clear();
        self.reg_uses.clear();
    }

    /// Run the spilling algorithm over `func`.
    pub fn run(
        &mut self,
        isa: &dyn TargetIsa,
        func: &mut Function,
        domtree: &DominatorTree,
        liveness: &mut Liveness,
        virtregs: &VirtRegs,
        topo: &mut TopoOrder,
        tracker: &mut LiveValueTracker,
    ) {
        let _tt = timing::ra_spilling();
        log::trace!("Spilling for:\n{}", func.display(isa));
        let reginfo = isa.register_info();
        let usable_regs = isa.allocatable_registers(func);
        let mut ctx = Context {
            cur: EncCursor::new(func, isa),
            reginfo: isa.register_info(),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
            virtregs,
            topo,
            pressure: Pressure::new(&reginfo, &usable_regs),
            spills: &mut self.spills,
            reg_uses: &mut self.reg_uses,
        };
        ctx.run(tracker)
    }
}

impl<'a> Context<'a> {
    fn run(&mut self, tracker: &mut LiveValueTracker) {
        self.topo.reset(self.cur.func.layout.blocks());
        while let Some(block) = self.topo.next(&self.cur.func.layout, self.domtree) {
            self.visit_block(block, tracker);
        }
    }

    fn visit_block(&mut self, block: Block, tracker: &mut LiveValueTracker) {
        log::trace!("Spilling {}:", block);
        self.cur.goto_top(block);
        self.visit_block_header(block, tracker);
        tracker.drop_dead_params();
        self.process_spills(tracker);

        while let Some(inst) = self.cur.next_inst() {
            if !self.cur.func.dfg[inst].opcode().is_ghost() {
                self.visit_inst(inst, block, tracker);
            } else {
                let (_throughs, kills) = tracker.process_ghost(inst);
                self.free_regs(kills);
            }
            tracker.drop_dead(inst);
            self.process_spills(tracker);
        }
    }

    // Take all live registers in `regs` from the pressure set.
    // This doesn't cause any spilling, it is assumed there are enough registers.
    fn take_live_regs(&mut self, regs: &[LiveValue]) {
        for lv in regs {
            if !lv.is_dead {
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc = self.reginfo.rc(rci);
                    self.pressure.take(rc);
                }
            }
        }
    }

    // Free all registers in `kills` from the pressure set.
    fn free_regs(&mut self, kills: &[LiveValue]) {
        for lv in kills {
            if let Affinity::Reg(rci) = lv.affinity {
                if !self.spills.contains(&lv.value) {
                    let rc = self.reginfo.rc(rci);
                    self.pressure.free(rc);
                }
            }
        }
    }

    // Free all dead registers in `regs` from the pressure set.
    fn free_dead_regs(&mut self, regs: &[LiveValue]) {
        for lv in regs {
            if lv.is_dead {
                if let Affinity::Reg(rci) = lv.affinity {
                    if !self.spills.contains(&lv.value) {
                        let rc = self.reginfo.rc(rci);
                        self.pressure.free(rc);
                    }
                }
            }
        }
    }

    fn visit_block_header(&mut self, block: Block, tracker: &mut LiveValueTracker) {
        let (liveins, params) = tracker.block_top(
            block,
            &self.cur.func.dfg,
            self.liveness,
            &self.cur.func.layout,
            self.domtree,
        );

        // Count the live-in registers. These should already fit in registers; they did at the
        // dominator.
        self.pressure.reset();
        self.take_live_regs(liveins);

        // A block can have an arbitrary (up to 2^16...) number of parameters, so they are not
        // guaranteed to fit in registers.
        for lv in params {
            if let Affinity::Reg(rci) = lv.affinity {
                let rc = self.reginfo.rc(rci);
                'try_take: while let Err(mask) = self.pressure.take_transient(rc) {
                    log::trace!("Need {} reg for block param {}", rc, lv.value);
                    match self.spill_candidate(mask, liveins) {
                        Some(cand) => {
                            log::trace!(
                                "Spilling live-in {} to make room for {} block param {}",
                                cand,
                                rc,
                                lv.value
                            );
                            self.spill_reg(cand);
                        }
                        None => {
                            // We can't spill any of the live-in registers, so we have to spill an
                            // block argument. Since the current spill metric would consider all the
                            // block arguments equal, just spill the present register.
                            log::trace!("Spilling {} block argument {}", rc, lv.value);

                            // Since `spill_reg` will free a register, add the current one here.
                            self.pressure.take(rc);
                            self.spill_reg(lv.value);
                            break 'try_take;
                        }
                    }
                }
            }
        }

        // The transient pressure counts for the block arguments are accurate. Just preserve them.
        self.pressure.preserve_transient();
        self.free_dead_regs(params);
    }

    fn visit_inst(&mut self, inst: Inst, block: Block, tracker: &mut LiveValueTracker) {
        log::trace!("Inst {}, {}", self.cur.display_inst(inst), self.pressure);
        debug_assert_eq!(self.cur.current_inst(), Some(inst));
        debug_assert_eq!(self.cur.current_block(), Some(block));

        let constraints = self
            .encinfo
            .operand_constraints(self.cur.func.encodings[inst]);

        // We may need to resolve register constraints if there are any noteworthy uses.
        debug_assert!(self.reg_uses.is_empty());
        self.collect_reg_uses(inst, block, constraints);

        // Calls usually have fixed register uses.
        let call_sig = self.cur.func.dfg.call_signature(inst);
        if let Some(sig) = call_sig {
            self.collect_abi_reg_uses(inst, sig);
        }

        if !self.reg_uses.is_empty() {
            self.process_reg_uses(inst, tracker);
        }

        // Update the live value tracker with this instruction.
        let (throughs, kills, defs) = tracker.process_inst(inst, &self.cur.func.dfg, self.liveness);

        // Remove kills from the pressure tracker.
        self.free_regs(kills);

        // If inst is a call, spill all register values that are live across the call.
        // This means that we don't currently take advantage of callee-saved registers.
        // TODO: Be more sophisticated.
        let opcode = self.cur.func.dfg[inst].opcode();
        if call_sig.is_some() || opcode.clobbers_all_regs() {
            for lv in throughs {
                if lv.affinity.is_reg() && !self.spills.contains(&lv.value) {
                    self.spill_reg(lv.value);
                }
            }
        }

        // Make sure we have enough registers for the register defs.
        // Dead defs are included here. They need a register too.
        // No need to process call return values, they are in fixed registers.
        if let Some(constraints) = constraints {
            for op in constraints.outs {
                if op.kind != ConstraintKind::Stack {
                    // Add register def to pressure, spill if needed.
                    while let Err(mask) = self.pressure.take_transient(op.regclass) {
                        log::trace!("Need {} reg from {} throughs", op.regclass, throughs.len());
                        match self.spill_candidate(mask, throughs) {
                            Some(cand) => self.spill_reg(cand),
                            None => panic!(
                                "Ran out of {} registers for {}",
                                op.regclass,
                                self.cur.display_inst(inst)
                            ),
                        }
                    }
                }
            }
            self.pressure.reset_transient();
        }

        // Restore pressure state, compute pressure with affinities from `defs`.
        // Exclude dead defs. Includes call return values.
        // This won't cause spilling.
        self.take_live_regs(defs);
    }

    // Collect register uses that are noteworthy in one of the following ways:
    //
    // 1. It's a fixed register constraint.
    // 2. It's a use of a spilled value.
    // 3. It's a tied register constraint and the value isn't killed.
    //
    // We are assuming here that if a value is used both by a fixed register operand and a register
    // class operand, they two are compatible. We are also assuming that two register class
    // operands are always compatible.
    fn collect_reg_uses(
        &mut self,
        inst: Inst,
        block: Block,
        constraints: Option<&RecipeConstraints>,
    ) {
        let args = self.cur.func.dfg.inst_args(inst);
        let num_fixed_ins = if let Some(constraints) = constraints {
            for (idx, (op, &arg)) in constraints.ins.iter().zip(args).enumerate() {
                let mut reguse = RegUse::new(arg, idx, op.regclass.into());
                let lr = &self.liveness[arg];
                match op.kind {
                    ConstraintKind::Stack => continue,
                    ConstraintKind::FixedReg(_) => reguse.fixed = true,
                    ConstraintKind::Tied(_) => {
                        // A tied operand must kill the used value.
                        reguse.tied = !lr.killed_at(inst, block, &self.cur.func.layout);
                    }
                    ConstraintKind::FixedTied(_) => {
                        reguse.fixed = true;
                        reguse.tied = !lr.killed_at(inst, block, &self.cur.func.layout);
                    }
                    ConstraintKind::Reg => {}
                }
                if lr.affinity.is_stack() {
                    reguse.spilled = true;
                }

                // Only collect the interesting register uses.
                if reguse.fixed || reguse.tied || reguse.spilled {
                    log::trace!("  reguse: {}", reguse);
                    self.reg_uses.push(reguse);
                }
            }
            constraints.ins.len()
        } else {
            // A non-ghost instruction with no constraints can't have any
            // fixed operands.
            0
        };

        // Similarly, for return instructions, collect uses of ABI-defined
        // return values.
        if self.cur.func.dfg[inst].opcode().is_return() {
            debug_assert_eq!(
                self.cur.func.dfg.inst_variable_args(inst).len(),
                self.cur.func.signature.returns.len(),
                "The non-fixed arguments in a return should follow the function's signature."
            );
            for (ret_idx, (ret, &arg)) in
                self.cur.func.signature.returns.iter().zip(args).enumerate()
            {
                let idx = num_fixed_ins + ret_idx;
                let unit = match ret.location {
                    ArgumentLoc::Unassigned => {
                        panic!("function return signature should be legalized")
                    }
                    ArgumentLoc::Reg(unit) => unit,
                    ArgumentLoc::Stack(_) => continue,
                };
                let toprc = toprc_containing_regunit(unit, &self.reginfo);
                let mut reguse = RegUse::new(arg, idx, toprc.into());
                reguse.fixed = true;

                log::trace!("  reguse: {}", reguse);
                self.reg_uses.push(reguse);
            }
        }
    }

    // Collect register uses from the ABI input constraints.
    fn collect_abi_reg_uses(&mut self, inst: Inst, sig: SigRef) {
        let num_fixed_args = self.cur.func.dfg[inst]
            .opcode()
            .constraints()
            .num_fixed_value_arguments();
        let args = self.cur.func.dfg.inst_variable_args(inst);
        for (idx, (abi, &arg)) in self.cur.func.dfg.signatures[sig]
            .params
            .iter()
            .zip(args)
            .enumerate()
        {
            if abi.location.is_reg() {
                let (rci, spilled) = match self.liveness[arg].affinity {
                    Affinity::Reg(rci) => (rci, false),
                    Affinity::Stack => (
                        self.cur.isa.regclass_for_abi_type(abi.value_type).into(),
                        true,
                    ),
                    Affinity::Unassigned => panic!("Missing affinity for {}", arg),
                };
                let mut reguse = RegUse::new(arg, num_fixed_args + idx, rci);
                reguse.fixed = true;
                reguse.spilled = spilled;
                self.reg_uses.push(reguse);
            }
        }
    }

    // Process multiple register uses to resolve potential conflicts.
    //
    // Look for multiple uses of the same value in `self.reg_uses` and insert copies as necessary.
    // Trigger spilling if any of the temporaries cause the register pressure to become too high.
    //
    // Leave `self.reg_uses` empty.
    fn process_reg_uses(&mut self, inst: Inst, tracker: &LiveValueTracker) {
        // We're looking for multiple uses of the same value, so start by sorting by value. The
        // secondary `opidx` key makes it possible to use an unstable (non-allocating) sort.
        self.reg_uses.sort_unstable_by_key(|u| (u.value, u.opidx));

        self.cur.use_srcloc(inst);
        for i in 0..self.reg_uses.len() {
            let ru = self.reg_uses[i];

            // Do we need to insert a copy for this use?
            let need_copy = if ru.tied {
                true
            } else if ru.fixed {
                // This is a fixed register use which doesn't necessarily require a copy.
                // Make a copy only if this is not the first use of the value.
                self.reg_uses
                    .get(i.wrapping_sub(1))
                    .map_or(false, |ru2| ru2.value == ru.value)
            } else {
                false
            };

            if need_copy {
                let copy = self.insert_copy(ru.value, ru.rci);
                self.cur.func.dfg.inst_args_mut(inst)[ru.opidx as usize] = copy;
            }

            // Even if we don't insert a copy, we may need to account for register pressure for the
            // reload pass.
            if need_copy || ru.spilled {
                let rc = self.reginfo.rc(ru.rci);
                while let Err(mask) = self.pressure.take_transient(rc) {
                    log::trace!("Copy of {} reg causes spill", rc);
                    // Spill a live register that is *not* used by the current instruction.
                    // Spilling a use wouldn't help.
                    //
                    // Do allow spilling of block arguments on branches. This is safe since we spill
                    // the whole virtual register which includes the matching block parameter value
                    // at the branch destination. It is also necessary since there can be
                    // arbitrarily many block arguments.
                    match {
                        let args = if self.cur.func.dfg[inst].opcode().is_branch() {
                            self.cur.func.dfg.inst_fixed_args(inst)
                        } else {
                            self.cur.func.dfg.inst_args(inst)
                        };
                        self.spill_candidate(
                            mask,
                            tracker.live().iter().filter(|lv| !args.contains(&lv.value)),
                        )
                    } {
                        Some(cand) => self.spill_reg(cand),
                        None => panic!(
                            "Ran out of {} registers when inserting copy before {}",
                            rc,
                            self.cur.display_inst(inst)
                        ),
                    }
                }
            }
        }
        self.pressure.reset_transient();
        self.reg_uses.clear()
    }

    // Find a spill candidate from `candidates` whose top-level register class is in `mask`.
    fn spill_candidate<'ii, II>(&self, mask: RegClassMask, candidates: II) -> Option<Value>
    where
        II: IntoIterator<Item = &'ii LiveValue>,
    {
        // Find the best viable spill candidate.
        //
        // The very simple strategy implemented here is to spill the value with the earliest def in
        // the reverse post-order. This strategy depends on a good reload pass to generate good
        // code.
        //
        // We know that all candidate defs dominate the current instruction, so one of them will
        // dominate the others. That is the earliest def.
        candidates
            .into_iter()
            .filter_map(|lv| {
                // Viable candidates are registers in one of the `mask` classes, and not already in
                // the spill set.
                if let Affinity::Reg(rci) = lv.affinity {
                    let rc = self.reginfo.rc(rci);
                    if (mask & (1 << rc.toprc)) != 0 && !self.spills.contains(&lv.value) {
                        // Here, `lv` is a viable spill candidate.
                        return Some(lv.value);
                    }
                }
                None
            })
            .min_by(|&a, &b| {
                // Find the minimum candidate according to the RPO of their defs.
                self.domtree.rpo_cmp(
                    self.cur.func.dfg.value_def(a),
                    self.cur.func.dfg.value_def(b),
                    &self.cur.func.layout,
                )
            })
    }

    /// Spill `value` immediately by
    ///
    /// 1. Changing its affinity to `Stack` which marks the spill.
    /// 2. Removing the value from the pressure tracker.
    /// 3. Adding the value to `self.spills` for later reference by `process_spills`.
    ///
    /// Note that this does not update the cached affinity in the live value tracker. Call
    /// `process_spills` to do that.
    fn spill_reg(&mut self, value: Value) {
        if let Affinity::Reg(rci) = self.liveness.spill(value) {
            let rc = self.reginfo.rc(rci);
            self.pressure.free(rc);
            self.spills.push(value);
            log::trace!("Spilled {}:{} -> {}", value, rc, self.pressure);
        } else {
            panic!("Cannot spill {} that was already on the stack", value);
        }

        // Assign a spill slot for the whole virtual register.
        let ss = self
            .cur
            .func
            .stack_slots
            .make_spill_slot(self.cur.func.dfg.value_type(value));
        for &v in self.virtregs.congruence_class(&value) {
            self.liveness.spill(v);
            self.cur.func.locations[v] = ValueLoc::Stack(ss);
        }
    }

    /// Process any pending spills in the `self.spills` vector.
    ///
    /// It is assumed that spills are removed from the pressure tracker immediately, see
    /// `spill_reg` above.
    ///
    /// We also need to update the live range affinity and remove spilled values from the live
    /// value tracker.
    fn process_spills(&mut self, tracker: &mut LiveValueTracker) {
        if !self.spills.is_empty() {
            tracker.process_spills(|v| self.spills.contains(&v));
            self.spills.clear()
        }
    }

    /// Insert a `copy value` before the current instruction and give it a live range extending to
    /// the current instruction.
    ///
    /// Returns the new local value created.
    fn insert_copy(&mut self, value: Value, rci: RegClassIndex) -> Value {
        let copy = self.cur.ins().copy(value);
        let inst = self.cur.built_inst();

        // Update live ranges.
        self.liveness.create_dead(copy, inst, Affinity::Reg(rci));
        self.liveness.extend_locally(
            copy,
            self.cur.func.layout.pp_block(inst),
            self.cur.current_inst().expect("must be at an instruction"),
            &self.cur.func.layout,
        );

        copy
    }
}

/// Struct representing a register use of a value.
/// Used to detect multiple uses of the same value with incompatible register constraints.
#[derive(Clone, Copy)]
struct RegUse {
    value: Value,
    opidx: u16,

    // Register class required by the use.
    rci: RegClassIndex,

    // A use with a fixed register constraint.
    fixed: bool,

    // A register use of a spilled value.
    spilled: bool,

    // A use with a tied register constraint *and* the used value is not killed.
    tied: bool,
}

impl RegUse {
    fn new(value: Value, idx: usize, rci: RegClassIndex) -> Self {
        Self {
            value,
            opidx: idx as u16,
            rci,
            fixed: false,
            spilled: false,
            tied: false,
        }
    }
}

impl fmt::Display for RegUse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@op{}", self.value, self.opidx)?;
        if self.fixed {
            write!(f, "/fixed")?;
        }
        if self.spilled {
            write!(f, "/spilled")?;
        }
        if self.tied {
            write!(f, "/tied")?;
        }
        Ok(())
    }
}
