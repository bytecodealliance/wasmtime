//! Implementation of a vanilla ABI, shared between several machines. The
//! implementation here assumes that arguments will be passed in registers
//! first, then additional args on the stack; that the stack grows downward,
//! contains a standard frame (return address and frame pointer), and the
//! compiler is otherwise free to allocate space below that with its choice of
//! layout; and that the machine has some notion of caller- and callee-save
//! registers. Most modern machines, e.g. x86-64 and AArch64, should fit this
//! mold and thus both of these backends use this shared implementation.
//!
//! See the documentation in specific machine backends for the "instantiation"
//! of this generic ABI, i.e., which registers are caller/callee-save, arguments
//! and return values, and any other special requirements.
//!
//! For now the implementation here assumes a 64-bit machine, but we intend to
//! make this 32/64-bit-generic shortly.
//!
//! # Vanilla ABI
//!
//! First, arguments and return values are passed in registers up to a certain
//! fixed count, after which they overflow onto the stack. Multiple return
//! values either fit in registers, or are returned in a separate return-value
//! area on the stack, given by a hidden extra parameter.
//!
//! Note that the exact stack layout is up to us. We settled on the
//! below design based on several requirements. In particular, we need
//! to be able to generate instructions (or instruction sequences) to
//! access arguments, stack slots, and spill slots before we know how
//! many spill slots or clobber-saves there will be, because of our
//! pass structure. We also prefer positive offsets to negative
//! offsets because of an asymmetry in some machines' addressing modes
//! (e.g., on AArch64, positive offsets have a larger possible range
//! without a long-form sequence to synthesize an arbitrary
//! offset). We also need clobber-save registers to be "near" the
//! frame pointer: Windows unwind information requires it to be within
//! 240 bytes of RBP. Finally, it is not allowed to access memory
//! below the current SP value.
//!
//! We assume that a prologue first pushes the frame pointer (and
//! return address above that, if the machine does not do that in
//! hardware). We set FP to point to this two-word frame record. We
//! store all other frame slots below this two-word frame record, as
//! well as enough space for arguments to the largest possible
//! function call. The stack pointer then remains at this position
//! for the duration of the function, allowing us to address all
//! frame storage at positive offsets from SP.
//!
//! Note that if we ever support dynamic stack-space allocation (for
//! `alloca`), we will need a way to reference spill slots and stack
//! slots relative to a dynamic SP, because we will no longer be able
//! to know a static offset from SP to the slots at any particular
//! program point. Probably the best solution at that point will be to
//! revert to using the frame pointer as the reference for all slots,
//! to allow generating spill/reload and stackslot accesses before we
//! know how large the clobber-saves will be.
//!
//! # Stack Layout
//!
//! The stack looks like:
//!
//! ```plain
//!   (high address)
//!                              |          ...              |
//!                              | caller frames             |
//!                              |          ...              |
//!                              +===========================+
//!                              |          ...              |
//!                              | stack args                |
//! Canonical Frame Address -->  | (accessed via FP)         |
//!                              +---------------------------+
//! SP at function entry ----->  | return address            |
//!                              +---------------------------+
//! FP after prologue -------->  | FP (pushed by prologue)   |
//!                              +---------------------------+           -----
//!                              |          ...              |             |
//!                              | clobbered callee-saves    |             |
//! unwind-frame base -------->  | (pushed by prologue)      |             |
//!                              +---------------------------+   -----     |
//!                              |          ...              |     |       |
//!                              | spill slots               |     |       |
//!                              | (accessed via SP)         |   fixed   active
//!                              |          ...              |   frame    size
//!                              | stack slots               |  storage    |
//!                              | (accessed via SP)         |    size     |
//!                              | (alloc'd by prologue)     |     |       |
//!                              +---------------------------+   -----     |
//!                              | [alignment as needed]     |             |
//!                              |          ...              |             |
//!                              | args for largest call     |             |
//! SP ----------------------->  | (alloc'd by prologue)     |             |
//!                              +===========================+           -----
//!
//!   (low address)
//! ```
//!
//! # Multi-value Returns
//!
//! We support multi-value returns by using multiple return-value
//! registers. In some cases this is an extension of the base system
//! ABI. See each platform's `abi.rs` implementation for details.

use crate::entity::SecondaryMap;
use crate::ir::types::*;
use crate::ir::{ArgumentExtension, ArgumentPurpose, Signature};
use crate::isa::TargetIsa;
use crate::settings::ProbestackStrategy;
use crate::CodegenError;
use crate::{ir, isa};
use crate::{machinst::*, trace};
use regalloc2::{MachineEnv, PReg, PRegSet};
use rustc_hash::FxHashMap;
use smallvec::smallvec;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;

/// A small vector of instructions (with some reasonable size); appropriate for
/// a small fixed sequence implementing one operation.
pub type SmallInstVec<I> = SmallVec<[I; 4]>;

/// A type used by backends to track argument-binding info in the "args"
/// pseudoinst. The pseudoinst holds a vec of `ArgPair` structs.
#[derive(Clone, Debug)]
pub struct ArgPair {
    /// The vreg that is defined by this args pseudoinst.
    pub vreg: Writable<Reg>,
    /// The preg that the arg arrives in; this constrains the vreg's
    /// placement at the pseudoinst.
    pub preg: Reg,
}

/// A type used by backends to track return register binding info in the "ret"
/// pseudoinst. The pseudoinst holds a vec of `RetPair` structs.
#[derive(Clone, Debug)]
pub struct RetPair {
    /// The vreg that is returned by this pseudionst.
    pub vreg: Reg,
    /// The preg that the arg is returned through; this constrains the vreg's
    /// placement at the pseudoinst.
    pub preg: Reg,
}

/// A location for (part of) an argument or return value. These "storage slots"
/// are specified for each register-sized part of an argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ABIArgSlot {
    /// In a real register.
    Reg {
        /// Register that holds this arg.
        reg: RealReg,
        /// Value type of this arg.
        ty: ir::Type,
        /// Should this arg be zero- or sign-extended?
        extension: ir::ArgumentExtension,
    },
    /// Arguments only: on stack, at given offset from SP at entry.
    Stack {
        /// Offset of this arg relative to the base of stack args.
        offset: i64,
        /// Value type of this arg.
        ty: ir::Type,
        /// Should this arg be zero- or sign-extended?
        extension: ir::ArgumentExtension,
    },
}

impl ABIArgSlot {
    /// The type of the value that will be stored in this slot.
    pub fn get_type(&self) -> ir::Type {
        match self {
            ABIArgSlot::Reg { ty, .. } => *ty,
            ABIArgSlot::Stack { ty, .. } => *ty,
        }
    }
}

/// A vector of `ABIArgSlot`s. Inline capacity for one element because basically
/// 100% of values use one slot. Only `i128`s need multiple slots, and they are
/// super rare (and never happen with Wasm).
pub type ABIArgSlotVec = SmallVec<[ABIArgSlot; 1]>;

/// An ABIArg is composed of one or more parts. This allows for a CLIF-level
/// Value to be passed with its parts in more than one location at the ABI
/// level. For example, a 128-bit integer may be passed in two 64-bit registers,
/// or even a 64-bit register and a 64-bit stack slot, on a 64-bit machine. The
/// number of "parts" should correspond to the number of registers used to store
/// this type according to the machine backend.
///
/// As an invariant, the `purpose` for every part must match. As a further
/// invariant, a `StructArg` part cannot appear with any other part.
#[derive(Clone, Debug)]
pub enum ABIArg {
    /// Storage slots (registers or stack locations) for each part of the
    /// argument value. The number of slots must equal the number of register
    /// parts used to store a value of this type.
    Slots {
        /// Slots, one per register part.
        slots: ABIArgSlotVec,
        /// Purpose of this arg.
        purpose: ir::ArgumentPurpose,
    },
    /// Structure argument. We reserve stack space for it, but the CLIF-level
    /// semantics are a little weird: the value passed to the call instruction,
    /// and received in the corresponding block param, is a *pointer*. On the
    /// caller side, we memcpy the data from the passed-in pointer to the stack
    /// area; on the callee side, we compute a pointer to this stack area and
    /// provide that as the argument's value.
    StructArg {
        /// Register or stack slot holding a pointer to the buffer as passed
        /// by the caller to the callee.  If None, the ABI defines the buffer
        /// to reside at a well-known location (i.e. at `offset` below).
        pointer: Option<ABIArgSlot>,
        /// Offset of this arg relative to base of stack args.
        offset: i64,
        /// Size of this arg on the stack.
        size: u64,
        /// Purpose of this arg.
        purpose: ir::ArgumentPurpose,
    },
    /// Implicit argument. Similar to a StructArg, except that we have the
    /// target type, not a pointer type, at the CLIF-level. This argument is
    /// still being passed via reference implicitly.
    ImplicitPtrArg {
        /// Register or stack slot holding a pointer to the buffer.
        pointer: ABIArgSlot,
        /// Offset of the argument buffer.
        offset: i64,
        /// Type of the implicit argument.
        ty: Type,
        /// Purpose of this arg.
        purpose: ir::ArgumentPurpose,
    },
}

impl ABIArg {
    /// Create an ABIArg from one register.
    pub fn reg(
        reg: RealReg,
        ty: ir::Type,
        extension: ir::ArgumentExtension,
        purpose: ir::ArgumentPurpose,
    ) -> ABIArg {
        ABIArg::Slots {
            slots: smallvec![ABIArgSlot::Reg { reg, ty, extension }],
            purpose,
        }
    }

    /// Create an ABIArg from one stack slot.
    pub fn stack(
        offset: i64,
        ty: ir::Type,
        extension: ir::ArgumentExtension,
        purpose: ir::ArgumentPurpose,
    ) -> ABIArg {
        ABIArg::Slots {
            slots: smallvec![ABIArgSlot::Stack {
                offset,
                ty,
                extension,
            }],
            purpose,
        }
    }
}

/// Are we computing information about arguments or return values? Much of the
/// handling is factored out into common routines; this enum allows us to
/// distinguish which case we're handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgsOrRets {
    /// Arguments.
    Args,
    /// Return values.
    Rets,
}

/// Abstract location for a machine-specific ABI impl to translate into the
/// appropriate addressing mode.
#[derive(Clone, Copy, Debug)]
pub enum StackAMode {
    /// Offset into the current frame's argument area.
    IncomingArg(i64, u32),
    /// Offset within the stack slots in the current frame.
    Slot(i64),
    /// Offset into the callee frame's argument area.
    OutgoingArg(i64),
}

