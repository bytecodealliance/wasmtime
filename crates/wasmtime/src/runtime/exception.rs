//! Types for the public API around exceptions.
//!
//! To allow host code to interact with exceptions, Wasmtime provides
//! two basic areas of API:
//!
//! - The [`crate::ExnRef`] type and associated types allow the host
//!   to create exception objects. In the Wasm execution model, every
//!   thrown exception is a unique instance of an exception object,
//!   which carries a reference to the associated tag and any payload
//!   values specified by the exception's signature.
//!
//! - The [`crate::Store::throw`] method to throw an exception, and
//!   associated methods to take ([`crate::Store::take_exception`]) or
//!   peek at ([`crate::Store::peek_exception`]) a thrown exception,
//!   along with the `Error` type [`ThrownException`] that indicates
//!   an exception is being thrown. This API allows access to a
//!   "pending exception" slot on the `Store` which roots an exception
//!   object and allows it to be propagated through Wasm and hostcall
//!   layers. If Wasm code throws an uncaught exception, it will be
//!   set as the pending exception and the call into Wasm will return
//!   an `Err(ThrownException.into())`; if a hostcall wishes to throw
//!   an exception to be caught by Wasm (or the outer call into Wasm
//!   by the host), it can call `Store::throw` and return the
//!   associated error.

/// An error type that represents that a pending WebAssembly exception
/// is set on the associated `Store`.
///
/// When used as an error type and returned from a Wasm-to-host call,
/// or host-to-Wasm call, it indicates that the caller should either
/// continue propagating the error upward, or take and handle the
/// exception using [`crate::Store::take_exception`] (or a helper such
/// as [`crate::Store::catch`].
///
/// Wasmtime uses an error type *without* payload, and stores the
/// exception itself on the store, to maintain proper GC rooting;
/// otherwise, it is difficult to get exception propagation up the
/// stack right in the presence of nested handle scopes. A pending
/// exception on the store is safely rooted as long as it is stored
/// there.
#[derive(Debug)]
pub struct ThrownException;

/// We need to implement Error for `ThrownException` so it can be boxed up into
/// a `wasmtime::Error`.
impl core::error::Error for ThrownException {}

/// `Error` requires `Display`.
impl core::fmt::Display for ThrownException {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "thrown Wasm exception")
    }
}
