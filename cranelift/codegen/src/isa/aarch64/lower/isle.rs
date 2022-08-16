//! ISLE integration glue code for aarch64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{
    insn_inputs, lower_constant_f128, writable_zero_reg, zero_reg, AMode, ASIMDFPModImm,
    ASIMDMovModImm, BranchTarget, CallIndInfo, CallInfo, Cond, CondBrKind, ExtendOp, FPUOpRI,
    FloatCC, Imm12, ImmLogic, ImmShift, Inst as MInst, IntCC, JTSequenceInfo, MachLabel,
    MoveWideConst, MoveWideOp, NarrowValueMode, Opcode, OperandSize, PairAMode, Reg, ScalarSize,
    ShiftOpAndAmt, UImm5, VecMisc2, VectorSize, NZCV,
};
use crate::isa::aarch64::lower::{lower_address, lower_splat_const};
use crate::isa::aarch64::settings::Flags as IsaFlags;
use crate::machinst::{isle::*, InputSourceInst};
use crate::settings::Flags;
use crate::{
    binemit::CodeOffset,
    ir::{
        immediates::*, types::*, AtomicRmwOp, ExternalName, Inst, InstructionData, MemFlags,
        TrapCode, Value, ValueList,
    },
    isa::aarch64::inst::args::{ShiftOp, ShiftOpShiftImm},
    isa::aarch64::lower::{writable_vreg, writable_xreg, xreg},
    isa::unwind::UnwindInst,
    machinst::{ty_bits, InsnOutput, Lower, VCodeConstant, VCodeConstantData},
};
use regalloc2::PReg;
use std::boxed::Box;
use std::convert::TryFrom;
use std::vec::Vec;
use target_lexicon::Triple;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type VecMachLabel = Vec<MachLabel>;
type BoxJTSequenceInfo = Box<JTSequenceInfo>;
type BoxExternalName = Box<ExternalName>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    triple: &Triple,
    flags: &Flags,
    isa_flags: &IsaFlags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()> {
    lower_common(
        lower_ctx,
        triple,
        flags,
        isa_flags,
        outputs,
        inst,
        |cx, insn| generated_code::constructor_lower(cx, insn),
    )
}

pub struct ExtendedValue {
    val: Value,
    extend: ExtendOp,
}

pub struct SinkableAtomicLoad {
    atomic_load: Inst,
    atomic_addr: Value,
}

impl generated_code::Context for IsleContext<'_, '_, MInst, Flags, IsaFlags, 6> {
    isle_prelude_methods!();

    fn use_lse(&mut self, _: Inst) -> Option<()> {
        if self.isa_flags.use_lse() {
            Some(())
        } else {
            None
        }
    }

    fn imm_logic_from_u64(&mut self, ty: Type, n: u64) -> Option<ImmLogic> {
        ImmLogic::maybe_from_u64(n, ty)
    }

