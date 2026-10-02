//! Raw fibers on this crate's stack-switching routines, which keep the
//! suspended context on the fiber's stack in the reserved-slot layout
//! documented in `unix.rs`.

use super::RawFiberEntry;
use crate::stackswitch::{
    RAW_FIBERS, wasmtime_fiber_init, wasmtime_fiber_switch, wasmtime_fiber_switch_,
};
use wasmtime_environ::error::{Result, format_err};

pub fn supported() -> bool {
    RAW_FIBERS
}

pub fn switch_routine() -> unsafe extern "C" fn(*mut u8) {
    wasmtime_fiber_switch_
}

// Indices, in words from the saved stack pointer, of the frame
// pointer and return address that the switch routine spills. Keep
// in sync with `wasmtime_fiber_switch_` in `stackswitch/*.rs`.
pub const SUSPENDED_FRAME: Option<(usize, usize)> = if cfg!(target_arch = "x86_64") {
    Some((5, 6))
} else if cfg!(target_arch = "aarch64") {
    Some((18, 19))
} else {
    None
};

pub struct Fiber;

impl Fiber {
    pub unsafe fn new(
        stack: &mut crate::FiberStack,
        entry: RawFiberEntry,
        arg: *mut u8,
    ) -> Result<Self> {
        let top = stack
            .top()
            .ok_or_else(|| format_err!("raw fibers require a known stack top"))?;
        // SAFETY: the stack is owned by the raw fiber and large
        // enough for the initial frame, which the existing
        // closure-based fibers rely on too. The fiber-start
        // trampoline only forwards both arguments to `entry`.
        unsafe {
            wasmtime_fiber_init(
                top,
                core::mem::transmute::<RawFiberEntry, extern "C" fn(*mut u8, *mut u8) -> *mut u8>(
                    entry,
                ),
                arg,
            );
        }
        Ok(Fiber)
    }

    pub unsafe fn resume(&mut self, top: *mut u8) {
        // SAFETY: the saved stack pointer of a stopped fiber is in
        // the reserved slot. The switch stores this host
        // continuation in the same slot, so the fiber always
        // suspends back to here.
        unsafe { wasmtime_fiber_switch(top) }
    }

    pub fn destroy(self, _stack: &mut crate::FiberStack) {}
}
