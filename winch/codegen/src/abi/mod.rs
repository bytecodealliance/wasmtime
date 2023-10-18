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

/// Internal classification for params or returns,
/// mainly used for params and return register assignment.
#[derive(Clone, Copy)]
pub(super) enum ParamsOrReturns {
    Params,
    Returns,
}

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

    /// Construct [`ABIResults`] from a slice of [`WasmType`].
    fn abi_results(returns: &[WasmType], call_conv: &CallingConvention) -> ABIResults;

    /// Returns the number of bits in a word.
    fn word_bits() -> u32;

    /// Returns the number of bytes in a word.
    fn word_bytes() -> u32 {
        Self::word_bits() / 8
    }

    /// Returns the designated general purpose scratch register.
    fn scratch_reg() -> Reg;

    /// Returns the designated floating point scratch register.
    fn float_scratch_reg() -> Reg;

    /// Returns the designated scratch register for the given [WasmType].
    fn scratch_for(ty: &WasmType) -> Reg {
        match ty {
            WasmType::I32 | WasmType::I64 => Self::scratch_reg(),
            WasmType::F32 | WasmType::F64 => Self::float_scratch_reg(),
            _ => unimplemented!(),
        }
    }

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

    /// The size, in bytes, of each stack slot used for stack parameter passing.
    fn stack_slot_size() -> u32;

    /// Returns the size in bytes of the given [`WasmType`].
    fn sizeof(ty: &WasmType) -> u32 {
        match ty {
            WasmType::Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => Self::word_bytes(),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
            WasmType::F64 | WasmType::I64 => Self::word_bytes(),
            WasmType::F32 | WasmType::I32 => Self::word_bytes() / 2,
            ty => unimplemented!("Support for WasmType: {ty}"),
        }
    }
}

/// ABI-specific representation of function argument or result.
#[derive(Clone, Debug)]
pub enum ABIOperand {
    /// A register [`ABIOperand`].
    Reg {
        /// The type of the [`ABIOperand`].
        ty: WasmType,
        /// Register holding the [`ABIOperand`].
        reg: Reg,
        /// The size of the [`ABIOperand`], in bytes.
        size: u32,
    },
    /// A stack [`ABIOperand`].
    Stack {
        /// The type of the [`ABIOperand`].
        ty: WasmType,
        /// Offset of the operand referenced through FP by the callee and
        /// through SP by the caller.
        offset: u32,
        /// The size of the [`ABIOperand`], in bytes.
        size: u32,
    },
}

impl ABIOperand {
    /// Allocate a new register [`ABIOperand`].
    pub fn reg(reg: Reg, ty: WasmType, size: u32) -> Self {
        Self::Reg { reg, ty, size }
    }

    /// Allocate a new stack [`ABIOperand`].
    pub fn stack_offset(offset: u32, ty: WasmType, size: u32) -> Self {
        Self::Stack { ty, offset, size }
    }

    /// Is this [`ABIOperand`] in a register.
    pub fn is_reg(&self) -> bool {
        match *self {
            ABIOperand::Reg { .. } => true,
            _ => false,
        }
    }

    /// Unwraps the underlying register if it is one.
    ///
    /// # Panics
    /// This function panics if the [`ABIOperand`] is not a register.
    pub fn unwrap_reg(&self) -> Reg {
        match self {
            ABIOperand::Reg { reg, .. } => *reg,
            _ => unreachable!(),
        }
    }

    /// Get the register associated to this [`ABIOperand`].
    pub fn get_reg(&self) -> Option<Reg> {
        match *self {
            ABIOperand::Reg { reg, .. } => Some(reg),
            _ => None,
        }
    }

    /// Get the type associated to this [`ABIOperand`].
    pub fn ty(&self) -> WasmType {
        match *self {
            ABIOperand::Reg { ty, .. } | ABIOperand::Stack { ty, .. } => ty,
        }
    }
}

/// Information about the [`ABIOperand`] information used in [`ABISig`].
#[derive(Clone, Debug)]
pub(crate) struct ABIOperands {
    /// All the operands.
    pub inner: SmallVec<[ABIOperand; 6]>,
    /// All the registers used as operands.
    pub regs: HashSet<Reg>,
    /// Stack bytes used by the operands.
    pub bytes: u32,
}

impl Default for ABIOperands {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            regs: HashSet::with_capacity(0),
            bytes: 0,
        }
    }
}

/// ABI-specific representation of an [`ABISig`].
#[derive(Clone, Debug, Default)]
pub(crate) struct ABIResults {
    /// The result operands.
    operands: ABIOperands,
}