/// Trait implemented by machine-specific backend to represent ISA flags.
pub trait IsaFlags: Clone {
    /// Get a flag indicating whether forward-edge CFI is enabled.
    fn is_forward_edge_cfi_enabled(&self) -> bool {
        false
    }
}

/// Used as an out-parameter to accumulate a sequence of `ABIArg`s in
/// `ABIMachineSpec::compute_arg_locs`. Wraps the shared allocation for all
/// `ABIArg`s in `SigSet` and exposes just the args for the current
/// `compute_arg_locs` call.
pub struct ArgsAccumulator<'a> {
    sig_set_abi_args: &'a mut Vec<ABIArg>,
    start: usize,
    non_formal_flag: bool,
}

impl<'a> ArgsAccumulator<'a> {
    fn new(sig_set_abi_args: &'a mut Vec<ABIArg>) -> Self {
        let start = sig_set_abi_args.len();
        ArgsAccumulator {
            sig_set_abi_args,
            start,
            non_formal_flag: false,
        }
    }

    #[inline]
    pub fn push(&mut self, arg: ABIArg) {
        debug_assert!(!self.non_formal_flag);
        self.sig_set_abi_args.push(arg)
    }

    #[inline]
    pub fn push_non_formal(&mut self, arg: ABIArg) {
        self.non_formal_flag = true;
        self.sig_set_abi_args.push(arg)
    }

    #[inline]
    pub fn args(&self) -> &[ABIArg] {
        &self.sig_set_abi_args[self.start..]
    }

    #[inline]
    pub fn args_mut(&mut self) -> &mut [ABIArg] {
        &mut self.sig_set_abi_args[self.start..]
    }
}

/// Trait implemented by machine-specific backend to provide information about
/// register assignments and to allow generating the specific instructions for
/// stack loads/saves, prologues/epilogues, etc.
pub trait ABIMachineSpec {
    /// The instruction type.
    type I: VCodeInst;

    /// The ISA flags type.
    type F: IsaFlags;

    /// Returns the number of bits in a word, that is 32/64 for 32/64-bit architecture.
    fn word_bits() -> u32;

    /// Returns the number of bytes in a word.
    fn word_bytes() -> u32 {
        return Self::word_bits() / 8;
    }

    /// Returns word-size integer type.
    fn word_type() -> Type {
        match Self::word_bits() {
            32 => I32,
            64 => I64,
            _ => unreachable!(),
        }
    }

    /// Returns word register class.
    fn word_reg_class() -> RegClass {
        RegClass::Int
    }

    /// Returns required stack alignment in bytes.
    fn stack_align(call_conv: isa::CallConv) -> u32;

    /// Process a list of parameters or return values and allocate them to registers
    /// and stack slots.
    ///
    /// The argument locations should be pushed onto the given `ArgsAccumulator`
    /// in order. Any extra arguments added (such as return area pointers)
    /// should come at the end of the list so that the first N lowered
    /// parameters align with the N clif parameters.
    ///
    /// Returns the stack-space used (rounded up to as alignment requires), and
    /// if `add_ret_area_ptr` was passed, the index of the extra synthetic arg
    /// that was added.
    fn compute_arg_locs(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)>;

    /// Generate a load from the stack.
    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Self::I;

    /// Generate a store to the stack.
    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Self::I;

    /// Generate a move.
    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self::I;

    /// Generate an integer-extend operation.
    fn gen_extend(
        to_reg: Writable<Reg>,
        from_reg: Reg,
        is_signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Self::I;

    /// Generate an "args" pseudo-instruction to capture input args in
    /// registers.
    fn gen_args(args: Vec<ArgPair>) -> Self::I;

    /// Generate a "rets" pseudo-instruction that moves vregs to return
    /// registers.
    fn gen_rets(rets: Vec<RetPair>) -> Self::I;

    /// Generate an add-with-immediate. Note that even if this uses a scratch
    /// register, it must satisfy two requirements:
    ///
    /// - The add-imm sequence must only clobber caller-save registers that are
    ///   not used for arguments, because it will be placed in the prologue
    ///   before the clobbered callee-save registers are saved.
    ///
    /// - The add-imm sequence must work correctly when `from_reg` and/or
    ///   `into_reg` are the register returned by `get_stacklimit_reg()`.
    fn gen_add_imm(
        call_conv: isa::CallConv,
        into_reg: Writable<Reg>,
        from_reg: Reg,
        imm: u32,
    ) -> SmallInstVec<Self::I>;

    /// Generate a sequence that traps with a `TrapCode::StackOverflow` code if
    /// the stack pointer is less than the given limit register (assuming the
    /// stack grows downward).
    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Self::I>;

    /// Generate an instruction to compute an address of a stack slot (FP- or
    /// SP-based offset).
    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>) -> Self::I;

    /// Get a fixed register to use to compute a stack limit. This is needed for
    /// certain sequences generated after the register allocator has already
    /// run. This must satisfy two requirements:
    ///
    /// - It must be a caller-save register that is not used for arguments,
    ///   because it will be clobbered in the prologue before the clobbered
    ///   callee-save registers are saved.
    ///
    /// - It must be safe to pass as an argument and/or destination to
    ///   `gen_add_imm()`. This is relevant when an addition with a large
    ///   immediate needs its own temporary; it cannot use the same fixed
    ///   temporary as this one.
    fn get_stacklimit_reg(call_conv: isa::CallConv) -> Reg;

    /// Generate a load to the given [base+offset] address.
    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Self::I;

    /// Generate a store from the given [base+offset] address.
    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Self::I;

    /// Adjust the stack pointer up or down.
    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Self::I>;

    /// Compute a FrameLayout structure containing a sorted list of all clobbered
    /// registers that are callee-saved according to the ABI, as well as the sizes
    /// of all parts of the stack frame.  The result is used to emit the prologue
    /// and epilogue routines.
    fn compute_frame_layout(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        sig: &Signature,
        regs: &[Writable<RealReg>],
        is_leaf: bool,
        incoming_args_size: u32,
        tail_args_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout;

    /// Generate the usual frame-setup sequence for this architecture: e.g.,
    /// `push rbp / mov rbp, rsp` on x86-64, or `stp fp, lr, [sp, #-16]!` on
    /// AArch64.
    fn gen_prologue_frame_setup(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        isa_flags: &Self::F,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I>;

    /// Generate the usual frame-restore sequence for this architecture.
    fn gen_epilogue_frame_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        isa_flags: &Self::F,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I>;

    /// Generate a return instruction.
    fn gen_return(
        call_conv: isa::CallConv,
        isa_flags: &Self::F,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I>;

    /// Generate a probestack call.
    fn gen_probestack(insts: &mut SmallInstVec<Self::I>, frame_size: u32);

    /// Generate a inline stack probe.
    fn gen_inline_probestack(
        insts: &mut SmallInstVec<Self::I>,
        call_conv: isa::CallConv,
        frame_size: u32,
        guard_size: u32,
    );

    /// Generate a clobber-save sequence. The implementation here should return
    /// a sequence of instructions that "push" or otherwise save to the stack all
    /// registers written/modified by the function body that are callee-saved.
    /// The sequence of instructions should adjust the stack pointer downward,
    /// and should align as necessary according to ABI requirements.
    fn gen_clobber_save(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]>;

    /// Generate a clobber-restore sequence. This sequence should perform the
    /// opposite of the clobber-save sequence generated above, assuming that SP
    /// going into the sequence is at the same point that it was left when the
    /// clobber-save sequence finished.
    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]>;

    /// Generate a call instruction/sequence. This method is provided one
    /// temporary register to use to synthesize the called address, if needed.
    fn gen_call(
        dest: &CallDest,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
        callee_pop_size: u32,
    ) -> SmallVec<[Self::I; 2]>;

    /// Generate a memcpy invocation. Used to set up struct
    /// args. Takes `src`, `dst` as read-only inputs and passes a temporary
    /// allocator.
    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
        alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]>;

    /// Get the number of spillslots required for the given register-class.
    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        target_vector_bytes: u32,
        isa_flags: &Self::F,
    ) -> u32;

    /// Get the ABI-dependent MachineEnv for managing register allocation.
    fn get_machine_env(flags: &settings::Flags, call_conv: isa::CallConv) -> &MachineEnv;

    /// Get all caller-save registers, that is, registers that we expect
    /// not to be saved across a call to a callee with the given ABI.
    fn get_regs_clobbered_by_call(call_conv_of_callee: isa::CallConv) -> PRegSet;

    /// Get the needed extension mode, given the mode attached to the argument
    /// in the signature and the calling convention. The input (the attribute in
    /// the signature) specifies what extension type should be done *if* the ABI
    /// requires extension to the full register; this method's return value
    /// indicates whether the extension actually *will* be done.
    fn get_ext_mode(
        call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension;
}

/// The id of an ABI signature within the `SigSet`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sig(u32);
cranelift_entity::entity_impl!(Sig);

impl Sig {
    fn prev(self) -> Option<Sig> {
        self.0.checked_sub(1).map(Sig)
    }
}

/// ABI information shared between body (callee) and caller.
#[derive(Clone, Debug)]
pub struct SigData {
    /// Currently both return values and arguments are stored in a continuous space vector
    /// in `SigSet::abi_args`.
    ///
    /// ```plain
    ///                  +----------------------------------------------+
    ///                  | return values                                |
    ///                  | ...                                          |
    ///   rets_end   --> +----------------------------------------------+
    ///                  | arguments                                    |
    ///                  | ...                                          |
    ///   args_end   --> +----------------------------------------------+
    ///
    /// ```
    ///
    /// Note we only store two offsets as rets_end == args_start, and rets_start == prev.args_end.
    ///
    /// Argument location ending offset (regs or stack slots). Stack offsets are relative to
    /// SP on entry to function.
    ///
    /// This is a index into the `SigSet::abi_args`.
    args_end: u32,

    /// Return-value location ending offset. Stack offsets are relative to the return-area
    /// pointer.
    ///
    /// This is a index into the `SigSet::abi_args`.
    rets_end: u32,

    /// Space on stack used to store arguments. We're storing the size in u32 to
    /// reduce the size of the struct.
    sized_stack_arg_space: u32,

    /// Space on stack used to store return values. We're storing the size in u32 to
    /// reduce the size of the struct.
    sized_stack_ret_space: u32,

    /// Index in `args` of the stack-return-value-area argument.
    stack_ret_arg: Option<u16>,

