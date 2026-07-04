//! ARM32-specific ISLE lowering glue code.

// Pull in the ISLE generated code from OUT_DIR.
pub mod generated_code;

use crate::isa::arm32::Arm32Backend;
use crate::isa::arm32::inst::*;

use crate::ir::{
    BlockCall, ExternalName, Inst, InstructionData, MemFlagsData, Opcode, TrapCode, Value,
    ValueList, immediates::*, types::*,
};
use crate::machinst::{
    CallArgList, CallRetList, InstOutput, MachInst, Reg, TryCallInfo, VCodeConstant,
    VCodeConstantData, isle::*,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PReg;

/// Re-exported Inst type (from ISLE-generated code) for use by callers.
pub(crate) type MInst = generated_code::MInst;

// type BoxCallInfo = Box<CallInfo<ExternalName>>;
// type BoxCallIndInfo = Box<CallInfo<Reg>>;
// type BoxReturnCallInfo = Box<CallInfo<Box<ExternalName>>>;
type BoxExternalName = Box<ExternalName>;
// type VecMachLabel = Vec<MachLabel>;
// type VecArgPair = Vec<ArgPair>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<generated_code::MInst>,
    backend: &Arm32Backend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<generated_code::MInst>,
    backend: &Arm32Backend,
    _branch: Inst,
    _targets: &[MachLabel],
) -> Option<()> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let _isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    //    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
    todo!()
}

pub(crate) struct Arm32IsleContext<'a, 'b> {
    pub lower_ctx: &'a mut Lower<'b, generated_code::MInst>,
    pub backend: &'a Arm32Backend,
}

impl<'a, 'b> Arm32IsleContext<'a, 'b> {
    fn new(lower_ctx: &'a mut Lower<'b, generated_code::MInst>, backend: &'a Arm32Backend) -> Self {
        Self { lower_ctx, backend }
    }

    pub(crate) fn dfg(&self) -> &crate::ir::DataFlowGraph {
        &self.lower_ctx.f.dfg
    }
}

impl generated_code::Context for Arm32IsleContext<'_, '_> {
    isle_lower_prelude_methods!();

    fn emit(&mut self, inst: &generated_code::MInst) -> Unit {
        self.lower_ctx.emit(*inst);
    }
}
