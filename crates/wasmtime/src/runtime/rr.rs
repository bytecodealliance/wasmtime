//! Core WebAssembly record/replay for a whole store.
//!
//! Recording begins with an empty store, created by
//! [`Store::new_recording`](crate::Store::new_recording). Objects receive
//! numeric identities automatically; all construction is part of the trace.
//! Traces identify core modules only by the checksums of their bytecode, so
//! that they stay small.
//!
//! Core function boundaries support numbers, vectors, and abstract function
//! references. Modules that use GC or exceptions are unsupported.
//! Shared memory, host table/global mutation, call hooks,
//! custom signal handlers, Wasm stack switching, epoch deadline callbacks, and
//! guest debugging are unsupported. Fuel and epoch interruption may yield to
//! the async executor, which the guest cannot observe, but a recording in
//! which they stop the guest (by trapping, or by the call being cancelled
//! while it yields) cannot be replayed. Host writes through the
//! memory APIs, including slices from
//! [`Memory::data_mut`](crate::Memory::data_mut), are recorded; writes through
//! raw pointers such as [`Memory::data_ptr`](crate::Memory::data_ptr) are not.
//!
//! Traces describe execution at the level of WebAssembly semantics, and their
//! format is versioned independently of Wasmtime, so that a trace does not have
//! to be replayed by the version of Wasmtime that recorded it. They contain
//! host-supplied data and can be large; applications should impose their own
//! storage limits.

// The rest of the runtime uses record/replay through the crate-private API that
// both of these modules provide, which does nothing when the feature is
// disabled.

#[cfg(feature = "rr")]
mod enabled;
#[cfg(feature = "rr")]
pub use enabled::*;

#[cfg(not(feature = "rr"))]
mod disabled;
#[cfg(not(feature = "rr"))]
pub(crate) use disabled::*;