    /// Calling convention used.
    call_conv: isa::CallConv,
}

impl SigData {
    /// Get total stack space required for arguments.
    pub fn sized_stack_arg_space(&self) -> i64 {
        self.sized_stack_arg_space.into()
    }

    /// Get total stack space required for return values.
    pub fn sized_stack_ret_space(&self) -> i64 {
        self.sized_stack_ret_space.into()
    }

    /// Get calling convention used.
    pub fn call_conv(&self) -> isa::CallConv {
        self.call_conv
    }

    /// The index of the stack-return-value-area argument, if any.
    pub fn stack_ret_arg(&self) -> Option<u16> {
        self.stack_ret_arg
    }
}

/// A (mostly) deduplicated set of ABI signatures.
///
/// We say "mostly" because we do not dedupe between signatures interned via
/// `ir::SigRef` (direct and indirect calls; the vast majority of signatures in
/// this set) vs via `ir::Signature` (the callee itself and libcalls). Doing
/// this final bit of deduplication would require filling out the
/// `ir_signature_to_abi_sig`, which is a bunch of allocations (not just the
/// hash map itself but params and returns vecs in each signature) that we want
/// to avoid.
///
/// In general, prefer using the `ir::SigRef`-taking methods to the
/// `ir::Signature`-taking methods when you can get away with it, as they don't
/// require cloning non-copy types that will trigger heap allocations.
///
/// This type can be indexed by `Sig` to access its associated `SigData`.
pub struct SigSet {
    /// Interned `ir::Signature`s that we already have an ABI signature for.
    ir_signature_to_abi_sig: FxHashMap<ir::Signature, Sig>,

    /// Interned `ir::SigRef`s that we already have an ABI signature for.
    ir_sig_ref_to_abi_sig: SecondaryMap<ir::SigRef, Option<Sig>>,

    /// A single, shared allocation for all `ABIArg`s used by all
    /// `SigData`s. Each `SigData` references its args/rets via indices into
    /// this allocation.
    abi_args: Vec<ABIArg>,

    /// The actual ABI signatures, keyed by `Sig`.
    sigs: PrimaryMap<Sig, SigData>,
}

impl SigSet {
    /// Construct a new `SigSet`, interning all of the signatures used by the
    /// given function.
    pub fn new<M>(func: &ir::Function, flags: &settings::Flags) -> CodegenResult<Self>
    where
        M: ABIMachineSpec,
    {
        let arg_estimate = func.dfg.signatures.len() * 6;

        let mut sigs = SigSet {
            ir_signature_to_abi_sig: FxHashMap::default(),
            ir_sig_ref_to_abi_sig: SecondaryMap::with_capacity(func.dfg.signatures.len()),
            abi_args: Vec::with_capacity(arg_estimate),
            sigs: PrimaryMap::with_capacity(1 + func.dfg.signatures.len()),
        };

        sigs.make_abi_sig_from_ir_signature::<M>(func.signature.clone(), flags)?;
        for sig_ref in func.dfg.signatures.keys() {
            sigs.make_abi_sig_from_ir_sig_ref::<M>(sig_ref, &func.dfg, flags)?;
        }

        Ok(sigs)
    }

    /// Have we already interned an ABI signature for the given `ir::Signature`?
    pub fn have_abi_sig_for_signature(&self, signature: &ir::Signature) -> bool {
        self.ir_signature_to_abi_sig.contains_key(signature)
    }

    /// Construct and intern an ABI signature for the given `ir::Signature`.
    pub fn make_abi_sig_from_ir_signature<M>(
        &mut self,
        signature: ir::Signature,
        flags: &settings::Flags,
    ) -> CodegenResult<Sig>
    where
        M: ABIMachineSpec,
    {
        // Because the `HashMap` entry API requires taking ownership of the
        // lookup key -- and we want to avoid unnecessary clones of
        // `ir::Signature`s, even at the cost of duplicate lookups -- we can't
        // have a single, get-or-create-style method for interning
        // `ir::Signature`s into ABI signatures. So at least (debug) assert that
        // we aren't creating duplicate ABI signatures for the same
        // `ir::Signature`.
        debug_assert!(!self.have_abi_sig_for_signature(&signature));

        let sig_data = self.from_func_sig::<M>(&signature, flags)?;
        let sig = self.sigs.push(sig_data);
        self.ir_signature_to_abi_sig.insert(signature, sig);
        Ok(sig)
    }

    fn make_abi_sig_from_ir_sig_ref<M>(
        &mut self,
        sig_ref: ir::SigRef,
        dfg: &ir::DataFlowGraph,
        flags: &settings::Flags,
    ) -> CodegenResult<Sig>
    where
        M: ABIMachineSpec,
    {
        if let Some(sig) = self.ir_sig_ref_to_abi_sig[sig_ref] {
            return Ok(sig);
        }
        let signature = &dfg.signatures[sig_ref];
        let sig_data = self.from_func_sig::<M>(signature, flags)?;
        let sig = self.sigs.push(sig_data);
        self.ir_sig_ref_to_abi_sig[sig_ref] = Some(sig);
        Ok(sig)
    }

    /// Get the already-interned ABI signature id for the given `ir::SigRef`.
    pub fn abi_sig_for_sig_ref(&self, sig_ref: ir::SigRef) -> Sig {
        self.ir_sig_ref_to_abi_sig
            .get(sig_ref)
            // Should have a secondary map entry...
            .expect("must call `make_abi_sig_from_ir_sig_ref` before `get_abi_sig_for_sig_ref`")
            // ...and that entry should be initialized.
            .expect("must call `make_abi_sig_from_ir_sig_ref` before `get_abi_sig_for_sig_ref`")
    }

    /// Get the already-interned ABI signature id for the given `ir::Signature`.
    pub fn abi_sig_for_signature(&self, signature: &ir::Signature) -> Sig {
        self.ir_signature_to_abi_sig
            .get(signature)
            .copied()
            .expect("must call `make_abi_sig_from_ir_signature` before `get_abi_sig_for_signature`")
    }

    pub fn from_func_sig<M: ABIMachineSpec>(
        &mut self,
        sig: &ir::Signature,
        flags: &settings::Flags,
    ) -> CodegenResult<SigData> {
        use std::borrow::Cow;

        let returns = if let Some(sret) = missing_struct_return(sig) {
            Cow::from_iter(std::iter::once(&sret).chain(&sig.returns).copied())
        } else {
            Cow::from(sig.returns.as_slice())
        };

        // Compute args and retvals from signature. Handle retvals first,
        // because we may need to add a return-area arg to the args.

        // NOTE: We rely on the order of the args (rets -> args) inserted to compute the offsets in
        // `SigSet::args()` and `SigSet::rets()`. Therefore, we cannot change the two
        // compute_arg_locs order.
        let (sized_stack_ret_space, _) = M::compute_arg_locs(
            sig.call_conv,
            flags,
            &returns,
            ArgsOrRets::Rets,
            /* extra ret-area ptr = */ false,
            ArgsAccumulator::new(&mut self.abi_args),
        )?;
        let rets_end = u32::try_from(self.abi_args.len()).unwrap();

        let need_stack_return_area = sized_stack_ret_space > 0;
        let (sized_stack_arg_space, stack_ret_arg) = M::compute_arg_locs(
            sig.call_conv,
            flags,
            &sig.params,
            ArgsOrRets::Args,
            need_stack_return_area,
            ArgsAccumulator::new(&mut self.abi_args),
        )?;
        let args_end = u32::try_from(self.abi_args.len()).unwrap();

        trace!(
            "ABISig: sig {:?} => args end = {} rets end = {}
             arg stack = {} ret stack = {} stack_ret_arg = {:?}",
            sig,
            args_end,
            rets_end,
            sized_stack_arg_space,
            sized_stack_ret_space,
            need_stack_return_area,
        );

        let stack_ret_arg = stack_ret_arg.map(|s| u16::try_from(s).unwrap());
        Ok(SigData {
            args_end,
            rets_end,
            sized_stack_arg_space,
            sized_stack_ret_space,
            stack_ret_arg,
            call_conv: sig.call_conv,
        })
    }

    /// Get this signature's ABI arguments.
    pub fn args(&self, sig: Sig) -> &[ABIArg] {
        let sig_data = &self.sigs[sig];
        // Please see comments in `SigSet::from_func_sig` of how we store the offsets.
        let start = usize::try_from(sig_data.rets_end).unwrap();
        let end = usize::try_from(sig_data.args_end).unwrap();
        &self.abi_args[start..end]
    }

    /// Get information specifying how to pass the implicit pointer
    /// to the return-value area on the stack, if required.
    pub fn get_ret_arg(&self, sig: Sig) -> Option<ABIArg> {
        let sig_data = &self.sigs[sig];
        if let Some(i) = sig_data.stack_ret_arg {
            Some(self.args(sig)[usize::from(i)].clone())
        } else {
            None
        }
    }

    /// Get information specifying how to pass one argument.
    pub fn get_arg(&self, sig: Sig, idx: usize) -> ABIArg {
        self.args(sig)[idx].clone()
    }

    /// Get this signature's ABI returns.
    pub fn rets(&self, sig: Sig) -> &[ABIArg] {
        let sig_data = &self.sigs[sig];
        // Please see comments in `SigSet::from_func_sig` of how we store the offsets.
        let start = usize::try_from(sig.prev().map_or(0, |prev| self.sigs[prev].args_end)).unwrap();
        let end = usize::try_from(sig_data.rets_end).unwrap();
        &self.abi_args[start..end]
    }

    /// Get information specifying how to pass one return value.
    pub fn get_ret(&self, sig: Sig, idx: usize) -> ABIArg {
        self.rets(sig)[idx].clone()
    }

    /// Get the number of arguments expected.
    pub fn num_args(&self, sig: Sig) -> usize {
        let len = self.args(sig).len();
        if self.sigs[sig].stack_ret_arg.is_some() {
            len - 1
        } else {
            len
        }
    }

    /// Get the number of return values expected.
    pub fn num_rets(&self, sig: Sig) -> usize {
        self.rets(sig).len()
    }
}

// NB: we do _not_ implement `IndexMut` because these signatures are
// deduplicated and shared!
impl std::ops::Index<Sig> for SigSet {
    type Output = SigData;

    fn index(&self, sig: Sig) -> &Self::Output {
        &self.sigs[sig]
    }
}

/// Structure describing the layout of a function's stack frame.
#[derive(Clone, Debug, Default)]
pub struct FrameLayout {
    /// N.B. The areas whose sizes are given in this structure fully
    /// cover the current function's stack frame, from high to low
    /// stack addresses in the sequence below.  Each size contains
    /// any alignment padding that may be required by the ABI.

