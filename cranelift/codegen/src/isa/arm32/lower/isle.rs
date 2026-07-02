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
use crate::machinst::isle::*;
use crate::machinst::{
    ArgPair, CallArgList, CallRetList, InstOutput, Lower, MachInst, MachLabel, RetPair,
    VCodeConstant, VCodeConstantData, VCodeInst,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PReg;

type VecArgPair = Vec<ArgPair>;
type VecRetPair = Vec<RetPair>;

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
