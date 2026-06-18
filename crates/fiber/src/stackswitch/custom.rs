//! Embedder-provided stack-switching routines.
//!
//! This module is used when the `custom` feature is enabled and the target
//! architecture has no built-in inline-assembly implementation. It is the
//! stack-switching analogue of Wasmtime's `custom-virtual-memory` feature:
//! rather than relying on inline assembly for a known architecture, the
//! routines below are declared as `extern "C"` imports which the embedder is
//! expected to supply.
//!
//! These declarations are duplicated in
//! `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`,
//! keep the two copies in-sync.

unsafe extern "C" {
    pub(crate) fn wasmtime_fiber_init(
        top_of_stack: *mut u8,
        entry: extern "C" fn(*mut u8, *mut u8) -> *mut u8,
        entry_arg0: *mut u8,
    );

    pub(crate) fn wasmtime_fiber_switch(top_of_stack: *mut u8);
}