    /// Size of incoming arguments on the stack.  This is not technically
    /// part of this function's frame, but code in the function will still
    /// need to access it.  Depending on the ABI, we may need to set up a
    /// frame pointer to do so; we also may need to pop this area from the
    /// stack upon return.
    pub incoming_args_size: u32,

    /// The size of the incoming argument area, taking into account any
    /// potential increase in size required for tail calls present in the
    /// function. In the case that no tail calls are present, this value
    /// will be the same as [`Self::incoming_args_size`].
    pub tail_args_size: u32,

    /// Size of the "setup area", typically holding the return address
    /// and/or the saved frame pointer.  This may be written either during
    /// the call itself (e.g. a pushed return address) or by code emitted
    /// from gen_prologue_frame_setup.  In any case, after that code has
    /// completed execution, the stack pointer is expected to point to the
    /// bottom of this area.  The same holds at the start of code emitted
    /// by gen_epilogue_frame_restore.
    pub setup_area_size: u32,

    /// Size of the area used to save callee-saved clobbered registers.
    /// This area is accessed by code emitted from gen_clobber_save and
    /// gen_clobber_restore.
    pub clobber_size: u32,

    /// Storage allocated for the fixed part of the stack frame.
    /// This contains stack slots and spill slots.
    pub fixed_frame_storage_size: u32,

    /// Stack size to be reserved for outgoing arguments, if used by
    /// the current ABI, or 0 otherwise.  After gen_clobber_save and
    /// before gen_clobber_restore, the stack pointer points to the
    /// bottom of this area.
    pub outgoing_args_size: u32,

    /// Sorted list of callee-saved registers that are clobbered
    /// according to the ABI.  These registers will be saved and
    /// restored by gen_clobber_save and gen_clobber_restore.
    pub clobbered_callee_saves: Vec<Writable<RealReg>>,
}

impl FrameLayout {
    /// Split the clobbered callee-save registers into integer-class and
    /// float-class groups.
    ///
    /// This method does not currently support vector-class callee-save
    /// registers because no current backend has them.
    pub fn clobbered_callee_saves_by_class(&self) -> (&[Writable<RealReg>], &[Writable<RealReg>]) {
        let (ints, floats) = self.clobbered_callee_saves.split_at(
            self.clobbered_callee_saves
                .partition_point(|r| r.to_reg().class() == RegClass::Int),
        );
        debug_assert!(floats.iter().all(|r| r.to_reg().class() == RegClass::Float));
        (ints, floats)
    }

    /// The size of FP to SP while the frame is active (not during prologue
    /// setup or epilogue tear down).
    pub fn active_size(&self) -> u32 {
        self.outgoing_args_size + self.fixed_frame_storage_size + self.clobber_size
    }

    /// Get the offset from the SP to the sized stack slots area.
    pub fn sp_to_sized_stack_slots(&self) -> u32 {
        self.outgoing_args_size
    }
}

/// ABI object for a function body.
pub struct Callee<M: ABIMachineSpec> {
    /// CLIF-level signature, possibly normalized.
    ir_sig: ir::Signature,
    /// Signature: arg and retval regs.
    sig: Sig,
    /// Defined dynamic types.
    dynamic_type_sizes: HashMap<Type, u32>,
    /// Offsets to each dynamic stackslot.
    dynamic_stackslots: PrimaryMap<DynamicStackSlot, u32>,
    /// Offsets to each sized stackslot.
    sized_stackslots: PrimaryMap<StackSlot, u32>,
    /// Total stack size of all stackslots
    stackslots_size: u32,
    /// Stack size to be reserved for outgoing arguments.
    outgoing_args_size: u32,
    /// Initially the number of bytes originating in the callers frame where stack arguments will
    /// live. After lowering this number may be larger than the size expected by the function being
    /// compiled, as tail calls potentially require more space for stack arguments.
    tail_args_size: u32,
    /// Register-argument defs, to be provided to the `args`
    /// pseudo-inst, and pregs to constrain them to.
    reg_args: Vec<ArgPair>,
    /// Finalized frame layout for this function.
    frame_layout: Option<FrameLayout>,
    /// The register holding the return-area pointer, if needed.
    ret_area_ptr: Option<Reg>,
    /// Calling convention this function expects.
    call_conv: isa::CallConv,
    /// The settings controlling this function's compilation.
    flags: settings::Flags,
    /// The ISA-specific flag values controlling this function's compilation.
    isa_flags: M::F,
    /// Whether or not this function is a "leaf", meaning it calls no other
    /// functions
    is_leaf: bool,
    /// If this function has a stack limit specified, then `Reg` is where the
    /// stack limit will be located after the instructions specified have been
    /// executed.
    ///
    /// Note that this is intended for insertion into the prologue, if
    /// present. Also note that because the instructions here execute in the
    /// prologue this happens after legalization/register allocation/etc so we
    /// need to be extremely careful with each instruction. The instructions are
    /// manually register-allocated and carefully only use caller-saved
    /// registers and keep nothing live after this sequence of instructions.
    stack_limit: Option<(Reg, SmallInstVec<M::I>)>,

    _mach: PhantomData<M>,
}

fn get_special_purpose_param_register(
    f: &ir::Function,
    sigs: &SigSet,
    sig: Sig,
    purpose: ir::ArgumentPurpose,
) -> Option<Reg> {
    let idx = f.signature.special_param_index(purpose)?;
    match &sigs.args(sig)[idx] {
        &ABIArg::Slots { ref slots, .. } => match &slots[0] {
            &ABIArgSlot::Reg { reg, .. } => Some(reg.into()),
            _ => None,
        },
        _ => None,
    }
}

fn checked_round_up(val: u32, mask: u32) -> Option<u32> {
    Some(val.checked_add(mask)? & !mask)
}

impl<M: ABIMachineSpec> Callee<M> {
    /// Create a new body ABI instance.
    pub fn new(
        f: &ir::Function,
        isa: &dyn TargetIsa,
        isa_flags: &M::F,
        sigs: &SigSet,
    ) -> CodegenResult<Self> {
        trace!("ABI: func signature {:?}", f.signature);

        let flags = isa.flags().clone();
        let sig = sigs.abi_sig_for_signature(&f.signature);

        let call_conv = f.signature.call_conv;
        // Only these calling conventions are supported.
        debug_assert!(
            call_conv == isa::CallConv::SystemV
                || call_conv == isa::CallConv::Tail
                || call_conv == isa::CallConv::Fast
                || call_conv == isa::CallConv::Cold
                || call_conv.extends_windows_fastcall()
                || call_conv == isa::CallConv::WasmtimeSystemV
                || call_conv == isa::CallConv::AppleAarch64
                || call_conv == isa::CallConv::Winch,
            "Unsupported calling convention: {call_conv:?}"
        );

        // Compute sized stackslot locations and total stackslot size.
        let mut sized_stack_offset: u32 = 0;
        let mut sized_stackslots = PrimaryMap::new();
        for (stackslot, data) in f.sized_stack_slots.iter() {
            let off = sized_stack_offset;
            sized_stack_offset = sized_stack_offset
                .checked_add(data.size)
                .ok_or(CodegenError::ImplLimitExceeded)?;
            // Always at least machine-word-align slots, but also
            // satisfy the user's requested alignment.
            debug_assert!(data.align_shift < 32);
            let align = std::cmp::max(M::word_bytes(), 1u32 << data.align_shift);
            let mask = align - 1;
            sized_stack_offset = checked_round_up(sized_stack_offset, mask)
                .ok_or(CodegenError::ImplLimitExceeded)?;
            debug_assert_eq!(stackslot.as_u32() as usize, sized_stackslots.len());
            sized_stackslots.push(off);
        }

        // Compute dynamic stackslot locations and total stackslot size.
        let mut dynamic_stackslots = PrimaryMap::new();
        let mut dynamic_stack_offset: u32 = sized_stack_offset;
        for (stackslot, data) in f.dynamic_stack_slots.iter() {
            debug_assert_eq!(stackslot.as_u32() as usize, dynamic_stackslots.len());
            let off = dynamic_stack_offset;
            let ty = f.get_concrete_dynamic_ty(data.dyn_ty).ok_or_else(|| {
                CodegenError::Unsupported(format!("invalid dynamic vector type: {}", data.dyn_ty))
            })?;
            dynamic_stack_offset = dynamic_stack_offset
                .checked_add(isa.dynamic_vector_bytes(ty))
                .ok_or(CodegenError::ImplLimitExceeded)?;
            let mask = M::word_bytes() - 1;
            dynamic_stack_offset = checked_round_up(dynamic_stack_offset, mask)
                .ok_or(CodegenError::ImplLimitExceeded)?;
            dynamic_stackslots.push(off);
        }
        let stackslots_size = dynamic_stack_offset;

        let mut dynamic_type_sizes = HashMap::with_capacity(f.dfg.dynamic_types.len());
        for (dyn_ty, _data) in f.dfg.dynamic_types.iter() {
            let ty = f
                .get_concrete_dynamic_ty(dyn_ty)
                .unwrap_or_else(|| panic!("invalid dynamic vector type: {dyn_ty}"));
            let size = isa.dynamic_vector_bytes(ty);
            dynamic_type_sizes.insert(ty, size);
        }

        // Figure out what instructions, if any, will be needed to check the
        // stack limit. This can either be specified as a special-purpose
        // argument or as a global value which often calculates the stack limit
        // from the arguments.
        let stack_limit = f
            .stack_limit
            .map(|gv| gen_stack_limit::<M>(f, sigs, sig, gv));

        let tail_args_size = sigs[sig].sized_stack_arg_space;

        Ok(Self {
            ir_sig: ensure_struct_return_ptr_is_returned(&f.signature),
            sig,
            dynamic_stackslots,
            dynamic_type_sizes,
            sized_stackslots,
            stackslots_size,
            outgoing_args_size: 0,
            tail_args_size,
            reg_args: vec![],
            frame_layout: None,
            ret_area_ptr: None,
            call_conv,
            flags,
            isa_flags: isa_flags.clone(),
            is_leaf: f.is_leaf(),
            stack_limit,
            _mach: PhantomData,
        })
    }

