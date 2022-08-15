//! ABI definitions.

use crate::machinst::*;
use smallvec::SmallVec;

/// A small vector of instructions (with some reasonable size); appropriate for
/// a small fixed sequence implementing one operation.
pub type SmallInstVec<I> = SmallVec<[I; 4]>;

/// Trait implemented by an object that tracks ABI-related state and can
/// generate code while emitting a *call* to a function.
///
/// An instance of this trait returns information for a *particular*
/// callsite. It will usually be computed from the called function's
/// signature.
///
/// Unlike `Callee`, methods on this trait are not invoked directly by the
/// machine-independent code. Rather, the machine-specific lowering code will
/// typically create an `ABICaller` when creating machine instructions for an IR
/// call instruction inside `lower()`, directly emit the arg and and retval
/// copies, and attach the register use/def info to the call.
///
/// This trait is thus provided for convenience to the backends.
pub trait ABICaller {
    /// The instruction type for the ISA associated with this ABI.
    type I: VCodeInst;

    /// Get the number of arguments expected.
    fn num_args(&self) -> usize;

    /// Emit a copy of an argument value from a source register, prior to the call.
    /// For large arguments with associated stack buffer, this may load the address
    /// of the buffer into the argument register, if required by the ABI.
    fn emit_copy_regs_to_arg(&self, ctx: &mut Lower<Self::I>, idx: usize, from_reg: ValueRegs<Reg>);

    /// Emit a copy of a large argument into its associated stack buffer, if any.
    /// We must be careful to perform all these copies (as necessary) before setting
    /// up the argument registers, since we may have to invoke memcpy(), which could
    /// clobber any registers already set up.  The back-end should call this routine
    /// for all arguments before calling emit_copy_regs_to_arg for all arguments.
    fn emit_copy_regs_to_buffer(
        &self,
        ctx: &mut Lower<Self::I>,
        idx: usize,
        from_reg: ValueRegs<Reg>,
    );

    /// Emit a copy a return value into a destination register, after the call returns.
    fn emit_copy_retval_to_regs(
        &self,
        ctx: &mut Lower<Self::I>,
        idx: usize,
        into_reg: ValueRegs<Writable<Reg>>,
    );

    /// Emit code to pre-adjust the stack, prior to argument copies and call.
    fn emit_stack_pre_adjust(&self, ctx: &mut Lower<Self::I>);

    /// Emit code to post-adjust the satck, after call return and return-value copies.
    fn emit_stack_post_adjust(&self, ctx: &mut Lower<Self::I>);

    /// Accumulate outgoing arguments.  This ensures that the caller (as
    /// identified via the CTX argument) allocates enough space in the
    /// prologue to hold all arguments and return values for this call.
    /// There is no code emitted at the call site, everything is done
    /// in the caller's function prologue.
    fn accumulate_outgoing_args_size(&self, ctx: &mut Lower<Self::I>);

    /// Emit the call itself.
    ///
    /// The returned instruction should have proper use- and def-sets according
    /// to the argument registers, return-value registers, and clobbered
    /// registers for this function signature in this ABI.
    ///
    /// (Arg registers are uses, and retval registers are defs. Clobbered
    /// registers are also logically defs, but should never be read; their
    /// values are "defined" (to the regalloc) but "undefined" in every other
    /// sense.)
    ///
    /// This function should only be called once, as it is allowed to re-use
    /// parts of the ABICaller object in emitting instructions.
    fn emit_call(&mut self, ctx: &mut Lower<Self::I>);
}
