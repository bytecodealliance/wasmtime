use super::{
    abi::Aarch64ABI,
    address::Address,
    asm::Assembler,
    regs::{self, scratch},
};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, local::LocalSlot, vmctx},
    codegen::{ptr_type_from_ptr_size, CodeGenContext, CodeGenError, Emission, FuncEnv},
    isa::{
        reg::{writable, Reg, WritableReg},
        CallingConvention,
    },
    masm::{
        CalleeKind, DivKind, Extend, ExtendKind, ExtractLaneKind, FloatCmpKind, Imm as I, IntCmpKind, LoadKind, MacroAssembler as Masm, MulWideKind, OperandSize, RegImm, RemKind, ReplaceLaneKind, RmwOp, RoundingMode, SPOffset, ShiftKind, SplatKind, StackSlot, StoreKind, TrapCode, TruncKind, Zero
    },
    stack::TypedReg,
};
use anyhow::{anyhow, bail, Result};
use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{MemFlags, RelSourceLoc, SourceLoc},
    isa::aarch64::inst::{Cond, VectorSize},
    settings, Final, MachBufferFinalized, MachLabel,
};
use regalloc2::RegClass;
use wasmtime_environ::{PtrSize, WasmValType};

/// Aarch64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// Low level assembler.
    asm: Assembler,
    /// Stack pointer offset.
    sp_offset: u32,
    /// The target pointer size.
    ptr_size: OperandSize,
}

impl MacroAssembler {
    /// Create an Aarch64 MacroAssembler.
    pub fn new(ptr_size: impl PtrSize, shared_flags: settings::Flags) -> Result<Self> {
        Ok(Self {
            asm: Assembler::new(shared_flags),
            sp_offset: 0u32,
            ptr_size: ptr_type_from_ptr_size(ptr_size.size()).try_into()?,
        })
    }
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = Aarch64ABI;

    fn frame_setup(&mut self) -> Result<()> {
        let lr = regs::lr();
        let fp = regs::fp();
        let sp = regs::sp();
        let addr = Address::pre_indexed_from_sp(-16);

        self.asm.stp(fp, lr, addr);
        self.asm.mov_rr(sp, writable!(fp), OperandSize::S64);
        self.move_sp_to_shadow_sp();
        Ok(())
    }

    fn check_stack(&mut self, _vmctx: Reg) -> Result<()> {
        // TODO: Implement when we have more complete assembler support.
        Ok(())
    }

    fn frame_restore(&mut self) -> Result<()> {
        debug_assert_eq!(self.sp_offset, 0);

        let lr = regs::lr();
        let fp = regs::fp();
        let addr = Address::post_indexed_from_sp(16);

        self.asm.ldp(fp, lr, addr);
        self.asm.ret();
        Ok(())
    }

    fn reserve_stack(&mut self, bytes: u32) -> Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let sp = regs::sp();
        self.asm
            .sub_ir(bytes as u64, sp, writable!(sp), OperandSize::S64);
        self.move_sp_to_shadow_sp();

