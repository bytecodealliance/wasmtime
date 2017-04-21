//! Liveness verifier.

use flowgraph::ControlFlowGraph;
use ir::{Function, Inst, Value, ProgramOrder, ProgramPoint, ExpandedProgramPoint};
use ir::entities::AnyEntity;
use isa::TargetIsa;
use regalloc::liveness::Liveness;
use regalloc::liverange::LiveRange;
use std::cmp::Ordering;
use verifier::Result;

/// Verify liveness information for `func`.
///
/// The provided control flow graph is assumed to be sound.
///
/// - All values in the program must have a live range.
/// - The live range def point must match where the value is defined.
/// - The live range must reach all uses.
/// - When a live range is live-in to an EBB, it must be live at all the predecessors.
/// - The live range affinity must be compatible with encoding constraints.
///
/// We don't verify that live ranges are minimal. This would require recomputing live ranges for
/// all values.
pub fn verify_liveness(_isa: &TargetIsa,
                       func: &Function,
                       cfg: &ControlFlowGraph,
                       liveness: &Liveness)
                       -> Result {
    let verifier = LivenessVerifier {
        func: func,
        cfg: cfg,
        liveness: liveness,
    };
    verifier.check_ebbs()?;
    verifier.check_insts()?;
    Ok(())
}

struct LivenessVerifier<'a> {
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
    liveness: &'a Liveness,
}

impl<'a> LivenessVerifier<'a> {
    /// Check all EBB arguments.
    fn check_ebbs(&self) -> Result {
        for ebb in self.func.layout.ebbs() {
            for &val in self.func.dfg.ebb_args(ebb) {
                let lr = match self.liveness.get(val) {
                    Some(lr) => lr,
                    None => return err!(ebb, "EBB arg {} has no live range", val),
                };
                self.check_lr(ebb.into(), val, lr)?;
            }
        }
        Ok(())
    }

    /// Check all instructions.
    fn check_insts(&self) -> Result {
        for ebb in self.func.layout.ebbs() {
            for inst in self.func.layout.ebb_insts(ebb) {
                // Check the defs.
                for &val in self.func.dfg.inst_results(inst) {
                    let lr = match self.liveness.get(val) {
                        Some(lr) => lr,
                        None => return err!(inst, "{} has no live range", val),
                    };
                    self.check_lr(inst.into(), val, lr)?;
                }

                // Check the uses.
                for &val in self.func.dfg.inst_args(inst) {
                    let lr = match self.liveness.get(val) {
                        Some(lr) => lr,
                        None => return err!(inst, "{} has no live range", val),
                    };
                    if !self.live_at_use(lr, inst) {
                        return err!(inst, "{} is not live at this use", val);
                    }
                }
            }
        }
        Ok(())
    }

    /// Is `lr` live at the use `inst`?
    fn live_at_use(&self, lr: &LiveRange, inst: Inst) -> bool {
        let l = &self.func.layout;

        // Check if `inst` is in the def range, not including the def itself.
        if l.cmp(lr.def(), inst) == Ordering::Less &&
           l.cmp(inst, lr.def_local_end()) != Ordering::Greater {
            return true;
        }

        // Otherwise see if `inst` is in one of the live-in ranges.
        match lr.livein_local_end(l.inst_ebb(inst).unwrap(), l) {
            Some(end) => l.cmp(inst, end) != Ordering::Greater,
            None => false,
        }
    }

    /// Check the integrity of the live range `lr`.
    fn check_lr(&self, def: ProgramPoint, val: Value, lr: &LiveRange) -> Result {
        let l = &self.func.layout;

        let loc: AnyEntity = match def.into() {
            ExpandedProgramPoint::Ebb(e) => e.into(),
            ExpandedProgramPoint::Inst(i) => i.into(),
        };
        if lr.def() != def {
            return err!(loc, "Wrong live range def ({}) for {}", lr.def(), val);
        }
        if lr.is_dead() {
            if !lr.is_local() {
                return err!(loc, "Dead live range {} should be local", val);
            } else {
                return Ok(());
            }
        }
        let def_ebb = match def.into() {
            ExpandedProgramPoint::Ebb(e) => e,
            ExpandedProgramPoint::Inst(i) => l.inst_ebb(i).unwrap(),
        };
        match lr.def_local_end().into() {
            ExpandedProgramPoint::Ebb(e) => {
                return err!(loc, "Def local range for {} can't end at {}", val, e)
            }
            ExpandedProgramPoint::Inst(i) => {
                if self.func.layout.inst_ebb(i) != Some(def_ebb) {
                    return err!(loc, "Def local end for {} in wrong ebb", val);
                }
            }
        }

        // Now check the live-in intervals against the CFG.
        for &livein in lr.liveins() {
            let mut ebb = livein.begin;
            if !l.is_ebb_inserted(ebb) {
                return err!(loc, "{} livein at {} which is not in the layout", val, ebb);
            }
            let end_ebb = match l.inst_ebb(livein.end) {
                Some(e) => e,
                None => {
                    return err!(loc,
                                "{} livein for {} ends at {} which is not in the layout",
                                val,
                                ebb,
                                livein.end)
                }
            };

            // Check all the EBBs in the interval independently.
            loop {
                // If `val` is live-in at `ebb`, it must be live at all the predecessors.
                for &(_, pred) in self.cfg.get_predecessors(ebb) {
                    if !self.live_at_use(lr, pred) {
                        return err!(pred,
                                    "{} is live in to {} but not live at predecessor",
                                    val,
                                    ebb);
                    }
                }

                if ebb == end_ebb {
                    break;
                }
                ebb = match l.next_ebb(ebb) {
                    Some(e) => e,
                    None => return err!(loc, "end of {} livein ({}) never reached", val, end_ebb),
                };
            }
        }

        Ok(())
    }
}
