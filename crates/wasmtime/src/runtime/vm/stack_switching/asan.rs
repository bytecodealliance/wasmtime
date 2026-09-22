//! AddressSanitizer integration for continuation stack switching.
//!
//! ASan maintains per-stack bookkeeping, including an opaque "fake
//! stack" used to detect stack-use-after-return. A stack switch must
//! bracket the actual switch with ASan's fiber switch handshake:
//!
//! ```text
//!       stack A                                      stack B
//!       -------                                      -------
//!       start_switch_fiber(&save_a, B's bounds)
//!       stack_switch(A -> B) ----------------------> resumes
//!                                                    finish_switch_fiber(save_b)
//!                                                    ... runs on B ...
//!                                                    start_switch_fiber(&save_b, A's bounds)
//!       resumes <----------------------------------- stack_switch(B -> A)
//!       finish_switch_fiber(save_a)
//! ```
//!
//! Here `save_a` and `save_b` are stack slots in the generated Wasm
//! frames.  The `start_switch_fiber` writes an opaque ASan fake-stack
//! token into the slot belonging to the stack being suspended. This
//! token is exclusively for ASan bookkeeping.
//!
//! A fresh continuation has no suspended generated frame at which to
//! execute the right-hand `finish_switch_fiber`. Its entry trampoline
//! therefore calls `fiber_start_complete` with a null fake-stack
//! token. ASan reports the old stack's bounds through that call, and
//! we record them in the parent's `VMCommonStackInformation` for
//! subsequent switches back to it.

#[cfg(asan)]
use crate::vm::{VMCommonStackInformation, VMContRef, VMStackChain, VmPtr};
#[cfg(asan)]
use core::ptr::NonNull;
#[cfg(asan)]
unsafe fn continuation_from_args(args: *mut crate::vm::VMHostArray) -> *mut VMContRef {
    unsafe {
        args.cast::<u8>()
            .byte_sub(core::mem::offset_of!(VMContRef, args))
            .cast()
    }
}

#[cfg(asan)]
unsafe fn parent_csi(contref: *mut VMContRef) -> *mut VMCommonStackInformation {
    let parent = unsafe { &mut (*contref).parent_chain };
    match parent {
        VMStackChain::InitialStack(csi) => csi.as_ptr(),
        VMStackChain::Continuation(contref) => unsafe {
            core::ptr::addr_of_mut!((*contref.as_ptr()).common_stack_information)
        },
        VMStackChain::Absent => panic!("a running continuation must have a parent stack"),
    }
}

#[cfg(asan)]
unsafe fn stack_range(csi: *const VMCommonStackInformation) -> (*const u8, usize) {
    let csi = unsafe { &*csi };
    let bottom = csi
        .asan_stack_bottom
        .expect("ASan requires the destination stack's bounds");
    (bottom.as_ptr(), csi.asan_stack_size)
}

/// Begins ASan's stack-switch handshake.
///
/// `fake_stack_save` points to storage in the generated Wasm frame
/// that survives until this same stack is resumed.
#[allow(dead_code, reason = "Used by ASan builds")]
pub unsafe extern "C" fn start_switch_fiber(fake_stack_save: *mut u8, target_csi: *mut u8) {
    #[cfg(asan)]
    unsafe {
        let (bottom, size) = stack_range(target_csi.cast());
        __sanitizer_start_switch_fiber(Some(&mut *fake_stack_save.cast()), bottom, size);
    }

    #[cfg(not(asan))]
    let _ = (fake_stack_save, target_csi);
}

/// Completes ASan's stack-switch handshake after this stack is
/// resumed.
#[allow(dead_code, reason = "Used by ASan builds")]
pub unsafe extern "C" fn finish_switch_fiber(fake_stack: *mut u8) {
    #[cfg(asan)]
    unsafe {
        let mut bottom = core::ptr::null();
        let mut size = 0;
        __sanitizer_finish_switch_fiber(fake_stack, &mut bottom, &mut size);
    }

    #[cfg(not(asan))]
    let _ = fake_stack;
}

/// Completes the first switch onto a newly-created continuation stack
/// and records the parent stack's bounds.
#[cfg_attr(asan, sanitize(address = "off"))]
#[cfg(all(feature = "stack-switching", asan))]
pub unsafe fn fiber_start_complete(args: *mut crate::vm::VMHostArray) {
    #[cfg(asan)]
    unsafe {
        let mut bottom = core::ptr::null();
        let mut size = 0;
        __sanitizer_finish_switch_fiber(core::ptr::null_mut(), &mut bottom, &mut size);

        let parent = &mut *parent_csi(continuation_from_args(args));
        parent.asan_stack_bottom = Some(VmPtr::from(
            NonNull::new(bottom.cast_mut()).expect("ASan must report the previous stack's bounds"),
        ));
        parent.asan_stack_size = size;
    }

    #[cfg(not(asan))]
    let _ = args;
}

/// Begins a non-returning switch from a completed or trapped
/// continuation to its parent stack.
#[cfg_attr(asan, sanitize(address = "off"))]
#[cfg(all(feature = "stack-switching", asan))]
pub unsafe extern "C" fn fiber_exit(args: *mut crate::vm::VMHostArray) {
    #[cfg(asan)]
    unsafe {
        let parent = parent_csi(continuation_from_args(args));
        let (bottom, size) = stack_range(parent);
        __sanitizer_start_switch_fiber(None, bottom, size);
    }

    #[cfg(not(asan))]
    let _ = args;
}

#[cfg(asan)]
unsafe extern "C" {
    fn __sanitizer_start_switch_fiber(
        fake_stack_save: Option<&mut *mut u8>,
        bottom: *const u8,
        size: usize,
    );
    fn __sanitizer_finish_switch_fiber(
        fake_stack: *mut u8,
        bottom_old: &mut *const u8,
        size_old: &mut usize,
    );
}
