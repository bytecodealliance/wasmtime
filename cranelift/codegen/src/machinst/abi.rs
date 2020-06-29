//! ABI definitions.

use crate::binemit::Stackmap;
use crate::ir::{ArgumentExtension, StackSlot};
use crate::machinst::*;
use crate::settings;

use regalloc::{Reg, Set, SpillSlot, Writable};

/// Trait implemented by an object that tracks ABI-related state (e.g., stack
/// layout) and can generate code while emitting the *body* of a function.
pub trait ABIBody {
    /// The instruction type for the ISA associated with this ABI.
    type I: VCodeInst;

    /// Does the ABI-body code need a temp reg? One will be provided to `init()`
    /// as the `maybe_tmp` arg if so.
    fn temp_needed(&self) -> bool;

    /// Initialize. This is called after the ABIBody is constructed because it
    /// may be provided with a temp vreg, which can only be allocated once the
    /// lowering context exists.
    fn init(&mut self, maybe_tmp: Option<Writable<Reg>>);

    /// Get the settings controlling this function's compilation.
    fn flags(&self) -> &settings::Flags;

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
    fn gen_copy_arg_to_reg(&self, idx: usize, into_reg: Writable<Reg>) -> Self::I;

    /// Generate any setup instruction needed to save values to the
    /// return-value area. This is usually used when were are multiple return
    /// values or an otherwise large return value that must be passed on the
    /// stack; typically the ABI specifies an extra hidden argument that is a
    /// pointer to that memory.
    fn gen_retval_area_setup(&self) -> Option<Self::I>;

    /// Generate an instruction which copies a source register to a return value slot.
    fn gen_copy_reg_to_retval(
        &self,
        idx: usize,
        from_reg: Writable<Reg>,
        ext: ArgumentExtension,
    ) -> Vec<Self::I>;

    /// Generate a return instruction.
    fn gen_ret(&self) -> Self::I;

    /// Generate an epilogue placeholder. The returned instruction should return `true` from
    /// `is_epilogue_placeholder()`; this is used to indicate to the lowering driver when
    /// the epilogue should be inserted.
    fn gen_epilogue_placeholder(&self) -> Self::I;

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

    /// Get the address of a stackslot.
    fn stackslot_addr(&self, slot: StackSlot, offset: u32, into_reg: Writable<Reg>) -> Self::I;

    /// Load from a stackslot.
    fn load_stackslot(
        &self,
        slot: StackSlot,
        offset: u32,
        ty: Type,
        into_reg: Writable<Reg>,
    ) -> Self::I;

    /// Store to a stackslot.
    fn store_stackslot(&self, slot: StackSlot, offset: u32, ty: Type, from_reg: Reg) -> Self::I;

    /// Load from a spillslot.
    fn load_spillslot(&self, slot: SpillSlot, ty: Type, into_reg: Writable<Reg>) -> Self::I;

    /// Store to a spillslot.
    fn store_spillslot(&self, slot: SpillSlot, ty: Type, from_reg: Reg) -> Self::I;

    /// Generate a stackmap, given a list of spillslots and the emission state
    /// at a given program point (prior to emission fo the safepointing
    /// instruction).
    fn spillslots_to_stackmap(
        &self,
        slots: &[SpillSlot],
        state: &<Self::I as MachInstEmit>::State,
    ) -> Stackmap;

    /// Generate a prologue, post-regalloc. This should include any stack
    /// frame or other setup necessary to use the other methods (`load_arg`,
    /// `store_retval`, and spillslot accesses.)  `self` is mutable so that we
    /// can store information in it which will be useful when creating the
    /// epilogue.
    fn gen_prologue(&mut self) -> Vec<Self::I>;

    /// Generate an epilogue, post-regalloc. Note that this must generate the
    /// actual return instruction (rather than emitting this in the lowering
    /// logic), because the epilogue code comes before the return and the two are
    /// likely closely related.
    fn gen_epilogue(&self) -> Vec<Self::I>;

    /// Returns the full frame size for the given function, after prologue
    /// emission has run. This comprises the spill slots and stack-storage slots
    /// (but not storage for clobbered callee-save registers, arguments pushed
    /// at callsites within this function, or other ephemeral pushes).  This is
    /// used for ABI variants where the client generates prologue/epilogue code,
    /// as in Baldrdash (SpiderMonkey integration).
    fn frame_size(&self) -> u32;

    /// Returns the size of arguments expected on the stack.
    fn stack_args_size(&self) -> u32;

    /// Get the spill-slot size.
    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32;

    /// Generate a spill. The type, if known, is given; this can be used to
    /// generate a store instruction optimized for the particular type rather
    /// than the RegClass (e.g., only F64 that resides in a V128 register). If
    /// no type is given, the implementation should spill the whole register.
    fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg, ty: Option<Type>) -> Self::I;

    /// Generate a reload (fill). As for spills, the type may be given to allow
    /// a more optimized load instruction to be generated.
    fn gen_reload(
        &self,
        to_reg: Writable<RealReg>,
        from_slot: SpillSlot,
        ty: Option<Type>,
    ) -> Self::I;
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
pub trait ABICall {
    /// The instruction type for the ISA associated with this ABI.
    type I: VCodeInst;

    /// Get the number of arguments expected.
    fn num_args(&self) -> usize;

    /// Emit a copy of an argument value from a source register, prior to the call.
    fn emit_copy_reg_to_arg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        from_reg: Reg,
    );

    /// Emit a copy a return value into a destination register, after the call returns.
    fn emit_copy_retval_to_reg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        into_reg: Writable<Reg>,
    );

    /// Emit code to pre-adjust the stack, prior to argument copies and call.
    fn emit_stack_pre_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C);

    /// Emit code to post-adjust the satck, after call return and return-value copies.
    fn emit_stack_post_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C);

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
    /// parts of the ABICall object in emitting instructions.
    fn emit_call<C: LowerCtx<I = Self::I>>(&mut self, ctx: &mut C);
}
