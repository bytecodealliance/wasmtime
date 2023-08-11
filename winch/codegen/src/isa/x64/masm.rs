use super::{
    abi::X64ABI,
    address::Address,
    asm::Assembler,
    regs::{self, rbp, rsp},
};

use crate::abi::ABI;
use crate::masm::{
    CmpKind, DivKind, Imm as I, MacroAssembler as Masm, OperandSize, RegImm, RemKind, ShiftKind,
};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, LocalSlot},
    codegen::CodeGenContext,
    stack::Val,
};
use crate::{isa::reg::Reg, masm::CalleeKind};
use cranelift_codegen::{
    ir::TrapCode, isa::x64::settings as x64_settings, settings, Final, MachBufferFinalized,
    MachLabel,
};

/// x64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// Stack pointer offset.
    sp_offset: u32,
    /// Low level assembler.
    asm: Assembler,
    /// ISA flags.
    flags: x64_settings::Flags,
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = X64ABI;

    fn prologue(&mut self) {
        let frame_pointer = rbp();
        let stack_pointer = rsp();

        self.asm.push_r(frame_pointer);
        self.asm
            .mov_rr(stack_pointer, frame_pointer, OperandSize::S64);
    }

    fn push(&mut self, reg: Reg) -> u32 {
        self.asm.push_r(reg);
        let increment = <Self::ABI as ABI>::word_bytes();
        self.increment_sp(increment);
        self.sp_offset
    }

    fn reserve_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        self.asm.sub_ir(bytes as i32, rsp(), OperandSize::S64);
        self.increment_sp(bytes);
    }

    fn free_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }
        self.asm.add_ir(bytes as i32, rsp(), OperandSize::S64);
        self.decrement_sp(bytes);
    }

    fn reset_stack_pointer(&mut self, offset: u32) {
        self.sp_offset = offset;
    }

    fn local_address(&mut self, local: &LocalSlot) -> Address {
        let (reg, offset) = local
            .addressed_from_sp()
            .then(|| {
                let offset = self.sp_offset.checked_sub(local.offset).expect(&format!(
                    "Invalid local offset = {}; sp offset = {}",
                    local.offset, self.sp_offset
                ));
                (rsp(), offset)
            })
            .unwrap_or((rbp(), local.offset));

        Address::offset(reg, offset)
    }

    fn address_from_sp(&self, offset: u32) -> Self::Address {
        Address::offset(regs::rsp(), self.sp_offset - offset)
    }

    fn address_at_sp(&self, offset: u32) -> Self::Address {
        Address::offset(regs::rsp(), offset)
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
        match src {
            RegImm::Imm(imm) => match imm {
                I::I32(v) => self.asm.mov_im(v as u64, &dst, size),
                I::I64(v) => self.asm.mov_im(v, &dst, size),
                _ => unreachable!(),
            },
            RegImm::Reg(reg) => self.asm.mov_rm(reg, &dst, size),
        }
    }

    fn pop(&mut self, dst: Reg) {
        self.asm.pop_r(dst);
        self.decrement_sp(<Self::ABI as abi::ABI>::word_bytes());
    }

    fn call(
        &mut self,
        stack_args_size: u32,
        mut load_callee: impl FnMut(&mut Self) -> CalleeKind,
    ) -> u32 {
        let alignment: u32 = <Self::ABI as abi::ABI>::call_stack_align().into();
        let addend: u32 = <Self::ABI as abi::ABI>::arg_base_offset().into();
        let delta = calculate_frame_adjustment(self.sp_offset(), addend, alignment);
        let aligned_args_size = align_to(stack_args_size, alignment);
        let total_stack = delta + aligned_args_size;
        self.reserve_stack(total_stack);
        let callee = load_callee(self);
        self.asm.call(callee);
        total_stack
    }

    fn load(&mut self, src: Address, dst: Reg, size: OperandSize) {
        self.asm.mov_mr(&src, dst, size);
    }

    fn sp_offset(&self) -> u32 {
        self.sp_offset
    }

    fn zero(&mut self, reg: Reg) {
        self.asm.xor_rr(reg, reg, OperandSize::S32);
    }

    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize) {
        match (src, dst) {
            (RegImm::Reg(src), RegImm::Reg(dst)) => self.asm.mov_rr(src, dst, size),
            (RegImm::Imm(imm), RegImm::Reg(dst)) => match imm {
                I::I32(v) => self.asm.mov_ir(v as u64, dst, size),
                I::I64(v) => self.asm.mov_ir(v, dst, size),
                _ => unreachable!(),
            },
            _ => Self::handle_invalid_operand_combination(src, dst),
        }
    }

    fn add(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.add_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.add_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.add_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn sub(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.sub_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.sub_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.sub_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn mul(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.mul_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.mul_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.mul_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn and(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.and_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.and_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.and_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn or(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.or_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.or_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.or_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn xor(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), RegImm::Reg(reg)) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.xor_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.xor_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), RegImm::Reg(dst)) => {
                self.asm.xor_rr(src, dst, size);
            }
            _ => Self::handle_invalid_operand_combination(rhs, dst),
        }
    }

    fn shift(&mut self, context: &mut CodeGenContext, kind: ShiftKind, size: OperandSize) {
        let top = context.stack.peek().expect("value at stack top");

        if size == OperandSize::S32 && top.is_i32_const() {
            let val = context
                .stack
                .pop_i32_const()
                .expect("i32 const value at stack top");
            let reg = context.pop_to_reg(self, None, size);

            self.asm.shift_ir(val as u8, reg, kind, size);

            context.stack.push(Val::reg(reg));
        } else if size == OperandSize::S64 && top.is_i64_const() {
            let val = context
                .stack
                .pop_i64_const()
                .expect("i64 const value at stack top");
            let reg = context.pop_to_reg(self, None, size);

            self.asm.shift_ir(val as u8, reg, kind, size);

            context.stack.push(Val::reg(reg));
        } else {
            // Number of bits to shift must be in the CL register.
            let src = context.pop_to_reg(self, Some(regs::rcx()), size);
            let dst = context.pop_to_reg(self, None, size);

            self.asm.shift_rr(src.into(), dst.into(), kind, size);

            context.regalloc.free_gpr(src);
            context.stack.push(Val::reg(dst));
        }
    }

    fn div(&mut self, context: &mut CodeGenContext, kind: DivKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.gpr(regs::rdx(), self);
        let rax = context.gpr(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None, size);

        // Mark rax as allocatable.
        context.regalloc.free_gpr(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax), size);
        self.asm.div(divisor, (rax, rdx), kind, size);

        // Free the divisor and rdx.
        context.free_gpr(divisor);
        context.free_gpr(rdx);

        // Push the quotient.
        context.stack.push(Val::reg(rax));
    }

    fn rem(&mut self, context: &mut CodeGenContext, kind: RemKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.gpr(regs::rdx(), self);
        let rax = context.gpr(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None, size);

        // Mark rax as allocatable.
        context.regalloc.free_gpr(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax), size);
        self.asm.rem(divisor, (rax, rdx), kind, size);

        // Free the divisor and rax.
        context.free_gpr(divisor);
        context.free_gpr(rax);

        // Push the remainder.
        context.stack.push(Val::reg(rdx));
    }

    fn epilogue(&mut self, locals_size: u32) {
        assert!(self.sp_offset == locals_size);

        let rsp = rsp();
        if locals_size > 0 {
            self.asm.add_ir(locals_size as i32, rsp, OperandSize::S64);
        }
        self.asm.pop_r(rbp());
        self.asm.ret();
    }

    fn finalize(self) -> MachBufferFinalized<Final> {
        self.asm.finalize()
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Self::Address {
        Address::offset(reg, offset)
    }

    fn cmp(&mut self, src: RegImm, dst: Reg, size: OperandSize) {
        match src {
            RegImm::Imm(imm) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.cmp_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.cmp_rr(scratch, dst, size);
                }
            }
            RegImm::Reg(src) => {
                self.asm.cmp_rr(src, dst, size);
            }
        }
    }

    fn cmp_with_set(&mut self, src: RegImm, dst: Reg, kind: CmpKind, size: OperandSize) {
        self.cmp(src, dst, size);
        self.asm.setcc(kind, dst);
    }

    fn clz(&mut self, src: Reg, dst: Reg, size: OperandSize) {
        if self.flags.has_lzcnt() {
            self.asm.lzcnt(src, dst, size);
        } else {
            let scratch = regs::scratch();

            // Use the following approach:
            // dst = size.num_bits() - bsr(src) - is_not_zero
            //     = size.num.bits() + -bsr(src) - is_not_zero.
            self.asm.bsr(src.into(), dst.into(), size);
            self.asm.setcc(CmpKind::Ne, scratch.into());
            self.asm.neg(dst, dst, size);
            self.asm.add_ir(size.num_bits(), dst, size);
            self.asm.sub_rr(scratch, dst, size);
        }
    }

    fn ctz(&mut self, src: Reg, dst: Reg, size: OperandSize) {
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
            self.asm.setcc(CmpKind::Eq, scratch.into());
            self.asm
                .shift_ir(size.log2(), scratch, ShiftKind::Shl, size);
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
        kind: CmpKind,
        lhs: RegImm,
        rhs: RegImm,
        taken: MachLabel,
        size: OperandSize,
    ) {
        use CmpKind::*;

        match &(lhs, rhs) {
            (RegImm::Reg(rlhs), RegImm::Reg(rrhs)) => {
                // If the comparision kind is zero or not zero and both operands
                // are the same register, emit a test instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.test_rr(*rrhs, *rlhs, size);
                } else {
                    self.cmp(lhs, rhs.get_reg().unwrap(), size);
                }
            }
            _ => self.cmp(lhs, rhs.get_reg().unwrap(), size),
        }
        self.asm.jmp_if(kind, taken);
    }

    fn jmp(&mut self, target: MachLabel) {
        self.asm.jmp(target);
    }

    fn popcnt(&mut self, context: &mut CodeGenContext, size: OperandSize) {
        let src = context.pop_to_reg(self, None, size);
        if self.flags.has_popcnt() {
            self.asm.popcnt(src, size);
            context.stack.push(Val::reg(src));
        } else {
            // The fallback functionality here is based on `MacroAssembler::popcnt64` in:
            // https://searchfox.org/mozilla-central/source/js/src/jit/x64/MacroAssembler-x64-inl.h#495

            let tmp = context.any_gpr(self);
            let dst = src;
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
            };
            self.asm.mov_rr(src, tmp, size);

            // x -= (x >> 1) & m1;
            self.asm.shift_ir(1u8, dst, ShiftKind::ShrU, size);
            let lhs = dst.into();
            self.and(lhs, lhs, RegImm::i64(masks[0]), size);
            self.asm.sub_rr(dst, tmp, size);

            // x = (x & m2) + ((x >> 2) & m2);
            self.asm.mov_rr(tmp, dst, size);
            // Load `0x3333...` into the scratch reg once, allowing us to use
            // `and_rr` and avoid inadvertently loading it twice as with `and`
            let scratch = regs::scratch();
            self.load_constant(&I::i64(masks[1]), scratch, size);
            self.asm.and_rr(scratch, dst.into(), size);
            self.asm.shift_ir(2u8, tmp, ShiftKind::ShrU, size);
            self.asm.and_rr(scratch, tmp, size);
            self.asm.add_rr(dst, tmp, size);

            // x = (x + (x >> 4)) & m4;
            self.asm.mov_rr(tmp.into(), dst.into(), size);
            self.asm.shift_ir(4u8, dst, ShiftKind::ShrU, size);
            self.asm.add_rr(tmp, dst, size);
            let lhs = dst.into();
            self.and(lhs, lhs, RegImm::i64(masks[2]), size);

            // (x * h01) >> shift_amt
            let lhs = dst.into();
            self.mul(lhs, lhs, RegImm::i64(masks[3]), size);
            self.asm.shift_ir(shift_amt, dst, ShiftKind::ShrU, size);

            context.stack.push(Val::reg(dst));
            context.free_gpr(tmp);
        }
    }

    fn unreachable(&mut self) {
        self.asm.trap(TrapCode::UnreachableCodeReached)
    }
}

impl MacroAssembler {
    /// Create an x64 MacroAssembler.
    pub fn new(shared_flags: settings::Flags, isa_flags: x64_settings::Flags) -> Self {
        Self {
            sp_offset: 0,
            asm: Assembler::new(shared_flags, isa_flags.clone()),
            flags: isa_flags,
        }
    }

    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;
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

    fn load_constant(&mut self, constant: &I, dst: Reg, size: OperandSize) {
        match constant {
            I::I32(v) => self.asm.mov_ir(*v as u64, dst, size),
            I::I64(v) => self.asm.mov_ir(*v, dst, size),
            _ => panic!(),
        }
    }

    fn handle_invalid_operand_combination<T>(src: RegImm, dst: RegImm) -> T {
        panic!("Invalid operand combination; src={:?}, dst={:?}", src, dst);
    }

    fn ensure_two_argument_form(dst: &RegImm, lhs: &RegImm) {
        assert!(
            dst == lhs,
            "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
            dst,
            lhs
        );
    }
}
