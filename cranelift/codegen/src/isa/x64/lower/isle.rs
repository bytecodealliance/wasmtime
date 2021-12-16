//! ISLE integration glue code for x64 lowering.

// Pull in the ISLE generated code.
mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{
    is_mergeable_load, lower_to_amode, AluRmiROpcode, Inst as MInst, OperandSize, Reg, RegMemImm,
};
use crate::isa::x64::inst::args::SyntheticAmode;
use crate::isa::x64::inst::regs;
use crate::isa::x64::settings::Flags;
use crate::machinst::isle::*;
use crate::{
    ir::{immediates::*, types::*, Inst, InstructionData, Opcode, TrapCode, Value, ValueList},
    isa::x64::inst::{
        args::{
            Avx512Opcode, CmpOpcode, ExtMode, FcmpImm, Imm8Reg, RegMem, ShiftKind, SseOpcode, CC,
        },
        x64_map_regs,
    },
    machinst::{InsnInput, InsnOutput, LowerCtx},
};
use std::convert::TryFrom;

pub struct SinkableLoad {
    inst: Inst,
    addr_input: InsnInput,
    offset: i32,
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    isa_flags: &Flags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()>
where
    C: LowerCtx<I = MInst>,
{
    lower_common(
        lower_ctx,
        isa_flags,
        outputs,
        inst,
        |cx, insn| generated_code::constructor_lower(cx, insn),
        x64_map_regs,
    )
}

impl<C> generated_code::Context for IsleContext<'_, C, Flags, 6>
where
    C: LowerCtx<I = MInst>,
{
    isle_prelude_methods!();

    #[inline]
    fn operand_size_of_type_32_64(&mut self, ty: Type) -> OperandSize {
        if ty.bits() == 64 {
            OperandSize::Size64
        } else {
            OperandSize::Size32
        }
    }

    #[inline]
    fn raw_operand_size_of_type(&mut self, ty: Type) -> OperandSize {
        OperandSize::from_ty(ty)
    }

    fn put_in_reg_mem(&mut self, val: Value) -> RegMem {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            // Generate constants fresh at each use to minimize long-range
            // register pressure.
            let ty = self.value_type(val);
            return RegMem::reg(generated_code::constructor_imm(self, ty, c).unwrap());
        }

        if let Some((src_insn, 0)) = inputs.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, src_insn) {
                self.lower_ctx.sink_inst(src_insn);
                let amode = lower_to_amode(self.lower_ctx, addr_input, offset);
                return RegMem::mem(amode);
            }
        }

        RegMem::reg(self.put_in_reg(val))
    }

    #[inline]
    fn encode_fcmp_imm(&mut self, imm: &FcmpImm) -> u8 {
        imm.encode()
    }

    #[inline]
    fn avx512vl_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512vl_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn avx512dq_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512dq_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn avx512f_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512f_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn imm8_from_value(&mut self, val: Value) -> Option<Imm8Reg> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant = self.lower_ctx.get_constant(inst)?;
        let imm = u8::try_from(constant).ok()?;
        Some(Imm8Reg::Imm8 { imm })
    }

    #[inline]
    fn mask_imm8_const(&mut self, imm8: &Imm8Reg, mask: u64) -> Imm8Reg {
        match imm8 {
            &Imm8Reg::Reg { reg } => Imm8Reg::Reg { reg },
            &Imm8Reg::Imm8 { imm } => Imm8Reg::Imm8 {
                imm: imm & (mask as u8),
            },
        }
    }

    #[inline]
    fn simm32_from_value(&mut self, val: Value) -> Option<RegMemImm> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant: u64 = self.lower_ctx.get_constant(inst)?;
        let constant = constant as i64;
        to_simm32(constant)
    }

    #[inline]
    fn simm32_from_imm64(&mut self, imm: Imm64) -> Option<RegMemImm> {
        to_simm32(imm.bits())
    }

    fn sinkable_load(&mut self, val: Value) -> Option<SinkableLoad> {
        let input = self.lower_ctx.get_value_as_source_or_const(val);
        if let Some((inst, 0)) = input.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, inst) {
                return Some(SinkableLoad {
                    inst,
                    addr_input,
                    offset,
                });
            }
        }
        None
    }

    fn sink_load(&mut self, load: &SinkableLoad) -> RegMemImm {
        self.lower_ctx.sink_inst(load.inst);
        let addr = lower_to_amode(self.lower_ctx, load.addr_input, load.offset);
        RegMemImm::Mem {
            addr: SyntheticAmode::Real(addr),
        }
    }

    #[inline]
    fn ext_mode(&mut self, from_bits: u16, to_bits: u16) -> ExtMode {
        ExtMode::new(from_bits, to_bits).unwrap()
    }

    fn emit(&mut self, inst: &MInst) -> Unit {
        for inst in inst.clone().mov_mitosis() {
            self.emitted_insts.push(inst);
        }
    }

    #[inline]
    fn nonzero_u64_fits_in_u32(&mut self, x: u64) -> Option<u64> {
        if x != 0 && x < u64::from(u32::MAX) {
            Some(x)
        } else {
            None
        }
    }

    #[inline]
    fn sse_insertps_lane_imm(&mut self, lane: u8) -> u8 {
        // Insert 32-bits from replacement (at index 00, bits 7:8) to vector (lane
        // shifted into bits 5:6).
        0b00_00_00_00 | lane << 4
    }

    #[inline]
    fn xmm0(&mut self) -> WritableReg {
        WritableReg::from_reg(regs::xmm0())
    }
}

#[inline]
fn to_simm32(constant: i64) -> Option<RegMemImm> {
    if constant == ((constant << 32) >> 32) {
        Some(RegMemImm::Imm {
            simm32: constant as u32,
        })
    } else {
        None
    }
}
