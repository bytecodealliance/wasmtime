use super::{
    abi::Aarch64ABI,
    address::Address,
    asm::Assembler,
    regs::{self, scratch},
};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, local::LocalSlot, vmctx},
    codegen::{ptr_type_from_ptr_size, CodeGenContext, Emission, FuncEnv},
    isa::{
        reg::{writable, Reg, WritableReg},
        CallingConvention,
    },
    masm::{
        CalleeKind, DivKind, ExtendKind, FloatCmpKind, Imm as I, IntCmpKind,
        MacroAssembler as Masm, MulWideKind, OperandSize, RegImm, RemKind, RoundingMode, SPOffset,
        ShiftKind, StackSlot, TrapCode, TruncKind,
    },
    stack::TypedReg,
};
use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{RelSourceLoc, SourceLoc},
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
    pub fn new(ptr_size: impl PtrSize, shared_flags: settings::Flags) -> Self {
        Self {
            asm: Assembler::new(shared_flags),
            sp_offset: 0u32,
            ptr_size: ptr_type_from_ptr_size(ptr_size.size()).into(),
        }
    }
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = Aarch64ABI;

    fn frame_setup(&mut self) {
        let lr = regs::lr();
        let fp = regs::fp();
        let sp = regs::sp();
        let addr = Address::pre_indexed_from_sp(-16);

        self.asm.stp(fp, lr, addr);
        self.asm.mov_rr(sp, writable!(fp), OperandSize::S64);
        self.move_sp_to_shadow_sp();
    }

    fn check_stack(&mut self, _vmctx: Reg) {
        // TODO: Implement when we have more complete assembler support.
    }

    fn frame_restore(&mut self) {
        assert_eq!(self.sp_offset, 0);

        let lr = regs::lr();
        let fp = regs::fp();
        let addr = Address::post_indexed_from_sp(16);

        self.asm.ldp(fp, lr, addr);
        self.asm.ret();
    }

    fn reserve_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        let sp = regs::sp();
        self.asm
            .sub_ir(bytes as u64, sp, writable!(sp), OperandSize::S64);
        self.move_sp_to_shadow_sp();

        self.increment_sp(bytes);
    }

    fn free_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        let sp = regs::sp();
        self.asm
            .add_ir(bytes as u64, sp, writable!(sp), OperandSize::S64);
        self.move_sp_to_shadow_sp();

        self.decrement_sp(bytes);
    }

    fn reset_stack_pointer(&mut self, offset: SPOffset) {
        self.sp_offset = offset.as_u32();
    }

    fn local_address(&mut self, local: &LocalSlot) -> Address {
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

        Address::offset(reg, offset as i64)
    }

    fn address_from_sp(&self, offset: SPOffset) -> Self::Address {
        Address::from_shadow_sp((self.sp_offset - offset.as_u32()) as i64)
    }

    fn address_at_sp(&self, offset: SPOffset) -> Self::Address {
        Address::from_shadow_sp(offset.as_u32() as i64)
    }

    fn address_at_vmctx(&self, offset: u32) -> Self::Address {
        Address::offset(vmctx!(Self), offset as i64)
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) {
        self.store(src.into(), dst, self.ptr_size);
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
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
    }

    fn wasm_store(&mut self, src: Reg, dst: Self::Address, size: OperandSize) {
        self.asm.str(src, dst, size);
    }

    fn call(
        &mut self,
        stack_args_size: u32,
        mut load_callee: impl FnMut(&mut Self) -> (CalleeKind, CallingConvention),
    ) -> u32 {
        let alignment: u32 = <Self::ABI as abi::ABI>::call_stack_align().into();
        let addend: u32 = <Self::ABI as abi::ABI>::arg_base_offset().into();
        let delta = calculate_frame_adjustment(self.sp_offset().as_u32(), addend, alignment);
        let aligned_args_size = align_to(stack_args_size, alignment);
        let total_stack = delta + aligned_args_size;
        self.reserve_stack(total_stack);
        let (callee, call_conv) = load_callee(self);
        match callee {
            CalleeKind::Indirect(reg) => self.asm.call_with_reg(reg, call_conv),
            CalleeKind::Direct(idx) => self.asm.call_with_name(idx, call_conv),
            CalleeKind::LibCall(lib) => self.asm.call_with_lib(lib, scratch(), call_conv),
        }

        total_stack
    }

    fn load(&mut self, src: Address, dst: WritableReg, size: OperandSize) {
        self.asm.uload(src, dst, size);
    }

    fn load_ptr(&mut self, src: Self::Address, dst: WritableReg) {
        self.load(src, dst, self.ptr_size);
    }

    fn wasm_load(
        &mut self,
        src: Self::Address,
        dst: WritableReg,
        size: OperandSize,
        kind: Option<ExtendKind>,
    ) {
        // kind is some if the value is signed
        // unlike x64, unused bits are set to zero so we don't need to extend
        if kind.is_some() {
            self.asm.sload(src, dst, size);
        } else {
            self.asm.uload(src, dst, size);
        }
    }

    fn load_addr(&mut self, _src: Self::Address, _dst: WritableReg, _size: OperandSize) {
        todo!()
    }

    fn pop(&mut self, dst: WritableReg, size: OperandSize) {
        let addr = self.address_from_sp(SPOffset::from_u32(self.sp_offset));
        self.asm.uload(addr, dst, size);
        self.free_stack(size.bytes());
    }

    fn sp_offset(&self) -> SPOffset {
        SPOffset::from_u32(self.sp_offset)
    }

    fn finalize(self, base: Option<SourceLoc>) -> MachBufferFinalized<Final> {
        self.asm.finalize(base)
    }

    fn mov(&mut self, dst: WritableReg, src: RegImm, size: OperandSize) {
        match (src, dst) {
            (RegImm::Imm(v), rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    I::F32(v) => v as u64,
                    I::F64(v) => v,
                    I::V128(_) => todo!(),
                };

                let scratch = regs::scratch();
                self.asm.load_constant(imm, writable!(scratch));
                match rd.to_reg().class() {
                    RegClass::Int => self.asm.mov_rr(scratch, rd, size),
                    RegClass::Float => self.asm.mov_to_fpu(scratch, rd, size),
                    _ => todo!(),
                }
            }
            (RegImm::Reg(rs), rd) => match (rs.class(), rd.to_reg().class()) {
                (RegClass::Int, RegClass::Int) => self.asm.mov_rr(rs, rd, size),
                (RegClass::Float, RegClass::Float) => self.asm.fmov_rr(rs, rd, size),
                (RegClass::Int, RegClass::Float) => self.asm.mov_to_fpu(rs, rd, size),
                _ => todo!(),
            },
        }
    }

    fn cmov(&mut self, dst: WritableReg, src: Reg, cc: IntCmpKind, _size: OperandSize) {
        self.asm.csel(src, src, dst, Cond::from(cc));
    }

    fn add(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.add_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.add_rrr(rm, rn, rd, size);
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
    ) {
        self.add(dst, lhs, rhs, size);
        self.asm.trapif(Cond::Hs, trap);
    }

    fn sub(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.sub_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.sub_rrr(rm, rn, rd, size);
            }
        }
    }

    fn mul(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.mul_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.mul_rrr(rm, rn, rd, size);
            }
        }
    }

    fn float_add(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fadd_rrr(rhs, lhs, dst, size);
    }

    fn float_sub(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fsub_rrr(rhs, lhs, dst, size);
    }

    fn float_mul(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmul_rrr(rhs, lhs, dst, size);
    }

    fn float_div(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fdiv_rrr(rhs, lhs, dst, size);
    }

    fn float_min(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmin_rrr(rhs, lhs, dst, size);
    }

    fn float_max(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmax_rrr(rhs, lhs, dst, size);
    }

    fn float_copysign(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let max_shift = match size {
            OperandSize::S32 => 0x1f,
            OperandSize::S64 => 0x3f,
            _ => unreachable!(),
        };
        self.asm.fushr_rri(rhs, writable!(rhs), max_shift, size);
        self.asm.fsli_rri_mod(lhs, rhs, dst, max_shift, size);
    }

    fn float_neg(&mut self, dst: WritableReg, size: OperandSize) {
        self.asm.fneg_rr(dst.to_reg(), dst, size);
    }

    fn float_abs(&mut self, dst: WritableReg, size: OperandSize) {
        self.asm.fabs_rr(dst.to_reg(), dst, size);
    }

    fn float_round<F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext<Emission>, &mut Self)>(
        &mut self,
        mode: RoundingMode,
        _env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext<Emission>,
        size: OperandSize,
        _fallback: F,
    ) {
        let src = context.pop_to_reg(self, None);
        self.asm
            .fround_rr(src.into(), writable!(src.into()), mode, size);
        context.stack.push(src.into());
    }

    fn float_sqrt(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.fsqrt_rr(src, dst, size);
    }

    fn and(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.and_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.and_rrr(rm, rn, rd, size);
            }
        }
    }

    fn or(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.or_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.or_rrr(rm, rn, rd, size);
            }
        }
    }

    fn xor(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };

                self.asm.xor_ir(imm, rn, rd, size);
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.xor_rrr(rm, rn, rd, size);
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
    ) {
        self.asm.shift_ir(imm, lhs, dst, kind, size)
    }

    fn shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: ShiftKind,
        size: OperandSize,
    ) {
        let src = context.pop_to_reg(self, None);
        let dst = context.pop_to_reg(self, None);

        self.asm
            .shift_rrr(src.into(), dst.into(), writable!(dst.into()), kind, size);

        context.free_reg(src);
        context.stack.push(dst.into());
    }

    fn div(&mut self, context: &mut CodeGenContext<Emission>, kind: DivKind, size: OperandSize) {
        context.binop(self, size, |this, dividend, divisor, size| {
            this.asm
                .div_rrr(divisor, dividend, writable!(dividend), kind, size);
            match size {
                OperandSize::S32 => TypedReg::new(WasmValType::I32, dividend),
                OperandSize::S64 => TypedReg::new(WasmValType::I64, dividend),
                s => unreachable!("invalid size for division: {s:?}"),
            }
        })
    }

    fn rem(&mut self, _context: &mut CodeGenContext<Emission>, _kind: RemKind, _size: OperandSize) {
        todo!()
    }

    fn zero(&mut self, reg: WritableReg) {
        self.asm.load_constant(0, reg);
    }

    fn popcnt(&mut self, context: &mut CodeGenContext<Emission>, size: OperandSize) {
        let src = context.pop_to_reg(self, None);
        let tmp = regs::float_scratch();
        self.asm.mov_to_fpu(src.into(), writable!(tmp), size);
        self.asm.cnt(writable!(tmp));
        self.asm.addv(tmp, writable!(tmp), VectorSize::Size8x8);
        self.asm
            .mov_from_vec(tmp, writable!(src.into()), 0, OperandSize::S8);
        context.stack.push(src.into());
    }

    fn signed_truncate(
        &mut self,
        _dst: WritableReg,
        _src: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
        _kind: TruncKind,
    ) {
        todo!()
    }

    fn unsigned_truncate(
        &mut self,
        _dst: WritableReg,
        _src: Reg,
        _tmp_fpr: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
        _kind: TruncKind,
    ) {
        todo!()
    }

    fn signed_convert(
        &mut self,
        _dst: WritableReg,
        _src: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
    ) {
        todo!()
    }

    fn unsigned_convert(
        &mut self,
        _dst: WritableReg,
        _src: Reg,
        _tmp_gpr: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
    ) {
        todo!()
    }

    fn reinterpret_float_as_int(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.fpu_to_int(src, dst, size);
    }

    fn reinterpret_int_as_float(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.int_to_fpu(src, dst, size);
    }

    fn demote(&mut self, dst: WritableReg, src: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst, OperandSize::S64, OperandSize::S32);
    }

    fn promote(&mut self, dst: WritableReg, src: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst, OperandSize::S32, OperandSize::S64);
    }

    fn push(&mut self, reg: Reg, size: OperandSize) -> StackSlot {
        self.reserve_stack(size.bytes());
        let address = self.address_from_sp(SPOffset::from_u32(self.sp_offset));
        self.asm.str(reg, address, size);

        StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: size.bytes(),
        }
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Self::Address {
        Address::offset(reg, offset as i64)
    }

    fn cmp_with_set(&mut self, dst: WritableReg, src: RegImm, kind: IntCmpKind, size: OperandSize) {
        self.cmp(dst.to_reg(), src, size);
        self.asm.cset(dst, kind.into());
    }

    fn cmp(&mut self, src1: Reg, src2: RegImm, size: OperandSize) {
        match src2 {
            RegImm::Reg(src2) => {
                self.asm.subs_rrr(src2, src1, size);
            }
            RegImm::Imm(v) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };
                self.asm.subs_ir(imm, src1, size);
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
    ) {
        self.asm.fcmp(src1, src2, size);
        self.asm.cset(dst, kind.into());
    }

    fn clz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.clz(src, dst, size);
    }

    fn ctz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        let scratch = regs::scratch();
        self.asm.rbit(src, writable!(scratch), size);
        self.asm.clz(scratch, dst, size);
    }

    fn wrap(&mut self, dst: WritableReg, src: Reg) {
        self.asm.mov_rr(src, dst, OperandSize::S32);
    }

    fn extend(&mut self, dst: WritableReg, src: Reg, kind: ExtendKind) {
        self.asm.extend(src, dst, kind);
    }

    fn get_label(&mut self) -> MachLabel {
        self.asm.get_label()
    }

    fn bind(&mut self, label: MachLabel) {
        let buffer = self.asm.buffer_mut();
        buffer.bind_label(label, &mut Default::default());
    }

    fn branch(
        &mut self,
        kind: IntCmpKind,
        lhs: Reg,
        rhs: RegImm,
        taken: MachLabel,
        size: OperandSize,
    ) {
        use IntCmpKind::*;

        match &(lhs, rhs) {
            (rlhs, RegImm::Reg(rrhs)) => {
                // If the comparison kind is zero or not zero and both operands
                // are the same register, emit a ands instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.ands_rr(*rlhs, *rrhs, size);
                } else {
                    self.cmp(lhs, rhs, size);
                }
            }
            _ => self.cmp(lhs, rhs, size),
        }
        self.asm.jmp_if(kind.into(), taken);
    }

    fn jmp(&mut self, target: MachLabel) {
        self.asm.jmp(target);
    }

    fn unreachable(&mut self) {
        self.asm.udf(wasmtime_cranelift::TRAP_UNREACHABLE);
    }

    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) {
        // At least one default target.
        assert!(targets.len() >= 1);
        let max = targets.len() as u64 - 1;
        self.asm.subs_ir(max, index, OperandSize::S64);
        let default_index = max as usize;
        let default = targets[default_index];
        let rest = &targets[..default_index];
        let tmp1 = regs::scratch();
        self.asm.jmp_table(rest, default, index, tmp1, tmp);
    }

    fn trap(&mut self, code: TrapCode) {
        self.asm.udf(code);
    }

    fn trapz(&mut self, src: Reg, code: TrapCode) {
        self.asm.trapz(src, code);
    }

    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) {
        self.asm.trapif(cc.into(), code);
    }

    fn start_source_loc(&mut self, loc: RelSourceLoc) -> (CodeOffset, RelSourceLoc) {
        self.asm.buffer_mut().start_srcloc(loc)
    }

    fn end_source_loc(&mut self) {
        self.asm.buffer_mut().end_srcloc();
    }

    fn current_code_offset(&self) -> CodeOffset {
        self.asm.buffer().cur_offset()
    }

    fn add128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) {
        let _ = (dst_lo, dst_hi, lhs_lo, lhs_hi, rhs_lo, rhs_hi);
        todo!()
    }

    fn sub128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) {
        let _ = (dst_lo, dst_hi, lhs_lo, lhs_hi, rhs_lo, rhs_hi);
        todo!()
    }

    fn mul_wide(&mut self, context: &mut CodeGenContext<Emission>, kind: MulWideKind) {
        let _ = (context, kind);
        todo!()
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
