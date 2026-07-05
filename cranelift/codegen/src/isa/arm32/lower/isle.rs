//! ARM32-specific ISLE lowering glue code.

// Pull in the ISLE generated code from OUT_DIR.
pub mod generated_code;

use crate::alloc::{boxed::Box, vec::Vec};
// many of these imports are used by the generated code
use crate::ir::{
    BlockCall, ExternalName, Inst, InstructionData, MemFlags, MemFlagsData, Opcode, TrapCode,
    Value, ValueList,
    condcodes::{FloatCC, IntCC},
    immediates::*,
    types::*,
};
use crate::isa::arm32::Arm32Backend;
use crate::machinst::{
    ArgPair, CallArgList, CallRetList, InstOutput, MachInst, Reg, TryCallInfo, VCodeConstant,
    VCodeConstantData, isle::*,
};
use regalloc2::PReg;

/// Re-exported Inst type (from ISLE-generated code) for use by callers.
pub(crate) type MInst = generated_code::MInst;

type BoxExternalName = Box<ExternalName>;
type VecArgPair = Vec<ArgPair>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<generated_code::MInst>,
    backend: &Arm32Backend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext { lower_ctx, backend };
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    _lower_ctx: &mut Lower<generated_code::MInst>,
    _backend: &Arm32Backend,
    _branch: Inst,
    _targets: &[MachLabel],
) -> Option<()> {
    todo!()
}

impl generated_code::Context for IsleContext<'_, '_, MInst, Arm32Backend> {
    isle_lower_prelude_methods!();

    fn emit(&mut self, inst: &generated_code::MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }
}
