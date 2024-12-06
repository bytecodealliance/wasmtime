use super::{
    abi::X64ABI,
    address::Address,
    asm::{Assembler, PatchableAddToReg},
    regs::{self, rbp, rsp},
};

use crate::masm::{
    DivKind, ExtendKind, FloatCmpKind, Imm as I, IntCmpKind, MacroAssembler as Masm, MulWideKind,
    OperandSize, RegImm, RemKind, RoundingMode, ShiftKind, TrapCode, TruncKind, TRUSTED_FLAGS,
    UNTRUSTED_FLAGS,
};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, LocalSlot},
    codegen::{ptr_type_from_ptr_size, CodeGenContext, Emission, FuncEnv},
    stack::{TypedReg, Val},
};
use crate::{
    abi::{vmctx, ABI},
    masm::{SPOffset, StackSlot},
};
use crate::{
    isa::{
        reg::{writable, Reg, RegClass, WritableReg},
        CallingConvention,
    },
    masm::CalleeKind,
};
use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{MemFlags, RelSourceLoc, SourceLoc},
    isa::unwind::UnwindInst,
    isa::x64::{
        args::{ExtMode, CC},
        settings as x64_settings,
    },
    settings, Final, MachBufferFinalized, MachLabel,
};
use wasmtime_cranelift::TRAP_UNREACHABLE;
use wasmtime_environ::{PtrSize, WasmValType};

