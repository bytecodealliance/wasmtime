//! ISLE integration glue code for x64 lowering.

// Pull in the ISLE generated code.
mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{
    is_mergeable_load, lower_to_amode, AluRmiROpcode, Inst as MInst, OperandSize, Reg, RegMemImm,
};
use crate::isa::x64::inst::args::SyntheticAmode;
use crate::isa::x64::inst::regs;
use crate::isa::x64::settings as x64_settings;
use crate::machinst::isle::*;
use crate::{
    ir::{immediates::*, types::*, Inst, InstructionData, Opcode, TrapCode, Value, ValueList},
    isa::x64::inst::{
        args::{
            Avx512Opcode, CmpOpcode, ExtMode, FcmpImm, Imm8Reg, RegMem, ShiftKind, SseOpcode, CC,
        },
        x64_map_regs,
    },
    machinst::{get_output_reg, InsnInput, InsnOutput, LowerCtx, RegRenamer},
};
use smallvec::SmallVec;
use std::convert::TryFrom;

pub struct SinkableLoad {
    inst: Inst,
    addr_input: InsnInput,
    offset: i32,
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    isa_flags: &x64_settings::Flags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()>
where
    C: LowerCtx<I = MInst>,
{
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext::new(lower_ctx, isa_flags);

    let temp_regs = generated_code::constructor_lower(&mut isle_ctx, inst).ok_or(())?;
    let mut temp_regs = temp_regs.regs().iter();

    #[cfg(debug_assertions)]
    {
        let all_dsts_len = outputs
            .iter()
            .map(|out| get_output_reg(isle_ctx.lower_ctx, *out).len())
            .sum();
        debug_assert_eq!(
            temp_regs.len(),
            all_dsts_len,
            "the number of temporary registers and destination registers do \
         not match ({} != {}); ensure the correct registers are being \
         returned.",
            temp_regs.len(),
            all_dsts_len,
        );
    }

    // The ISLE generated code emits its own registers to define the
    // instruction's lowered values in. We rename those registers to the
    // registers they were assigned when their value was used as an operand in
    // earlier lowerings.
    let mut renamer = RegRenamer::default();
    for output in outputs {
        let dsts = get_output_reg(isle_ctx.lower_ctx, *output);
        for (temp, dst) in temp_regs.by_ref().zip(dsts.regs()) {
            renamer.add_rename(*temp, dst.to_reg());
        }
    }

    for mut inst in isle_ctx.into_emitted_insts() {
        x64_map_regs(&mut inst, &renamer);
        lower_ctx.emit(inst);
    }

    Ok(())
}

pub struct IsleContext<'a, C> {
    lower_ctx: &'a mut C,
    isa_flags: &'a x64_settings::Flags,
    emitted_insts: SmallVec<[MInst; 6]>,
}

impl<'a, C> IsleContext<'a, C> {
    pub fn new(lower_ctx: &'a mut C, isa_flags: &'a x64_settings::Flags) -> Self {
        IsleContext {
            lower_ctx,
            isa_flags,
            emitted_insts: SmallVec::new(),
        }
    }

    pub fn into_emitted_insts(self) -> SmallVec<[MInst; 6]> {
        self.emitted_insts
    }
}

impl<'a, C> generated_code::Context for IsleContext<'a, C>
where
    C: LowerCtx<I = MInst>,
{
    isle_prelude_methods!();

    #[inline]
    fn operand_size_of_type(&mut self, ty: Type) -> OperandSize {
        if ty.bits() == 64 {
            OperandSize::Size64
        } else {
            OperandSize::Size32
        }
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
