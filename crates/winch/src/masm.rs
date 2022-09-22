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

    /// Finalize the assembly and return the result
    // NB: Interim, debug approach
    fn finalize(self) -> Vec<String>;
}
