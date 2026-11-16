//! This module contains a modified version of the `wasmtime_fiber` crate,
//! specialized for executing stack switching continuations.

use crate::Result;
#[cfg(feature = "stack-switching")]
use crate::prelude::*;
use core::ops::Range;

use crate::runtime::vm::VMPayloads;
#[cfg(feature = "stack-switching")]
use crate::runtime::vm::ValRaw;
#[cfg(all(feature = "stack-switching", has_virtual_memory))]
use crate::runtime::vm::host_page_size;
use crate::runtime::vm::{VMContext, VMFuncRef};

/// Calculate the native stack space needed to reach the
/// continuation's entry function.
///
/// On x86_64, the array-to-Wasm trampoline needs at most one
/// `ValRaw`-sized spill slot per live parameter or result and one
/// dynamically allocated outgoing slot per parameter for a core Wasm
/// entry continuation function. We retain one host page for the
/// continuation entry stub, Rust's `fiber_start` frame, and call
/// setup.
///
/// NOTE(dhil): It may be more robust to add stack frame metadata
/// directly to compiled artifacts, however, since stack-switching has
/// status of an experimental feature we deliberately opt for deriving
/// the bound based on the signature as it is non-invasive.
#[cfg(feature = "stack-switching")]
pub(crate) fn continuation_runtime_headroom(
    parameter_count: u32,
    return_value_count: u32,
) -> Result<usize> {
    let fixed_headroom = if cfg!(has_virtual_memory) {
        host_page_size()
    } else {
        // Targets without virtual memory use the dummy continuation
        // stack implementation, so this value is never used to
        // initialize a stack.
        0usize
    };

    let slot_size = core::mem::size_of::<ValRaw>();
    let trampoline_values_size = usize::try_from(parameter_count.max(return_value_count))?
        .checked_mul(slot_size)
        .ok_or_else(|| format_err!("continuation trampoline stack size overflow"))?;
    let outgoing_args_size = usize::try_from(parameter_count)?
        .checked_mul(slot_size)
        .ok_or_else(|| format_err!("continuation parameter stack size overflow"))?;
    let headroom = fixed_headroom
        .checked_add(trampoline_values_size)
        .and_then(|n| n.checked_add(outgoing_args_size))
        .ok_or_else(|| format_err!("continuation runtime headroom overflow"))?;
    Ok(headroom)
}

cfg_select! {
    all(feature = "stack-switching", unix, target_arch = "x86_64") => {
        mod unix;
        use unix as imp;
    }
    _ => {
        mod dummy;
        use dummy as imp;
    }
}

/// Represents an execution stack to use for a fiber.
#[derive(Debug)]
#[repr(C)]
pub struct VMContinuationStack(imp::VMContinuationStack);

impl VMContinuationStack {
    /// Creates a new fiber stack of the given size.
    pub fn new(size: usize) -> Result<Self> {
        Ok(Self(imp::VMContinuationStack::new(size)?))
    }

    /// Returns a stack of size 0.
    pub fn unallocated() -> Self {
        Self(imp::VMContinuationStack::unallocated())
    }

    /// Is this stack unallocated/of size 0?
    pub fn is_unallocated(&self) -> bool {
        imp::VMContinuationStack::is_unallocated(&self.0)
    }

    /// Creates a new fiber stack with the given pointer to the bottom of the
    /// stack plus the byte length of the stack.
    ///
    /// The `bottom` pointer should be addressable for `len` bytes. The page
    /// beneath `bottom` should be unmapped as a guard page.
    ///
    /// # Safety
    ///
    /// This is unsafe because there is no validation of the given pointer.
    ///
    /// The caller must properly allocate the stack space with a guard page and
    /// make the pages accessible for correct behavior.
    pub unsafe fn from_raw_parts(bottom: *mut u8, guard_size: usize, len: usize) -> Result<Self> {
        Ok(Self(unsafe {
            imp::VMContinuationStack::from_raw_parts(bottom, guard_size, len)?
        }))
    }

    /// Is this a manually-managed stack created from raw parts? If so, it is up
    /// to whoever created it to manage the stack's memory allocation.
    pub fn is_from_raw_parts(&self) -> bool {
        self.0.is_from_raw_parts()
    }

    /// Gets the top of the stack.
    ///
    /// Returns `None` if the platform does not support getting the top of the
    /// stack.
    pub fn top(&self) -> Option<*mut u8> {
        self.0.top()
    }

    /// Returns the range of where this stack resides in memory if the platform
    /// supports it.
    pub fn range(&self) -> Option<Range<usize>> {
        self.0.range()
    }

    /// Returns the usable stack range to report to AddressSanitizer. Unlike
    /// [`Self::range`], this excludes any inaccessible guard region.
    #[cfg(all(feature = "stack-switching", asan))]
    pub(crate) fn asan_range(&self) -> Option<Range<usize>> {
        self.0.asan_range()
    }

    /// Returns the instruction pointer stored in the Fiber's ControlContext.
    pub fn control_context_instruction_pointer(&self) -> usize {
        self.0.control_context_instruction_pointer()
    }

    /// Returns the frame pointer stored in the Fiber's ControlContext.
    pub fn control_context_frame_pointer(&self) -> usize {
        self.0.control_context_frame_pointer()
    }

    /// Returns the stack pointer stored in the Fiber's ControlContext.
    pub fn control_context_stack_pointer(&self) -> usize {
        self.0.control_context_stack_pointer()
    }

    /// Initializes this stack, such that it will execute the function denoted
    /// by `func_ref`. `parameter_count` and `return_value_count` must be the
    /// corresponding number of parameters and return values of `func_ref`.
    /// `args` must point to the `args` field of the `VMContRef` owning this
    /// stack. When `gc_refs` is true, its `gc_ref_data` field receives the
    /// corresponding root-marker buffer.
    ///
    /// It will be updated by this function to correctly describe
    /// the buffer used by this function for its arguments and return values.
    pub fn initialize(
        &self,
        func_ref: *const VMFuncRef,
        caller_vmctx: *mut VMContext,
        args: *mut VMPayloads,
        parameter_count: u32,
        return_value_count: u32,
        gc_refs: bool,
        runtime_headroom: usize,
    ) -> Result<()> {
        self.0.initialize(
            func_ref,
            caller_vmctx,
            args,
            parameter_count,
            return_value_count,
            gc_refs,
            runtime_headroom,
        )
    }
}
