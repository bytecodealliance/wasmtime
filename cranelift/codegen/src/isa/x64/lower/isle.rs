//! ISLE integration glue code for x64 lowering.

// Pull in the ISLE generated code.
pub(crate) mod generated_code;
use crate::{
    ir::types,
    ir::AtomicRmwOp,
    machinst::{InputSourceInst, Reg, Writable},
};
use generated_code::{Context, MInst};

// Types that the generated ISLE code uses via `use super::*`.
use super::{is_int_or_ref_ty, is_mergeable_load, lower_to_amode};
use crate::ir::LibCall;
use crate::isa::x64::lower::emit_vm_call;
use crate::{
    ir::{
        condcodes::{CondCode, FloatCC, IntCC},
        immediates::*,
        types::*,
        Inst, InstructionData, MemFlags, Opcode, TrapCode, Value, ValueList,
    },
    isa::{
        settings::Flags,
        unwind::UnwindInst,
        x64::{
            abi::{X64ABICaller, X64ABIMachineSpec},
            inst::{args::*, regs, CallInfo},
            settings::Flags as IsaFlags,
        },
    },
    machinst::{
        isle::*, valueregs, ABICaller, InsnInput, InsnOutput, Lower, MachAtomicRmwOp, MachInst,
        VCodeConstant, VCodeConstantData,
    },
};
use regalloc2::PReg;
use smallvec::SmallVec;
use std::boxed::Box;
use std::convert::TryFrom;
use target_lexicon::Triple;

type BoxCallInfo = Box<CallInfo>;
type BoxVecMachLabel = Box<SmallVec<[MachLabel; 4]>>;
type MachLabelSlice = [MachLabel];

pub struct SinkableLoad {
    inst: Inst,
    addr_input: InsnInput,
    offset: i32,
}

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

pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    triple: &Triple,
    flags: &Flags,
    isa_flags: &IsaFlags,
    branch: Inst,
    targets: &[MachLabel],
) -> Result<(), ()> {
    lower_common(
        lower_ctx,
        triple,
        flags,
        isa_flags,
        &[],
        branch,
        |cx, insn| generated_code::constructor_lower_branch(cx, insn, targets),
    )
}

impl Context for IsleContext<'_, '_, MInst, Flags, IsaFlags, 6> {
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

    fn put_in_reg_mem_imm(&mut self, val: Value) -> RegMemImm {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            if let Some(imm) = to_simm32(c as i64) {
                return imm.to_reg_mem_imm();
            }

            // A load from the constant pool is better than a
            // rematerialization into a register, because it reduces
            // register pressure.
            let vcode_constant = self.emit_u64_le_const(c);
            return RegMemImm::mem(SyntheticAmode::ConstantOffset(vcode_constant));
        }