    /// Inserts instructions necessary for checking the stack limit into the
    /// prologue.
    ///
    /// This function will generate instructions necessary for perform a stack
    /// check at the header of a function. The stack check is intended to trap
    /// if the stack pointer goes below a particular threshold, preventing stack
    /// overflow in wasm or other code. The `stack_limit` argument here is the
    /// register which holds the threshold below which we're supposed to trap.
    /// This function is known to allocate `stack_size` bytes and we'll push
    /// instructions onto `insts`.
    ///
    /// Note that the instructions generated here are special because this is
    /// happening so late in the pipeline (e.g. after register allocation). This
    /// means that we need to do manual register allocation here and also be
    /// careful to not clobber any callee-saved or argument registers. For now
    /// this routine makes do with the `spilltmp_reg` as one temporary
    /// register, and a second register of `tmp2` which is caller-saved. This
    /// should be fine for us since no spills should happen in this sequence of
    /// instructions, so our register won't get accidentally clobbered.
    ///
    /// No values can be live after the prologue, but in this case that's ok
    /// because we just need to perform a stack check before progressing with
    /// the rest of the function.
    fn insert_stack_check(
        &self,
        stack_limit: Reg,
        stack_size: u32,
        insts: &mut SmallInstVec<M::I>,
    ) {
        // With no explicit stack allocated we can just emit the simple check of
        // the stack registers against the stack limit register, and trap if
        // it's out of bounds.
        if stack_size == 0 {
            insts.extend(M::gen_stack_lower_bound_trap(stack_limit));
            return;
        }

        // Note that the 32k stack size here is pretty special. See the
        // documentation in x86/abi.rs for why this is here. The general idea is
        // that we're protecting against overflow in the addition that happens
        // below.
        if stack_size >= 32 * 1024 {
            insts.extend(M::gen_stack_lower_bound_trap(stack_limit));
        }

        // Add the `stack_size` to `stack_limit`, placing the result in
        // `scratch`.
        //
        // Note though that `stack_limit`'s register may be the same as
        // `scratch`. If our stack size doesn't fit into an immediate this
        // means we need a second scratch register for loading the stack size
        // into a register.
        let scratch = Writable::from_reg(M::get_stacklimit_reg(self.call_conv));
        insts.extend(M::gen_add_imm(self.call_conv, scratch, stack_limit, stack_size).into_iter());
        insts.extend(M::gen_stack_lower_bound_trap(scratch.to_reg()));
    }
}

/// Generates the instructions necessary for the `gv` to be materialized into a
/// register.
///
/// This function will return a register that will contain the result of
/// evaluating `gv`. It will also return any instructions necessary to calculate
/// the value of the register.
///
/// Note that global values are typically lowered to instructions via the
/// standard legalization pass. Unfortunately though prologue generation happens
/// so late in the pipeline that we can't use these legalization passes to
/// generate the instructions for `gv`. As a result we duplicate some lowering
/// of `gv` here and support only some global values. This is similar to what
/// the x86 backend does for now, and hopefully this can be somewhat cleaned up
/// in the future too!
///
/// Also note that this function will make use of `writable_spilltmp_reg()` as a
/// temporary register to store values in if necessary. Currently after we write
/// to this register there's guaranteed to be no spilled values between where
/// it's used, because we're not participating in register allocation anyway!
fn gen_stack_limit<M: ABIMachineSpec>(
    f: &ir::Function,
    sigs: &SigSet,
    sig: Sig,
    gv: ir::GlobalValue,
) -> (Reg, SmallInstVec<M::I>) {
    let mut insts = smallvec![];
    let reg = generate_gv::<M>(f, sigs, sig, gv, &mut insts);
    return (reg, insts);
}

fn generate_gv<M: ABIMachineSpec>(
    f: &ir::Function,
    sigs: &SigSet,
    sig: Sig,
    gv: ir::GlobalValue,
    insts: &mut SmallInstVec<M::I>,
) -> Reg {
    match f.global_values[gv] {
        // Return the direct register the vmcontext is in
        ir::GlobalValueData::VMContext => {
            get_special_purpose_param_register(f, sigs, sig, ir::ArgumentPurpose::VMContext)
                .expect("no vmcontext parameter found")
        }
        // Load our base value into a register, then load from that register
        // in to a temporary register.
        ir::GlobalValueData::Load {
            base,
            offset,
            global_type: _,
            flags: _,
        } => {
            let base = generate_gv::<M>(f, sigs, sig, base, insts);
            let into_reg = Writable::from_reg(M::get_stacklimit_reg(f.stencil.signature.call_conv));
            insts.push(M::gen_load_base_offset(
                into_reg,
                base,
                offset.into(),
                M::word_type(),
            ));
            return into_reg.to_reg();
        }
        ref other => panic!("global value for stack limit not supported: {other}"),
    }
}

/// If the signature needs to be legalized, then return the struct-return
/// parameter that should be prepended to its returns. Otherwise, return `None`.
fn missing_struct_return(sig: &ir::Signature) -> Option<ir::AbiParam> {
    let struct_ret_index = sig.special_param_index(ArgumentPurpose::StructReturn)?;
    if !sig.uses_special_return(ArgumentPurpose::StructReturn) {
        return Some(sig.params[struct_ret_index]);
    }

    None
}

fn ensure_struct_return_ptr_is_returned(sig: &ir::Signature) -> ir::Signature {
    let mut sig = sig.clone();
    if let Some(sret) = missing_struct_return(&sig) {
        sig.returns.insert(0, sret);
    }
    sig
}

/// ### Pre-Regalloc Functions
///
/// These methods of `Callee` may only be called before regalloc.
impl<M: ABIMachineSpec> Callee<M> {
    /// Access the (possibly legalized) signature.
    pub fn signature(&self) -> &ir::Signature {
        debug_assert!(
            missing_struct_return(&self.ir_sig).is_none(),
            "`Callee::ir_sig` is always legalized"
        );
        &self.ir_sig
    }

    /// Initialize. This is called after the Callee is constructed because it
    /// may allocate a temp vreg, which can only be allocated once the lowering
    /// context exists.
    pub fn init_retval_area(
        &mut self,
        sigs: &SigSet,
        vregs: &mut VRegAllocator<M::I>,
    ) -> CodegenResult<()> {
        if sigs[self.sig].stack_ret_arg.is_some() {
            let ret_area_ptr = vregs.alloc(M::word_type())?;
            self.ret_area_ptr = Some(ret_area_ptr.only_reg().unwrap());
        }
        Ok(())
    }

    /// Get the return area pointer register, if any.
    pub fn ret_area_ptr(&self) -> Option<Reg> {
        self.ret_area_ptr
    }

    /// Accumulate outgoing arguments.
    ///
    /// This ensures that at least `size` bytes are allocated in the prologue to
    /// be available for use in function calls to hold arguments and/or return
    /// values. If this function is called multiple times, the maximum of all
    /// `size` values will be available.
    pub fn accumulate_outgoing_args_size(&mut self, size: u32) {
        if size > self.outgoing_args_size {
            self.outgoing_args_size = size;
        }
    }

    /// Accumulate the incoming argument area size requirements for a tail call,
    /// as it could be larger than the incoming arguments of the function
    /// currently being compiled.
    pub fn accumulate_tail_args_size(&mut self, size: u32) {
        if size > self.tail_args_size {
            self.tail_args_size = size;
        }
    }

    pub fn is_forward_edge_cfi_enabled(&self) -> bool {
        self.isa_flags.is_forward_edge_cfi_enabled()
    }

    /// Get the calling convention implemented by this ABI object.
    pub fn call_conv(&self, sigs: &SigSet) -> isa::CallConv {
        sigs[self.sig].call_conv
    }

    /// Get the ABI-dependent MachineEnv for managing register allocation.
    pub fn machine_env(&self, sigs: &SigSet) -> &MachineEnv {
        M::get_machine_env(&self.flags, self.call_conv(sigs))
    }

    /// The offsets of all sized stack slots (not spill slots) for debuginfo purposes.
    pub fn sized_stackslot_offsets(&self) -> &PrimaryMap<StackSlot, u32> {
        &self.sized_stackslots
    }

    /// The offsets of all dynamic stack slots (not spill slots) for debuginfo purposes.
    pub fn dynamic_stackslot_offsets(&self) -> &PrimaryMap<DynamicStackSlot, u32> {
        &self.dynamic_stackslots
    }

    /// Generate an instruction which copies an argument to a destination
    /// register.
    pub fn gen_copy_arg_to_regs(
        &mut self,
        sigs: &SigSet,
        idx: usize,
        into_regs: ValueRegs<Writable<Reg>>,
        vregs: &mut VRegAllocator<M::I>,
    ) -> SmallInstVec<M::I> {
        let mut insts = smallvec![];
        let mut copy_arg_slot_to_reg = |slot: &ABIArgSlot, into_reg: &Writable<Reg>| {
            match slot {
                &ABIArgSlot::Reg { reg, .. } => {
                    // Add a preg -> def pair to the eventual `args`
                    // instruction.  Extension mode doesn't matter
                    // (we're copying out, not in; we ignore high bits
                    // by convention).
                    let arg = ArgPair {
                        vreg: *into_reg,
                        preg: reg.into(),
                    };
                    self.reg_args.push(arg);
                }
                &ABIArgSlot::Stack {
                    offset,
                    ty,
                    extension,
                    ..
                } => {
                    // However, we have to respect the extension mode for stack
                    // slots, or else we grab the wrong bytes on big-endian.
                    let ext = M::get_ext_mode(sigs[self.sig].call_conv, extension);
                    let ty =
                        if ext != ArgumentExtension::None && M::word_bits() > ty_bits(ty) as u32 {
                            M::word_type()
                        } else {
                            ty
                        };
                    insts.push(M::gen_load_stack(
                        StackAMode::IncomingArg(offset, sigs[self.sig].sized_stack_arg_space),
                        *into_reg,
                        ty,
                    ));
                }
            }
        };

        match &sigs.args(self.sig)[idx] {
            &ABIArg::Slots { ref slots, .. } => {
                assert_eq!(into_regs.len(), slots.len());
                for (slot, into_reg) in slots.iter().zip(into_regs.regs().iter()) {
                    copy_arg_slot_to_reg(&slot, &into_reg);
                }
            }
            &ABIArg::StructArg {
                pointer, offset, ..
            } => {
                let into_reg = into_regs.only_reg().unwrap();
                if let Some(slot) = pointer {
                    // Buffer address is passed in a register or stack slot.
                    copy_arg_slot_to_reg(&slot, &into_reg);
                } else {
                    // Buffer address is implicitly defined by the ABI.
                    insts.push(M::gen_get_stack_addr(
                        StackAMode::IncomingArg(offset, sigs[self.sig].sized_stack_arg_space),
                        into_reg,
                    ));
                }
            }
            &ABIArg::ImplicitPtrArg { pointer, ty, .. } => {
                let into_reg = into_regs.only_reg().unwrap();
                // We need to dereference the pointer.
                let base = match &pointer {
                    &ABIArgSlot::Reg { reg, ty, .. } => {
                        let tmp = vregs.alloc_with_deferred_error(ty).only_reg().unwrap();
                        self.reg_args.push(ArgPair {
                            vreg: Writable::from_reg(tmp),
                            preg: reg.into(),
                        });
                        tmp
                    }
                    &ABIArgSlot::Stack { offset, ty, .. } => {
                        let addr_reg = writable_value_regs(vregs.alloc_with_deferred_error(ty))
                            .only_reg()
                            .unwrap();
                        insts.push(M::gen_load_stack(
                            StackAMode::IncomingArg(offset, sigs[self.sig].sized_stack_arg_space),
                            addr_reg,
                            ty,
                        ));
                        addr_reg.to_reg()
                    }
                };
                insts.push(M::gen_load_base_offset(into_reg, base, 0, ty));
            }
        }
        insts
    }

