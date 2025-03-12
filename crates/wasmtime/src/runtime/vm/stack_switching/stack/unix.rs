//! The stack layout is expected to look like so:
//!
//!
//! ```text
//! 0xB000 +-----------------------+   <- top of stack (TOS)
//!        | saved RIP             |
//! 0xAff8 +-----------------------+
//!        | saved RBP             |
//! 0xAff0 +-----------------------+
//!        | saved RSP             |
//! 0xAfe8 +-----------------------+   <- beginning of "control context",
//!        | args_capacity         |
//! 0xAfe0 +-----------------------+
//!        | args buffer, size:    |
//!        | (16 * args_capacity)  |
//! 0xAfc0 +-----------------------+   <- below: beginning of usable stack space
//!        |                       |      (16-byte aligned)
//!        |                       |
//!        ~        ...            ~   <- actual native stack space to use
//!        |                       |
//! 0x1000 +-----------------------+
//!        |  guard page           |   <- (not currently enabled)
//! 0x0000 +-----------------------+
//! ```
//!
//! The "control context" indicates how to resume a computation. The layout is
//! determined by Cranelift's stack_switch instruction, which reads and writes
//! these fields. The fields are used as follows, where we distinguish two
//! cases:
//!
//! 1.
//! If the continuation is currently active (i.e., running directly, or ancestor
//! of the running continuation), it stores the PC, RSP, and RBP of the *parent*
//! of the running continuation.
//!
//! 2.
//! If the picture shows a suspended computation, the fields store the PC, RSP,
//! and RBP at the time of the suspension.
//!
//! Note that this design ensures that external tools can construct backtraces
//! in the presence of stack switching by using frame pointers only: The
//! wasmtime_continuation_start trampoline uses the address of the RBP field in the
//! control context (0xAff0 above) as its frame pointer. This means that when
//! passing the wasmtime_continuation_start frame while doing frame pointer walking,
//! the parent of that frame is the last frame in the parent of this
//! continuation.
//!
//! Wasmtime's own mechanism for constructing backtraces also relies on frame
//! pointer chains. However, it understands continuations and does not rely on
//! the trickery outlined here to go from the frames in one continuation to the
//! parent.
//!
//! The args buffer is used as follows: It is used by the array calling
//! trampoline to read and store the arguments and return values of the function
//! running inside the continuation. If this function has m parameters and n
//! return values, then args_capacity is defined as max(m, n) and the size of
//! the args buffer is args_capacity * 16 bytes. The start address (0xAfc0 in
//! the example above, thus assuming args_capacity = 2) is saved as the `data`
//! field of the VMContRef's `args` object.

#![allow(unused_macros)]

use core::ptr::NonNull;
use std::io;
use std::ops::Range;
use std::ptr;

use crate::runtime::vm::stack_switching::VMArray;
use crate::runtime::vm::{VMContext, VMFuncRef, VMOpaqueContext, ValRaw};

#[derive(Debug, PartialEq, Eq)]
pub enum Allocator {
    Mmap,
    Custom,
}

#[derive(Debug)]
#[repr(C)]
pub struct VMContinuationStack {
    // The top of the stack; for stacks allocated by the fiber implementation itself,
    // the base address of the allocation will be `top.sub(len.unwrap())`
    top: *mut u8,
    // The length of the stack
    len: usize,
    // allocation strategy
    allocator: Allocator,
}

