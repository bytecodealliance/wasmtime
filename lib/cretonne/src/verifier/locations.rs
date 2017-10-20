//! Verify value locations.

use ir;
use isa;
use regalloc::RegDiversions;
use regalloc::liveness::Liveness;
use verifier::Result;

/// Verify value locations for `func`.
///
/// After register allocation, every value must be assigned to a location - either a register or a
/// stack slot. These locations must be compatible with the constraints described by the
/// instruction encoding recipes.
///
/// Values can be temporarily diverted to a different location by using the `regmove`, `regspill`,
/// and `regfill` instructions, but only inside an EBB.
///
/// If a liveness analysis is provided, it is used to verify that there are no active register
/// diversions across control flow edges.
pub fn verify_locations(
    isa: &isa::TargetIsa,
    func: &ir::Function,
    liveness: Option<&Liveness>,
) -> Result {
    let verifier = LocationVerifier {
        isa,
        func,
        reginfo: isa.register_info(),
        encinfo: isa.encoding_info(),
        liveness,
    };
    verifier.check_constraints()?;
    Ok(())
}

struct LocationVerifier<'a> {
    isa: &'a isa::TargetIsa,
    func: &'a ir::Function,
    reginfo: isa::RegInfo,
    encinfo: isa::EncInfo,
    liveness: Option<&'a Liveness>,
}

impl<'a> LocationVerifier<'a> {
    /// Check that the assigned value locations match the operand constraints of their uses.
    fn check_constraints(&self) -> Result {
        let dfg = &self.func.dfg;
        let mut divert = RegDiversions::new();

        for ebb in self.func.layout.ebbs() {
            // Diversions are reset at the top of each EBB. No diversions can exist across control
            // flow edges.
            divert.clear();
            for inst in self.func.layout.ebb_insts(ebb) {
                let enc = self.func.encodings[inst];

                if enc.is_legal() {
                    self.check_enc_constraints(inst, enc, &divert)?
                } else {
                    self.check_ghost_results(inst)?;
                }

                if let Some(sig) = dfg.call_signature(inst) {
                    self.check_call_abi(inst, sig, &divert)?;
                }

                let opcode = dfg[inst].opcode();
                if opcode.is_return() {
                    self.check_return_abi(inst, &divert)?;
                }

                if opcode.is_branch() && !divert.is_empty() {
                    self.check_cfg_edges(inst, &divert)?;
                }

                self.update_diversions(inst, &mut divert)?;
            }
        }

        Ok(())
    }

    /// Check encoding constraints against the current value locations.
    fn check_enc_constraints(
        &self,
        inst: ir::Inst,
        enc: isa::Encoding,
        divert: &RegDiversions,
    ) -> Result {
        let constraints = self.encinfo.operand_constraints(enc).expect(
            "check_enc_constraints requires a legal encoding",
        );

        if constraints.satisfied(inst, divert, self.func) {
            return Ok(());
        }

        // TODO: We could give a better error message here.
        err!(
            inst,
            "{} constraints not satisfied",
            self.encinfo.display(enc)
        )
    }

    /// Check that the result values produced by a ghost instruction are not assigned a value
    /// location.
    fn check_ghost_results(&self, inst: ir::Inst) -> Result {
        let results = self.func.dfg.inst_results(inst);

        for &res in results {
            let loc = self.func.locations[res];
            if loc.is_assigned() {
                return err!(
                    inst,
                    "ghost result {} value must not have a location ({}).",
                    res,
                    loc.display(&self.reginfo)
                );
            }
        }

        Ok(())
    }

    /// Check the ABI argument and result locations for a call.
    fn check_call_abi(&self, inst: ir::Inst, sig: ir::SigRef, divert: &RegDiversions) -> Result {
        let sig = &self.func.dfg.signatures[sig];
        let varargs = self.func.dfg.inst_variable_args(inst);
        let results = self.func.dfg.inst_results(inst);

        for (abi, &value) in sig.params.iter().zip(varargs) {
            self.check_abi_location(
                inst,
                value,
                abi,
                divert.get(value, &self.func.locations),
                ir::StackSlotKind::OutgoingArg,
            )?;
        }

        for (abi, &value) in sig.returns.iter().zip(results) {
            self.check_abi_location(
                inst,
                value,
                abi,
                self.func.locations[value],
                ir::StackSlotKind::OutgoingArg,
            )?;
        }

        Ok(())
    }

