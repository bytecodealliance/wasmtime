use crate::abi::{addressing_mode::Address, local::LocalSlot};
use crate::isa::reg::Reg;

/// Operand size, in bits
pub(crate) enum OperandSize {
    S32,
    S64,
}

/// Generic MacroAssembler interface used by the compilation environment
///
/// The MacroAssembler trait aims to expose a high-level enough interface so that
/// each ISA can define and use their own low-level Assembler implementation
/// to fulfill the interface

pub(crate) trait MacroAssembler {
    /// Emit the function prologue
    fn prologue(&mut self);

    /// Emit the function epilogue
    fn epilogue(&mut self);

    /// Reserve stack space
    fn reserve_stack(&mut self, bytes: u32);

    /// Get the address of a local slot
    fn local_address(&mut self, local: &LocalSlot) -> Address;

    /// Get stack pointer offset
    fn sp_offset(&mut self) -> u32;

    /// Perform a sized register stack store
    // TODO augent the src argument to be an enum;
    //      in this case the src can also be an immediate
    fn store(&mut self, src: Reg, dst: Address, size: OperandSize);

    /// Finalize the assembly and return the result
    // NOTE Interim, debug approach
    fn finalize(&mut self) -> &[String];
}
