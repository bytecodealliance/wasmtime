//! ISLE integration glue code for arm32 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;
use generated_code::MInst;

// Types that the generated ISLE code refers to via `use super::*`.
use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::immediates::*;
use crate::ir::types::*;
use crate::ir::{
    BlockCall, ExternalName, Inst, InstructionData, MemFlags, Opcode, TrapCode, Value, ValueList,
};
use crate::isa::arm32::Arm32Backend;
use crate::isa::arm32::inst::{Cond, encode_rotated_imm};
use crate::machinst::isle::*;
use crate::machinst::{
    ArgPair, CallArgList, CallInfo, CallRetList, InstOutput, Lower, MachInst, MachLabel, RetPair,
    VCodeConstant, VCodeConstantData, VCodeInst,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PReg;

type VecArgPair = Vec<ArgPair>;
type VecRetPair = Vec<RetPair>;
type BoxCallInfo = Box<CallInfo<ExternalName>>;

/// The ISLE lowering context for arm32.
pub(crate) struct Arm32IsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    #[allow(dead_code, reason = "kept for symmetry with other backends")]
    pub backend: &'a B,
}

impl<'a, 'b> Arm32IsleContext<'a, 'b, MInst, Arm32Backend> {
    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a Arm32Backend) -> Self {
        Self { lower_ctx, backend }
    }

    pub(crate) fn dfg(&self) -> &crate::ir::DataFlowGraph {
        &self.lower_ctx.f.dfg
    }
}

impl generated_code::Context for Arm32IsleContext<'_, '_, MInst, Arm32Backend> {
    isle_lower_prelude_methods!();

    fn emit(&mut self, inst: &MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }

    /// Materialize a 32-bit constant into a register with the shortest sequence.
    fn gen_constant(&mut self, val: u64) -> Reg {
        let val = val as u32;
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        let inst = if let Some(imm12) = encode_rotated_imm(val) {
            MInst::MovRotImm { rd, imm12 }
        } else if let Some(imm12) = encode_rotated_imm(!val) {
            MInst::MvnRotImm { rd, imm12 }
        } else if val >> 16 == 0 {
            MInst::Movw {
                rd,
                imm16: val,
            }
        } else {
            MInst::MovImm {
                rd,
                imm: u64::from(val),
            }
        };
        self.lower_ctx.emit(inst);
        rd.to_reg()
    }

    /// Succeeds if the low 32 bits of `val` are encodable as a rotated imm12.
    fn u64_from_rotated_imm12(&mut self, val: u64) -> Option<u32> {
        encode_rotated_imm(val as u32)
    }

    fn cond_from_intcc(&mut self, cc: &IntCC) -> Cond {
        match cc {
            IntCC::Equal => Cond::Eq,
            IntCC::NotEqual => Cond::Ne,
            IntCC::SignedLessThan => Cond::Lt,
            IntCC::SignedGreaterThanOrEqual => Cond::Ge,
            IntCC::SignedGreaterThan => Cond::Gt,
            IntCC::SignedLessThanOrEqual => Cond::Le,
            IntCC::UnsignedLessThan => Cond::Lo,
            IntCC::UnsignedGreaterThanOrEqual => Cond::Hs,
            IntCC::UnsignedGreaterThan => Cond::Hi,
            IntCC::UnsignedLessThanOrEqual => Cond::Ls,
        }
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &Arm32Backend,
    inst: Inst,
) -> Option<InstOutput> {
    let mut isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &Arm32Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    let mut isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
}
