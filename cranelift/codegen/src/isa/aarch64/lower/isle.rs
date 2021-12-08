//! ISLE integration glue code for aarch64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{
    zero_reg, AMode, ASIMDFPModImm, ASIMDMovModImm, AtomicRmwOp, BranchTarget, CallIndInfo,
    CallInfo, Cond, CondBrKind, ExtendOp, FPUOpRI, Imm12, ImmLogic, ImmShift, Inst as MInst,
    JTSequenceInfo, MachLabel, MoveWideConst, NarrowValueMode, Opcode, OperandSize, PairAMode, Reg,
    ScalarSize, ShiftOpAndAmt, UImm5, VectorSize, NZCV,
};
use crate::isa::aarch64::settings as aarch64_settings;
use crate::machinst::isle::*;
use crate::{
    binemit::CodeOffset,
    ir::{
        immediates::*, types::*, ExternalName, Inst, InstructionData, MemFlags, TrapCode, Value,
        ValueLabel, ValueList,
    },
    isa::aarch64::inst::aarch64_map_regs,
    isa::aarch64::inst::args::{ShiftOp, ShiftOpShiftImm},
    isa::unwind::UnwindInst,
    machinst::{get_output_reg, ty_bits, InsnOutput, LowerCtx, RegRenamer},
};
use smallvec::SmallVec;
use std::boxed::Box;
use std::vec::Vec;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type VecMachLabel = Vec<MachLabel>;
type BoxJTSequenceInfo = Box<JTSequenceInfo>;
type BoxExternalName = Box<ExternalName>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    isa_flags: &aarch64_settings::Flags,
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
        aarch64_map_regs(&mut inst, &renamer);
        lower_ctx.emit(inst);
    }

    Ok(())
}

pub struct IsleContext<'a, C> {
    lower_ctx: &'a mut C,
    #[allow(dead_code)] // dead for now, but probably not for long
    isa_flags: &'a aarch64_settings::Flags,
    emitted_insts: SmallVec<[MInst; 6]>,
}

pub struct ExtendedValue {
    val: Value,
    extend: ExtendOp,
}

impl<'a, C> IsleContext<'a, C> {
    pub fn new(lower_ctx: &'a mut C, isa_flags: &'a aarch64_settings::Flags) -> Self {
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

    fn move_wide_const_from_u64(&mut self, n: u64) -> Option<MoveWideConst> {
        MoveWideConst::maybe_from_u64(n)
    }

    fn move_wide_const_from_negated_u64(&mut self, n: u64) -> Option<MoveWideConst> {
        MoveWideConst::maybe_from_u64(!n)
    }

    fn imm_logic_from_u64(&mut self, n: u64) -> Option<ImmLogic> {
        ImmLogic::maybe_from_u64(n, I64)
    }

    fn imm12_from_u64(&mut self, n: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64(n)
    }

    fn imm12_from_negated_u64(&mut self, n: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64((n as i64).wrapping_neg() as u64)
    }

    fn imm_shift_from_u8(&mut self, n: u8) -> ImmShift {
        ImmShift::maybe_from_u64(n.into()).unwrap()
    }

    fn lshl_from_imm64(&mut self, n: Imm64, ty: Type) -> Option<ShiftOpAndAmt> {
        let shiftimm = ShiftOpShiftImm::maybe_from_shift(n.bits() as u64)?;
        let shiftee_bits = ty_bits(ty);
        if shiftee_bits <= std::u8::MAX as usize {
            let shiftimm = shiftimm.mask(shiftee_bits as u8);
            Some(ShiftOpAndAmt::new(ShiftOp::LSL, shiftimm))
        } else {
            None
        }
    }

    fn integral_ty(&mut self, ty: Type) -> Option<Type> {
        match ty {
            I8 | I16 | I32 | I64 | R64 => Some(ty),
            ty if ty.is_bool() => Some(ty),
            _ => None,
        }
    }

    /// This is the fallback case for loading a 64-bit integral constant into a
    /// register.
    ///
    /// The logic here is nontrivial enough that it's not really worth porting
    /// this over to ISLE.
    fn load_constant64_full(&mut self, value: u64) -> Reg {
        // If the top 32 bits are zero, use 32-bit `mov` operations.
        let (num_half_words, size, negated) = if value >> 32 == 0 {
            (2, OperandSize::Size32, (!value << 32) >> 32)
        } else {
            (4, OperandSize::Size64, !value)
        };
        // If the number of 0xffff half words is greater than the number of 0x0000 half words
        // it is more efficient to use `movn` for the first instruction.
        let first_is_inverted = count_zero_half_words(negated, num_half_words)
            > count_zero_half_words(value, num_half_words);
        // Either 0xffff or 0x0000 half words can be skipped, depending on the first
        // instruction used.
        let ignored_halfword = if first_is_inverted { 0xffff } else { 0 };
        let mut first_mov_emitted = false;

        let rd = self.temp_writable_reg(I64);

        for i in 0..num_half_words {
            let imm16 = (value >> (16 * i)) & 0xffff;
            if imm16 != ignored_halfword {
                if !first_mov_emitted {
                    first_mov_emitted = true;
                    if first_is_inverted {
                        let imm =
                            MoveWideConst::maybe_with_shift(((!imm16) & 0xffff) as u16, i * 16)
                                .unwrap();
                        self.emitted_insts.push(MInst::MovN { rd, imm, size });
                    } else {
                        let imm = MoveWideConst::maybe_with_shift(imm16 as u16, i * 16).unwrap();
                        self.emitted_insts.push(MInst::MovZ { rd, imm, size });
                    }
                } else {
                    let imm = MoveWideConst::maybe_with_shift(imm16 as u16, i * 16).unwrap();
                    self.emitted_insts.push(MInst::MovK { rd, imm, size });
                }
            }
        }

        assert!(first_mov_emitted);

        return self.writable_reg_to_reg(rd);

        fn count_zero_half_words(mut value: u64, num_half_words: u8) -> usize {
            let mut count = 0;
            for _ in 0..num_half_words {
                if value & 0xffff == 0 {
                    count += 1;
                }
                value >>= 16;
            }

            count
        }
    }

    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }

    fn extended_value_from_value(&mut self, val: Value) -> Option<ExtendedValue> {
        let (val, extend) =
            super::get_as_extended_value(self.lower_ctx, val, NarrowValueMode::None)?;
        Some(ExtendedValue { val, extend })
    }

    fn put_extended_in_reg(&mut self, reg: &ExtendedValue) -> Reg {
        self.put_in_reg(reg.val)
    }

    fn get_extended_op(&mut self, reg: &ExtendedValue) -> ExtendOp {
        reg.extend
    }

    fn emit(&mut self, inst: &MInst) -> Unit {
        self.emitted_insts.push(inst.clone());
    }
}
