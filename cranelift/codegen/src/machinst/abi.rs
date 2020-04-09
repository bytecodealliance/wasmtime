//! ABI definitions.

use crate::ir;
use crate::ir::StackSlot;
use crate::machinst::*;
use crate::settings;

use regalloc::{Reg, Set, SpillSlot, VirtualReg, Writable};

/// Trait implemented by an object that tracks ABI-related state (e.g., stack
/// layout) and can generate code while emitting the *body* of a function.
pub trait ABIBody<I: VCodeInst> {
    /// Get the liveins of the function.
    fn liveins(&self) -> Set<RealReg>;

    /// Get the liveouts of the function.
    fn liveouts(&self) -> Set<RealReg>;

    /// Number of arguments.
    fn num_args(&self) -> usize;

    /// Number of return values.
    fn num_retvals(&self) -> usize;

    /// Number of stack slots (not spill slots).
    fn num_stackslots(&self) -> usize;

    /// Generate an instruction which copies an argument to a destination
    /// register.
    fn gen_copy_arg_to_reg(&self, idx: usize, into_reg: Writable<Reg>) -> I;

    /// Generate an instruction which copies a source register to a return
    /// value slot.
    fn gen_copy_reg_to_retval(&self, idx: usize, from_reg: Reg) -> I;

    /// Generate a return instruction.
    fn gen_ret(&self) -> I;

    /// Generate an epilogue placeholder.
    fn gen_epilogue_placeholder(&self) -> I;

    // -----------------------------------------------------------------
    // Every function above this line may only be called pre-regalloc.
    // Every function below this line may only be called post-regalloc.
    // `spillslots()` must be called before any other post-regalloc
    // function.
    // ----------------------------------------------------------------

    /// Update with the number of spillslots, post-regalloc.
    fn set_num_spillslots(&mut self, slots: usize);

    /// Update with the clobbered registers, post-regalloc.
    fn set_clobbered(&mut self, clobbered: Set<Writable<RealReg>>);

    /// Load from a stackslot.
    fn load_stackslot(
        &self,
        slot: StackSlot,
        offset: usize,
        ty: Type,
        into_reg: Writable<Reg>,
    ) -> I;

    /// Store to a stackslot.
    fn store_stackslot(&self, slot: StackSlot, offset: usize, ty: Type, from_reg: Reg) -> I;

    /// Load from a spillslot.
    fn load_spillslot(&self, slot: SpillSlot, ty: Type, into_reg: Writable<Reg>) -> I;

    /// Store to a spillslot.
    fn store_spillslot(&self, slot: SpillSlot, ty: Type, from_reg: Reg) -> I;

    /// Generate a prologue, post-regalloc. This should include any stack
    /// frame or other setup necessary to use the other methods (`load_arg`,
    /// `store_retval`, and spillslot accesses.)  |self| is mutable so that we
    /// can store information in it which will be useful when creating the
    /// epilogue.
    fn gen_prologue(&mut self, flags: &settings::Flags) -> Vec<I>;

    /// Generate an epilogue, post-regalloc. Note that this must generate the
    /// actual return instruction (rather than emitting this in the lowering
    /// logic), because the epilogue code comes before the return and the two are
    /// likely closely related.
    fn gen_epilogue(&self, flags: &settings::Flags) -> Vec<I>;

    /// Returns the full frame size for the given function, after prologue emission has run. This
    /// comprises the spill space, incoming argument space, alignment padding, etc.
    fn frame_size(&self) -> u32;

    /// Get the spill-slot size.
    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32;

    /// Generate a spill.
    fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg, ty: Type) -> I;

    /// Generate a reload (fill).
    fn gen_reload(&self, to_reg: Writable<RealReg>, from_slot: SpillSlot, ty: Type) -> I;
}

/// Trait implemented by an object that tracks ABI-related state and can
/// generate code while emitting a *call* to a function.
///
/// An instance of this trait returns information for a *particular*
/// callsite. It will usually be computed from the called function's
/// signature.
///
/// Unlike `ABIBody` above, methods on this trait are not invoked directly
/// by the machine-independent code. Rather, the machine-specific lowering
/// code will typically create an `ABICall` when creating machine instructions
/// for an IR call instruction inside `lower()`, directly emit the arg and
/// and retval copies, and attach the register use/def info to the call.
///
/// This trait is thus provided for convenience to the backends.
pub trait ABICall<I: VCodeInst> {
    /// Get the number of arguments expected.
    fn num_args(&self) -> usize;

    /// Save the clobbered registers.
    /// Copy an argument value from a source register, prior to the call.
    fn gen_copy_reg_to_arg(&self, idx: usize, from_reg: Reg) -> I;

    /// Copy a return value into a destination register, after the call returns.
    fn gen_copy_retval_to_reg(&self, idx: usize, into_reg: Writable<Reg>) -> I;

    /// Pre-adjust the stack, prior to argument copies and call.
    fn gen_stack_pre_adjust(&self) -> Vec<I>;

    /// Post-adjust the satck, after call return and return-value copies.
    fn gen_stack_post_adjust(&self) -> Vec<I>;

    /// Generate the call itself.
    ///
    /// The returned instruction should have proper use- and def-sets according
    /// to the argument registers, return-value registers, and clobbered
    /// registers for this function signature in this ABI.
    ///
    /// (Arg registers are uses, and retval registers are defs. Clobbered
    /// registers are also logically defs, but should never be read; their
    /// values are "defined" (to the regalloc) but "undefined" in every other
    /// sense.)
    fn gen_call(&self) -> Vec<I>;
}
