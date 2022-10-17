use crate::abi::{addressing_mode::Address, local::LocalSlot};
use crate::isa::reg::Reg;

/// Operand size, in bits
#[derive(Copy, Clone)]
pub(crate) enum OperandSize {
    S32,
    S64,
}

/// An abstraction over a register or immediate
pub(crate) enum RegImm {
    Reg(Reg),
    Imm(i32),
}

impl RegImm {
    pub fn reg(r: Reg) -> Self {
        RegImm::Reg(r)
    }

    pub fn imm(imm: i32) -> Self {
        RegImm::Imm(imm)
    }
}

impl From<Reg> for RegImm {
    fn from(r: Reg) -> Self {
        Self::Reg(r)
    }
}

/// Generic MacroAssembler interface used by the compilation environment
///
/// The MacroAssembler trait aims to expose a high-level enough interface so that
/// each ISA can define and use their own low-level Assembler implementation
/// to fulfill the interface

// TODO
//
// Modify the interface in the next iteration once
// there's more support for aarch64;
//
// One of the ideas that I discussed with @cfallin is to
// modify certain sigantures (e.g. add) to take three arguments instead of
// two; assuming params named dst, lhs, rhs, in the case of x64 dst and lhs
// will be always the same

// The other idea, is to have a superset of signatures; which in some cases
// some signatures won't be implemented by all supported ISA's.

pub(crate) trait MacroAssembler {
    /// Emit the function prologue
    fn prologue(&mut self);

    /// Emit the function epilogue
    fn epilogue(&mut self, locals_size: u32);

    /// Reserve stack space
    fn reserve_stack(&mut self, bytes: u32);

    /// Get the address of a local slot
    fn local_address(&mut self, local: &LocalSlot) -> Address;

    /// Get stack pointer offset
    fn sp_offset(&mut self) -> u32;

    /// Perform a stack store
    // TODO is RegImm the best name for the src?
    //      I'd like to think a bit more if there's a better abstraction for this
    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize);

    /// Perform a stack load
    fn load(&mut self, src: Address, dst: Reg, size: OperandSize);

    /// Perform a move
    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize);

    /// Perform add operation
    fn add(&mut self, src: RegImm, dst: RegImm, size: OperandSize);

    /// Push the register to the stack, returning the offset
    fn push(&mut self, src: Reg) -> u32;

    /// Finalize the assembly and return the result
    // NOTE Interim, debug approach
    fn finalize(&mut self) -> &[String];

    /// Spill registers and locals to memory
    // TODO not sure if this should be exposed
    //      at the masm interface level
    fn spill(&mut self) {}

    /// Zero a particular register
    fn zero(&mut self, reg: Reg);
}