        self.increment_sp(bytes);
        Ok(())
    }

    fn free_stack(&mut self, bytes: u32) -> Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let sp = regs::sp();
        self.asm
            .add_ir(bytes as u64, sp, writable!(sp), OperandSize::S64);
        self.move_sp_to_shadow_sp();

        self.decrement_sp(bytes);
        Ok(())
    }

    fn reset_stack_pointer(&mut self, offset: SPOffset) -> Result<()> {
        self.sp_offset = offset.as_u32();
        Ok(())
    }

    fn local_address(&mut self, local: &LocalSlot) -> Result<Address> {
        let (reg, offset) = local
            .addressed_from_sp()
            .then(|| {
                let offset = self.sp_offset.checked_sub(local.offset).expect(&format!(
                    "Invalid local offset = {}; sp offset = {}",
                    local.offset, self.sp_offset
                ));
                (regs::shadow_sp(), offset)
            })
            .unwrap_or((regs::fp(), local.offset));

        Ok(Address::offset(reg, offset as i64))
    }

    fn address_from_sp(&self, offset: SPOffset) -> Result<Self::Address> {
        Ok(Address::from_shadow_sp(
            (self.sp_offset - offset.as_u32()) as i64,
        ))
    }

    fn address_at_sp(&self, offset: SPOffset) -> Result<Self::Address> {
        Ok(Address::from_shadow_sp(offset.as_u32() as i64))
    }

    fn address_at_vmctx(&self, offset: u32) -> Result<Self::Address> {
        Ok(Address::offset(vmctx!(Self), offset as i64))
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) -> Result<()> {
        self.store(src.into(), dst, self.ptr_size)
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) -> Result<()> {
        let src = match src {
            RegImm::Imm(v) => {
                let imm = match v {
                    I::I32(v) | I::F32(v) => v as u64,
                    I::F64(v) | I::I64(v) => v,
                    I::V128(_) => unreachable!(),
                };
                let scratch = regs::scratch();
                self.asm.load_constant(imm, writable!(scratch));
                if v.is_float() {
                    let float_scratch = regs::float_scratch();
                    self.asm
                        .mov_to_fpu(scratch, writable!(float_scratch), v.size());
                    float_scratch
                } else {
                    scratch
                }
            }
            RegImm::Reg(reg) => reg,
        };

        self.asm.str(src, dst, size);
        Ok(())
    }

    fn wasm_store(
        &mut self,
        src: Reg,
        dst: Self::Address,
        kind: StoreKind,
    ) -> Result<()> {
        match kind {
            StoreKind::Operand(size) => {
                self.asm.str(src, dst, size);
                Ok(())
            },
            StoreKind::Atomic(_size) => Err(anyhow!(CodeGenError::unimplemented_masm_instruction())),
            StoreKind::VectorLane(_selector) => Err(anyhow!(CodeGenError::unimplemented_masm_instruction())),
        }
    }

    fn call(
        &mut self,
        stack_args_size: u32,
        mut load_callee: impl FnMut(&mut Self) -> Result<(CalleeKind, CallingConvention)>,
    ) -> Result<u32> {
        let alignment: u32 = <Self::ABI as abi::ABI>::call_stack_align().into();
        let addend: u32 = <Self::ABI as abi::ABI>::arg_base_offset().into();
        let delta = calculate_frame_adjustment(self.sp_offset()?.as_u32(), addend, alignment);
        let aligned_args_size = align_to(stack_args_size, alignment);
        let total_stack = delta + aligned_args_size;
        self.reserve_stack(total_stack)?;
        let (callee, call_conv) = load_callee(self)?;
        match callee {
            CalleeKind::Indirect(reg) => self.asm.call_with_reg(reg, call_conv),
            CalleeKind::Direct(idx) => self.asm.call_with_name(idx, call_conv),
            CalleeKind::LibCall(lib) => self.asm.call_with_lib(lib, scratch(), call_conv),
        }

        Ok(total_stack)
    }

    fn load(&mut self, src: Address, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.uload(src, dst, size);
        Ok(())
    }

    fn load_ptr(&mut self, src: Self::Address, dst: WritableReg) -> Result<()> {
        self.load(src, dst, self.ptr_size)
    }

    fn wasm_load(
        &mut self,
        src: Self::Address,
        dst: WritableReg,
        kind: LoadKind,
    ) -> Result<()> {
        let size = kind.derive_operand_size();
        match kind {
            LoadKind::Operand(_) => self.asm.uload(src, dst, size),
            LoadKind::Splat(_) => bail!(CodeGenError::UnimplementedWasmLoadKind),
            LoadKind::ScalarExtend(extend_kind) => {
                if extend_kind.signed() {
                    self.asm.sload(src, dst, size)
                } else {
                    // unlike x64, unused bits are set to zero so we don't need to extend
                    self.asm.uload(src, dst, size)
                }
            }
            LoadKind::VectorExtend(_vector_extend_kind) => {
                bail!(CodeGenError::UnimplementedWasmLoadKind)
            }
            LoadKind::VectorLane(_selector) => bail!(CodeGenError::unimplemented_masm_instruction()),
            LoadKind::Atomic(_, _) => bail!(CodeGenError::unimplemented_masm_instruction()),

        }

        Ok(())
    }

    fn load_addr(&mut self, src: Self::Address, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.uload(src, dst, size);
        Ok(())
    }

    fn pop(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        let addr = self.address_from_sp(SPOffset::from_u32(self.sp_offset))?;
        self.asm.uload(addr, dst, size);
        self.free_stack(size.bytes())
    }

    fn sp_offset(&self) -> Result<SPOffset> {
        Ok(SPOffset::from_u32(self.sp_offset))
    }

    fn finalize(self, base: Option<SourceLoc>) -> Result<MachBufferFinalized<Final>> {
        Ok(self.asm.finalize(base))
    }

    fn mov(&mut self, dst: WritableReg, src: RegImm, size: OperandSize) -> Result<()> {
        match (src, dst) {
            (RegImm::Imm(v), rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    I::F32(v) => v as u64,
                    I::F64(v) => v,
                    I::V128(_) => bail!(CodeGenError::unsupported_imm()),
                };

                let scratch = regs::scratch();
                self.asm.load_constant(imm, writable!(scratch));
                match rd.to_reg().class() {
                    RegClass::Int => Ok(self.asm.mov_rr(scratch, rd, size)),
                    RegClass::Float => Ok(self.asm.mov_to_fpu(scratch, rd, size)),
                    _ => bail!(CodeGenError::invalid_operand_combination()),
                }
            }
            (RegImm::Reg(rs), rd) => match (rs.class(), rd.to_reg().class()) {
                (RegClass::Int, RegClass::Int) => Ok(self.asm.mov_rr(rs, rd, size)),
                (RegClass::Float, RegClass::Float) => Ok(self.asm.fmov_rr(rs, rd, size)),
                (RegClass::Int, RegClass::Float) => Ok(self.asm.mov_to_fpu(rs, rd, size)),
                _ => bail!(CodeGenError::invalid_operand_combination()),
            },
        }
    }

    fn cmov(
        &mut self,
        dst: WritableReg,
        src: Reg,
        cc: IntCmpKind,
        _size: OperandSize,
    ) -> Result<()> {
        self.asm.csel(src, src, dst, Cond::from(cc));
        Ok(())
    }

    fn add(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.add_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.add_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn checked_uadd(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: RegImm,
        size: OperandSize,
        trap: TrapCode,
    ) -> Result<()> {
        self.add(dst, lhs, rhs, size)?;
        self.asm.trapif(Cond::Hs, trap);
        Ok(())
    }

    fn sub(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.sub_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.sub_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn mul(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.mul_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.mul_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn float_add(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fadd_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_sub(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fsub_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_mul(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmul_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_div(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fdiv_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_min(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmin_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_max(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmax_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_copysign(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        size: OperandSize,
    ) -> Result<()> {
        let max_shift = match size {
            OperandSize::S32 => 0x1f,
            OperandSize::S64 => 0x3f,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };
        self.asm.fushr_rri(rhs, writable!(rhs), max_shift, size);
        self.asm.fsli_rri_mod(lhs, rhs, dst, max_shift, size);
        Ok(())
    }

    fn float_neg(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.fneg_rr(dst.to_reg(), dst, size);
        Ok(())
    }

    fn float_abs(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.fabs_rr(dst.to_reg(), dst, size);
        Ok(())
    }

    fn float_round<
        F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext<Emission>, &mut Self) -> Result<()>,
    >(
        &mut self,
        mode: RoundingMode,
        _env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext<Emission>,
        size: OperandSize,
        _fallback: F,
    ) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        self.asm
            .fround_rr(src.into(), writable!(src.into()), mode, size);
        context.stack.push(src.into());
        Ok(())
    }

    fn float_sqrt(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        self.asm.fsqrt_rr(src, dst, size);
        Ok(())
    }

    fn and(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.and_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.and_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn or(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.or_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.or_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn xor(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };

                self.asm.xor_ir(imm, rn, rd, size);
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.xor_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn shift_ir(
        &mut self,
        dst: WritableReg,
        imm: u64,
        lhs: Reg,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.shift_ir(imm, lhs, dst, kind, size);
        Ok(())
    }

    fn shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        let dst = context.pop_to_reg(self, None)?;

        self.asm
            .shift_rrr(src.into(), dst.into(), writable!(dst.into()), kind, size);

        context.free_reg(src);
        context.stack.push(dst.into());

        Ok(())
    }

    fn div(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: DivKind,
        size: OperandSize,
    ) -> Result<()> {
        context.binop(self, size, |this, dividend, divisor, size| {
            this.asm
                .div_rrr(divisor, dividend, writable!(dividend), kind, size);
            match size {
                OperandSize::S32 => Ok(TypedReg::new(WasmValType::I32, dividend)),
                OperandSize::S64 => Ok(TypedReg::new(WasmValType::I64, dividend)),
                _ => Err(anyhow!(CodeGenError::unexpected_operand_size())),
            }
        })
    }

    fn rem(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: RemKind,
        size: OperandSize,
    ) -> Result<()> {
        context.binop(self, size, |this, dividend, divisor, size| {
            this.asm
                .rem_rrr(divisor, dividend, writable!(dividend), kind, size);
            match size {
                OperandSize::S32 => Ok(TypedReg::new(WasmValType::I32, dividend)),
                OperandSize::S64 => Ok(TypedReg::new(WasmValType::I64, dividend)),
                _ => Err(anyhow!(CodeGenError::unexpected_operand_size())),
            }
        })
    }

    fn zero(&mut self, reg: WritableReg) -> Result<()> {
        self.asm.load_constant(0, reg);
        Ok(())
    }

    fn popcnt(&mut self, context: &mut CodeGenContext<Emission>, size: OperandSize) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        let tmp = regs::float_scratch();
        self.asm.mov_to_fpu(src.into(), writable!(tmp), size);
        self.asm.cnt(writable!(tmp));
        self.asm.addv(tmp, writable!(tmp), VectorSize::Size8x8);
        self.asm
            .mov_from_vec(tmp, writable!(src.into()), 0, OperandSize::S8);
        context.stack.push(src.into());
        Ok(())
    }

    fn signed_truncate(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()> {
        self.asm
            .fpu_to_int(dst, src, src_size, dst_size, kind, true);

        Ok(())
    }

    fn unsigned_truncate(
        &mut self,
        ctx: &mut CodeGenContext<Emission>,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()> {
        let dst_ty = match dst_size {
            OperandSize::S32 => WasmValType::I32,
            OperandSize::S64 => WasmValType::I64,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };

        ctx.convert_op(self, dst_ty, |masm, dst, src, dst_size| {
            masm.asm
                .fpu_to_int(writable!(dst), src, src_size, dst_size, kind, false);

            Ok(())
        })
    }

    fn signed_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()> {
        self.asm.cvt_sint_to_float(src, dst, src_size, dst_size);
        Ok(())
    }

    fn unsigned_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        _tmp_gpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()> {
        self.asm.cvt_uint_to_float(src, dst, src_size, dst_size);
        Ok(())
    }

    fn reinterpret_float_as_int(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.mov_from_vec(src, dst, 0, size);
        Ok(())
    }

    fn reinterpret_int_as_float(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.mov_to_fpu(src, dst, size);
        Ok(())
    }

    fn demote(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm
            .cvt_float_to_float(src.into(), dst, OperandSize::S64, OperandSize::S32);
        Ok(())
    }

    fn promote(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm
            .cvt_float_to_float(src.into(), dst, OperandSize::S32, OperandSize::S64);
        Ok(())
    }

    fn push(&mut self, reg: Reg, size: OperandSize) -> Result<StackSlot> {
        self.reserve_stack(size.bytes())?;
        let address = self.address_from_sp(SPOffset::from_u32(self.sp_offset))?;
        self.asm.str(reg, address, size);

        Ok(StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: size.bytes(),
        })
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Result<Self::Address> {
        Ok(Address::offset(reg, offset as i64))
    }

    fn cmp_with_set(
        &mut self,
        dst: WritableReg,
        src: RegImm,
        kind: IntCmpKind,
        size: OperandSize,
    ) -> Result<()> {
        self.cmp(dst.to_reg(), src, size)?;
        self.asm.cset(dst, kind.into());
        Ok(())
    }

    fn cmp(&mut self, src1: Reg, src2: RegImm, size: OperandSize) -> Result<()> {
        match src2 {
            RegImm::Reg(src2) => {
                self.asm.subs_rrr(src2, src1, size);
                Ok(())
            }
            RegImm::Imm(v) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => bail!(CodeGenError::unsupported_imm()),
                };
                self.asm.subs_ir(imm, src1, size);
                Ok(())
            }
        }
    }

    fn float_cmp_with_set(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: Reg,
        kind: FloatCmpKind,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.fcmp(src1, src2, size);
        self.asm.cset(dst, kind.into());
        Ok(())
    }

    fn clz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        self.asm.clz(src, dst, size);
        Ok(())
    }

    fn ctz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        let scratch = regs::scratch();
        self.asm.rbit(src, writable!(scratch), size);
        self.asm.clz(scratch, dst, size);
        Ok(())
    }

    fn wrap(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm.mov_rr(src, dst, OperandSize::S32);
        Ok(())
    }

    fn extend(&mut self, dst: WritableReg, src: Reg, kind: ExtendKind) -> Result<()> {
        self.asm.extend(src, dst, kind);
        Ok(())
    }

    fn get_label(&mut self) -> Result<MachLabel> {
        Ok(self.asm.get_label())
    }

    fn bind(&mut self, label: MachLabel) -> Result<()> {
        let buffer = self.asm.buffer_mut();
        buffer.bind_label(label, &mut Default::default());
        Ok(())
    }

    fn branch(
        &mut self,
        kind: IntCmpKind,
        lhs: Reg,
        rhs: RegImm,
        taken: MachLabel,
        size: OperandSize,
    ) -> Result<()> {
        use IntCmpKind::*;

        match &(lhs, rhs) {
            (rlhs, RegImm::Reg(rrhs)) => {
                // If the comparison kind is zero or not zero and both operands
                // are the same register, emit a ands instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.ands_rr(*rlhs, *rrhs, size);
                } else {
                    self.cmp(lhs, rhs, size)?;
                }
            }
            _ => self.cmp(lhs, rhs, size)?,
        }
        self.asm.jmp_if(kind.into(), taken);
        Ok(())
    }

    fn jmp(&mut self, target: MachLabel) -> Result<()> {
        self.asm.jmp(target);
        Ok(())
    }

    fn unreachable(&mut self) -> Result<()> {
        self.asm.udf(wasmtime_cranelift::TRAP_UNREACHABLE);
        Ok(())
    }

    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) -> Result<()> {
        // At least one default target.
        debug_assert!(targets.len() >= 1);
        let max = targets.len() as u64 - 1;
        self.asm.subs_ir(max, index, OperandSize::S64);
        let default_index = max as usize;
        let default = targets[default_index];
        let rest = &targets[..default_index];
        let tmp1 = regs::scratch();
        self.asm.jmp_table(rest, default, index, tmp1, tmp);
        Ok(())
    }

    fn trap(&mut self, code: TrapCode) -> Result<()> {
        self.asm.udf(code);
        Ok(())
    }

    fn trapz(&mut self, src: Reg, code: TrapCode) -> Result<()> {
        self.asm.trapz(src, code, OperandSize::S64);
        Ok(())
    }

    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) -> Result<()> {
        self.asm.trapif(cc.into(), code);
        Ok(())
    }

    fn start_source_loc(&mut self, loc: RelSourceLoc) -> Result<(CodeOffset, RelSourceLoc)> {
        Ok(self.asm.buffer_mut().start_srcloc(loc))
    }

    fn end_source_loc(&mut self) -> Result<()> {
        self.asm.buffer_mut().end_srcloc();
        Ok(())
    }

    fn current_code_offset(&self) -> Result<CodeOffset> {
        Ok(self.asm.buffer().cur_offset())
    }

    fn add128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()> {
        let _ = (dst_lo, dst_hi, lhs_lo, lhs_hi, rhs_lo, rhs_hi);
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn sub128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()> {
        let _ = (dst_lo, dst_hi, lhs_lo, lhs_hi, rhs_lo, rhs_hi);
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn mul_wide(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: MulWideKind,
    ) -> Result<()> {
        let _ = (context, kind);
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn splat(&mut self, _context: &mut CodeGenContext<Emission>, _size: SplatKind) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn shuffle(&mut self, _dst: WritableReg, _lhs: Reg, _rhs: Reg, _lanes: [u8; 16]) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn swizzle(&mut self, _dst: WritableReg, _lhs: Reg, _rhs: Reg) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn atomic_rmw(
        &mut self,
        _context: &mut CodeGenContext<Emission>,
        _addr: Self::Address,
        _size: OperandSize,
        _op: RmwOp,
        _flags: MemFlags,
        _extend: Option<Extend<Zero>>,
    ) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn extract_lane(
        &mut self,
        _src: Reg,
        _dst: WritableReg,
        _lane: u8,
        _kind: ExtractLaneKind,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn replace_lane(
        &mut self,
        _src: RegImm,
        _dst: WritableReg,
        _lane: u8,
        _kind: ReplaceLaneKind,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn atomic_cas(
        &mut self,
        _context: &mut CodeGenContext<Emission>,
        _addr: Self::Address,
        _size: OperandSize,
        _flags: MemFlags,
        _extend: Option<Extend<Zero>>,
    ) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_not(&mut self, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn fence(&mut self) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_and(&mut self, _src1: Reg, _src2: Reg, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_and_not(&mut self, _src1: Reg, _src2: Reg, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_or(&mut self, _src1: Reg, _src2: Reg, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_xor(&mut self, _src1: Reg, _src2: Reg, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_bitselect(
        &mut self,
        _src1: Reg,
        _src2: Reg,
        _mask: Reg,
        _dst: WritableReg,
    ) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_any_true(&mut self, _src: Reg, _dst: WritableReg) -> Result<()> {
        Err(anyhow!(CodeGenError::unimplemented_masm_instruction()))
    }
}

impl MacroAssembler {
    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;
    }

    fn decrement_sp(&mut self, bytes: u32) {
        self.sp_offset -= bytes;
    }

    // Copies the value of the stack pointer to the shadow stack
    // pointer: mov x28, sp

    // This function is usually called whenever the real stack pointer
    // changes, for example after allocating or deallocating stack
    // space, or after performing a push or pop.
    // For more details around the stack pointer and shadow stack
    // pointer see the docs at regs::shadow_sp().
    fn move_sp_to_shadow_sp(&mut self) {
        let sp = regs::sp();
        let shadow_sp = regs::shadow_sp();
        self.asm.mov_rr(sp, writable!(shadow_sp), OperandSize::S64);
    }
}