        if let InputSourceInst::UniqueUse(src_insn, 0) = inputs.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, src_insn) {
                self.lower_ctx.sink_inst(src_insn);
                let amode = lower_to_amode(self.lower_ctx, addr_input, offset);
                return RegMemImm::mem(amode);
            }
        }

        RegMemImm::reg(self.put_in_reg(val))
    }

    fn put_in_reg_mem(&mut self, val: Value) -> RegMem {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            // A load from the constant pool is better than a
            // rematerialization into a register, because it reduces
            // register pressure.
            let vcode_constant = self.emit_u64_le_const(c);
            return RegMem::mem(SyntheticAmode::ConstantOffset(vcode_constant));
        }

        if let InputSourceInst::UniqueUse(src_insn, 0) = inputs.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, src_insn) {
                self.lower_ctx.sink_inst(src_insn);
                let amode = lower_to_amode(self.lower_ctx, addr_input, offset);
                return RegMem::mem(amode);
            }
        }

        RegMem::reg(self.put_in_reg(val))
    }

    fn put_masked_in_imm8_gpr(&mut self, val: Value, ty: Type) -> Imm8Gpr {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            let mask = 1_u64.checked_shl(ty.bits()).map_or(u64::MAX, |x| x - 1);
            return Imm8Gpr::new(Imm8Reg::Imm8 {
                imm: (c & mask) as u8,
            })
            .unwrap();
        }

        Imm8Gpr::new(Imm8Reg::Reg {
            reg: self.put_in_regs(val).regs()[0],
        })
        .unwrap()
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
    fn avx512bitalg_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512bitalg_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn use_lzcnt(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_lzcnt() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn use_bmi1(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_bmi1() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn use_popcnt(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_popcnt() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn use_fma(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_fma() {
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
    fn const_to_type_masked_imm8(&mut self, c: u64, ty: Type) -> Imm8Gpr {
        let mask = 1_u64.checked_shl(ty.bits()).map_or(u64::MAX, |x| x - 1);
        Imm8Gpr::new(Imm8Reg::Imm8 {
            imm: (c & mask) as u8,
        })
        .unwrap()
    }

    #[inline]
    fn shift_mask(&mut self, ty: Type) -> u32 {
        ty.lane_bits() - 1
    }

    #[inline]
    fn simm32_from_value(&mut self, val: Value) -> Option<GprMemImm> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant: u64 = self.lower_ctx.get_constant(inst)?;
        let constant = constant as i64;
        to_simm32(constant)
    }

    #[inline]
    fn simm32_from_imm64(&mut self, imm: Imm64) -> Option<GprMemImm> {
        to_simm32(imm.bits())
    }

    fn sinkable_load(&mut self, val: Value) -> Option<SinkableLoad> {
        let input = self.lower_ctx.get_value_as_source_or_const(val);
        if let InputSourceInst::UniqueUse(inst, 0) = input.inst {
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
        self.lower_ctx.emit(inst.clone());
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
    fn xmm0(&mut self) -> WritableXmm {
        WritableXmm::from_reg(Xmm::new(regs::xmm0()).unwrap())
    }

    #[inline]
    fn synthetic_amode_to_reg_mem(&mut self, addr: &SyntheticAmode) -> RegMem {
        RegMem::mem(addr.clone())
    }

    #[inline]
    fn amode_imm_reg_reg_shift(&mut self, simm32: u32, base: Gpr, index: Gpr, shift: u8) -> Amode {
        Amode::imm_reg_reg_shift(simm32, base, index, shift)
    }

    #[inline]
    fn amode_imm_reg(&mut self, simm32: u32, base: Gpr) -> Amode {
        Amode::imm_reg(simm32, base.to_reg())
    }

    #[inline]
    fn amode_with_flags(&mut self, amode: &Amode, flags: MemFlags) -> Amode {
        amode.with_flags(flags)
    }

    #[inline]
    fn amode_to_synthetic_amode(&mut self, amode: &Amode) -> SyntheticAmode {
        amode.clone().into()
    }

    #[inline]
    fn writable_gpr_to_reg(&mut self, r: WritableGpr) -> WritableReg {
        r.to_writable_reg()
    }

    #[inline]
    fn writable_xmm_to_reg(&mut self, r: WritableXmm) -> WritableReg {
        r.to_writable_reg()
    }

    fn ishl_i8x16_mask_for_const(&mut self, amt: u32) -> SyntheticAmode {
        // When the shift amount is known, we can statically (i.e. at compile
        // time) determine the mask to use and only emit that.
        debug_assert!(amt < 8);
        let mask_offset = amt as usize * 16;
        let mask_constant = self.lower_ctx.use_constant(VCodeConstantData::WellKnown(
            &I8X16_ISHL_MASKS[mask_offset..mask_offset + 16],
        ));
        SyntheticAmode::ConstantOffset(mask_constant)
    }

    fn ishl_i8x16_mask_table(&mut self) -> SyntheticAmode {
        let mask_table = self
            .lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&I8X16_ISHL_MASKS));
        SyntheticAmode::ConstantOffset(mask_table)
    }

    fn ushr_i8x16_mask_for_const(&mut self, amt: u32) -> SyntheticAmode {
        // When the shift amount is known, we can statically (i.e. at compile
        // time) determine the mask to use and only emit that.
        debug_assert!(amt < 8);
        let mask_offset = amt as usize * 16;
        let mask_constant = self.lower_ctx.use_constant(VCodeConstantData::WellKnown(
            &I8X16_USHR_MASKS[mask_offset..mask_offset + 16],
        ));
        SyntheticAmode::ConstantOffset(mask_constant)
    }

    fn ushr_i8x16_mask_table(&mut self) -> SyntheticAmode {
        let mask_table = self
            .lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&I8X16_USHR_MASKS));
        SyntheticAmode::ConstantOffset(mask_table)
    }

    fn popcount_4bit_table(&mut self) -> VCodeConstant {
        self.lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&POPCOUNT_4BIT_TABLE))
    }

    fn popcount_low_mask(&mut self) -> VCodeConstant {
        self.lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&POPCOUNT_LOW_MASK))
    }

    #[inline]
    fn writable_reg_to_xmm(&mut self, r: WritableReg) -> WritableXmm {
        Writable::from_reg(Xmm::new(r.to_reg()).unwrap())
    }

    #[inline]
    fn writable_xmm_to_xmm(&mut self, r: WritableXmm) -> Xmm {
        r.to_reg()
    }

    #[inline]
    fn writable_gpr_to_gpr(&mut self, r: WritableGpr) -> Gpr {
        r.to_reg()
    }

    #[inline]
    fn gpr_to_reg(&mut self, r: Gpr) -> Reg {
        r.into()
    }

    #[inline]
    fn xmm_to_reg(&mut self, r: Xmm) -> Reg {
        r.into()
    }

    #[inline]
    fn xmm_to_xmm_mem_imm(&mut self, r: Xmm) -> XmmMemImm {
        r.into()
    }

    #[inline]
    fn temp_writable_gpr(&mut self) -> WritableGpr {
        Writable::from_reg(Gpr::new(self.temp_writable_reg(I64).to_reg()).unwrap())
    }

    #[inline]
    fn temp_writable_xmm(&mut self) -> WritableXmm {
        Writable::from_reg(Xmm::new(self.temp_writable_reg(I8X16).to_reg()).unwrap())
    }

    #[inline]
    fn reg_to_reg_mem_imm(&mut self, reg: Reg) -> RegMemImm {
        RegMemImm::Reg { reg }
    }

    #[inline]
    fn reg_mem_to_xmm_mem(&mut self, rm: &RegMem) -> XmmMem {
        XmmMem::new(rm.clone()).unwrap()
    }

    #[inline]
    fn gpr_mem_imm_new(&mut self, rmi: &RegMemImm) -> GprMemImm {
        GprMemImm::new(rmi.clone()).unwrap()
    }

    #[inline]
    fn xmm_mem_imm_new(&mut self, rmi: &RegMemImm) -> XmmMemImm {
        XmmMemImm::new(rmi.clone()).unwrap()
    }

    #[inline]
    fn xmm_to_xmm_mem(&mut self, r: Xmm) -> XmmMem {
        r.into()
    }

    #[inline]
    fn xmm_mem_to_reg_mem(&mut self, xm: &XmmMem) -> RegMem {
        xm.clone().into()
    }

    #[inline]
    fn gpr_mem_to_reg_mem(&mut self, gm: &GprMem) -> RegMem {
        gm.clone().into()
    }

    #[inline]
    fn xmm_new(&mut self, r: Reg) -> Xmm {
        Xmm::new(r).unwrap()
    }

    #[inline]
    fn gpr_new(&mut self, r: Reg) -> Gpr {
        Gpr::new(r).unwrap()
    }

    #[inline]
    fn reg_mem_to_gpr_mem(&mut self, rm: &RegMem) -> GprMem {
        GprMem::new(rm.clone()).unwrap()
    }

    #[inline]
    fn reg_to_gpr_mem(&mut self, r: Reg) -> GprMem {
        GprMem::new(RegMem::reg(r)).unwrap()
    }

    #[inline]
    fn imm8_reg_to_imm8_gpr(&mut self, ir: &Imm8Reg) -> Imm8Gpr {
        Imm8Gpr::new(ir.clone()).unwrap()
    }

    #[inline]
    fn gpr_to_gpr_mem(&mut self, gpr: Gpr) -> GprMem {
        GprMem::from(gpr)
    }

    #[inline]
    fn gpr_to_gpr_mem_imm(&mut self, gpr: Gpr) -> GprMemImm {
        GprMemImm::from(gpr)
    }

    #[inline]
    fn gpr_to_imm8_gpr(&mut self, gpr: Gpr) -> Imm8Gpr {
        Imm8Gpr::from(gpr)
    }

    #[inline]
    fn imm8_to_imm8_gpr(&mut self, imm: u8) -> Imm8Gpr {
        Imm8Gpr::new(Imm8Reg::Imm8 { imm }).unwrap()
    }

    fn is_gpr_type(&mut self, ty: Type) -> Option<Type> {
        if is_int_or_ref_ty(ty) || ty == I128 || ty == B128 {
            Some(ty)
        } else {
            None
        }
    }

    #[inline]
    fn is_xmm_type(&mut self, ty: Type) -> Option<Type> {
        if ty == F32 || ty == F64 || (ty.is_vector() && ty.bits() == 128) {
            Some(ty)
        } else {
            None
        }
    }

    #[inline]
    fn is_single_register_type(&mut self, ty: Type) -> Option<Type> {
        if ty != I128 {
            Some(ty)
        } else {
            None
        }
    }

    #[inline]
    fn ty_int_bool_or_ref(&mut self, ty: Type) -> Option<()> {
        match ty {
            types::I8 | types::I16 | types::I32 | types::I64 | types::R64 => Some(()),
            types::B1 | types::B8 | types::B16 | types::B32 | types::B64 => Some(()),
            types::R32 => panic!("shouldn't have 32-bits refs on x64"),
            _ => None,
        }
    }

    #[inline]
    fn intcc_neq(&mut self, x: &IntCC, y: &IntCC) -> Option<IntCC> {
        if x != y {
            Some(*x)
        } else {
            None
        }
    }

    #[inline]
    fn intcc_without_eq(&mut self, x: &IntCC) -> IntCC {
        x.without_equal()
    }

    #[inline]
    fn intcc_unsigned(&mut self, x: &IntCC) -> IntCC {
        x.unsigned()
    }

    #[inline]
    fn intcc_to_cc(&mut self, intcc: &IntCC) -> CC {
        CC::from_intcc(*intcc)
    }

    #[inline]
    fn cc_invert(&mut self, cc: &CC) -> CC {
        cc.invert()
    }

    #[inline]
    fn cc_nz_or_z(&mut self, cc: &CC) -> Option<CC> {
        match cc {
            CC::Z => Some(*cc),
            CC::NZ => Some(*cc),
            _ => None,
        }
    }

    #[inline]
    fn intcc_reverse(&mut self, cc: &IntCC) -> IntCC {
        cc.reverse()
    }

    #[inline]
    fn floatcc_inverse(&mut self, cc: &FloatCC) -> FloatCC {
        cc.inverse()
    }

    #[inline]
    fn sum_extend_fits_in_32_bits(
        &mut self,
        extend_from_ty: Type,
        constant_value: Imm64,
        offset: Offset32,
    ) -> Option<u32> {
        let offset: i64 = offset.into();
        let constant_value: u64 = constant_value.bits() as u64;
        // If necessary, zero extend `constant_value` up to 64 bits.
        let shift = 64 - extend_from_ty.bits();
        let zero_extended_constant_value = (constant_value << shift) >> shift;
        // Sum up the two operands.
        let sum = offset.wrapping_add(zero_extended_constant_value as i64);
        // Check that the sum will fit in 32-bits.
        if sum == ((sum << 32) >> 32) {
            Some(sum as u32)
        } else {
            None
        }
    }

    #[inline]
    fn amode_offset(&mut self, addr: &Amode, offset: u32) -> Amode {
        addr.offset(offset)
    }

    #[inline]
    fn zero_offset(&mut self) -> Offset32 {
        Offset32::new(0)
    }

    #[inline]
    fn atomic_rmw_op_to_mach_atomic_rmw_op(&mut self, op: &AtomicRmwOp) -> MachAtomicRmwOp {
        MachAtomicRmwOp::from(*op)
    }

    #[inline]
    fn gen_move(&mut self, ty: Type, dst: WritableReg, src: Reg) -> MInst {
        MInst::gen_move(dst, src, ty)
    }

    fn gen_call(
        &mut self,
        sig_ref: SigRef,
        extname: ExternalName,
        dist: RelocDistance,
        args @ (inputs, off): ValueSlice,
    ) -> InstOutput {
        let caller_conv = self.lower_ctx.abi().call_conv();
        let sig = &self.lower_ctx.dfg().signatures[sig_ref];
        let num_rets = sig.returns.len();
        let abi = ABISig::from_func_sig::<X64ABIMachineSpec>(sig, self.flags).unwrap();
        let caller = X64ABICaller::from_func(sig, &extname, dist, caller_conv, self.flags).unwrap();

        assert_eq!(
            inputs.len(&self.lower_ctx.dfg().value_lists) - off,
            sig.params.len()
        );

        self.gen_call_common(abi, num_rets, caller, args)
    }

    fn gen_call_indirect(
        &mut self,
        sig_ref: SigRef,
        val: Value,
        args @ (inputs, off): ValueSlice,
    ) -> InstOutput {
        let caller_conv = self.lower_ctx.abi().call_conv();
        let ptr = self.put_in_reg(val);
        let sig = &self.lower_ctx.dfg().signatures[sig_ref];
        let num_rets = sig.returns.len();
        let abi = ABISig::from_func_sig::<X64ABIMachineSpec>(sig, self.flags).unwrap();
        let caller =
            X64ABICaller::from_ptr(sig, ptr, Opcode::CallIndirect, caller_conv, self.flags)
                .unwrap();

        assert_eq!(
            inputs.len(&self.lower_ctx.dfg().value_lists) - off,
            sig.params.len()
        );

        self.gen_call_common(abi, num_rets, caller, args)
    }

    #[inline]
    fn preg_rbp(&mut self) -> PReg {
        regs::rbp().to_real_reg().unwrap().into()
    }

    #[inline]
    fn preg_rsp(&mut self) -> PReg {
        regs::rsp().to_real_reg().unwrap().into()
    }

    fn libcall_3(&mut self, libcall: &LibCall, a: Reg, b: Reg, c: Reg) -> Reg {
        let call_conv = self.lower_ctx.abi().call_conv();
        let ret_ty = libcall.signature(call_conv).returns[0].value_type;
        let output_reg = self.lower_ctx.alloc_tmp(ret_ty).only_reg().unwrap();

        emit_vm_call(
            self.lower_ctx,
            self.flags,
            self.triple,
            libcall.clone(),
            &[a, b, c],
            &[output_reg],
        )
        .expect("Failed to emit LibCall");

        output_reg.to_reg()
    }

    #[inline]
    fn single_target(&mut self, targets: &MachLabelSlice) -> Option<MachLabel> {
        if targets.len() == 1 {
            Some(targets[0])
        } else {
            None
        }
    }

    #[inline]
    fn two_targets(&mut self, targets: &MachLabelSlice) -> Option<(MachLabel, MachLabel)> {
        if targets.len() == 2 {
            Some((targets[0], targets[1]))
        } else {
            None
        }
    }

    #[inline]
    fn jump_table_targets(
        &mut self,
        targets: &MachLabelSlice,
    ) -> Option<(MachLabel, BoxVecMachLabel)> {
        if targets.is_empty() {
            return None;
        }

        let default_label = targets[0];
        let jt_targets = Box::new(SmallVec::from(&targets[1..]));
        Some((default_label, jt_targets))
    }

    #[inline]
    fn jump_table_size(&mut self, targets: &BoxVecMachLabel) -> u32 {
        targets.len() as u32
    }

    #[inline]
    fn fcvt_uint_mask_const(&mut self) -> VCodeConstant {
        self.lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&UINT_MASK))
    }

    #[inline]
    fn fcvt_uint_mask_high_const(&mut self) -> VCodeConstant {
        self.lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&UINT_MASK_HIGH))
    }
}

