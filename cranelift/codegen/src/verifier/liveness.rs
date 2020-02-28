//! Liveness verifier.

use crate::flowgraph::{BlockPredecessor, ControlFlowGraph};
use crate::ir::entities::AnyEntity;
use crate::ir::{ExpandedProgramPoint, Function, ProgramPoint, Value};
use crate::isa::TargetIsa;
use crate::regalloc::liveness::Liveness;
use crate::regalloc::liverange::LiveRange;
use crate::timing;
use crate::verifier::{VerifierErrors, VerifierStepResult};

/// Verify liveness information for `func`.
///
/// The provided control flow graph is assumed to be sound.
///
/// - All values in the program must have a live range.
/// - The live range def point must match where the value is defined.
/// - The live range must reach all uses.
/// - When a live range is live-in to an block, it must be live at all the predecessors.
/// - The live range affinity must be compatible with encoding constraints.
///
/// We don't verify that live ranges are minimal. This would require recomputing live ranges for
/// all values.
pub fn verify_liveness(
    isa: &dyn TargetIsa,
    func: &Function,
    cfg: &ControlFlowGraph,
    liveness: &Liveness,
    errors: &mut VerifierErrors,
) -> VerifierStepResult<()> {
    let _tt = timing::verify_liveness();
    let verifier = LivenessVerifier {
        isa,
        func,
        cfg,
        liveness,
    };
    verifier.check_blocks(errors)?;
    verifier.check_insts(errors)?;
    Ok(())
}

struct LivenessVerifier<'a> {
    isa: &'a dyn TargetIsa,
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
    liveness: &'a Liveness,
}

impl<'a> LivenessVerifier<'a> {
    /// Check all block arguments.
    fn check_blocks(&self, errors: &mut VerifierErrors) -> VerifierStepResult<()> {
        for block in self.func.layout.blocks() {
            for &val in self.func.dfg.block_params(block) {
                let lr = match self.liveness.get(val) {
                    Some(lr) => lr,
                    None => {
                        return errors
                            .fatal((block, format!("block arg {} has no live range", val)))
                    }
                };
                self.check_lr(block.into(), val, lr, errors)?;
            }
        }
        Ok(())
    }

    /// Check all instructions.
    fn check_insts(&self, errors: &mut VerifierErrors) -> VerifierStepResult<()> {
        for block in self.func.layout.blocks() {
            for inst in self.func.layout.block_insts(block) {
                let encoding = self.func.encodings[inst];

                // Check the defs.
                for &val in self.func.dfg.inst_results(inst) {
                    let lr = match self.liveness.get(val) {
                        Some(lr) => lr,
                        None => return errors.fatal((inst, format!("{} has no live range", val))),
                    };
                    self.check_lr(inst.into(), val, lr, errors)?;

                    if encoding.is_legal() {
                        // A legal instruction is not allowed to define ghost values.
                        if lr.affinity.is_unassigned() {
                            return errors.fatal((
                                inst,
                                format!(
                                    "{} is a ghost value defined by a real [{}] instruction",
                                    val,
                                    self.isa.encoding_info().display(encoding)
                                ),
                            ));
                        }
                    } else if !lr.affinity.is_unassigned() {
                        // A non-encoded instruction can only define ghost values.
                        return errors.fatal((
                            inst,
                            format!(
                                "{} is a real {} value defined by a ghost instruction",
                                val,
                                lr.affinity.display(&self.isa.register_info())
                            ),
                        ));
                    }
                }

                // Check the uses.
                for &val in self.func.dfg.inst_args(inst) {
                    let lr = match self.liveness.get(val) {
                        Some(lr) => lr,
                        None => return errors.fatal((inst, format!("{} has no live range", val))),
                    };

                    debug_assert!(self.func.layout.inst_block(inst).unwrap() == block);
                    if !lr.reaches_use(inst, block, &self.func.layout) {
                        return errors.fatal((inst, format!("{} is not live at this use", val)));
                    }

                    // A legal instruction is not allowed to depend on ghost values.
                    if encoding.is_legal() && lr.affinity.is_unassigned() {
                        return errors.fatal((
                            inst,
                            format!(
                                "{} is a ghost value used by a real [{}] instruction",
                                val,
                                self.isa.encoding_info().display(encoding),
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check the integrity of the live range `lr`.
    fn check_lr(
        &self,
        def: ProgramPoint,
        val: Value,
        lr: &LiveRange,
        errors: &mut VerifierErrors,
    ) -> VerifierStepResult<()> {
        let l = &self.func.layout;

        let loc: AnyEntity = match def.into() {
            ExpandedProgramPoint::Block(e) => e.into(),
            ExpandedProgramPoint::Inst(i) => i.into(),
        };
        if lr.def() != def {
            return errors.fatal((
                loc,
                format!("Wrong live range def ({}) for {}", lr.def(), val),
            ));
        }
        if lr.is_dead() {
            if !lr.is_local() {
                return errors.fatal((loc, format!("Dead live range {} should be local", val)));
            } else {
                return Ok(());
            }
        }
        let def_block = match def.into() {
            ExpandedProgramPoint::Block(e) => e,
            ExpandedProgramPoint::Inst(i) => l.inst_block(i).unwrap(),
        };
        match lr.def_local_end().into() {
            ExpandedProgramPoint::Block(e) => {
                return errors.fatal((
                    loc,
                    format!("Def local range for {} can't end at {}", val, e),
                ));
            }
            ExpandedProgramPoint::Inst(i) => {
                if self.func.layout.inst_block(i) != Some(def_block) {
                    return errors
                        .fatal((loc, format!("Def local end for {} in wrong block", val)));
                }
            }
        }

        // Now check the live-in intervals against the CFG.
        for (mut block, end) in lr.liveins() {
            if !l.is_block_inserted(block) {
                return errors.fatal((
                    loc,
                    format!("{} livein at {} which is not in the layout", val, block),
                ));
            }
            let end_block = match l.inst_block(end) {
                Some(e) => e,
                None => {
                    return errors.fatal((
                        loc,
                        format!(
                            "{} livein for {} ends at {} which is not in the layout",
                            val, block, end
                        ),
                    ));
                }
            };

            // Check all the blocks in the interval independently.
            loop {
                // If `val` is live-in at `block`, it must be live at all the predecessors.
                for BlockPredecessor { inst: pred, block } in self.cfg.pred_iter(block) {
                    if !lr.reaches_use(pred, block, &self.func.layout) {
                        return errors.fatal((
                            pred,
                            format!(
                                "{} is live in to {} but not live at predecessor",
                                val, block
                            ),
                        ));
                    }
                }

                if block == end_block {
                    break;
                }
                block = match l.next_block(block) {
                    Some(e) => e,
                    None => {
                        return errors.fatal((
                            loc,
                            format!("end of {} livein ({}) never reached", val, end_block),
                        ));
                    }
                };
            }
        }

        Ok(())
    }
}
