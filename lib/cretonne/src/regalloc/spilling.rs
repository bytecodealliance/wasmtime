//! Spilling pass.
//!
//! The spilling pass is the first to run after the liveness analysis. Its primary function is to
//! ensure that the register pressure never exceeds the number of available registers by moving
//! some SSA values to spill slots on the stack. This is encoded in the affinity of the value's
//! live range.

use dominator_tree::DominatorTree;
use ir::{DataFlowGraph, Layout, Cursor};
use ir::{Function, Ebb, Inst, Value};
use isa::{TargetIsa, RegInfo, EncInfo, RecipeConstraints, ConstraintKind};
use isa::registers::RegClassMask;
use regalloc::affinity::Affinity;
use regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use regalloc::liveness::Liveness;
use regalloc::pressure::Pressure;
use topo_order::TopoOrder;

/// Persistent data structures for the spilling pass.
pub struct Spilling {
    spills: Vec<Value>,
}

/// Context data structure that gets instantiated once per pass.
struct Context<'a> {
    // Cached ISA information.
    reginfo: RegInfo,
    encinfo: EncInfo,

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,
    topo: &'a mut TopoOrder,

    // Current register pressure.
    pressure: Pressure,

    // Values spilled for the current instruction. These values have already been removed from the
    // pressure tracker, but they are still present in the live value tracker and their affinity
    // hasn't been changed yet.
    spills: &'a mut Vec<Value>,
}

impl Spilling {
    /// Create a new spilling data structure.
    pub fn new() -> Spilling {
        Spilling { spills: Vec::new() }
    }

    /// Run the spilling algorithm over `func`.
    pub fn run(&mut self,
               isa: &TargetIsa,
               func: &mut Function,
               domtree: &DominatorTree,
               liveness: &mut Liveness,
               topo: &mut TopoOrder,
               tracker: &mut LiveValueTracker) {
        dbg!("Spilling for:\n{}", func.display(isa));
        let reginfo = isa.register_info();
        let usable_regs = isa.allocatable_registers(func);
        let mut ctx = Context {
            reginfo: isa.register_info(),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
            topo,
            pressure: Pressure::new(&reginfo, &usable_regs),
            spills: &mut self.spills,
        };
        ctx.run(func, tracker)
    }
}

impl<'a> Context<'a> {
    fn run(&mut self, func: &mut Function, tracker: &mut LiveValueTracker) {
        self.topo.reset(func.layout.ebbs());
        while let Some(ebb) = self.topo.next(&func.layout, self.domtree) {
            self.visit_ebb(ebb, func, tracker);
        }
    }

    fn visit_ebb(&mut self, ebb: Ebb, func: &mut Function, tracker: &mut LiveValueTracker) {
        dbg!("Spilling {}:", ebb);
        self.visit_ebb_header(ebb, func, tracker);
        tracker.drop_dead_args();

        let mut pos = Cursor::new(&mut func.layout);
        pos.goto_top(ebb);
        while let Some(inst) = pos.next_inst() {
            if let Some(constraints) = self.encinfo.operand_constraints(func.encodings[inst]) {
                self.visit_inst(inst, constraints, &mut pos, &mut func.dfg, tracker);
                tracker.drop_dead(inst);
                self.process_spills(tracker);
            }
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
                let rc = self.reginfo.rc(rci);
                self.pressure.free(rc);
            }
        }
    }

    fn visit_ebb_header(&mut self, ebb: Ebb, func: &mut Function, tracker: &mut LiveValueTracker) {
        let (liveins, args) =
            tracker.ebb_top(ebb, &func.dfg, self.liveness, &func.layout, self.domtree);

        // Count the live-in registers. These should already fit in registers; they did at the
        // dominator.
        self.pressure.reset();
        self.take_live_regs(liveins);

        // TODO: Process and count EBB arguments. Some may need spilling.
        self.take_live_regs(args);
    }

    fn visit_inst(&mut self,
                  inst: Inst,
                  constraints: &RecipeConstraints,
                  pos: &mut Cursor,
                  dfg: &DataFlowGraph,
                  tracker: &mut LiveValueTracker) {
        dbg!("Inst {}, {}", dfg.display_inst(inst), self.pressure);
        // TODO: Repair constraint violations by copying input values.
        //
        // - Tied use of value that is not killed.
        // - Inconsistent uses of the same value.
        //
        // Each inserted copy may increase register pressure. Fix by spilling something not used by
        // the instruction.
        //
        // Count pressure for register uses of spilled values too.
        //
        // Finally, reset pressure state to level from before the input adjustments, minus spills.
        //
        // Spills should be removed from tracker. Otherwise they could be double-counted by
        // free_regs below.

        // Update the live value tracker with this instruction.
        let (throughs, kills, defs) = tracker.process_inst(inst, dfg, self.liveness);


        // Remove kills from the pressure tracker.
        self.free_regs(kills);

        // Make sure we have enough registers for the register defs.
        // Dead defs are included here. They need a register too.
        // No need to process call return values, they are in fixed registers.
        for op in constraints.outs {
            if op.kind != ConstraintKind::Stack {
                // Add register def to pressure, spill if needed.
                while let Err(mask) = self.pressure.take_transient(op.regclass) {
                    dbg!("Need {} reg from {} throughs", op.regclass, throughs.len());
                    self.spill_from(mask, throughs, dfg, &pos.layout);
                }
            }
        }
        self.pressure.reset_transient();

        // Restore pressure state, compute pressure with affinities from `defs`.
        // Exclude dead defs. Includes call return values.
        // This won't cause spilling.
        self.take_live_regs(defs);
    }

    // Spill a candidate from `candidates` whose top-level register class is in `mask`.
    fn spill_from<'ii, II>(&mut self,
                           mask: RegClassMask,
                           candidates: II,
                           dfg: &DataFlowGraph,
                           layout: &Layout)
        where II: IntoIterator<Item = &'ii LiveValue>
    {
        // Find the best viable spill candidate.
        //
        // The very simple strategy implemented here is to spill the value with the earliest def in
        // the reverse post-order. This strategy depends on a good reload pass to generate good
        // code.
        //
        // We know that all candidate defs dominate the current instruction, so one of them will
        // dominate the others. That is the earliest def.
        let best = candidates
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
                        self.domtree
                            .rpo_cmp(dfg.value_def(a), dfg.value_def(b), layout)
                    });

        if let Some(value) = best {
            // Found a spill candidate.
            self.spill_reg(value);
        } else {
            panic!("Ran out of registers for mask={}", mask);
        }
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
            dbg!("Spilled {}:{} -> {}", value, rc, self.pressure);
        } else {
            panic!("Cannot spill {} that was already on the stack", value);
        }
    }

    /// Process any pending spills in the `self.spills` vector.
    ///
    /// It is assumed that spills are removed from the pressure tracker immediately, see
    /// `spill_from` above.
    ///
    /// We also need to update the live range affinity and remove spilled values from the live
    /// value tracker.
    fn process_spills(&mut self, tracker: &mut LiveValueTracker) {
        if !self.spills.is_empty() {
            tracker.process_spills(|v| self.spills.contains(&v));
            self.spills.clear()
        }
    }
}
