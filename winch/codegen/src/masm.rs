use crate::abi::align_to;
use crate::abi::{Address, LocalSlot};
use crate::isa::reg::Reg;
use crate::regalloc::RegAlloc;
use cranelift_codegen::{Final, MachBufferFinalized};
use std::ops::Range;

/// Operand size, in bits.
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum OperandSize {
    /// 32 bits.
    S32,
    /// 64 bits.
    S64,
}

/// An abstraction over a register or immediate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegImm {
    /// A register.
    Reg(Reg),
    /// An immediate.
    Imm(i32),
}

impl RegImm {
    /// Register constructor.
    pub fn reg(r: Reg) -> Self {
        RegImm::Reg(r)
    }

    /// Immediate constructor.
    pub fn imm(imm: i32) -> Self {
        RegImm::Imm(imm)
    }
}

impl From<Reg> for RegImm {
    fn from(r: Reg) -> Self {
        Self::Reg(r)
    }
}

/// Generic MacroAssembler interface used by the code generation.
///
/// The MacroAssembler trait aims to expose an interface, high-level enough,
/// so that each ISA can provide its own lowering to machine code. For example,
/// for WebAssembly operators that don't have a direct mapping to a machine
/// a instruction, the interface defines a signature matching the WebAssembly
/// operator, allowing each implementation to lower such operator entirely.
/// This approach attributes more responsibility to the MacroAssembler, but frees
/// the caller from concerning about assembling the right sequence of
/// instructions at the operator callsite.
///
/// The interface defaults to a three-argument form for binary operations;
/// this allows a natural mapping to instructions for RISC architectures,
/// that use three-argument form.
/// This approach allows for a more general interface that can be restricted
/// where needed, in the case of architectures that use a two-argument form.

pub(crate) trait MacroAssembler {
    /// Emit the function prologue.
    fn prologue(&mut self);

    /// Emit the function epilogue.
    fn epilogue(&mut self, locals_size: u32);

    /// Reserve stack space.
    fn reserve_stack(&mut self, bytes: u32);

    /// Get the address of a local slot.
    fn local_address(&mut self, local: &LocalSlot) -> Address;

    /// Get stack pointer offset.
    fn sp_offset(&mut self) -> u32;

    /// Perform a stack store.
    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize);

    /// Perform a stack load.
    fn load(&mut self, src: Address, dst: Reg, size: OperandSize);

    /// Perform a move.
    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize);

    /// Perform add operation.
    fn add(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize);

    /// Push the register to the stack, returning the offset.
    fn push(&mut self, src: Reg) -> u32;

    /// Finalize the assembly and return the result.
    fn finalize(self) -> MachBufferFinalized<Final>;

    /// Zero a particular register.
    fn zero(&mut self, reg: Reg);

    /// Zero a given memory range.
    ///
    /// The default implementation divides the given memory range
    /// into word-sized slots. Then it unrolls a series of store
    /// instructions, effectively assigning zero to each slot.
    fn zero_mem_range(&mut self, mem: &Range<u32>, word_size: u32, regalloc: &mut RegAlloc) {
        if mem.is_empty() {
            return;
        }

        let start = if mem.start % word_size == 0 {
            mem.start
        } else {
            // Ensure that the start of the range is at least 4-byte aligned.
            assert!(mem.start % 4 == 0);
            let start = align_to(mem.start, word_size);
            let addr = self.local_address(&LocalSlot::i32(start));
            self.store(RegImm::imm(0), addr, OperandSize::S32);
            // Ensure that the new start of the range, is word-size aligned.
            assert!(start % word_size == 0);
            start
        };

        let end = align_to(mem.end, word_size);
        let slots = (end - start) / word_size;

        if slots == 1 {
            let slot = LocalSlot::i64(start + word_size);
            let addr = self.local_address(&slot);
            self.store(RegImm::imm(0), addr, OperandSize::S64);
        } else {
            // TODO
            // Add an upper bound to this generation;
            // given a considerably large amount of slots
            // this will be inefficient.
            let zero = regalloc.scratch;
            self.zero(zero);
            let zero = RegImm::reg(zero);

            for step in (start..end).into_iter().step_by(word_size as usize) {
                let slot = LocalSlot::i64(step + word_size);
                let addr = self.local_address(&slot);
                self.store(zero, addr, OperandSize::S64);
            }
        }
    }
}