impl ABIResults {
    /// Creates [`ABIResults`] from a slice of `WasmType`.
    pub fn from<F>(returns: &[WasmType], mut map: F) -> Self
    where
        F: FnMut(&WasmType, u32) -> (ABIOperand, u32),
    {
        if returns.len() == 0 {
            return Self::default();
        }

        let register_capacity = returns.len().min(2);
        let (operands, regs, bytes): (SmallVec<[ABIOperand; 6]>, HashSet<Reg>, u32) =
            returns.iter().fold(
                (
                    SmallVec::new(),
                    HashSet::with_capacity(register_capacity),
                    0,
                ),
                |(mut operands, mut regs, stack_bytes), arg| {
                    let (operand, bytes) = map(arg, stack_bytes);
                    if operand.is_reg() {
                        regs.insert(operand.unwrap_reg());
                    }
                    operands.push(operand);
                    (operands, regs, bytes)
                },
            );

        Self::new(ABIOperands {
            inner: operands,
            regs,
            bytes,
        })
    }

    /// Create a new [`ABIResults`] from [`ABIOperands`].
    pub fn new(operands: ABIOperands) -> Self {
        Self { operands }
    }

    /// Returns an iterator over the result registers.
    pub fn regs(&self) -> &HashSet<Reg> {
        &self.operands.regs
    }

    /// Get a slice over all the result [`ABIOperand`]s.
    pub fn operands(&self) -> &[ABIOperand] {
        &self.operands.inner
    }

    /// Returns the length of the result.
    pub fn len(&self) -> usize {
        self.operands.inner.len()
    }

    /// Get the [`ABIOperand`] result in the nth position.
    #[cfg(test)]
    pub fn get(&self, n: usize) -> Option<&ABIOperand> {
        self.operands.inner.get(n)
    }

    /// Returns the first [`ABIOperand`].
    /// Useful in situations where the function signature is known to
    /// have a single return.
    ///
    /// # Panics
    /// This function panics if the function signature contains more
    pub fn unwrap_singleton(&self) -> &ABIOperand {
        assert!(self.len() == 1);
        &self.operands.inner[0]
    }
}

/// ABI-specific representation of an [`ABISig`].
#[derive(Debug, Clone, Default)]
pub(crate) struct ABIParams {
    /// The param operands.
    operands: ABIOperands,
}

impl ABIParams {
    /// Creates [`ABIParams`] from a slice of `WasmType`.
    pub fn from<F>(params: &[WasmType], initial_bytes: u32, mut map: F) -> Self
    where
        F: FnMut(&WasmType, u32) -> (ABIOperand, u32),
    {
        if params.len() == 0 {
            return Self::with_bytes(initial_bytes);
        }

        let regiser_capacity = params.len().min(6);
        let (operands, regs, stack_bytes): (SmallVec<[ABIOperand; 6]>, HashSet<Reg>, u32) =
            params.iter().fold(
                (
                    SmallVec::new(),
                    HashSet::with_capacity(regiser_capacity),
                    initial_bytes,
                ),
                |(mut operands, mut regs, stack_bytes), arg| {
                    let (operand, bytes) = map(arg, stack_bytes);
                    if operand.is_reg() {
                        regs.insert(operand.unwrap_reg());
                    }
                    operands.push(operand);
                    (operands, regs, bytes)
                },
            );

        Self::new(ABIOperands {
            inner: operands,
            regs,
            bytes: stack_bytes,
        })
    }

    /// Creates new [`ABIParams`], with the specified amount of stack bytes.
    pub fn with_bytes(bytes: u32) -> Self {
        let mut params = Self::default();
        params.operands.bytes = bytes;
        params
    }

    /// Create [`ABIParams`] from [`ABIOperands`].
    pub fn new(operands: ABIOperands) -> Self {
        Self { operands }
    }

    /// Get the [`ABIOperand`] param in the nth position.
    pub fn get(&self, n: usize) -> Option<&ABIOperand> {
        self.operands.inner.get(n)
    }

    /// Get a slice over all the parameter [`ABIOperand`]s.
    pub fn operands(&self) -> &[ABIOperand] {
        &self.operands.inner
    }

    /// Returns the length of the params.
    pub fn len(&self) -> usize {
        self.operands.inner.len()
    }
}

/// An ABI-specific representation of a function signature.
#[derive(Debug, Clone)]
pub(crate) struct ABISig {
    /// Function parameters.
    pub params: ABIParams,
    /// Function result.
    pub results: ABIResults,
    /// A unique set of registers used in the entire [`ABISig`].
    pub regs: HashSet<Reg>,
}

impl ABISig {
    /// Create a new ABI signature.
    pub fn new(params: ABIParams, results: ABIResults) -> Self {
        let regs = params
            .operands
            .regs
            .union(&results.operands.regs)
            .copied()
            .collect();
        Self {
            params,
            results,
            regs,
        }
    }

    /// Returns an iterator over all the parameter operands.
    pub fn params(&self) -> &[ABIOperand] {
        self.params.operands()
    }

    /// Returns an iterator over all the result operands.
    pub fn results(&self) -> &[ABIOperand] {
        self.results.operands()
    }

    /// Returns the stack size, in bytes, needed for arguments on the stack.
    pub fn params_stack_size(&self) -> u32 {
        self.params.operands.bytes
    }

    /// Returns the stack size, in bytes, needed for results on the stack.
    #[allow(dead_code)] // Until multi-value is supported.
    pub fn results_stack_size(&self) -> u32 {
        self.results.operands.bytes
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
