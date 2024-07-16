use super::{abi::Aarch64ABI, address::Address, asm::Assembler, regs};
use crate::{
    abi::{self, local::LocalSlot},
    codegen::{ptr_type_from_ptr_size, CodeGenContext, FuncEnv},
    isa::reg::Reg,
    masm::{
        CalleeKind, DivKind, ExtendKind, FloatCmpKind, Imm as I, IntCmpKind,
        MacroAssembler as Masm, OperandSize, RegImm, RemKind, RoundingMode, SPOffset, ShiftKind,
        StackSlot, TrapCode, TruncKind,
    },
};
use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{RelSourceLoc, SourceLoc},
    settings, Final, MachBufferFinalized, MachLabel,
};
use regalloc2::RegClass;
use wasmtime_environ::PtrSize;

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
        self.asm.mov_rr(sp, fp, OperandSize::S64);
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
        self.asm.sub_ir(bytes as u64, sp, sp, OperandSize::S64);
        self.move_sp_to_shadow_sp();

        self.increment_sp(bytes);
    }

    fn free_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        let sp = regs::sp();
        self.asm.add_ir(bytes as u64, sp, sp, OperandSize::S64);
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

    fn address_from_sp(&self, _offset: SPOffset) -> Self::Address {
        todo!()
    }

    fn address_at_sp(&self, _offset: SPOffset) -> Self::Address {
        todo!()
    }

    fn address_at_vmctx(&self, _offset: u32) -> Self::Address {
        todo!()
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) {
        self.store(src.into(), dst, self.ptr_size);
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
        let src = match src {
            RegImm::Imm(v) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    _ => unreachable!(),
                };
                let scratch = regs::scratch();
                self.asm.load_constant(imm, scratch);
                scratch
            }
            RegImm::Reg(reg) => reg,
        };

        self.asm.str(src, dst, size);
    }

    fn wasm_store(&mut self, _src: Reg, _dst: Self::Address, _size: OperandSize) {
        todo!()
    }

    fn call(
        &mut self,
        _stack_args_size: u32,
        _load_callee: impl FnMut(&mut Self) -> CalleeKind,
    ) -> u32 {
        todo!()
    }

    fn load(&mut self, src: Address, dst: Reg, size: OperandSize) {
        self.asm.ldr(src, dst, size);
    }

    fn load_ptr(&mut self, _src: Self::Address, _dst: Reg) {
        todo!()
    }

    fn wasm_load(
        &mut self,
        _src: Self::Address,
        _dst: Reg,
        _size: OperandSize,
        _kind: Option<ExtendKind>,
    ) {
        todo!()
    }

    fn load_addr(&mut self, _src: Self::Address, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn pop(&mut self, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn sp_offset(&self) -> SPOffset {
        SPOffset::from_u32(self.sp_offset)
    }

    fn finalize(self, base: Option<SourceLoc>) -> MachBufferFinalized<Final> {
        self.asm.finalize(base)
    }

    fn mov(&mut self, src: RegImm, dst: Reg, size: OperandSize) {
        match (src, dst) {
            (RegImm::Imm(v), rd) => {
                let imm = match v {
                    I::I32(v) => v as u64,
                    I::I64(v) => v,
                    I::F32(v) => v as u64,
                    I::F64(v) => v,
                };

                let scratch = regs::scratch();
                self.asm.load_constant(imm, scratch);
                match rd.class() {
                    RegClass::Int => self.asm.mov_rr(scratch, rd, size),
                    RegClass::Float => self.asm.mov_to_fpu(scratch, rd, size),
                    _ => todo!(),
                }
            }
            (RegImm::Reg(rs), rd) => match (rs.class(), rd.class()) {
                (RegClass::Int, RegClass::Int) => self.asm.mov_rr(rs, rd, size),
                (RegClass::Float, RegClass::Float) => self.asm.fmov_rr(rs, rd, size),
                (RegClass::Int, RegClass::Float) => self.asm.mov_to_fpu(rs, rd, size),
                _ => todo!(),
            },
        }
    }

    fn cmov(&mut self, _src: Reg, _dst: Reg, _cc: IntCmpKind, _size: OperandSize) {
        todo!()
    }

    fn add(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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
        _dst: Reg,
        _lhs: Reg,
        _rhs: RegImm,
        _size: OperandSize,
        _trap: TrapCode,
    ) {
        todo!()
    }

    fn sub(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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

    fn mul(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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

    fn float_add(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fadd_rrr(rhs, lhs, dst, size);
    }

    fn float_sub(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fsub_rrr(rhs, lhs, dst, size);
    }

    fn float_mul(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmul_rrr(rhs, lhs, dst, size);
    }

    fn float_div(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fdiv_rrr(rhs, lhs, dst, size);
    }

    fn float_min(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmin_rrr(rhs, lhs, dst, size);
    }

    fn float_max(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        self.asm.fmax_rrr(rhs, lhs, dst, size);
    }

    fn float_copysign(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        let max_shift = match size {
            OperandSize::S32 => 0x1f,
            OperandSize::S64 => 0x3f,
            _ => unreachable!(),
        };
        self.asm.fushr_rri(rhs, rhs, max_shift, size);
        self.asm.fsli_rri_mod(lhs, rhs, dst, max_shift, size);
    }

    fn float_neg(&mut self, dst: Reg, size: OperandSize) {
        self.asm.fneg_rr(dst, dst, size);
    }

    fn float_abs(&mut self, dst: Reg, size: OperandSize) {
        self.asm.fabs_rr(dst, dst, size);
    }

    fn float_round<F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext, &mut Self)>(
        &mut self,
        mode: RoundingMode,
        _env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext,
        size: OperandSize,
        _fallback: F,
    ) {
        let src = context.pop_to_reg(self, None);
        self.asm.fround_rr(src.into(), src.into(), mode, size);
        context.stack.push(src.into());
    }

    fn float_sqrt(&mut self, dst: Reg, src: Reg, size: OperandSize) {
        self.asm.fsqrt_rr(src, dst, size);
    }

    fn and(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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

    fn or(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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

    fn xor(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
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

    fn shift_ir(&mut self, dst: Reg, imm: u64, lhs: Reg, kind: ShiftKind, size: OperandSize) {
        self.asm.shift_ir(imm, lhs, dst, kind, size)
    }

    fn shift(&mut self, context: &mut CodeGenContext, kind: ShiftKind, size: OperandSize) {
        let src = context.pop_to_reg(self, None);
        let dst = context.pop_to_reg(self, None);

        self.asm
            .shift_rrr(src.into(), dst.into(), dst.into(), kind, size);

        context.free_reg(src);
        context.stack.push(dst.into());
    }

    fn div(&mut self, _context: &mut CodeGenContext, _kind: DivKind, _size: OperandSize) {
        todo!()
    }

    fn rem(&mut self, _context: &mut CodeGenContext, _kind: RemKind, _size: OperandSize) {
        todo!()
    }

    fn zero(&mut self, reg: Reg) {
        self.asm.load_constant(0, reg);
    }

    fn popcnt(&mut self, _context: &mut CodeGenContext, _size: OperandSize) {
        todo!()
    }

    fn signed_truncate(
        &mut self,
        _src: Reg,
        _dst: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
        _kind: TruncKind,
    ) {
        todo!()
    }

    fn unsigned_truncate(
        &mut self,
        _src: Reg,
        _dst: Reg,
        _tmp_fpr: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
        _kind: TruncKind,
    ) {
        todo!()
    }

    fn signed_convert(
        &mut self,
        _src: Reg,
        _dst: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
    ) {
        todo!()
    }

    fn unsigned_convert(
        &mut self,
        _src: Reg,
        _dst: Reg,
        _tmp_gpr: Reg,
        _src_size: OperandSize,
        _dst_size: OperandSize,
    ) {
        todo!()
    }

    fn reinterpret_float_as_int(&mut self, _src: Reg, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn reinterpret_int_as_float(&mut self, _src: Reg, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn demote(&mut self, _src: Reg, _dst: Reg) {
        todo!()
    }

    fn promote(&mut self, _src: Reg, _dst: Reg) {
        todo!()
    }

    fn push(&mut self, reg: Reg, _size: OperandSize) -> StackSlot {
        let size = <Self::ABI as abi::ABI>::word_bytes();
        self.reserve_stack(size as u32);
        let address = Address::from_shadow_sp(size as i64);
        self.asm.str(reg, address, OperandSize::S64);

        StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: size as u32,
        }
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Self::Address {
        Address::offset(reg, offset as i64)
    }

    fn cmp_with_set(&mut self, _src: RegImm, _dst: Reg, _kind: IntCmpKind, _size: OperandSize) {
        todo!()
    }

    fn cmp(&mut self, _src1: Reg, _src2: RegImm, _size: OperandSize) {
        todo!()
    }

    fn float_cmp_with_set(
        &mut self,
        _src1: Reg,
        _src2: Reg,
        _dst: Reg,
        _kind: FloatCmpKind,
        _size: OperandSize,
    ) {
        todo!()
    }

    fn clz(&mut self, _src: Reg, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn ctz(&mut self, _src: Reg, _dst: Reg, _size: OperandSize) {
        todo!()
    }

    fn wrap(&mut self, _src: Reg, _dst: Reg) {
        todo!()
    }

    fn extend(&mut self, _src: Reg, _dst: Reg, _kind: ExtendKind) {
        todo!()
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
        _kind: IntCmpKind,
        _lhs: Reg,
        _rhs: RegImm,
        _taken: MachLabel,
        _size: OperandSize,
    ) {
        todo!()
    }

    fn jmp(&mut self, _target: MachLabel) {
        todo!()
    }

    fn unreachable(&mut self) {
        todo!()
    }

    fn jmp_table(&mut self, _targets: &[MachLabel], _index: Reg, _tmp: Reg) {
        todo!()
    }

    fn trap(&mut self, _code: TrapCode) {
        todo!()
    }

    fn trapz(&mut self, _src: Reg, _code: TrapCode) {
        todo!()
    }

    fn trapif(&mut self, _cc: IntCmpKind, _code: TrapCode) {
        todo!()
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
        self.asm.mov_rr(sp, shadow_sp, OperandSize::S64);
    }
}