impl VMContinuationStack {
    pub fn new(size: usize) -> io::Result<Self> {
        // Round up our stack size request to the nearest multiple of the
        // page size.
        let page_size = rustix::param::page_size();
        let size = if size == 0 {
            page_size
        } else {
            (size + (page_size - 1)) & (!(page_size - 1))
        };

        unsafe {
            // Add in one page for a guard page and then ask for some memory.
            let mmap_len = size + page_size;
            let mmap = rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                mmap_len,
                rustix::mm::ProtFlags::empty(),
                rustix::mm::MapFlags::PRIVATE,
            )?;

            rustix::mm::mprotect(
                mmap.cast::<u8>().add(page_size).cast(),
                size,
                rustix::mm::MprotectFlags::READ | rustix::mm::MprotectFlags::WRITE,
            )?;

            Ok(Self {
                top: mmap.cast::<u8>().add(mmap_len),
                len: mmap_len,
                allocator: Allocator::Mmap,
            })
        }
    }

    pub fn unallocated() -> Self {
        Self {
            top: std::ptr::null_mut(),
            len: 0,
            allocator: Allocator::Custom,
        }
    }

    pub fn is_unallocated(&self) -> bool {
        debug_assert_eq!(self.len == 0, self.top == std::ptr::null_mut());
        self.len == 0
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn from_raw_parts(
        base: *mut u8,
        _guard_size: usize,
        len: usize,
    ) -> io::Result<Self> {
        Ok(Self {
            top: base.add(len),
            len,
            allocator: Allocator::Custom,
        })
    }

    pub fn is_from_raw_parts(&self) -> bool {
        self.allocator == Allocator::Custom
    }

    pub fn top(&self) -> Option<*mut u8> {
        Some(self.top)
    }

    pub fn range(&self) -> Option<Range<usize>> {
        let base = unsafe { self.top.sub(self.len) as usize };
        Some(base..base + self.len)
    }

    pub fn control_context_instruction_pointer(&self) -> usize {
        // See picture at top of this file:
        // RIP is stored 8 bytes below top of stack.
        unsafe {
            let ptr = self.top.sub(8) as *mut usize;
            *ptr
        }
    }

    pub fn control_context_frame_pointer(&self) -> usize {
        // See picture at top of this file:
        // RBP is stored 16 bytes below top of stack.
        unsafe {
            let ptr = self.top.sub(16) as *mut usize;
            *ptr
        }
    }

    pub fn control_context_stack_pointer(&self) -> usize {
        // See picture at top of this file:
        // RSP is stored 24 bytes below top of stack.
        unsafe {
            let ptr = self.top.sub(24) as *mut usize;
            *ptr
        }
    }

    /// This function installs the launchpad for the computation to run on the
    /// fiber, such that executing a `stack_switch` instruction on the stack
    /// actually runs the desired computation.
    ///
    /// Concretely, switching to the stack prepared by this function
    /// causes that we enter `wasmtime_continuation_start`, which then in turn
    /// calls `fiber_start` with  the following arguments:
    /// TOS, func_ref, caller_vmctx, args_ptr, args_capacity
    ///
    /// Note that at this point we also allocate the args buffer
    /// (see picture at the top of this file).
    /// We define `args_capacity` as the max of parameter and return value count.
    /// Then the size s of the actual buffer size is calculated as follows:
    /// s = size_of(ValRaw) * `args_capacity`,
    ///
    /// Note that this value is used below, and we may have s = 0.
    ///
    /// The layout of the VMContinuationStack near the top of stack (TOS)
    /// *after* running this function is as follows:
    ///
    ///
    ///  Offset from    |
    ///       TOS       | Contents
    ///  ---------------|-------------------------------------------------------
    ///       -0x08     | address of wasmtime_continuation_start function (future PC)
    ///       -0x10     | TOS - 0x10 (future RBP)
    ///       -0x18     | TOS - 0x40 - s (future RSP)
    ///       -0x20     | args_capacity
    ///
    ///
    /// The data stored behind the args buffer is as follows:
    ///
    ///  Offset from    |
    ///       TOS       | Contents
    ///  ---------------|-------------------------------------------------------
    ///       -0x28 - s | func_ref
    ///       -0x30 - s | caller_vmctx
    ///       -0x38 - s | args (of type *mut ArrayRef<ValRaw>)
    ///       -0x40 - s | return_value_count
    pub fn initialize(
        &self,
        func_ref: *const VMFuncRef,
        caller_vmctx: *mut VMContext,
        args: *mut VMArray<ValRaw>,
        parameter_count: u32,
        return_value_count: u32,
    ) {
        let tos = self.top;

        unsafe {
            let store = |tos_neg_offset, value| {
                let target = tos.sub(tos_neg_offset) as *mut usize;
                target.write(value)
            };

            let args = &mut *args;
            let args_capacity = std::cmp::max(parameter_count, return_value_count);
            // The args object must currently be empty.
            debug_assert_eq!(args.capacity, 0);
            debug_assert_eq!(args.length, 0);

            // The size of the args buffer
            let s = args_capacity as usize * std::mem::size_of::<ValRaw>();

            // The actual pointer to the buffer
            let args_data_ptr = if args_capacity == 0 {
                0
            } else {
                tos as usize - 0x20 - s
            };

            args.capacity = args_capacity;
            args.data = args_data_ptr as *mut ValRaw;

            let to_store = [
                // Data near top of stack:
                (0x08, wasmtime_continuation_start as usize),
                (0x10, tos.sub(0x10) as usize),
                (0x18, tos.sub(0x40 + s) as usize),
                (0x20, args_capacity as usize),
                // Data after the args buffer:
                (0x28 + s, func_ref as usize),
                (0x30 + s, caller_vmctx as usize),
                (0x38 + s, args as *mut VMArray<ValRaw> as usize),
                (0x40 + s, return_value_count as usize),
            ];

            for (offset, data) in to_store {
                store(offset, data);
            }
        }
    }
}