    /// Generate an instruction which copies a source register to a return value slot.
    pub fn gen_copy_regs_to_retval(
        &self,
        sigs: &SigSet,
        idx: usize,
        from_regs: ValueRegs<Reg>,
        vregs: &mut VRegAllocator<M::I>,
    ) -> (SmallVec<[RetPair; 2]>, SmallInstVec<M::I>) {
        let mut reg_pairs = smallvec![];
        let mut ret = smallvec![];
        let word_bits = M::word_bits() as u8;
        match &sigs.rets(self.sig)[idx] {
            &ABIArg::Slots { ref slots, .. } => {
                assert_eq!(from_regs.len(), slots.len());
                for (slot, &from_reg) in slots.iter().zip(from_regs.regs().iter()) {
                    match slot {
                        &ABIArgSlot::Reg {
                            reg, ty, extension, ..
                        } => {
                            let from_bits = ty_bits(ty) as u8;
                            let ext = M::get_ext_mode(sigs[self.sig].call_conv, extension);
                            let vreg = match (ext, from_bits) {
                                (ir::ArgumentExtension::Uext, n)
                                | (ir::ArgumentExtension::Sext, n)
                                    if n < word_bits =>
                                {
                                    let signed = ext == ir::ArgumentExtension::Sext;
                                    let dst =
                                        writable_value_regs(vregs.alloc_with_deferred_error(ty))
                                            .only_reg()
                                            .unwrap();
                                    ret.push(M::gen_extend(
                                        dst, from_reg, signed, from_bits,
                                        /* to_bits = */ word_bits,
                                    ));
                                    dst.to_reg()
                                }
                                _ => {
                                    // No move needed, regalloc2 will emit it using the constraint
                                    // added by the RetPair.
                                    from_reg
                                }
                            };
                            reg_pairs.push(RetPair {
                                vreg,
                                preg: Reg::from(reg),
                            });
                        }
                        &ABIArgSlot::Stack {
                            offset,
                            ty,
                            extension,
                            ..
                        } => {
                            let mut ty = ty;
                            let from_bits = ty_bits(ty) as u8;
                            // A machine ABI implementation should ensure that stack frames
                            // have "reasonable" size. All current ABIs for machinst
                            // backends (aarch64 and x64) enforce a 128MB limit.
                            let off = i32::try_from(offset).expect(
                                "Argument stack offset greater than 2GB; should hit impl limit first",
                                );
                            let ext = M::get_ext_mode(sigs[self.sig].call_conv, extension);
                            // Trash the from_reg; it should be its last use.
                            match (ext, from_bits) {
                                (ir::ArgumentExtension::Uext, n)
                                | (ir::ArgumentExtension::Sext, n)
                                    if n < word_bits =>
                                {
                                    assert_eq!(M::word_reg_class(), from_reg.class());
                                    let signed = ext == ir::ArgumentExtension::Sext;
                                    let dst =
                                        writable_value_regs(vregs.alloc_with_deferred_error(ty))
                                            .only_reg()
                                            .unwrap();
                                    ret.push(M::gen_extend(
                                        dst, from_reg, signed, from_bits,
                                        /* to_bits = */ word_bits,
                                    ));
                                    // Store the extended version.
                                    ty = M::word_type();
                                }
                                _ => {}
                            };
                            ret.push(M::gen_store_base_offset(
                                self.ret_area_ptr.unwrap(),
                                off,
                                from_reg,
                                ty,
                            ));
                        }
                    }
                }
            }
            ABIArg::StructArg { .. } => {
                panic!("StructArg in return position is unsupported");
            }
            ABIArg::ImplicitPtrArg { .. } => {
                panic!("ImplicitPtrArg in return position is unsupported");
            }
        }
        (reg_pairs, ret)
    }

    /// Generate any setup instruction needed to save values to the
    /// return-value area. This is usually used when were are multiple return
    /// values or an otherwise large return value that must be passed on the
    /// stack; typically the ABI specifies an extra hidden argument that is a
    /// pointer to that memory.
    pub fn gen_retval_area_setup(
        &mut self,
        sigs: &SigSet,
        vregs: &mut VRegAllocator<M::I>,
    ) -> Option<M::I> {
        if let Some(i) = sigs[self.sig].stack_ret_arg {
            let ret_area_ptr = Writable::from_reg(self.ret_area_ptr.unwrap());
            let insts =
                self.gen_copy_arg_to_regs(sigs, i.into(), ValueRegs::one(ret_area_ptr), vregs);
            insts.into_iter().next().map(|inst| {
                trace!(
                    "gen_retval_area_setup: inst {:?}; ptr reg is {:?}",
                    inst,
                    ret_area_ptr.to_reg()
                );
                inst
            })
        } else {
            trace!("gen_retval_area_setup: not needed");
            None
        }
    }

    /// Generate a return instruction.
    pub fn gen_rets(&self, rets: Vec<RetPair>) -> M::I {
        M::gen_rets(rets)
    }

    /// Produce an instruction that computes a sized stackslot address.
    pub fn sized_stackslot_addr(
        &self,
        slot: StackSlot,
        offset: u32,
        into_reg: Writable<Reg>,
    ) -> M::I {
        // Offset from beginning of stackslot area.
        let stack_off = self.sized_stackslots[slot] as i64;
        let sp_off: i64 = stack_off + (offset as i64);
        M::gen_get_stack_addr(StackAMode::Slot(sp_off), into_reg)
    }

    /// Produce an instruction that computes a dynamic stackslot address.
    pub fn dynamic_stackslot_addr(&self, slot: DynamicStackSlot, into_reg: Writable<Reg>) -> M::I {
        let stack_off = self.dynamic_stackslots[slot] as i64;
        M::gen_get_stack_addr(StackAMode::Slot(stack_off), into_reg)
    }

    /// Get an `args` pseudo-inst, if any, that should appear at the
    /// very top of the function body prior to regalloc.
    pub fn take_args(&mut self) -> Option<M::I> {
        if self.reg_args.len() > 0 {
            // Very first instruction is an `args` pseudo-inst that
            // establishes live-ranges for in-register arguments and
            // constrains them at the start of the function to the
            // locations defined by the ABI.
            Some(M::gen_args(std::mem::take(&mut self.reg_args)))
        } else {
            None
        }
    }
}

/// ### Post-Regalloc Functions
///
/// These methods of `Callee` may only be called after
/// regalloc.
impl<M: ABIMachineSpec> Callee<M> {
    /// Compute the final frame layout, post-regalloc.
    ///
    /// This must be called before gen_prologue or gen_epilogue.
    pub fn compute_frame_layout(
        &mut self,
        sigs: &SigSet,
        spillslots: usize,
        clobbered: Vec<Writable<RealReg>>,
    ) {
        let bytes = M::word_bytes();
        let total_stacksize = self.stackslots_size + bytes * spillslots as u32;
        let mask = M::stack_align(self.call_conv) - 1;
        let total_stacksize = (total_stacksize + mask) & !mask; // 16-align the stack.
        self.frame_layout = Some(M::compute_frame_layout(
            self.call_conv,
            &self.flags,
            self.signature(),
            &clobbered,
            self.is_leaf,
            self.stack_args_size(sigs),
            self.tail_args_size,
            total_stacksize,
            self.outgoing_args_size,
        ));
    }

    /// Generate a prologue, post-regalloc.
    ///
    /// This should include any stack frame or other setup necessary to use the
    /// other methods (`load_arg`, `store_retval`, and spillslot accesses.)
    pub fn gen_prologue(&self) -> SmallInstVec<M::I> {
        let frame_layout = self.frame_layout();
        let mut insts = smallvec![];

        // Set up frame.
        insts.extend(M::gen_prologue_frame_setup(
            self.call_conv,
            &self.flags,
            &self.isa_flags,
            &frame_layout,
        ));

        // The stack limit check needs to cover all the stack adjustments we
        // might make, up to the next stack limit check in any function we
        // call. Since this happens after frame setup, the current function's
        // setup area needs to be accounted for in the caller's stack limit
        // check, but we need to account for any setup area that our callees
        // might need. Note that s390x may also use the outgoing args area for
        // backtrace support even in leaf functions, so that should be accounted
        // for unconditionally.
        let total_stacksize = (frame_layout.tail_args_size - frame_layout.incoming_args_size)
            + frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.outgoing_args_size
            + if self.is_leaf {
                0
            } else {
                frame_layout.setup_area_size
            };

        // Leaf functions with zero stack don't need a stack check if one's
        // specified, otherwise always insert the stack check.
        if total_stacksize > 0 || !self.is_leaf {
            if let Some((reg, stack_limit_load)) = &self.stack_limit {
                insts.extend(stack_limit_load.clone());
                self.insert_stack_check(*reg, total_stacksize, &mut insts);
            }

            if self.flags.enable_probestack() {
                let guard_size = 1 << self.flags.probestack_size_log2();
                if total_stacksize >= guard_size {
                    match self.flags.probestack_strategy() {
                        ProbestackStrategy::Inline => M::gen_inline_probestack(
                            &mut insts,
                            self.call_conv,
                            total_stacksize,
                            guard_size,
                        ),
                        ProbestackStrategy::Outline => {
                            M::gen_probestack(&mut insts, total_stacksize)
                        }
                    }
                }
            }
        }

        // Save clobbered registers.
        insts.extend(M::gen_clobber_save(
            self.call_conv,
            &self.flags,
            &frame_layout,
        ));

        insts
    }

