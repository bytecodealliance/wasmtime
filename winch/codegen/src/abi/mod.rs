//! This module provides all the necessary building blocks for
//! implementing ISA specific ABIs.
//!
//! # Default ABI
//!
//! Winch uses a default internal ABI, for all internal functions.
//! This allows us to push the complexity of system ABI compliance to
//! the trampolines (not yet implemented).  The default ABI treats all
//! allocatable registers as caller saved, which means that (i) all
//! register values in the Wasm value stack (which are normally
//! referred to as "live"), must be saved onto the machine stack (ii)
//! function prologues and epilogues don't store/restore other
//! registers more than the non-allocatable ones (e.g. rsp/rbp in
//! x86_64).
//!
//! The calling convention in the default ABI, uses registers to a
//! certain fixed count for arguments and return values, and then the
//! stack is used for all additional arguments.
//!
//! Generally the stack layout looks like:
//! +-------------------------------+
//! |                               |
//! |                               |
//! |         Stack Args            |
//! |                               |
//! |                               |
//! +-------------------------------+----> SP @ function entry
//! |         Ret addr              |
//! +-------------------------------+
//! |            SP                 |
//! +-------------------------------+----> SP @ Function prologue
//! |                               |
//! |                               |
//! |                               |
//! |        Stack slots            |
//! |        + `VMContext` slot     |
//! |        + dynamic space        |
//! |                               |
//! |                               |
//! |                               |
//! +-------------------------------+----> SP @ callsite (after)
//! |        alignment              |
//! |        + arguments            |
//! |                               | ----> Space allocated for calls
//! |                               |
use crate::isa::{reg::Reg, CallingConvention};
use crate::masm::OperandSize;
use smallvec::SmallVec;
use std::collections::HashSet;
use std::ops::{Add, BitAnd, Not, Sub};
use wasmtime_environ::{WasmFuncType, WasmHeapType, WasmType};

pub(crate) mod local;
pub(crate) use local::*;

/// Trait implemented by a specific ISA and used to provide
/// information about alignment, parameter passing, usage of
/// specific registers, etc.
pub(crate) trait ABI {
    /// The required stack alignment.
    fn stack_align() -> u8;

    /// The required stack alignment for calls.
    fn call_stack_align() -> u8;

    /// The offset to the argument base, relative to the frame pointer.
    fn arg_base_offset() -> u8;

    /// The offset to the return address, relative to the frame pointer.
    fn ret_addr_offset() -> u8;

    /// Construct the ABI-specific signature from a WebAssembly
    /// function type.
    fn sig(wasm_sig: &WasmFuncType, call_conv: &CallingConvention) -> ABISig;

    /// Construct an ABI signature from WasmType params and returns.
    fn sig_from(params: &[WasmType], returns: &[WasmType], call_conv: &CallingConvention)
        -> ABISig;

    /// Construct the ABI-specific result from a slice of
    /// [`wasmtime_environ::WasmtType`].
    fn result(returns: &[WasmType], call_conv: &CallingConvention) -> ABIResult;

    /// Returns the number of bits in a word.
    fn word_bits() -> u32;

    /// Returns the number of bytes in a word.
    fn word_bytes() -> u32 {
        Self::word_bits() / 8
    }

    /// Returns the designated scratch register.
    fn scratch_reg() -> Reg;

    /// Returns the frame pointer register.
    fn fp_reg() -> Reg;

    /// Returns the stack pointer register.
    fn sp_reg() -> Reg;

    /// Returns the pinned register used to hold
    /// the `VMContext`.
    fn vmctx_reg() -> Reg;

    /// Returns the callee-saved registers for the given
    /// calling convention.
    fn callee_saved_regs(call_conv: &CallingConvention) -> SmallVec<[(Reg, OperandSize); 18]>;

    /// Returns the size of each argument stack slot per argument type.
    fn stack_arg_slot_size_for_type(ty: WasmType) -> u32;
}

/// ABI-specific representation of a function argument.
#[derive(Clone, Debug)]
pub enum ABIArg {
    /// A register argument.
    Reg {
        /// Type of the argument.
        ty: WasmType,
        /// Register holding the argument.
        reg: Reg,
    },
    /// A stack argument.
    Stack {
        /// The type of the argument.
        ty: WasmType,
        /// Offset of the argument relative to the frame pointer.
        offset: u32,
    },
}

impl ABIArg {
    /// Allocate a new register abi arg.
    pub fn reg(reg: Reg, ty: WasmType) -> Self {
        Self::Reg { reg, ty }
    }

    /// Allocate a new stack abi arg.
    pub fn stack_offset(offset: u32, ty: WasmType) -> Self {
        Self::Stack { ty, offset }
    }

