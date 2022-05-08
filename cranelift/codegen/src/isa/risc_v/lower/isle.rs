//! ISLE integration glue code for risc_v lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{writable_zero_reg, zero_reg, Inst as MInst};

use crate::isa::risc_v::settings::Flags as IsaFlags;
use crate::machinst::{isle::*, SmallInstVec};
use crate::settings::Flags;

use crate::{
    ir::{
        immediates::*, types::*, ExternalName, Inst, InstructionData, MemFlags, TrapCode, Value,
        ValueList,
    },
    isa::risc_v::inst::*,
    machinst::{InsnOutput, LowerCtx},
};

use std::boxed::Box;
use std::convert::TryFrom;
use std::vec::Vec;

use crate::machinst::Reg;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type VecMachLabel = Vec<MachLabel>;
type BoxExternalName = Box<ExternalName>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    flags: &Flags,
    isa_flags: &IsaFlags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()>
where
    C: LowerCtx<I = MInst>,
{
    lower_common(lower_ctx, flags, isa_flags, outputs, inst, |cx, insn| {
        generated_code::constructor_lower(cx, insn)
    })
}

impl<C> generated_code::Context for IsleContext<'_, C, Flags, IsaFlags, 6>
where
    C: LowerCtx<I = MInst>,
{
    isle_prelude_methods!();

    fn imm(&mut self, ty: Type, mut val: u64) -> Reg {
        /*
        Boolean types
        Boolean values are either true or false.

        The b1 type represents an abstract boolean value. It can only exist as an SSA value, and can't be directly stored in memory. It can, however, be converted into an integer with value 0 or 1 by the bint instruction (and converted back with icmp_imm with 0).

        Several larger boolean types are also defined, primarily to be used as SIMD element types. They can be stored in memory, and are represented as either all zero bits or all one bits.

        b1
        b8
        b16
        b32
        b64
        ///////////////////////////////////////////////////////////
        "represented as either all zero bits or all one bits."
        \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
                        */
        if ty.is_bool() && val != 0 {
            // need all be one
            val = !0;
        }

        let tmp = self.temp_writable_reg(ty);
        self.emit_list(&MInst::load_constant_u64(tmp, val));
        tmp.to_reg()
    }
    #[inline(always)]
    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }
    #[inline(always)]
    fn imm12_from_u64(&mut self, arg0: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64(arg0)
    }
    #[inline(always)]
    fn writable_zero_reg(&mut self) -> WritableReg {
        writable_zero_reg()
    }
    #[inline(always)]
    fn neg_imm12(&mut self, arg0: Imm12) -> Imm12 {
        -arg0
    }
    #[inline(always)]
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }
}

impl<C> IsleContext<'_, C, Flags, IsaFlags, 6>
where
    C: LowerCtx<I = MInst>,
{
    #[inline(always)]
    fn emit_list(&mut self, list: &SmallInstVec<MInst>) {
        for i in list {
            self.lower_ctx.emit(i.clone());
        }
    }

    // i128 implemetation
}

#[cfg(test)]
mod test {}