    /// Generate an epilogue, post-regalloc.
    ///
    /// Note that this must generate the actual return instruction (rather than
    /// emitting this in the lowering logic), because the epilogue code comes
    /// before the return and the two are likely closely related.
    pub fn gen_epilogue(&self) -> SmallInstVec<M::I> {
        let frame_layout = self.frame_layout();
        let mut insts = smallvec![];

        // Restore clobbered registers.
        insts.extend(M::gen_clobber_restore(
            self.call_conv,
            &self.flags,
            &frame_layout,
        ));

        // Tear down frame.
        insts.extend(M::gen_epilogue_frame_restore(
            self.call_conv,
            &self.flags,
            &self.isa_flags,
            &frame_layout,
        ));

        // And return.
        insts.extend(M::gen_return(
            self.call_conv,
            &self.isa_flags,
            &frame_layout,
        ));

        trace!("Epilogue: {:?}", insts);
        insts
    }

    /// Return a reference to the computed frame layout information. This
    /// function will panic if it's called before [`Self::compute_frame_layout`].
    pub fn frame_layout(&self) -> &FrameLayout {
        self.frame_layout
            .as_ref()
            .expect("frame layout not computed before prologue generation")
    }

    /// Returns the full frame size for the given function, after prologue
    /// emission has run. This comprises the spill slots and stack-storage
    /// slots as well as storage for clobbered callee-save registers, but
    /// not arguments arguments pushed at callsites within this function,
    /// or other ephemeral pushes.
    pub fn frame_size(&self) -> u32 {
        let frame_layout = self.frame_layout();
        frame_layout.clobber_size + frame_layout.fixed_frame_storage_size
    }

    /// Returns offset from the slot base in the current frame to the caller's SP.
    pub fn slot_base_to_caller_sp_offset(&self) -> u32 {
        let frame_layout = self.frame_layout();
        frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.setup_area_size
    }

    /// Returns the size of arguments expected on the stack.
    pub fn stack_args_size(&self, sigs: &SigSet) -> u32 {
        sigs[self.sig].sized_stack_arg_space
    }

    /// Get the spill-slot size.
    pub fn get_spillslot_size(&self, rc: RegClass) -> u32 {
        let max = if self.dynamic_type_sizes.len() == 0 {
            16
        } else {
            *self
                .dynamic_type_sizes
                .iter()
                .max_by(|x, y| x.1.cmp(&y.1))
                .map(|(_k, v)| v)
                .unwrap()
        };
        M::get_number_of_spillslots_for_value(rc, max, &self.isa_flags)
    }

    /// Get the spill slot offset relative to the fixed allocation area start.
    pub fn get_spillslot_offset(&self, slot: SpillSlot) -> i64 {
        // Offset from beginning of spillslot area.
        let islot = slot.index() as i64;
        let spill_off = islot * M::word_bytes() as i64;
        let sp_off = self.stackslots_size as i64 + spill_off;

        sp_off
    }

    /// Generate a spill.
    pub fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg) -> M::I {
        let ty = M::I::canonical_type_for_rc(from_reg.class());
        debug_assert_eq!(<M>::I::rc_for_type(ty).unwrap().1, &[ty]);

        let sp_off = self.get_spillslot_offset(to_slot);
        trace!("gen_spill: {from_reg:?} into slot {to_slot:?} at offset {sp_off}");

        let from = StackAMode::Slot(sp_off);
        <M>::gen_store_stack(from, Reg::from(from_reg), ty)
    }

    /// Generate a reload (fill).
    pub fn gen_reload(&self, to_reg: Writable<RealReg>, from_slot: SpillSlot) -> M::I {
        let ty = M::I::canonical_type_for_rc(to_reg.to_reg().class());
        debug_assert_eq!(<M>::I::rc_for_type(ty).unwrap().1, &[ty]);

        let sp_off = self.get_spillslot_offset(from_slot);
        trace!("gen_reload: {to_reg:?} from slot {from_slot:?} at offset {sp_off}");

        let from = StackAMode::Slot(sp_off);
        <M>::gen_load_stack(from, to_reg.map(Reg::from), ty)
    }
}

/// An input argument to a call instruction: the vreg that is used,
/// and the preg it is constrained to (per the ABI).
#[derive(Clone, Debug)]
pub struct CallArgPair {
    /// The virtual register to use for the argument.
    pub vreg: Reg,
    /// The real register into which the arg goes.
    pub preg: Reg,
}

/// An output return value from a call instruction: the vreg that is
/// defined, and the preg it is constrained to (per the ABI).
#[derive(Clone, Debug)]
pub struct CallRetPair {
    /// The virtual register to define from this return value.
    pub vreg: Writable<Reg>,
    /// The real register from which the return value is read.
    pub preg: Reg,
}

pub type CallArgList = SmallVec<[CallArgPair; 8]>;
pub type CallRetList = SmallVec<[CallRetPair; 8]>;

pub enum IsTailCall {
    Yes,
    No,
}

/// ABI object for a callsite.
pub struct CallSite<M: ABIMachineSpec> {
    /// The called function's signature.
    sig: Sig,
    /// All register uses for the callsite, i.e., function args, with
    /// VReg and the physical register it is constrained to.
    uses: CallArgList,
    /// All defs for the callsite, i.e., return values.
    defs: CallRetList,
    /// Call destination.
    dest: CallDest,
    is_tail_call: IsTailCall,
    /// Caller's calling convention.
    caller_conv: isa::CallConv,
    /// The settings controlling this compilation.
    flags: settings::Flags,

    _mach: PhantomData<M>,
}

/// Destination for a call.
#[derive(Debug, Clone)]
pub enum CallDest {
    /// Call to an ExtName (named function symbol).
    ExtName(ir::ExternalName, RelocDistance),
    /// Indirect call to a function pointer in a register.
    Reg(Reg),
}

impl<M: ABIMachineSpec> CallSite<M> {
    /// Create a callsite ABI object for a call directly to the specified function.
    pub fn from_func(
        sigs: &SigSet,
        sig_ref: ir::SigRef,
        extname: &ir::ExternalName,
        is_tail_call: IsTailCall,
        dist: RelocDistance,
        caller_conv: isa::CallConv,
        flags: settings::Flags,
    ) -> CallSite<M> {
        let sig = sigs.abi_sig_for_sig_ref(sig_ref);
        CallSite {
            sig,
            uses: smallvec![],
            defs: smallvec![],
            dest: CallDest::ExtName(extname.clone(), dist),
            is_tail_call,
            caller_conv,
            flags,
            _mach: PhantomData,
        }
    }

    /// Create a callsite ABI object for a call directly to the specified
    /// libcall.
    pub fn from_libcall(
        sigs: &SigSet,
        sig: &ir::Signature,
        extname: &ir::ExternalName,
        dist: RelocDistance,
        caller_conv: isa::CallConv,
        flags: settings::Flags,
    ) -> CallSite<M> {
        let sig = sigs.abi_sig_for_signature(sig);
        CallSite {
            sig,
            uses: smallvec![],
            defs: smallvec![],
            dest: CallDest::ExtName(extname.clone(), dist),
            is_tail_call: IsTailCall::No,
            caller_conv,
            flags,
            _mach: PhantomData,
        }
    }

    /// Create a callsite ABI object for a call to a function pointer with the
    /// given signature.
    pub fn from_ptr(
        sigs: &SigSet,
        sig_ref: ir::SigRef,
        ptr: Reg,
        is_tail_call: IsTailCall,
        caller_conv: isa::CallConv,
        flags: settings::Flags,
    ) -> CallSite<M> {
        let sig = sigs.abi_sig_for_sig_ref(sig_ref);
        CallSite {
            sig,
            uses: smallvec![],
            defs: smallvec![],
            dest: CallDest::Reg(ptr),
            is_tail_call,
            caller_conv,
            flags,
            _mach: PhantomData,
        }
    }

    pub(crate) fn dest(&self) -> &CallDest {
        &self.dest
    }

    pub(crate) fn take_uses(self) -> CallArgList {
        self.uses
    }

    pub(crate) fn sig<'a>(&self, sigs: &'a SigSet) -> &'a SigData {
        &sigs[self.sig]
    }

    pub(crate) fn is_tail_call(&self) -> bool {
        matches!(self.is_tail_call, IsTailCall::Yes)
    }
}

impl<M: ABIMachineSpec> CallSite<M> {
    /// Get the number of arguments expected.
    pub fn num_args(&self, sigs: &SigSet) -> usize {
        sigs.num_args(self.sig)
    }

    /// Get the number of return values expected.
    pub fn num_rets(&self, sigs: &SigSet) -> usize {
        sigs.num_rets(self.sig)
    }

    /// Emit a copy of a large argument into its associated stack buffer, if
    /// any.  We must be careful to perform all these copies (as necessary)
    /// before setting up the argument registers, since we may have to invoke
    /// memcpy(), which could clobber any registers already set up.  The
    /// back-end should call this routine for all arguments before calling
    /// `gen_arg` for all arguments.
    pub fn emit_copy_regs_to_buffer(
        &self,
        ctx: &mut Lower<M::I>,
        idx: usize,
        from_regs: ValueRegs<Reg>,
    ) {
        match &ctx.sigs().args(self.sig)[idx] {
            &ABIArg::Slots { .. } | &ABIArg::ImplicitPtrArg { .. } => {}
            &ABIArg::StructArg { offset, size, .. } => {
                let src_ptr = from_regs.only_reg().unwrap();
                let dst_ptr = ctx.alloc_tmp(M::word_type()).only_reg().unwrap();
                ctx.emit(M::gen_get_stack_addr(
                    StackAMode::OutgoingArg(offset),
                    dst_ptr,
                ));
                // Emit a memcpy from `src_ptr` to `dst_ptr` of `size` bytes.
                // N.B.: because we process StructArg params *first*, this is
                // safe w.r.t. clobbers: we have not yet filled in any other
                // arg regs.
                let memcpy_call_conv =
                    isa::CallConv::for_libcall(&self.flags, ctx.sigs()[self.sig].call_conv);
                for insn in M::gen_memcpy(
                    memcpy_call_conv,
                    dst_ptr.to_reg(),
                    src_ptr,
                    size as usize,
                    |ty| ctx.alloc_tmp(ty).only_reg().unwrap(),
                )
                .into_iter()
                {
                    ctx.emit(insn);
                }
            }
        }
    }

