//! Support machinery for the mmu-interruption feature

use crate::prelude::*;
use core::ffi::c_void;
use core::ptr::NonNull;

mod wheel;
pub use wheel::TimingWheelInterrupter;

#[cfg(has_mmu_interruption)]
mod trampoline;
#[cfg(has_mmu_interruption)]
pub(crate) use trampoline::resume_into_task_switch_trampoline;

/// A store's claim on an MMU interrupt page. Dropping it renounces the claim,
/// declaring that the store no longer interrupts if the page becomes
/// unreadable. It is a logic error to drop a handle and not immediately acquire
/// a new one when the corresponding store has any fibers in the Executing
/// state.
///
/// Implementations must keep released pages mapped while any Engine using them
/// might yet run any Wasm: compiled code may load from a stale pointer to one,
/// and only an access fault (not an unmapped-address fault) is caught as an
/// interruption.
pub trait PageHandle: Send + Sync {
    /// Returns the interrupt page pointer: the memory address to attempt to
    /// load at checkpoints.
    fn page_ptr(&self) -> NonNull<c_void>;
}

/// A source of memory pages whose protection bits trigger the interruption of
/// Wasm when MMU interruption is enabled.
pub trait MmuInterrupter: Send + Sync {
    /// Fetches an unprotected interrupt page. A scheduling mechanism must
    /// protect it (rendering it unreadable) at an appropriate time in the
    /// future to effect interruption.
    fn acquire_page(&self) -> Box<dyn PageHandle>;
}
