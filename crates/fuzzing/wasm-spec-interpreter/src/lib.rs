//! This library provides a way to interpret Wasm functions in the official Wasm
//! specification interpreter, written in OCaml, from Rust.
//!
//! In order to not break Wasmtime's build, this library will always compile. It
//! does depend on certain tools (see `README.md`) that may or may not be
//! available in the environment:
//!  - when the tools are available, we build and link to an OCaml static
//!    library (see `with_library` module)
//!  - when the tools are not available, this library will panic at runtime (see
//!    `without_library` module).

/// Enumerate the kinds of Wasm values.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(i32),
    F64(i64),
}

#[cfg(feature = "has-libinterpret")]
mod with_library;
#[cfg(feature = "has-libinterpret")]
pub use with_library::*;

#[cfg(not(feature = "has-libinterpret"))]
mod without_library;
#[cfg(not(feature = "has-libinterpret"))]
pub use without_library::*;

// FIXME(#3251) should re-enable once spec interpreter won't time out
// If the user is fuzzing`, we expect the OCaml library to have been built.
// #[cfg(all(fuzzing, not(feature = "has-libinterpret")))]
// compile_error!("The OCaml library was not built.");