/// x64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// Stack pointer offset.
    sp_offset: u32,
    /// This value represents the maximum stack size seen while compiling the function. While the
    /// function is still being compiled its value will not be valid (the stack will grow and
    /// shrink as space is reserved and freed during compilation), but once all instructions have
    /// been seen this value will be the maximum stack usage seen.
    sp_max: u32,
    /// Add instructions that are used to add the constant stack max to a register.
    stack_max_use_add: Option<PatchableAddToReg>,
    /// Low level assembler.
    asm: Assembler,
    /// ISA flags.
    flags: x64_settings::Flags,
    /// Shared flags.
    shared_flags: settings::Flags,
    /// The target pointer size.
    ptr_size: OperandSize,
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = X64ABI;

    fn frame_setup(&mut self) {
        let frame_pointer = rbp();
        let stack_pointer = rsp();

        self.asm.push_r(frame_pointer);

        if self.shared_flags.unwind_info() {
            self.asm.emit_unwind_inst(UnwindInst::PushFrameRegs {
                offset_upward_to_caller_sp: Self::ABI::arg_base_offset().into(),
            })
        }

        self.asm
            .mov_rr(stack_pointer, writable!(frame_pointer), OperandSize::S64);
    }

    fn check_stack(&mut self, vmctx: Reg) {
        let ptr_size: u8 = self.ptr_size.bytes().try_into().unwrap();
        let scratch = regs::scratch();

        self.load_ptr(
            self.address_at_reg(vmctx, ptr_size.vmcontext_runtime_limits().into()),
            writable!(scratch),
        );

        self.load_ptr(
            Address::offset(scratch, ptr_size.vmruntime_limits_stack_limit().into()),
            writable!(scratch),
        );

        self.add_stack_max(scratch);

        self.asm.cmp_rr(scratch, regs::rsp(), self.ptr_size);
        self.asm.trapif(IntCmpKind::GtU, TrapCode::STACK_OVERFLOW);

        // Emit unwind info.
        if self.shared_flags.unwind_info() {
            self.asm.emit_unwind_inst(UnwindInst::DefineNewFrame {
                offset_upward_to_caller_sp: Self::ABI::arg_base_offset().into(),

                // The Winch calling convention has no callee-save registers, so nothing will be
                // clobbered.
                offset_downward_to_clobbers: 0,
            })
        }
    }

    fn push(&mut self, reg: Reg, size: OperandSize) -> StackSlot {
        let bytes = match (reg.class(), size) {
            (RegClass::Int, OperandSize::S64) => {
                let word_bytes = <Self::ABI as ABI>::word_bytes() as u32;
                self.asm.push_r(reg);
                self.increment_sp(word_bytes);
                word_bytes
            }
            (RegClass::Int, OperandSize::S32) => {
                let bytes = size.bytes();
                self.reserve_stack(bytes);
                let sp_offset = SPOffset::from_u32(self.sp_offset);
                self.asm
                    .mov_rm(reg, &self.address_from_sp(sp_offset), size, TRUSTED_FLAGS);
                bytes
            }
            (RegClass::Float, _) => {
                let bytes = size.bytes();
                self.reserve_stack(bytes);
                let sp_offset = SPOffset::from_u32(self.sp_offset);
                self.asm
                    .xmm_mov_rm(reg, &self.address_from_sp(sp_offset), size, TRUSTED_FLAGS);
                bytes
            }
            _ => unreachable!(),
        };

        StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: bytes,
        }
    }

    fn reserve_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        self.asm
            .sub_ir(bytes as i32, writable!(rsp()), OperandSize::S64);
        self.increment_sp(bytes);
    }

    fn free_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }
        self.asm
            .add_ir(bytes as i32, writable!(rsp()), OperandSize::S64);
        self.decrement_sp(bytes);
    }

    fn reset_stack_pointer(&mut self, offset: SPOffset) {
        self.sp_offset = offset.as_u32();
    }

    fn local_address(&mut self, local: &LocalSlot) -> Address {
        let (reg, offset) = local
            .addressed_from_sp()
            .then(|| {
                let offset = self.sp_offset.checked_sub(local.offset).unwrap_or_else(|| {
                    panic!(
                        "Invalid local offset = {}; sp offset = {}",
                        local.offset, self.sp_offset
                    )
                });
                (rsp(), offset)
            })
            .unwrap_or((rbp(), local.offset));

        Address::offset(reg, offset)
    }

    fn address_from_sp(&self, offset: SPOffset) -> Self::Address {
        Address::offset(regs::rsp(), self.sp_offset - offset.as_u32())
    }

    fn address_at_sp(&self, offset: SPOffset) -> Self::Address {
        Address::offset(regs::rsp(), offset.as_u32())
    }

    fn address_at_vmctx(&self, offset: u32) -> Self::Address {
        Address::offset(vmctx!(Self), offset)
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) {
        self.store(src.into(), dst, self.ptr_size);
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
        self.store_impl(src, dst, size, TRUSTED_FLAGS);
    }

    fn wasm_store(&mut self, src: Reg, dst: Self::Address, size: OperandSize) {
        self.store_impl(src.into(), dst, size, UNTRUSTED_FLAGS);
    }

    fn pop(&mut self, dst: WritableReg, size: OperandSize) {
        let current_sp = SPOffset::from_u32(self.sp_offset);
        match (dst.to_reg().class(), size) {
            (RegClass::Int, OperandSize::S32) => {
                let addr = self.address_from_sp(current_sp);
                self.asm.movzx_mr(&addr, dst, size.into(), TRUSTED_FLAGS);
                self.free_stack(size.bytes());
            }
            (RegClass::Int, OperandSize::S64) => {
                self.asm.pop_r(dst);
                self.decrement_sp(<Self::ABI as ABI>::word_bytes() as u32);
            }
            (RegClass::Float, _) | (RegClass::Vector, _) => {
                let addr = self.address_from_sp(current_sp);
                self.asm.xmm_mov_mr(&addr, dst, size, TRUSTED_FLAGS);
                self.free_stack(size.bytes());
            }
            _ => unreachable!(),
        }
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
        let (callee, cc) = load_callee(self);
        match callee {
            CalleeKind::Indirect(reg) => self.asm.call_with_reg(cc, reg),
            CalleeKind::Direct(idx) => self.asm.call_with_name(cc, idx),
            CalleeKind::LibCall(lib) => self.asm.call_with_lib(cc, lib, regs::scratch()),
        };
        total_stack
    }

    fn load_ptr(&mut self, src: Self::Address, dst: WritableReg) {
        self.load(src, dst, self.ptr_size);
    }

    fn load_addr(&mut self, src: Self::Address, dst: WritableReg, size: OperandSize) {
        self.asm.lea(&src, dst, size);
    }

    fn load(&mut self, src: Address, dst: WritableReg, size: OperandSize) {
        self.load_impl::<Self>(src, dst, size, TRUSTED_FLAGS);
    }

    fn wasm_load(
        &mut self,
        src: Self::Address,
        dst: WritableReg,
        size: OperandSize,
        kind: Option<ExtendKind>,
    ) {
        if let Some(ext) = kind {
            self.asm.movsx_mr(&src, dst, ext, UNTRUSTED_FLAGS);
        } else {
            self.load_impl::<Self>(src, dst, size, UNTRUSTED_FLAGS)
        }
    }

    fn sp_offset(&self) -> SPOffset {
        SPOffset::from_u32(self.sp_offset)
    }

    fn zero(&mut self, reg: WritableReg) {
        self.asm.xor_rr(
            reg.to_reg(),
            reg,
            OperandSize::from_bytes(<Self::ABI>::word_bytes()),
        );
    }

    fn mov(&mut self, dst: WritableReg, src: RegImm, size: OperandSize) {
        match (src, dst.to_reg()) {
            rr @ (RegImm::Reg(src), dst_reg) => match (src.class(), dst_reg.class()) {
                (RegClass::Int, RegClass::Int) => self.asm.mov_rr(src, dst, size),
                (RegClass::Float, RegClass::Float) => self.asm.xmm_mov_rr(src, dst, size),
                _ => Self::handle_invalid_operand_combination(rr.0, rr.1),
            },
            (RegImm::Imm(imm), _) => match imm {
                I::I32(v) => self.asm.mov_ir(v as u64, dst, size),
                I::I64(v) => self.asm.mov_ir(v, dst, size),
                I::F32(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, dst, size, TRUSTED_FLAGS);
                }
                I::F64(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, dst, size, TRUSTED_FLAGS);
                }
                I::V128(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, dst, size, TRUSTED_FLAGS);
                }
            },
        }
    }

    fn cmov(&mut self, dst: WritableReg, src: Reg, cc: IntCmpKind, size: OperandSize) {
        match (src.class(), dst.to_reg().class()) {
            (RegClass::Int, RegClass::Int) => self.asm.cmov(src, dst, cc, size),
            (RegClass::Float, RegClass::Float) => self.asm.xmm_cmov(src, dst, cc, size),
            _ => Self::handle_invalid_operand_combination(src, dst.to_reg()),
        }
    }

    fn add(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), _) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.add_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.add_rr(scratch, dst, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.add_rr(src, dst, size);
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
        self.asm.trapif(CC::B, trap);
    }

    fn sub(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.sub_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.sub_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.sub_rr(src, dst, size);
            }
        }
    }

    fn mul(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), _) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.mul_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.mul_rr(scratch, dst, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.mul_rr(src, dst, size);
            }
        }
    }

    fn float_add(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_add_rr(rhs, dst, size);
    }

    fn float_sub(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_sub_rr(rhs, dst, size);
    }

    fn float_mul(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_mul_rr(rhs, dst, size);
    }

    fn float_div(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_div_rr(rhs, dst, size);
    }

    fn float_min(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_min_seq(rhs, dst, size);
    }

    fn float_max(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.xmm_max_seq(rhs, dst, size);
    }

    fn float_copysign(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        let scratch_gpr = regs::scratch();
        let scratch_xmm = regs::scratch_xmm();
        let sign_mask = match size {
            OperandSize::S32 => I::I32(0x80000000),
            OperandSize::S64 => I::I64(0x8000000000000000),
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };
        self.load_constant(&sign_mask, writable!(scratch_gpr), size);
        self.asm
            .gpr_to_xmm(scratch_gpr, writable!(scratch_xmm), size);

        // Clear everything except sign bit in src.
        self.asm.xmm_and_rr(scratch_xmm, writable!(rhs), size);

        // Clear sign bit in dst using scratch to store result. Then copy the
        // result back to dst.
        self.asm
            .xmm_andn_rr(dst.to_reg(), writable!(scratch_xmm), size);
        self.asm.xmm_mov_rr(scratch_xmm, dst, size);

        // Copy sign bit from src to dst.
        self.asm.xmm_or_rr(rhs, dst, size);
    }

    fn float_neg(&mut self, dst: WritableReg, size: OperandSize) {
        assert_eq!(dst.to_reg().class(), RegClass::Float);
        let mask = match size {
            OperandSize::S32 => I::I32(0x80000000),
            OperandSize::S64 => I::I64(0x8000000000000000),
            OperandSize::S8 | OperandSize::S16 | OperandSize::S128 => unreachable!(),
        };
        let scratch_gpr = regs::scratch();
        self.load_constant(&mask, writable!(scratch_gpr), size);
        let scratch_xmm = regs::scratch_xmm();
        self.asm
            .gpr_to_xmm(scratch_gpr, writable!(scratch_xmm), size);
        self.asm.xmm_xor_rr(scratch_xmm, dst, size);
    }

    fn float_abs(&mut self, dst: WritableReg, size: OperandSize) {
        assert_eq!(dst.to_reg().class(), RegClass::Float);
        let mask = match size {
            OperandSize::S32 => I::I32(0x7fffffff),
            OperandSize::S64 => I::I64(0x7fffffffffffffff),
            OperandSize::S128 | OperandSize::S16 | OperandSize::S8 => unreachable!(),
        };
        let scratch_gpr = regs::scratch();
        self.load_constant(&mask, writable!(scratch_gpr), size);
        let scratch_xmm = regs::scratch_xmm();
        self.asm
            .gpr_to_xmm(scratch_gpr, writable!(scratch_xmm), size);
        self.asm.xmm_and_rr(scratch_xmm, dst, size);
    }

    fn float_round<F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext<Emission>, &mut Self)>(
        &mut self,
        mode: RoundingMode,
        env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext<Emission>,
        size: OperandSize,
        mut fallback: F,
    ) {
        if self.flags.has_sse41() {
            let src = context.pop_to_reg(self, None);
            self.asm
                .xmm_rounds_rr(src.into(), writable!(src.into()), mode, size);
            context.stack.push(src.into());
        } else {
            fallback(env, context, self);
        }
    }

    fn float_sqrt(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.sqrt(src, dst, size);
    }

    fn and(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), _) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.and_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.and_rr(scratch, dst, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.and_rr(src, dst, size);
            }
        }
    }

    fn or(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), _) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.or_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.or_rr(scratch, dst, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.or_rr(src, dst, size);
            }
        }
    }

    fn xor(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), _) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.xor_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.xor_rr(scratch, dst, size);
                }
            }

            (RegImm::Reg(src), _) => {
                self.asm.xor_rr(src, dst, size);
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
        Self::ensure_two_argument_form(&dst.to_reg(), &lhs);
        self.asm.shift_ir(imm as u8, dst, kind, size)
    }

    fn shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: ShiftKind,
        size: OperandSize,
    ) {
        // Number of bits to shift must be in the CL register.
        let src = context.pop_to_reg(self, Some(regs::rcx()));
        let dst = context.pop_to_reg(self, None);

        self.asm
            .shift_rr(src.into(), writable!(dst.into()), kind, size);

        context.free_reg(src);
        context.stack.push(dst.into());
    }

    fn div(&mut self, context: &mut CodeGenContext<Emission>, kind: DivKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.reg(regs::rdx(), self);
        let rax = context.reg(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None);

        // Mark rax as allocatable.
        context.free_reg(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax));
        self.asm.div(divisor.into(), (rax.into(), rdx), kind, size);

        // Free the divisor and rdx.
        context.free_reg(divisor);
        context.free_reg(rdx);

        // Push the quotient.
        context.stack.push(rax.into());
    }

    fn rem(&mut self, context: &mut CodeGenContext<Emission>, kind: RemKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.reg(regs::rdx(), self);
        let rax = context.reg(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None);

        // Mark rax as allocatable.
        context.free_reg(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax));
        self.asm.rem(divisor.reg, (rax.into(), rdx), kind, size);

        // Free the divisor and rax.
        context.free_reg(divisor);
        context.free_reg(rax);

        // Push the remainder.
        context.stack.push(Val::reg(rdx, divisor.ty));
    }

    fn frame_restore(&mut self) {
        assert_eq!(self.sp_offset, 0);
        self.asm.pop_r(writable!(rbp()));
        self.asm.ret();
    }

    fn finalize(mut self, base: Option<SourceLoc>) -> MachBufferFinalized<Final> {
        if let Some(patch) = self.stack_max_use_add {
            patch.finalize(i32::try_from(self.sp_max).unwrap(), self.asm.buffer_mut());
        }

        self.asm.finalize(base)
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Self::Address {
        Address::offset(reg, offset)
    }

    fn cmp(&mut self, src1: Reg, src2: RegImm, size: OperandSize) {
        match src2 {
            RegImm::Imm(imm) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.cmp_ir(src1, v, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, writable!(scratch), size);
                    self.asm.cmp_rr(src1, scratch, size);
                }
            }
            RegImm::Reg(src2) => {
                self.asm.cmp_rr(src1, src2, size);
            }
        }
    }

    fn cmp_with_set(&mut self, dst: WritableReg, src: RegImm, kind: IntCmpKind, size: OperandSize) {
        self.cmp(dst.to_reg(), src, size);
        self.asm.setcc(kind, dst);
    }

    fn float_cmp_with_set(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: Reg,
        kind: FloatCmpKind,
        size: OperandSize,
    ) {
        // Float comparisons needs to be ordered (that is, comparing with a NaN
        // should return 0) except for not equal which needs to be unordered.
        // We use ucomis{s, d} because comis{s, d} has an undefined result if
        // either operand is NaN. Since ucomis{s, d} is unordered, we need to
        // compensate to make the comparison ordered.  Ucomis{s, d} sets the
        // ZF, PF, and CF flags if there is an unordered result.
        let (src1, src2, set_kind) = match kind {
            FloatCmpKind::Eq => (src1, src2, IntCmpKind::Eq),
            FloatCmpKind::Ne => (src1, src2, IntCmpKind::Ne),
            FloatCmpKind::Gt => (src1, src2, IntCmpKind::GtU),
            FloatCmpKind::Ge => (src1, src2, IntCmpKind::GeU),
            // Reversing the operands and using the complementary comparison
            // avoids needing to perform an additional SETNP and AND
            // instruction.
            // SETNB and SETNBE check if the carry flag is unset (i.e., not
            // less than and not unordered) so we get the intended result
            // without having to look at the parity flag.
            FloatCmpKind::Lt => (src2, src1, IntCmpKind::GtU),
            FloatCmpKind::Le => (src2, src1, IntCmpKind::GeU),
        };
        self.asm.ucomis(src1, src2, size);
        self.asm.setcc(set_kind, dst);
        match kind {
            FloatCmpKind::Eq | FloatCmpKind::Gt | FloatCmpKind::Ge => {
                // Return false if either operand is NaN by ensuring PF is
                // unset.
                let scratch = regs::scratch();
                self.asm.setnp(writable!(scratch));
                self.asm.and_rr(scratch, dst, size);
            }
            FloatCmpKind::Ne => {
                // Return true if either operand is NaN by checking if PF is
                // set.
                let scratch = regs::scratch();
                self.asm.setp(writable!(scratch));
                self.asm.or_rr(scratch, dst, size);
            }
            FloatCmpKind::Lt | FloatCmpKind::Le => (),
        }
    }

    fn clz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        if self.flags.has_lzcnt() {
            self.asm.lzcnt(src, dst, size);
        } else {
            let scratch = regs::scratch();

            // Use the following approach:
            // dst = size.num_bits() - bsr(src) - is_not_zero
            //     = size.num.bits() + -bsr(src) - is_not_zero.
            self.asm.bsr(src.into(), dst, size);
            self.asm.setcc(IntCmpKind::Ne, writable!(scratch.into()));
            self.asm.neg(dst.to_reg(), dst, size);
            self.asm.add_ir(size.num_bits() as i32, dst, size);
            self.asm.sub_rr(scratch, dst, size);
        }
    }

    fn ctz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        if self.flags.has_bmi1() {
            self.asm.tzcnt(src, dst, size);
        } else {
            let scratch = regs::scratch();

            // Use the following approach:
            // dst = bsf(src) + (is_zero * size.num_bits())
            //     = bsf(src) + (is_zero << size.log2()).
            // BSF outputs the correct value for every value except 0.
            // When the value is 0, BSF outputs 0, correct output for ctz is
            // the number of bits.
            self.asm.bsf(src.into(), dst.into(), size);
            self.asm.setcc(IntCmpKind::Eq, writable!(scratch.into()));
            self.asm
                .shift_ir(size.log2(), writable!(scratch), ShiftKind::Shl, size);
            self.asm.add_rr(scratch, dst, size);
        }
    }

    fn get_label(&mut self) -> MachLabel {
        let buffer = self.asm.buffer_mut();
        buffer.get_label()
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
                // are the same register, emit a test instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.test_rr(*rlhs, *rrhs, size);
                } else {
                    self.cmp(lhs, rhs, size);
                }
            }
            _ => self.cmp(lhs, rhs, size),
        }
        self.asm.jmp_if(kind, taken);
    }

    fn jmp(&mut self, target: MachLabel) {
        self.asm.jmp(target);
    }

    fn popcnt(&mut self, context: &mut CodeGenContext<Emission>, size: OperandSize) {
        let src = context.pop_to_reg(self, None);
        if self.flags.has_popcnt() && self.flags.has_sse42() {
            self.asm.popcnt(src.into(), size);
            context.stack.push(src.into());
        } else {
            // The fallback functionality here is based on `MacroAssembler::popcnt64` in:
            // https://searchfox.org/mozilla-central/source/js/src/jit/x64/MacroAssembler-x64-inl.h#495

            let tmp = writable!(context.any_gpr(self));
            let dst = writable!(src.into());
            let (masks, shift_amt) = match size {
                OperandSize::S64 => (
                    [
                        0x5555555555555555, // m1
                        0x3333333333333333, // m2
                        0x0f0f0f0f0f0f0f0f, // m4
                        0x0101010101010101, // h01
                    ],
                    56u8,
                ),
                // 32-bit popcount is the same, except the masks are half as
                // wide and we shift by 24 at the end rather than 56
                OperandSize::S32 => (
                    [0x55555555i64, 0x33333333i64, 0x0f0f0f0fi64, 0x01010101i64],
                    24u8,
                ),
                _ => unreachable!(),
            };
            self.asm.mov_rr(src.into(), tmp, size);

            // x -= (x >> 1) & m1;
            self.asm.shift_ir(1u8, dst, ShiftKind::ShrU, size);
            let lhs = dst.to_reg();
            self.and(writable!(lhs), lhs, RegImm::i64(masks[0]), size);
            self.asm.sub_rr(dst.to_reg(), tmp, size);

            // x = (x & m2) + ((x >> 2) & m2);
            self.asm.mov_rr(tmp.to_reg(), dst, size);
            // Load `0x3333...` into the scratch reg once, allowing us to use
            // `and_rr` and avoid inadvertently loading it twice as with `and`
            let scratch = regs::scratch();
            self.load_constant(&I::i64(masks[1]), writable!(scratch), size);
            self.asm.and_rr(scratch, dst, size);
            self.asm.shift_ir(2u8, tmp, ShiftKind::ShrU, size);
            self.asm.and_rr(scratch, tmp, size);
            self.asm.add_rr(dst.to_reg(), tmp, size);

            // x = (x + (x >> 4)) & m4;
            self.asm.mov_rr(tmp.to_reg(), dst.into(), size);
            self.asm.shift_ir(4u8, dst.into(), ShiftKind::ShrU, size);
            self.asm.add_rr(tmp.to_reg(), dst, size);
            let lhs = dst.to_reg();
            self.and(writable!(lhs), lhs, RegImm::i64(masks[2]), size);

            // (x * h01) >> shift_amt
            let lhs = dst.to_reg();
            self.mul(writable!(lhs), lhs, RegImm::i64(masks[3]), size);
            self.asm
                .shift_ir(shift_amt, dst.into(), ShiftKind::ShrU, size);

            context.stack.push(src.into());
            context.free_reg(tmp.to_reg());
        }
    }

    fn wrap(&mut self, dst: WritableReg, src: Reg) {
        self.asm.mov_rr(src.into(), dst, OperandSize::S32);
    }

    fn extend(&mut self, dst: WritableReg, src: Reg, kind: ExtendKind) {
        if let ExtendKind::I64ExtendI32U = kind {
            self.asm.movzx_rr(src, dst, kind);
        } else {
            self.asm.movsx_rr(src, dst, kind);
        }
    }

    fn signed_truncate(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) {
        self.asm.cvt_float_to_sint_seq(
            src,
            dst,
            regs::scratch(),
            regs::scratch_xmm(),
            src_size,
            dst_size,
            kind.is_checked(),
        );
    }

    fn unsigned_truncate(
        &mut self,
        dst: WritableReg,
        src: Reg,
        tmp_fpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) {
        self.asm.cvt_float_to_uint_seq(
            src,
            dst,
            regs::scratch(),
            regs::scratch_xmm(),
            tmp_fpr,
            src_size,
            dst_size,
            kind.is_checked(),
        );
    }

    fn signed_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        self.asm.cvt_sint_to_float(src, dst, src_size, dst_size);
    }

    fn unsigned_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        tmp_gpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        // Need to convert unsigned uint32 to uint64 for conversion instruction sequence.
        if let OperandSize::S32 = src_size {
            self.extend(writable!(src), src, ExtendKind::I64ExtendI32U);
        }

        self.asm
            .cvt_uint64_to_float_seq(src, dst, regs::scratch(), tmp_gpr, dst_size);
    }

    fn reinterpret_float_as_int(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.xmm_to_gpr(src, dst, size);
    }

    fn reinterpret_int_as_float(&mut self, dst: WritableReg, src: Reg, size: OperandSize) {
        self.asm.gpr_to_xmm(src.into(), dst, size);
    }

    fn demote(&mut self, dst: WritableReg, src: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst.into(), OperandSize::S64, OperandSize::S32);
    }

    fn promote(&mut self, dst: WritableReg, src: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst, OperandSize::S32, OperandSize::S64);
    }

    fn unreachable(&mut self) {
        self.asm.trap(TRAP_UNREACHABLE)
    }

    fn trap(&mut self, code: TrapCode) {
        self.asm.trap(code);
    }

    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) {
        self.asm.trapif(cc, code);
    }

    fn trapz(&mut self, src: Reg, code: TrapCode) {
        self.asm.test_rr(src, src, self.ptr_size);
        self.asm.trapif(IntCmpKind::Eq, code);
    }

    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) {
        // At least one default target.
        assert!(targets.len() >= 1);
        let default_index = targets.len() - 1;
        // Emit bounds check, by conditionally moving the max cases
        // into the given index reg if the contents of the index reg
        // are greater.
        let max = default_index;
        let size = OperandSize::S32;
        self.asm.mov_ir(max as u64, writable!(tmp), size);
        self.asm.cmp_rr(tmp, index, size);
        self.asm.cmov(tmp, writable!(index), IntCmpKind::LtU, size);

        let default = targets[default_index];
        let rest = &targets[0..default_index];
        let tmp1 = regs::scratch();
        self.asm.jmp_table(rest.into(), default, index, tmp1, tmp);
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
        Self::ensure_two_argument_form(&dst_lo.to_reg(), &lhs_lo);
        Self::ensure_two_argument_form(&dst_hi.to_reg(), &lhs_hi);
        self.asm.add_rr(rhs_lo, dst_lo, OperandSize::S64);
        self.asm.adc_rr(rhs_hi, dst_hi, OperandSize::S64);
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
        Self::ensure_two_argument_form(&dst_lo.to_reg(), &lhs_lo);
        Self::ensure_two_argument_form(&dst_hi.to_reg(), &lhs_hi);
        self.asm.sub_rr(rhs_lo, dst_lo, OperandSize::S64);
        self.asm.sbb_rr(rhs_hi, dst_hi, OperandSize::S64);
    }

    fn mul_wide(&mut self, context: &mut CodeGenContext<Emission>, kind: MulWideKind) {
        // Reserve rax/rdx since they're required by the `mul_wide` instruction
        // being used here.
        let rax = context.reg(regs::rax(), self);
        let rdx = context.reg(regs::rdx(), self);

        // The rhs of this binop can be in any register
        let rhs = context.pop_to_reg(self, None);
        // Mark rax as allocatable. and then force the lhs operand to be placed
        // in `rax`.
        context.free_reg(rax);
        let lhs = context.pop_to_reg(self, Some(rax));

        self.asm.mul_wide(
            writable!(rax),
            writable!(rdx),
            lhs.reg,
            rhs.reg,
            kind,
            OperandSize::S64,
        );

        // No longer using the rhs register after the multiplication has been
        // executed.
        context.free_reg(rhs);

        // The low bits of the result are in rax, where `lhs` was allocated to
        context.stack.push(lhs.into());
        // The high bits of the result are in rdx, which we previously reserved.
        context.stack.push(Val::Reg(TypedReg::i64(rdx)));
    }
}

