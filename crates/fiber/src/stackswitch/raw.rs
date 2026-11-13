//! Raw fibers on this crate's stack-switching routines, which keep the
//! suspended context on the fiber's stack in the reserved-slot layout
//! documented in `unix.rs`.

use super::{RAW_FIBERS, wasmtime_fiber_init, wasmtime_fiber_switch};
use crate::RawFiberEntry;
use wasmtime_environ::error::{Result, format_err};

pub(crate) use super::SUSPENDED_FRAME;
pub(crate) use super::wasmtime_fiber_switch_ as switch_routine;

pub fn supported() -> bool {
    RAW_FIBERS
}

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