    /// Check the ABI argument locations for a return.
    fn check_return_abi(&self, inst: ir::Inst, divert: &RegDiversions) -> Result {
        let sig = &self.func.signature;
        let varargs = self.func.dfg.inst_variable_args(inst);

        for (abi, &value) in sig.returns.iter().zip(varargs) {
            self.check_abi_location(
                inst,
                value,
                abi,
                divert.get(value, &self.func.locations),
                ir::StackSlotKind::IncomingArg,
            )?;
        }

        Ok(())
    }

    /// Check a single ABI location.
    fn check_abi_location(
        &self,
        inst: ir::Inst,
        value: ir::Value,
        abi: &ir::AbiParam,
        loc: ir::ValueLoc,
        want_kind: ir::StackSlotKind,
    ) -> Result {
        match abi.location {
            ir::ArgumentLoc::Unassigned => {}
            ir::ArgumentLoc::Reg(reg) => {
                if loc != ir::ValueLoc::Reg(reg) {
                    return err!(
                        inst,
                        "ABI expects {} in {}, got {}",
                        value,
                        abi.location.display(&self.reginfo),
                        loc.display(&self.reginfo)
                    );
                }
            }
            ir::ArgumentLoc::Stack(offset) => {
                if let ir::ValueLoc::Stack(ss) = loc {
                    let slot = &self.func.stack_slots[ss];
                    if slot.kind != want_kind {
                        return err!(
                            inst,
                            "call argument {} should be in a {} slot, but {} is {}",
                            value,
                            want_kind,
                            ss,
                            slot.kind
                        );
                    }
                    if slot.offset != offset {
                        return err!(
                            inst,
                            "ABI expects {} at stack offset {}, but {} is at {}",
                            value,
                            offset,
                            ss,
                            slot.offset
                        );
                    }
                } else {
                    return err!(
                        inst,
                        "ABI expects {} at stack offset {}, got {}",
                        value,
                        offset,
                        loc.display(&self.reginfo)
                    );
                }
            }
        }

        Ok(())
    }

    /// Update diversions to reflect the current instruction and check their consistency.
    fn update_diversions(&self, inst: ir::Inst, divert: &mut RegDiversions) -> Result {
        let (arg, src) = match self.func.dfg[inst] {
            ir::InstructionData::RegMove { arg, src, .. } => (arg, ir::ValueLoc::Reg(src)),
            ir::InstructionData::RegSpill { arg, src, .. } => (arg, ir::ValueLoc::Reg(src)),
            ir::InstructionData::RegFill { arg, src, .. } => (arg, ir::ValueLoc::Stack(src)),
            _ => return Ok(()),
        };

        if let Some(d) = divert.diversion(arg) {
            if d.to != src {
                return err!(
                    inst,
                    "inconsistent with current diversion to {}",
                    d.to.display(&self.reginfo)
                );
            }
        } else if self.func.locations[arg] != src {
            return err!(
                inst,
                "inconsistent with global location {}",
                self.func.locations[arg].display(&self.reginfo)
            );
        }

        divert.apply(&self.func.dfg[inst]);

        Ok(())
    }

    /// We have active diversions before a branch. Make sure none of the diverted values are live
    /// on the outgoing CFG edges.
    fn check_cfg_edges(&self, inst: ir::Inst, divert: &RegDiversions) -> Result {
        use ir::instructions::BranchInfo::*;

        // We can only check CFG edges if we have a liveness analysis.
        let liveness = match self.liveness {
            Some(l) => l,
            None => return Ok(()),
        };
        let dfg = &self.func.dfg;

        match dfg[inst].analyze_branch(&dfg.value_lists) {
            NotABranch => {
                panic!(
                    "No branch information for {}",
                    dfg.display_inst(inst, self.isa)
                )
            }
            SingleDest(ebb, _) => {
                for d in divert.all() {
                    let lr = &liveness[d.value];
                    if lr.is_livein(ebb, &self.func.layout) {
                        return err!(
                            inst,
                            "{} is diverted to {} and live in to {}",
                            d.value,
                            d.to.display(&self.reginfo),
                            ebb
                        );
                    }
                }
            }
            Table(jt) => {
                for d in divert.all() {
                    let lr = &liveness[d.value];
                    for (_, ebb) in self.func.jump_tables[jt].entries() {
                        if lr.is_livein(ebb, &self.func.layout) {
                            return err!(
                                inst,
                                "{} is diverted to {} and live in to {}",
                                d.value,
                                d.to.display(&self.reginfo),
                                ebb
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
