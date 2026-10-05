//! AddressSanitizer integration for continuation stack switching.
//!
//! ASan maintains per-stack bookkeeping, including an opaque "fake
//! stack" used to detect stack-use-after-return. A stack switch must
//! bracket the actual switch with ASan's fiber switch handshake:
//!
//! ```text
//!       stack A                                      stack B
//!       -------                                      -------
//!       start_switch_fiber(&save_a, A's CSI, B's bounds)
//!       stack_switch(A -> B) ----------------------> resumes
//!                                                    finish_switch_fiber(save_b)
//!                                                      records A's bounds
//!                                                    ... runs on B ...
//!                                                    start_switch_fiber(
//!                                                      &save_b,
//!                                                      B's CSI,
//!                                                      A's bounds)
//!       resumes <----------------------------------- stack_switch(B -> A)
//!       finish_switch_fiber(save_a)
//!         records B's bounds
//! ```
//!
//! Here `save_a` and `save_b` are stack slots in the generated Wasm
//! frames.  The `start_switch_fiber` writes an opaque ASan fake-stack
//! token into the slot belonging to the stack being suspended. This
//! token is exclusively for ASan bookkeeping. We separately remember
//! the suspended stack's `VMCommonStackInformation` until the matching
//! `finish_switch_fiber`, which reports that stack's bounds.
//!
//! A fresh continuation has no suspended generated frame at which to
//! execute the right-hand `finish_switch_fiber`. Its entry trampoline
//! therefore calls `fiber_start_complete` with a null fake-stack
//! token. ASan reports the old stack's bounds through that call, and
//! we record them in the parent's `VMCommonStackInformation` for
//! subsequent switches back to it.

#[cfg(asan)]
mod enabled {

    use crate::vm::{VMCommonStackInformation, VMContRef, VMStackChain, VmPtr};
    use core::cell::Cell;
    use core::ptr::NonNull;

    std::thread_local! {
        /// The stack whose bounds ASan will report at the next matching
        /// `__sanitizer_finish_switch_fiber` call.
        static PENDING_SOURCE_CSI: Cell<*mut VMCommonStackInformation> =
            const { Cell::new(core::ptr::null_mut()) };
    }

    unsafe fn continuation_from_args(args: *mut crate::vm::VMHostArray) -> *mut VMContRef {
        unsafe {
            args.cast::<u8>()
                .byte_sub(core::mem::offset_of!(VMContRef, args))
                .cast()
        }
    }

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

    unsafe fn stack_range(csi: *const VMCommonStackInformation) -> (*const u8, usize) {
        let csi = unsafe { &*csi };
        let bottom = csi
            .asan_stack_bottom
            .expect("ASan requires the destination stack's bounds");
        (bottom.as_ptr(), csi.asan_stack_size)
    }

    #[cfg_attr(asan, sanitize(address = "off"))]
    unsafe fn begin_switch(
        fake_stack_save: Option<&mut *mut u8>,
        bottom: *const u8,
        size: usize,
        source_csi: *mut VMCommonStackInformation,
    ) {
        assert!(!source_csi.is_null());
        PENDING_SOURCE_CSI.with(|pending| {
            let previous = pending.replace(source_csi);
            assert!(
                previous.is_null(),
                "ASan stack-switch handshakes must not overlap"
            );
        });
        unsafe { __sanitizer_start_switch_fiber(fake_stack_save, bottom, size) };
    }

    #[cfg_attr(asan, sanitize(address = "off"))]
    unsafe fn finish_switch(fake_stack: *mut u8) {
        unsafe {
            let mut bottom = core::ptr::null();
            let mut size = 0;
            __sanitizer_finish_switch_fiber(fake_stack, &mut bottom, &mut size);

            let source_csi = PENDING_SOURCE_CSI.with(|pending| {
                let source_csi = pending.get();
                pending.set(core::ptr::null_mut());
                source_csi
            });
            let source_csi = &mut *NonNull::new(source_csi)
                .expect("ASan stack-switch completion requires a source stack")
                .as_ptr();
            source_csi.asan_stack_bottom = Some(VmPtr::from(
                NonNull::new(bottom.cast_mut())
                    .expect("ASan must report the previous stack's bounds"),
            ));
            source_csi.asan_stack_size = size;
        }
    }

    /// Begins ASan's stack-switch handshake.
    ///
    /// `fake_stack_save` points to storage in the generated Wasm frame
    /// that survives until this same stack is resumed.
    pub unsafe extern "C" fn start_switch_fiber(
        fake_stack_save: *mut u8,
        source_csi: *mut u8,
        target_csi: *mut u8,
    ) {
        unsafe {
            let (bottom, size) = stack_range(target_csi.cast());
            begin_switch(
                Some(&mut *fake_stack_save.cast()),
                bottom,
                size,
                source_csi.cast(),
            );
        }
    }

    /// Completes ASan's stack-switch handshake after this stack is
    /// resumed.
    pub unsafe extern "C" fn finish_switch_fiber(fake_stack: *mut u8) {
        unsafe { finish_switch(fake_stack) }
    }

    /// Completes the first switch onto a newly-created continuation stack
    /// and records the parent stack's bounds.
    #[cfg_attr(asan, sanitize(address = "off"))]
    #[cfg(all(feature = "stack-switching"))]
    pub unsafe fn fiber_start_complete(_args: *mut crate::vm::VMHostArray) {
        unsafe { finish_switch(core::ptr::null_mut()) }
    }

    /// Begins a non-returning switch from a completed or trapped
    /// continuation to its parent stack.
    #[cfg_attr(asan, sanitize(address = "off"))]
    #[cfg(all(feature = "stack-switching"))]
    pub unsafe extern "C" fn fiber_exit(args: *mut crate::vm::VMHostArray) {
        unsafe {
            let contref = continuation_from_args(args);
            let parent = parent_csi(contref);
            let (bottom, size) = stack_range(parent);
            let source = core::ptr::addr_of_mut!((*contref).common_stack_information);
            begin_switch(None, bottom, size, source);
        }
    }

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
}

#[cfg(not(asan))]
mod disabled {
    #[allow(dead_code, reason = "Used by ASan builds")]
    pub unsafe extern "C" fn start_switch_fiber(
        _fake_stack_save: *mut u8,
        _source_csi: *mut u8,
        _target_csi: *mut u8,
    ) {
    }

    /// Completes ASan's stack-switch handshake after this stack is
    /// resumed.
    #[allow(dead_code, reason = "Used by ASan builds")]
    pub unsafe extern "C" fn finish_switch_fiber(_fake_stack: *mut u8) {}
}

#[cfg(not(asan))]
#[allow(unused_imports, reason = "Used by ASan builds")]
pub use disabled::*;
#[cfg(asan)]
pub use enabled::*;
