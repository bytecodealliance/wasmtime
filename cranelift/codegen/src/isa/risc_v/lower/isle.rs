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

    fn imm(&mut self, arg0: Type, arg1: u64) -> Reg {
        let tmp = self.temp_writable_reg(arg0);
        self.emit_list(&MInst::load_constant_u64(tmp, arg1));
        tmp.to_reg()
    }

    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }

    // fn emit_safepoint(&mut self, arg0: &MInst) -> Unit {
    //     self.emitted_insts.push((arg0.clone(), true));
    // }

    fn imm12_from_u64(&mut self, arg0: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64(arg0)
    }

    fn writable_zero_reg(&mut self) -> WritableReg {
        writable_zero_reg()
    }
    fn neg_imm12(&mut self, arg0: Imm12) -> Imm12 {
        -arg0
    }
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }
}

impl<C> IsleContext<'_, C, Flags, IsaFlags, 6>
where
    C: LowerCtx<I = MInst>,
{
    fn emit_list(&mut self, list: &SmallInstVec<MInst>) {
        for i in list {
            self.lower_ctx.emit(i.clone());
        }
    }

    // i128 implemetation
}

// struct TestContext {
//     lower_ctx: TestContextLowerCtx,
// }
// struct TestContextLowerCtx {
//     cfg: TestContextLowerCtxCfg,
// }

// impl TestContextLowerCtx {
//     fn dfg(&self) -> &TestContextLowerCtxCfg {
//         &self.cfg
//     }
//     fn symbol_value_data(&mut self, global_value: GlobalValue) {
//         unimplemented!()
//     }
// }
// struct TestContextLowerCtxCfg {
//     value: ValueList,
// }

// impl TestContextLowerCtxCfg {
//     fn inst_results_list(&self, i: Inst) -> ValueList {
//         self.value
//         ext_funcs : Vec[],
//     }
//     fn (&self , , func_ref: FuncRef)
// }

// impl generated_code::Context for TestContext {
//     isle_prelude_methods!();

//     fn emit(&mut self, arg0: &MInst) -> Unit {
//         todo!()
//     }

//     fn emit_safepoint(&mut self, arg0: &MInst) -> Unit {
//         todo!()
//     }

//     fn zero_reg(&mut self) -> Reg {
//         todo!()
//     }

//     fn imm(&mut self, arg0: Type, arg1: u64) -> Reg {
//         todo!()
//     }

//     fn imm12_from_u64(&mut self, arg0: u64) -> Option<Imm12> {
//         todo!()
//     }

//     fn writable_zero_reg(&mut self) -> WritableReg {
//         todo!()
//     }

//     fn neg_imm12(&mut self, arg0: Imm12) -> Imm12 {
//         todo!()
//     }
// }

#[cfg(test)]
mod test {}