impl IsleContext<'_, '_, MInst, Flags, IsaFlags, 6> {
    fn abi_arg_slot_regs(&mut self, arg: &ABIArg) -> Option<WritableValueRegs> {
        match arg {
            &ABIArg::Slots { ref slots, .. } => match slots.len() {
                1 => {
                    let a = self.temp_writable_reg(slots[0].get_type());
                    Some(WritableValueRegs::one(a))
                }
                2 => {
                    let a = self.temp_writable_reg(slots[0].get_type());
                    let b = self.temp_writable_reg(slots[1].get_type());
                    Some(WritableValueRegs::two(a, b))
                }
                _ => panic!("Expected to see one or two slots only from {:?}", arg),
            },
            _ => None,
        }
    }

    fn gen_call_common(
        &mut self,
        abi: ABISig,
        num_rets: usize,
        mut caller: X64ABICaller,
        (inputs, off): ValueSlice,
    ) -> InstOutput {
        caller.emit_stack_pre_adjust(self.lower_ctx);

        assert_eq!(
            inputs.len(&self.lower_ctx.dfg().value_lists) - off,
            abi.num_args()
        );
        let mut arg_regs = vec![];
        for i in 0..abi.num_args() {
            let input = inputs
                .get(off + i, &self.lower_ctx.dfg().value_lists)
                .unwrap();
            arg_regs.push(self.lower_ctx.put_value_in_regs(input));
        }
        for (i, arg_regs) in arg_regs.iter().enumerate() {
            caller.emit_copy_regs_to_buffer(self.lower_ctx, i, *arg_regs);
        }
        for (i, arg_regs) in arg_regs.iter().enumerate() {
            caller.emit_copy_regs_to_arg(self.lower_ctx, i, *arg_regs);
        }
        caller.emit_call(self.lower_ctx);

        let mut outputs = InstOutput::new();
        for i in 0..num_rets {
            let ret = abi.get_ret(i);
            let retval_regs = self.abi_arg_slot_regs(&ret).unwrap();
            caller.emit_copy_retval_to_regs(self.lower_ctx, i, retval_regs.clone());
            outputs.push(valueregs::non_writable_value_regs(retval_regs));
        }
        caller.emit_stack_post_adjust(self.lower_ctx);

        outputs
    }
}