    /// Add a constraint for an argument value from a source register.
    /// For large arguments with associated stack buffer, this may
    /// load the address of the buffer into the argument register, if
    /// required by the ABI.
    pub fn gen_arg(&mut self, ctx: &mut Lower<M::I>, idx: usize, from_regs: ValueRegs<Reg>) {
        let stack_arg_space = ctx.sigs()[self.sig].sized_stack_arg_space;
        let stack_arg = if self.is_tail_call() {
            StackAMode::IncomingArg
        } else {
            |offset, _| StackAMode::OutgoingArg(offset)
        };
        let word_rc = M::word_reg_class();
        let word_bits = M::word_bits() as usize;

        match ctx.sigs().args(self.sig)[idx].clone() {
            ABIArg::Slots { ref slots, .. } => {
                assert_eq!(from_regs.len(), slots.len());
                for (slot, from_reg) in slots.iter().zip(from_regs.regs().iter()) {
                    match slot {
                        &ABIArgSlot::Reg {
                            reg, ty, extension, ..
                        } => {
                            let ext = M::get_ext_mode(ctx.sigs()[self.sig].call_conv, extension);
                            let vreg =
                                if ext != ir::ArgumentExtension::None && ty_bits(ty) < word_bits {
                                    assert_eq!(word_rc, reg.class());
                                    let signed = match ext {
                                        ir::ArgumentExtension::Uext => false,
                                        ir::ArgumentExtension::Sext => true,
                                        _ => unreachable!(),
                                    };
                                    let extend_result =
                                        ctx.alloc_tmp(M::word_type()).only_reg().unwrap();
                                    ctx.emit(M::gen_extend(
                                        extend_result,
                                        *from_reg,
                                        signed,
                                        ty_bits(ty) as u8,
                                        word_bits as u8,
                                    ));
                                    extend_result.to_reg()
                                } else {
                                    *from_reg
                                };

                            let preg = reg.into();
                            self.uses.push(CallArgPair { vreg, preg });
                        }
                        &ABIArgSlot::Stack {
                            offset,
                            ty,
                            extension,
                            ..
                        } => {
                            let ext = M::get_ext_mode(ctx.sigs()[self.sig].call_conv, extension);
                            let (data, ty) =
                                if ext != ir::ArgumentExtension::None && ty_bits(ty) < word_bits {
                                    assert_eq!(word_rc, from_reg.class());
                                    let signed = match ext {
                                        ir::ArgumentExtension::Uext => false,
                                        ir::ArgumentExtension::Sext => true,
                                        _ => unreachable!(),
                                    };
                                    let extend_result =
                                        ctx.alloc_tmp(M::word_type()).only_reg().unwrap();
                                    ctx.emit(M::gen_extend(
                                        extend_result,
                                        *from_reg,
                                        signed,
                                        ty_bits(ty) as u8,
                                        word_bits as u8,
                                    ));
                                    // Store the extended version.
                                    (extend_result.to_reg(), M::word_type())
                                } else {
                                    (*from_reg, ty)
                                };
                            ctx.emit(M::gen_store_stack(
                                stack_arg(offset, stack_arg_space),
                                data,
                                ty,
                            ));
                        }
                    }
                }
            }
            ABIArg::StructArg { pointer, .. } => {
                assert!(pointer.is_none()); // Only supported via ISLE.
            }
            ABIArg::ImplicitPtrArg {
                offset,
                pointer,
                ty,
                purpose: _,
            } => {
                assert_eq!(from_regs.len(), 1);
                let vreg = from_regs.regs()[0];
                let amode = StackAMode::OutgoingArg(offset);
                let tmp = ctx.alloc_tmp(M::word_type()).only_reg().unwrap();
                ctx.emit(M::gen_get_stack_addr(amode, tmp));
                let tmp = tmp.to_reg();
                ctx.emit(M::gen_store_base_offset(tmp, 0, vreg, ty));
                match pointer {
                    ABIArgSlot::Reg { reg, .. } => self.uses.push(CallArgPair {
                        vreg: tmp,
                        preg: reg.into(),
                    }),
                    ABIArgSlot::Stack { offset, .. } => ctx.emit(M::gen_store_stack(
                        stack_arg(offset, stack_arg_space),
                        tmp,
                        M::word_type(),
                    )),
                }
            }
        }
    }

    /// Call `gen_arg` for each non-hidden argument and emit all instructions
    /// generated.
    pub fn emit_args(&mut self, ctx: &mut Lower<M::I>, (inputs, off): isle::ValueSlice) {
        let num_args = self.num_args(ctx.sigs());
        assert_eq!(inputs.len(&ctx.dfg().value_lists) - off, num_args);

        let mut arg_value_regs: SmallVec<[_; 16]> = smallvec![];
        for i in 0..num_args {
            let input = inputs.get(off + i, &ctx.dfg().value_lists).unwrap();
            arg_value_regs.push(ctx.put_value_in_regs(input));
        }
        for (i, arg_regs) in arg_value_regs.iter().enumerate() {
            self.emit_copy_regs_to_buffer(ctx, i, *arg_regs);
        }
        for (i, value_regs) in arg_value_regs.iter().enumerate() {
            self.gen_arg(ctx, i, *value_regs);
        }
    }

    /// Emit the code to forward a stack-return pointer argument through a tail
    /// call.
    pub fn emit_stack_ret_arg_for_tail_call(&mut self, ctx: &mut Lower<M::I>) {
        if let Some(i) = ctx.sigs()[self.sig].stack_ret_arg() {
            let ret_area_ptr = ctx.abi().ret_area_ptr.expect(
                "if the tail callee has a return pointer, then the tail caller \
                 must as well",
            );
            self.gen_arg(ctx, i.into(), ValueRegs::one(ret_area_ptr));
        }
    }

    /// Define a return value after the call returns.
    pub fn gen_retval(
        &mut self,
        ctx: &mut Lower<M::I>,
        idx: usize,
    ) -> (SmallInstVec<M::I>, ValueRegs<Reg>) {
        let mut insts = smallvec![];
        let mut into_regs: SmallVec<[Reg; 2]> = smallvec![];
        let ret = ctx.sigs().rets(self.sig)[idx].clone();
        match ret {
            ABIArg::Slots { ref slots, .. } => {
                for slot in slots {
                    match slot {
                        // Extension mode doesn't matter because we're copying out, not in,
                        // and we ignore high bits in our own registers by convention.
                        &ABIArgSlot::Reg { reg, ty, .. } => {
                            let into_reg = ctx.alloc_tmp(ty).only_reg().unwrap();
                            self.defs.push(CallRetPair {
                                vreg: into_reg,
                                preg: reg.into(),
                            });
                            into_regs.push(into_reg.to_reg());
                        }
                        &ABIArgSlot::Stack { offset, ty, .. } => {
                            let into_reg = ctx.alloc_tmp(ty).only_reg().unwrap();
                            let sig_data = &ctx.sigs()[self.sig];
                            // The outgoing argument area must always be restored after a call,
                            // ensuring that the return values will be in a consistent place after
                            // any call.
                            let ret_area_base = sig_data.sized_stack_arg_space();
                            insts.push(M::gen_load_stack(
                                StackAMode::OutgoingArg(offset + ret_area_base),
                                into_reg,
                                ty,
                            ));
                            into_regs.push(into_reg.to_reg());
                        }
                    }
                }
            }
            ABIArg::StructArg { .. } => {
                panic!("StructArg not supported in return position");
            }
            ABIArg::ImplicitPtrArg { .. } => {
                panic!("ImplicitPtrArg not supported in return position");
            }
        }

        let value_regs = match *into_regs {
            [a] => ValueRegs::one(a),
            [a, b] => ValueRegs::two(a, b),
            _ => panic!("Expected to see one or two slots only from {ret:?}"),
        };
        (insts, value_regs)
    }

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
    /// parts of the `CallSite` object in emitting instructions.
    pub fn emit_call(&mut self, ctx: &mut Lower<M::I>) {
        let word_type = M::word_type();
        if let Some(i) = ctx.sigs()[self.sig].stack_ret_arg {
            let rd = ctx.alloc_tmp(word_type).only_reg().unwrap();
            let ret_area_base = ctx.sigs()[self.sig].sized_stack_arg_space();
            ctx.emit(M::gen_get_stack_addr(
                StackAMode::OutgoingArg(ret_area_base),
                rd,
            ));
            self.gen_arg(ctx, i.into(), ValueRegs::one(rd.to_reg()));
        }

        let uses = mem::take(&mut self.uses);
        let defs = mem::take(&mut self.defs);
        let clobbers = {
            // Get clobbers: all caller-saves. These may include return value
            // regs, which we will remove from the clobber set below.
            let mut clobbers = <M>::get_regs_clobbered_by_call(ctx.sigs()[self.sig].call_conv);

            // Remove retval regs from clobbers.
            for def in &defs {
                clobbers.remove(PReg::from(def.preg.to_real_reg().unwrap()));
            }

            clobbers
        };

        let sig = &ctx.sigs()[self.sig];
        let callee_pop_size = if sig.call_conv() == isa::CallConv::Tail {
            // The tail calling convention has callees pop stack arguments.
            sig.sized_stack_arg_space
        } else {
            0
        };

        let call_conv = sig.call_conv;
        let ret_space = sig.sized_stack_ret_space;
        let arg_space = sig.sized_stack_arg_space;

        ctx.abi_mut()
            .accumulate_outgoing_args_size(ret_space + arg_space);

        let tmp = ctx.alloc_tmp(word_type).only_reg().unwrap();

        // Any adjustment to SP to account for required outgoing arguments/stack return values must
        // be done inside of the call pseudo-op, to ensure that SP is always in a consistent
        // state for all other instructions. For example, if a tail-call abi function is called
        // here, the reclamation of the outgoing argument area must be done inside of the call
        // pseudo-op's emission to ensure that SP is consistent at all other points in the lowered
        // function. (Except the prologue and epilogue, but those are fairly special parts of the
        // function that establish the SP invariants that are relied on elsewhere and are generated
        // after the register allocator has run and thus cannot have register allocator-inserted
        // references to SP offsets.)
        for inst in M::gen_call(
            &self.dest,
            uses,
            defs,
            clobbers,
            tmp,
            call_conv,
            self.caller_conv,
            callee_pop_size,
        )
        .into_iter()
        {
            ctx.emit(inst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SigData;

    #[test]
    fn sig_data_size() {
        // The size of `SigData` is performance sensitive, so make sure
        // we don't regress it unintentionally.
        assert_eq!(std::mem::size_of::<SigData>(), 24);
    }
}
