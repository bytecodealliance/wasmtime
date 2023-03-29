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
//! |        + dynamic space        |
//! |                               |
//! |                               |
//! |                               |
//! +-------------------------------+----> SP @ callsite (after)
//! |        alignment              |
//! |        + arguments            |
//! |                               | ----> Space allocated for calls
//! |                               |
use crate::isa::reg::Reg;
use smallvec::SmallVec;
use std::ops::{Add, BitAnd, Not, Sub};
use wasmparser::{FuncType, ValType};

pub(crate) mod local;
pub(crate) use local::*;

/// Trait implemented by a specific ISA and used to provide
/// information about alignment, parameter passing, usage of
/// specific registers, etc.
pub(crate) trait ABI {
    /// The required stack alignment.
    fn stack_align(&self) -> u8;

    /// The required stack alignment for calls.
    fn call_stack_align(&self) -> u8;

    /// The offset to the argument base, relative to the frame pointer.
    fn arg_base_offset(&self) -> u8;

    /// Construct the ABI-specific signature from a WebAssembly
    /// function type.
    fn sig(&self, wasm_sig: &FuncType) -> ABISig;

    /// Returns the number of bits in a word.
    fn word_bits() -> u32;

    /// Returns the number of bytes in a word.
    fn word_bytes() -> u32 {
        Self::word_bits() / 8
    }

    /// Returns the designated scratch register.
    fn scratch_reg() -> Reg;
}

/// ABI-specific representation of a function argument.
#[derive(Debug)]
pub(crate) enum ABIArg {
    /// A register argument.
    Reg {
        /// Type of the argument.
        ty: ValType,
        /// Register holding the argument.
        reg: Reg,
    },
    /// A stack argument.
    Stack {
        /// The type of the argument.
        ty: ValType,
        /// Offset of the argument relative to the frame pointer.
        offset: u32,
    },
}

impl ABIArg {
    /// Allocate a new register abi arg.
    pub fn reg(reg: Reg, ty: ValType) -> Self {
        Self::Reg { reg, ty }
    }

    /// Allocate a new stack abi arg.
    pub fn stack_offset(offset: u32, ty: ValType) -> Self {
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
    pub fn ty(&self) -> ValType {
        match *self {
            ABIArg::Reg { ty, .. } | ABIArg::Stack { ty, .. } => ty,
        }
    }
}

/// ABI-specific representation of the function result.
pub(crate) enum ABIResult {
    Reg {
        /// Type of the result.
        ty: Option<ValType>,
        /// Register to hold the result.
        reg: Reg,
    },
}

impl ABIResult {
    /// Create a register ABI result.
    pub fn reg(ty: Option<ValType>, reg: Reg) -> Self {
        Self::Reg { ty, reg }
    }

    /// Get the result reg.
    pub fn result_reg(&self) -> Reg {
        match self {
            Self::Reg { reg, .. } => *reg,
        }
    }

    /// Checks if the result is void.
    pub fn is_void(&self) -> bool {
        match self {
            Self::Reg { ty, .. } => ty.is_none(),
        }
    }
}

pub(crate) type ABIParams = SmallVec<[ABIArg; 6]>;

/// An ABI-specific representation of a function signature.
pub(crate) struct ABISig {
    /// Function parameters.
    pub params: ABIParams,
    /// Function result.
    pub result: ABIResult,
    /// Stack space needed for stack arguments.
    pub stack_bytes: u32,
}

impl ABISig {
    /// Create a new ABI signature.
    pub fn new(params: ABIParams, result: ABIResult, stack_bytes: u32) -> Self {
        Self {
            params,
            result,
            stack_bytes,
        }
    }
}

/// Returns the size in bytes of a given WebAssembly type.
pub(crate) fn ty_size(ty: &ValType) -> u32 {
    match *ty {
        ValType::I32 | ValType::F32 => 4,
        ValType::I64 | ValType::F64 => 8,
        _ => panic!(),
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