impl Drop for VMContinuationStack {
    fn drop(&mut self) {
        unsafe {
            match self.allocator {
                Allocator::Mmap => {
                    let ret = rustix::mm::munmap(self.top.sub(self.len) as _, self.len);
                    debug_assert!(ret.is_ok());
                }
                Allocator::Custom => {} // It's the creator's responsibility to reclaim the memory.
            }
        }
    }
}

unsafe extern "C" {
    #[allow(dead_code)] // only used in inline assembly for some platforms
    fn wasmtime_continuation_start();
}

/// This function is responsible for actually running a wasm function inside a
/// continuation. It is only ever called from `wasmtime_continuation_start`. Hence, it
/// must never return.
unsafe extern "C" fn fiber_start(
    func_ref: *const VMFuncRef,
    caller_vmctx: *mut VMContext,
    args: *mut VMArray<ValRaw>,
    return_value_count: u32,
) {
    unsafe {
        let func_ref = func_ref.as_ref().expect("Non-null function reference");
        let caller_vmxtx = VMOpaqueContext::from_vmcontext(NonNull::new_unchecked(caller_vmctx));
        let args = &mut *args;
        let params_and_returns: NonNull<[ValRaw]> = if args.capacity == 0 {
            NonNull::from(&[])
        } else {
            std::slice::from_raw_parts_mut(args.data, args.capacity as usize).into()
        };

        // NOTE(frank-emrich) The usage of the `caller_vmctx` is probably not
        // 100% correct here. Currently, we determine the "caller" vmctx when
        // initilizing the fiber stack/continuation (i.e. as part of
        // `cont.new`). However, we may subsequenly `resume` the continuation
        // from a different Wasm instance. The way to fix this would be to make
        // the currently active `VMContext` an additional parameter of
        // `wasmtime_continuation_switch` and pipe it through to this point. However,
        // since the caller vmctx is only really used to access stuff in the
        // underlying `Store`, it's fine to be slightly sloppy about the exact
        // value we set.
        func_ref.array_call(None, caller_vmxtx, params_and_returns); // TODO(dhil): we are ignoring the boolean return value
                                                                     // here... we probably shouldn't.

        // The array call trampoline should have just written
        // `return_value_count` values to the `args` buffer. Let's reflect that
        // in its length field, to make various bounds checks happy.
        args.length = return_value_count;

        // Note that after this function returns, wasmtime_continuation_start
        // will switch back to the parent stack.
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
    } else {
        // Note that this shoul be unreachable: In stack.rs, we currently select
        // the module defined in the current file only if we are on unix AND
        // x86_64.
        compile_error!("the stack switching feature is not supported on this CPU architecture");
    }
}
