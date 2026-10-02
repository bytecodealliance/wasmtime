//! Raw fibers on top of Windows fibers.
//!
//! `SwitchToFiber` saves a fiber's stack pointer and nonvolatile registers in
//! the fiber's undocumented, heap-allocated `FIBER` structure rather than on
//! its stack, so the stack bytes alone do not capture where to resume a
//! suspended fiber. Instead every suspension goes through one canonical frame:
//! the suspend routine spills all nonvolatile registers onto the fiber's
//! stack, stores the stack pointer in a reserved slot, and calls
//! `SwitchToFiber` with a fixed stack pointer near the top of the stack. What
//! Windows saves is thus the same at every suspension, and when
//! `SwitchToFiber` returns the suspend routine takes everything else from the
//! slot and the stack. A raw fiber's snapshot therefore captures it completely,
//! just as on other platforms, while Windows keeps managing everything else
//! about the fiber: its stack bounds, fiber-local storage, and so on.
//!
//! The start routine suspends through the same frame once before calling the
//! entry point, so a new raw fiber is a suspended fiber whose continuation
//! calls its entry point.
//!
//! The top of a raw fiber's stack, where `top` is its `switch_arg`, is:
//!
//! ```text
//!                 frames of `fiber_start` and Windows' own fiber start (fixed)
//! top + 8         return address into `fiber_start`
//! top             frame pointer of `fiber_start`
//! top - 0x08      host fiber to suspend to, stored by each resume
//! top - 0x10      stack pointer to resume this fiber at
//!                 frame from which the suspend routine calls `SwitchToFiber`
//! top - SCRATCH   start of the entry point's stack
//! ```
//!
//! A raw fiber's `FiberStack` reports its range as the bottom of the stack's
//! reservation up to `top`, which is also what it reports as the stack's top.

use crate::RawFiberEntry;
use crate::windows::{
    FIBER_FLAG_FLOAT_SWITCH, delete_fiber, set_stack_guarantee, with_current_fiber,
};
use std::ffi::c_void;
use std::io;
use std::ptr;
use wasmtime_environ::prelude::*;
use windows_sys::Win32::System::Threading::*;

cfg_select! {
    target_arch = "x86_64" => {
        mod x86_64;
        use x86_64 as asm;
    }
    target_arch = "aarch64" => {
        mod aarch64;
        use aarch64 as asm;
    }
    _ => {
        mod asm {
            pub const SUPPORTED_ARCH: bool = false;
            pub const SUSPENDED_FRAME: Option<(usize, usize)> = None;

            pub unsafe extern "C" fn wasmtime_fiber_start(
                _entry: crate::RawFiberEntry,
                _arg: *mut u8,
                _host: *mut core::ffi::c_void,
                _top: *mut *mut u8,
            ) -> ! {
                unreachable!()
            }

            pub unsafe extern "C" fn wasmtime_fiber_suspend(_top: *mut u8) {
                unreachable!()
            }
        }
    }
}

pub(crate) use asm::SUSPENDED_FRAME;

/// A raw fiber's underlying Windows fiber, which owns its stack's memory.
pub(crate) struct Fiber(*mut c_void);

// SAFETY: a Windows fiber which is not running may be switched to from any
// thread, and `Fiber` only does so from `&mut self`.
unsafe impl Send for Fiber {}
unsafe impl Sync for Fiber {}

/// What `fiber_start` passes to `wasmtime_fiber_start`, and what it reports
/// back.
struct Start {
    entry: RawFiberEntry,
    arg: *mut u8,
    host: *mut c_void,
    top: *mut u8,
    bottom: usize,
}

/// Whether raw fibers are supported on this platform and in this process.
pub(crate) fn supported() -> bool {
    asm::SUPPORTED_ARCH && !user_shadow_stacks()
}

/// Whether hardware-enforced shadow stacks are enabled for this process.
/// Restoring a snapshot changes the return addresses on a fiber's stack but
/// not on its shadow stack, so a restored fiber would fault on return.
fn user_shadow_stacks() -> bool {
    // `PROCESS_MITIGATION_USER_SHADOW_STACK_POLICY` is a `DWORD` of flags.
    const ENABLE_USER_SHADOW_STACK: u32 = 1 << 0;
    let mut flags = 0u32;
    // SAFETY: `flags` has the size passed. On versions of Windows without
    // shadow stacks this fails and leaves `flags` zero.
    unsafe {
        GetProcessMitigationPolicy(
            GetCurrentProcess(),
            ProcessUserShadowStackPolicy,
            (&raw mut flags).cast(),
            size_of::<u32>(),
        );
    }
    flags & ENABLE_USER_SHADOW_STACK != 0
}

pub(crate) fn switch_routine() -> unsafe extern "C" fn(*mut u8) {
    asm::wasmtime_fiber_suspend
}

unsafe extern "system" fn fiber_start(data: *mut c_void) {
    unsafe {
        set_stack_guarantee();
        let start = data.cast::<Start>();
        let mut high = 0;
        GetCurrentThreadStackLimits(&raw mut (*start).bottom, &mut high);
        asm::wasmtime_fiber_start(
            (*start).entry,
            (*start).arg,
            (*start).host,
            &raw mut (*start).top,
        )
    }
}

impl Fiber {
    /// Creates a Windows fiber for `stack` and runs it until its initial
    /// suspension, after which `stack` reports the fiber's stack bounds.
    pub(crate) unsafe fn new(
        stack: &mut crate::FiberStack,
        entry: RawFiberEntry,
        arg: *mut u8,
    ) -> Result<Self> {
        let mut start = Start {
            entry,
            arg,
            host: ptr::null_mut(),
            top: ptr::null_mut(),
            bottom: 0,
        };
        let start = &raw mut start;
        unsafe {
            let fiber = CreateFiberEx(
                0,
                stack.0.size,
                FIBER_FLAG_FLOAT_SWITCH,
                Some(fiber_start),
                start.cast(),
            );
            if fiber.is_null() {
                return Err(io::Error::last_os_error().into());
            }
            with_current_fiber(|host| {
                (*start).host = host;
                SwitchToFiber(fiber);
            });
            stack.0.raw_range = Some((*start).bottom..(*start).top as usize);
            Ok(Fiber(fiber))
        }
    }

    /// Runs the fiber, whose stack top is `top`, until it next suspends.
    pub(crate) unsafe fn resume(&mut self, top: *mut u8) {
        unsafe {
            with_current_fiber(|host| {
                top.cast::<*mut c_void>().sub(1).write(host);
                SwitchToFiber(self.0);
            });
        }
    }

    /// Deletes the fiber, after which `stack` may be used for another one.
    pub(crate) fn destroy(self, stack: &mut crate::FiberStack) {
        stack.0.raw_range = None;
    }
}

impl Drop for Fiber {
    fn drop(&mut self) {
        // SAFETY: a raw fiber only runs during `resume`.
        unsafe { delete_fiber(self.0) }
    }
}