// Since x64 doesn't have 8x16 shifts and we must use a 16x8 shift instead, we
// need to fix up the bits that migrate from one half of the lane to the
// other. Each 16-byte mask is indexed by the shift amount: e.g. if we shift
// right by 0 (no movement), we want to retain all the bits so we mask with
// `0xff`; if we shift right by 1, we want to retain all bits except the MSB so
// we mask with `0x7f`; etc.

#[rustfmt::skip] // Preserve 16 bytes (i.e. one mask) per row.
const I8X16_ISHL_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
    0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc,
    0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8,
    0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0,
    0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0,
    0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
];

#[rustfmt::skip] // Preserve 16 bytes (i.e. one mask) per row.
const I8X16_USHR_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f,
    0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f,
    0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];

/// Number of bits set in a given nibble (4-bit value). Used in the
/// vector implementation of popcount.
#[rustfmt::skip] // Preserve 4x4 layout.
const POPCOUNT_4BIT_TABLE: [u8; 16] = [
    0x00, 0x01, 0x01, 0x02,
    0x01, 0x02, 0x02, 0x03,
    0x01, 0x02, 0x02, 0x03,
    0x02, 0x03, 0x03, 0x04,
];

const POPCOUNT_LOW_MASK: [u8; 16] = [0x0f; 16];

#[inline]
fn to_simm32(constant: i64) -> Option<GprMemImm> {
    if constant == ((constant << 32) >> 32) {
        Some(
            GprMemImm::new(RegMemImm::Imm {
                simm32: constant as u32,
            })
            .unwrap(),
        )
    } else {
        None
    }
}

const UINT_MASK: [u8; 16] = [
    0x00, 0x00, 0x30, 0x43, 0x00, 0x00, 0x30, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const UINT_MASK_HIGH: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x43,
];
