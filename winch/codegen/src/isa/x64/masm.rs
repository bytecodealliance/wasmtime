use super::{
    abi::X64ABI,
    address::Address,
    asm::{Assembler, Operand},
    regs::{self, rbp, rsp},
};
use crate::masm::{CmpKind, DivKind, MacroAssembler as Masm, OperandSize, RegImm, RemKind};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, LocalSlot},
    codegen::CodeGenContext,
    stack::Val,
};
use crate::{isa::reg::Reg, masm::CalleeKind};
use cranelift_codegen::{isa::x64::settings as x64_settings, settings, Final, MachBufferFinalized};

/// x64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// Stack pointer offset.
    sp_offset: u32,
    /// Low level assembler.
    asm: Assembler,
}

// Conversions between generic masm arguments and x64 operands.

impl From<RegImm> for Operand {
    fn from(rimm: RegImm) -> Self {
        match rimm {
            RegImm::Reg(r) => r.into(),
            RegImm::Imm(imm) => Operand::Imm(imm),
        }
    }
}

impl From<Reg> for Operand {
    fn from(reg: Reg) -> Self {
        Operand::Reg(reg)
    }
}

impl From<Address> for Operand {
    fn from(addr: Address) -> Self {
        Operand::Mem(addr)
    }
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
        self.increment_sp(<Self::ABI as abi::ABI>::word_bytes());

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
        let src: Operand = src.into();
        let dst: Operand = dst.into();

        self.asm.mov(src, dst, size);
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
        let src = src.into();
        let dst = dst.into();
        self.asm.mov(src, dst, size);
    }

    fn sp_offset(&self) -> u32 {
        self.sp_offset
    }

    fn zero(&mut self, reg: Reg) {
        self.asm.xor_rr(reg, reg, OperandSize::S32);
    }

    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize) {
        let src: Operand = src.into();
        let dst: Operand = dst.into();

        self.asm.mov(src, dst, size);
    }

    fn add(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        let (src, dst): (Operand, Operand) = if dst == lhs {
            (rhs.into(), dst.into())
        } else {
            panic!(
                "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
                dst, lhs
            );
        };

        self.asm.add(src, dst, size);
    }

    fn sub(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        let (src, dst): (Operand, Operand) = if dst == lhs {
            (rhs.into(), dst.into())
        } else {
            panic!(
                "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
                dst, lhs
            );
        };

        self.asm.sub(src, dst, size);
    }

    fn mul(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        let (src, dst): (Operand, Operand) = if dst == lhs {
            (rhs.into(), dst.into())
        } else {
            panic!(
                "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
                dst, lhs
            );
        };

        self.asm.mul(src, dst, size);
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

    fn cmp_with_set(&mut self, src: RegImm, dst: RegImm, kind: CmpKind, size: OperandSize) {
        let dst = dst.into();
        self.asm.cmp(src.into(), dst, size);
        self.asm.setcc(kind, dst);
    }
}

impl MacroAssembler {
    /// Create an x64 MacroAssembler.
    pub fn new(shared_flags: settings::Flags, isa_flags: x64_settings::Flags) -> Self {
        Self {
            sp_offset: 0,
            asm: Assembler::new(shared_flags, isa_flags),
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
}