impl MacroAssembler {
    /// Create an x64 MacroAssembler.
    pub fn new(
        ptr_size: impl PtrSize,
        shared_flags: settings::Flags,
        isa_flags: x64_settings::Flags,
    ) -> Self {
        let ptr_type: WasmValType = ptr_type_from_ptr_size(ptr_size.size()).into();

        Self {
            sp_offset: 0,
            sp_max: 0,
            stack_max_use_add: None,
            asm: Assembler::new(shared_flags.clone(), isa_flags.clone()),
            flags: isa_flags,
            shared_flags,
            ptr_size: ptr_type.into(),
        }
    }

    /// Add the maximum stack used to a register, recording an obligation to update the
    /// add-with-immediate instruction emitted to use the real stack max when the masm is being
    /// finalized.
    fn add_stack_max(&mut self, reg: Reg) {
        assert!(self.stack_max_use_add.is_none());
        let patch = PatchableAddToReg::new(reg, OperandSize::S64, self.asm.buffer_mut());
        self.stack_max_use_add.replace(patch);
    }

    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;

        // NOTE: we use `max` here to track the largest stack allocation in `sp_max`. Once we have
        // seen the entire function, this value will represent the maximum size for the stack
        // frame.
        self.sp_max = self.sp_max.max(self.sp_offset);
    }

    fn decrement_sp(&mut self, bytes: u32) {
        assert!(
            self.sp_offset >= bytes,
            "sp offset = {}; bytes = {}",
            self.sp_offset,
            bytes
        );
        self.sp_offset -= bytes;
    }

    fn load_constant(&mut self, constant: &I, dst: WritableReg, size: OperandSize) {
        match constant {
            I::I32(v) => self.asm.mov_ir(*v as u64, dst, size),
            I::I64(v) => self.asm.mov_ir(*v, dst, size),
            _ => panic!(),
        }
    }

    /// A common implementation for zero-extend stack loads.
    fn load_impl<M>(&mut self, src: Address, dst: WritableReg, size: OperandSize, flags: MemFlags)
    where
        M: Masm,
    {
        if dst.to_reg().is_int() {
            let access_bits = size.num_bits() as u16;

            let ext_mode = match access_bits {
                8 => Some(ExtMode::BQ),
                16 => Some(ExtMode::WQ),
                32 => Some(ExtMode::LQ),
                _ => None,
            };

            self.asm.movzx_mr(&src, dst, ext_mode, flags);
        } else {
            self.asm.xmm_mov_mr(&src, dst, size, flags);
        }
    }

    /// A common implementation for stack stores.
    fn store_impl(&mut self, src: RegImm, dst: Address, size: OperandSize, flags: MemFlags) {
        match src {
            RegImm::Imm(imm) => match imm {
                I::I32(v) => self.asm.mov_im(v as i32, &dst, size, flags),
                I::I64(v) => match v.try_into() {
                    Ok(v) => self.asm.mov_im(v, &dst, size, flags),
                    Err(_) => {
                        // If the immediate doesn't sign extend, use a scratch
                        // register.
                        let scratch = regs::scratch();
                        self.asm.mov_ir(v, writable!(scratch), size);
                        self.asm.mov_rm(scratch, &dst, size, flags);
                    }
                },
                I::F32(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    let float_scratch = regs::scratch_xmm();
                    // Always trusted, since we are loading the constant from
                    // the constant pool.
                    self.asm
                        .xmm_mov_mr(&addr, writable!(float_scratch), size, MemFlags::trusted());
                    self.asm.xmm_mov_rm(float_scratch, &dst, size, flags);
                }
                I::F64(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    let float_scratch = regs::scratch_xmm();
                    // Similar to above, always trusted since we are loading the
                    // constant from the constant pool.
                    self.asm
                        .xmm_mov_mr(&addr, writable!(float_scratch), size, MemFlags::trusted());
                    self.asm.xmm_mov_rm(float_scratch, &dst, size, flags);
                }
                I::V128(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    let vector_scratch = regs::scratch_xmm();
                    // Always trusted, since we are loading the constant from
                    // the constant pool.
                    self.asm.xmm_mov_mr(
                        &addr,
                        writable!(vector_scratch),
                        size,
                        MemFlags::trusted(),
                    );
                    self.asm.xmm_mov_rm(vector_scratch, &dst, size, flags);
                }
            },
            RegImm::Reg(reg) => {
                if reg.is_int() {
                    self.asm.mov_rm(reg, &dst, size, flags);
                } else {
                    self.asm.xmm_mov_rm(reg, &dst, size, flags);
                }
            }
        }
    }

    fn handle_invalid_operand_combination<T>(src: impl Into<RegImm>, dst: impl Into<RegImm>) -> T {
        panic!(
            "Invalid operand combination; src={:?}, dst={:?}",
            src.into(),
            dst.into()
        );
    }

    fn ensure_two_argument_form(dst: &Reg, lhs: &Reg) {
        assert!(
            dst == lhs,
            "the destination and first source argument must be the same, dst={dst:?}, lhs={lhs:?}"
        );
    }
}