    fn imm_logic_from_imm64(&mut self, ty: Type, n: Imm64) -> Option<ImmLogic> {
        let ty = if ty.bits() < 32 { I32 } else { ty };
        self.imm_logic_from_u64(ty, n.bits() as u64)
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

    fn lshr_from_u64(&mut self, ty: Type, n: u64) -> Option<ShiftOpAndAmt> {
        let shiftimm = ShiftOpShiftImm::maybe_from_shift(n)?;
        if let Ok(bits) = u8::try_from(ty_bits(ty)) {
            let shiftimm = shiftimm.mask(bits);
            Some(ShiftOpAndAmt::new(ShiftOp::LSR, shiftimm))
        } else {
            None
        }
    }

    fn lshl_from_imm64(&mut self, ty: Type, n: Imm64) -> Option<ShiftOpAndAmt> {
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

    /// This is target-word-size dependent.  And it excludes booleans and reftypes.
    fn valid_atomic_transaction(&mut self, ty: Type) -> Option<Type> {
        match ty {
            I8 | I16 | I32 | I64 => Some(ty),
            _ => None,
        }
    }

    /// This is the fallback case for loading a 64-bit integral constant into a
    /// register.
    ///
    /// The logic here is nontrivial enough that it's not really worth porting
    /// this over to ISLE.
    fn load_constant64_full(
        &mut self,
        ty: Type,
        extend: &generated_code::ImmExtend,
        value: u64,
    ) -> Reg {
        let bits = ty.bits();
        let value = if bits < 64 {
            if *extend == generated_code::ImmExtend::Sign {
                let shift = 64 - bits;
                let value = value as i64;

                ((value << shift) >> shift) as u64
            } else {
                value & !(u64::MAX << bits)
            }
        } else {
            value
        };
        let rd = self.temp_writable_reg(I64);
        let size = OperandSize::Size64;

        // If the top 32 bits are zero, use 32-bit `mov` operations.
        if value >> 32 == 0 {
            let size = OperandSize::Size32;
            let lower_halfword = value as u16;
            let upper_halfword = (value >> 16) as u16;

            if upper_halfword == u16::MAX {
                self.emit(&MInst::MovWide {
                    op: MoveWideOp::MovN,
                    rd,
                    imm: MoveWideConst::maybe_with_shift(!lower_halfword, 0).unwrap(),
                    size,
                });
            } else {
                self.emit(&MInst::MovWide {
                    op: MoveWideOp::MovZ,
                    rd,
                    imm: MoveWideConst::maybe_with_shift(lower_halfword, 0).unwrap(),
                    size,
                });

                if upper_halfword != 0 {
                    self.emit(&MInst::MovWide {
                        op: MoveWideOp::MovK,
                        rd,
                        imm: MoveWideConst::maybe_with_shift(upper_halfword, 16).unwrap(),
                        size,
                    });
                }
            }

            return rd.to_reg();
        } else if value == u64::MAX {
            self.emit(&MInst::MovWide {
                op: MoveWideOp::MovN,
                rd,
                imm: MoveWideConst::zero(),
                size,
            });
            return rd.to_reg();
        };

        // If the number of 0xffff half words is greater than the number of 0x0000 half words
        // it is more efficient to use `movn` for the first instruction.
        let first_is_inverted = count_zero_half_words(!value) > count_zero_half_words(value);
        // Either 0xffff or 0x0000 half words can be skipped, depending on the first
        // instruction used.
        let ignored_halfword = if first_is_inverted { 0xffff } else { 0 };
        let mut first_mov_emitted = false;

        for i in 0..4 {
            let imm16 = (value >> (16 * i)) & 0xffff;
            if imm16 != ignored_halfword {
                if !first_mov_emitted {
                    first_mov_emitted = true;
                    if first_is_inverted {
                        let imm =
                            MoveWideConst::maybe_with_shift(((!imm16) & 0xffff) as u16, i * 16)
                                .unwrap();
                        self.emit(&MInst::MovWide {
                            op: MoveWideOp::MovN,
                            rd,
                            imm,
                            size,
                        });
                    } else {
                        let imm = MoveWideConst::maybe_with_shift(imm16 as u16, i * 16).unwrap();
                        self.emit(&MInst::MovWide {
                            op: MoveWideOp::MovZ,
                            rd,
                            imm,
                            size,
                        });
                    }
                } else {
                    let imm = MoveWideConst::maybe_with_shift(imm16 as u16, i * 16).unwrap();
                    self.emit(&MInst::MovWide {
                        op: MoveWideOp::MovK,
                        rd,
                        imm,
                        size,
                    });
                }
            }
        }

        assert!(first_mov_emitted);

        return self.writable_reg_to_reg(rd);

        fn count_zero_half_words(mut value: u64) -> usize {
            let mut count = 0;
            for _ in 0..4 {
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

    fn xreg(&mut self, index: u8) -> Reg {
        xreg(index)
    }

    fn writable_xreg(&mut self, index: u8) -> WritableReg {
        writable_xreg(index)
    }

    fn writable_vreg(&mut self, index: u8) -> WritableReg {
        writable_vreg(index)
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
        self.lower_ctx.emit(inst.clone());
    }

    fn cond_br_zero(&mut self, reg: Reg) -> CondBrKind {
        CondBrKind::Zero(reg)
    }

    fn cond_br_cond(&mut self, cond: &Cond) -> CondBrKind {
        CondBrKind::Cond(*cond)
    }

    fn nzcv(&mut self, n: bool, z: bool, c: bool, v: bool) -> NZCV {
        NZCV::new(n, z, c, v)
    }

    fn u8_into_uimm5(&mut self, x: u8) -> UImm5 {
        UImm5::maybe_from_u8(x).unwrap()
    }

    fn u8_into_imm12(&mut self, x: u8) -> Imm12 {
        Imm12::maybe_from_u64(x.into()).unwrap()
    }

    fn writable_zero_reg(&mut self) -> WritableReg {
        writable_zero_reg()
    }

    fn safe_divisor_from_imm64(&mut self, val: Imm64) -> Option<u64> {
        match val.bits() {
            0 | -1 => None,
            n => Some(n as u64),
        }
    }

    fn sinkable_atomic_load(&mut self, val: Value) -> Option<SinkableAtomicLoad> {
        let input = self.lower_ctx.get_value_as_source_or_const(val);
        if let InputSourceInst::UniqueUse(atomic_load, 0) = input.inst {
            if self.lower_ctx.data(atomic_load).opcode() == Opcode::AtomicLoad {
                let atomic_addr = self.lower_ctx.input_as_value(atomic_load, 0);
                return Some(SinkableAtomicLoad {
                    atomic_load,
                    atomic_addr,
                });
            }
        }
        None
    }

    fn sink_atomic_load(&mut self, load: &SinkableAtomicLoad) -> Reg {
        self.lower_ctx.sink_inst(load.atomic_load);
        self.put_in_reg(load.atomic_addr)
    }

    fn shift_mask(&mut self, ty: Type) -> ImmLogic {
        debug_assert!(ty.lane_bits().is_power_of_two());

        let mask = (ty.lane_bits() - 1) as u64;
        ImmLogic::maybe_from_u64(mask, I32).unwrap()
    }

    fn imm_shift_from_imm64(&mut self, ty: Type, val: Imm64) -> Option<ImmShift> {
        let imm_value = (val.bits() as u64) & ((ty.bits() - 1) as u64);
        ImmShift::maybe_from_u64(imm_value)
    }

    fn u64_into_imm_logic(&mut self, ty: Type, val: u64) -> ImmLogic {
        ImmLogic::maybe_from_u64(val, ty).unwrap()
    }

    fn negate_imm_shift(&mut self, ty: Type, mut imm: ImmShift) -> ImmShift {
        let size = u8::try_from(ty.bits()).unwrap();
        imm.imm = size.wrapping_sub(imm.value());
        imm.imm &= size - 1;
        imm
    }

    fn rotr_mask(&mut self, ty: Type) -> ImmLogic {
        ImmLogic::maybe_from_u64((ty.bits() - 1) as u64, I32).unwrap()
    }

    fn rotr_opposite_amount(&mut self, ty: Type, val: ImmShift) -> ImmShift {
        let amount = val.value() & u8::try_from(ty.bits() - 1).unwrap();
        ImmShift::maybe_from_u64(u64::from(ty.bits()) - u64::from(amount)).unwrap()
    }

    fn icmp_zero_cond(&mut self, cond: &IntCC) -> Option<IntCC> {
        match cond {
            &IntCC::Equal
            | &IntCC::SignedGreaterThanOrEqual
            | &IntCC::SignedGreaterThan
            | &IntCC::SignedLessThanOrEqual
            | &IntCC::SignedLessThan => Some(*cond),
            _ => None,
        }
    }

    fn fcmp_zero_cond(&mut self, cond: &FloatCC) -> Option<FloatCC> {
        match cond {
            &FloatCC::Equal
            | &FloatCC::GreaterThanOrEqual
            | &FloatCC::GreaterThan
            | &FloatCC::LessThanOrEqual
            | &FloatCC::LessThan => Some(*cond),
            _ => None,
        }
    }

    fn fcmp_zero_cond_not_eq(&mut self, cond: &FloatCC) -> Option<FloatCC> {
        match cond {
            &FloatCC::NotEqual => Some(FloatCC::NotEqual),
            _ => None,
        }
    }

    fn icmp_zero_cond_not_eq(&mut self, cond: &IntCC) -> Option<IntCC> {
        match cond {
            &IntCC::NotEqual => Some(IntCC::NotEqual),
            _ => None,
        }
    }

    fn float_cc_cmp_zero_to_vec_misc_op(&mut self, cond: &FloatCC) -> VecMisc2 {
        match cond {
            &FloatCC::Equal => VecMisc2::Fcmeq0,
            &FloatCC::GreaterThanOrEqual => VecMisc2::Fcmge0,
            &FloatCC::LessThanOrEqual => VecMisc2::Fcmle0,
            &FloatCC::GreaterThan => VecMisc2::Fcmgt0,
            &FloatCC::LessThan => VecMisc2::Fcmlt0,
            _ => panic!(),
        }
    }

    fn int_cc_cmp_zero_to_vec_misc_op(&mut self, cond: &IntCC) -> VecMisc2 {
        match cond {
            &IntCC::Equal => VecMisc2::Cmeq0,
            &IntCC::SignedGreaterThanOrEqual => VecMisc2::Cmge0,
            &IntCC::SignedLessThanOrEqual => VecMisc2::Cmle0,
            &IntCC::SignedGreaterThan => VecMisc2::Cmgt0,
            &IntCC::SignedLessThan => VecMisc2::Cmlt0,
            _ => panic!(),
        }
    }

    fn float_cc_cmp_zero_to_vec_misc_op_swap(&mut self, cond: &FloatCC) -> VecMisc2 {
        match cond {
            &FloatCC::Equal => VecMisc2::Fcmeq0,
            &FloatCC::GreaterThanOrEqual => VecMisc2::Fcmle0,
            &FloatCC::LessThanOrEqual => VecMisc2::Fcmge0,
            &FloatCC::GreaterThan => VecMisc2::Fcmlt0,
            &FloatCC::LessThan => VecMisc2::Fcmgt0,
            _ => panic!(),
        }
    }

    fn int_cc_cmp_zero_to_vec_misc_op_swap(&mut self, cond: &IntCC) -> VecMisc2 {
        match cond {
            &IntCC::Equal => VecMisc2::Cmeq0,
            &IntCC::SignedGreaterThanOrEqual => VecMisc2::Cmle0,
            &IntCC::SignedLessThanOrEqual => VecMisc2::Cmge0,
            &IntCC::SignedGreaterThan => VecMisc2::Cmlt0,
            &IntCC::SignedLessThan => VecMisc2::Cmgt0,
            _ => panic!(),
        }
    }

    fn amode(&mut self, ty: Type, mem_op: Inst, offset: u32) -> AMode {
        lower_address(
            self.lower_ctx,
            ty,
            &insn_inputs(self.lower_ctx, mem_op)[..],
            offset as i32,
        )
    }

    fn amode_is_reg(&mut self, address: &AMode) -> Option<Reg> {
        address.is_reg()
    }

    fn constant_f128(&mut self, value: u128) -> Reg {
        let rd = self.temp_writable_reg(I8X16);

        lower_constant_f128(self.lower_ctx, rd, value);

        rd.to_reg()
    }

    fn splat_const(&mut self, value: u64, size: &VectorSize) -> Reg {
        let rd = self.temp_writable_reg(I8X16);

        lower_splat_const(self.lower_ctx, rd, value, *size);

        rd.to_reg()
    }

    fn preg_sp(&mut self) -> PReg {
        super::regs::stack_reg().to_real_reg().unwrap().into()
    }

    fn preg_fp(&mut self) -> PReg {
        super::regs::fp_reg().to_real_reg().unwrap().into()
    }

    fn preg_link(&mut self) -> PReg {
        super::regs::link_reg().to_real_reg().unwrap().into()
    }
}