    /// Is this abi arg in a register.
    pub fn is_reg(&self) -> bool {
        match *self {
            ABIArg::Reg { .. } => true,
            _ => false,
        }
    }

    /// Get the register associated to this arg.
    pub fn get_reg(&self) -> Option<Reg> {
        match *self {
            ABIArg::Reg { reg, .. } => Some(reg),
            _ => None,
        }
    }

    /// Get the type associated to this arg.
    pub fn ty(&self) -> WasmType {
        match *self {
            ABIArg::Reg { ty, .. } | ABIArg::Stack { ty, .. } => ty,
        }
    }
}

/// ABI-specific representation of the function result.
#[derive(Copy, Clone, Debug)]
pub(crate) enum ABIResult {
    Void,
    Reg {
        /// Type of the result.
        ty: WasmType,
        /// Register to hold the result.
        reg: Reg,
    },
}

impl Default for ABIResult {
    fn default() -> Self {
        Self::Void
    }
}

impl ABIResult {
    /// Create a register ABI result.
    pub fn reg(ty: WasmType, reg: Reg) -> Self {
        Self::Reg { ty, reg }
    }

    /// Createa void ABI result.
    pub fn void() -> Self {
        Self::Void
    }

    /// Get the result reg.
    pub fn result_reg(&self) -> Option<Reg> {
        match self {
            Self::Reg { reg, .. } => Some(*reg),
            _ => None,
        }
    }

    /// Checks if the result is void.
    pub fn is_void(&self) -> bool {
        match self {
            Self::Void => true,
            _ => false,
        }
    }

    /// Returns the length of the result.
    pub fn len(&self) -> usize {
        if self.is_void() {
            0
        } else {
            1
        }
    }

    /// Returns an iterator over the result registers.
    ///
    /// NOTE: Currently only one or zero registers
    /// will be returned until suport for multi-value is introduced.
    pub fn regs(&self) -> impl Iterator<Item = Reg> + '_ {
        std::iter::once(self.result_reg()).filter_map(|v| v)
    }
}

pub(crate) type ABIParams = SmallVec<[ABIArg; 6]>;

/// An ABI-specific representation of a function signature.
#[derive(Debug, Clone)]
pub(crate) struct ABISig {
    /// Function parameters.
    pub params: ABIParams,
    /// Function result.
    pub result: ABIResult,
    /// Stack space needed for stack arguments.
    pub stack_bytes: u32,
    /// All the registers used in the [`ABISig`].
    /// Note that this collection is guaranteed to
    /// be unique: in some cases some registers might
    /// be used as params as a well as returns (e.g. xmm0 in x64).
    pub regs: HashSet<Reg>,
}

impl ABISig {
    /// Create a new ABI signature.
    pub fn new(params: ABIParams, result: ABIResult, stack_bytes: u32) -> Self {
        let regs = params
            .iter()
            .filter_map(|r| r.get_reg())
            .collect::<SmallVec<[Reg; 6]>>();
        let result_regs = result.regs();
        let chained = regs.into_iter().chain(result_regs);

        Self {
            params,
            result,
            stack_bytes,
            regs: HashSet::from_iter(chained),
        }
    }
}

/// Returns the size in bytes of a given WebAssembly type.
pub(crate) fn ty_size(ty: &WasmType) -> u32 {
    match *ty {
        WasmType::I32 | WasmType::F32 => 4,
        WasmType::I64 | WasmType::F64 => 8,
        WasmType::Ref(rt) => match rt.heap_type {
            // TODO: Similar to the comment in visitor.rs at impl From<WasmType> for
            // OperandSize, Once Wasmtime supports 32-bit architectures, this will
            // need to be updated to derive operand size from the target's pointer
            // size.
            WasmHeapType::Func => 8,
            ht => unimplemented!("Support for WasmHeapType: {ht}"),
        },
        t => unimplemented!("Support for WasmType: {t}"),
    }
}

/// Align a value up to the given power-of-two-alignment.
// See https://sites.google.com/site/theoryofoperatingsystems/labs/malloc/align8
pub(crate) fn align_to<N>(value: N, alignment: N) -> N
where
    N: Not<Output = N>
        + BitAnd<N, Output = N>
        + Add<N, Output = N>
        + Sub<N, Output = N>
        + From<u8>
        + Copy,
{
    let alignment_mask = alignment - 1.into();
    (value + alignment_mask) & !alignment_mask
}

/// Calculates the delta needed to adjust a function's frame plus some
/// addend to a given alignment.
pub(crate) fn calculate_frame_adjustment(frame_size: u32, addend: u32, alignment: u32) -> u32 {
    let total = frame_size + addend;
    (alignment - (total % alignment)) % alignment
}
