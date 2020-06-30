//! Shared traits, types, and macros for Peepmatic.
//!
//! This crate is used both at build time when constructing peephole optimizers
//! (i.e. in the `peepmatic` crate), and at run time when using pre-built
//! peephole optimizers (i.e. in the `peepmatic-runtime` crate and in
//! Cranelift's Peepmatic integration at `cranelift/codegen/src/peepmatic.rs`).
//!
//! This crate is similar to a header file: it should generally only contain
//! trait/type/macro definitions, not any code.

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[macro_use]
mod operator;
pub use operator::*;

mod typing;
pub use typing::*;

/// Raise a panic about an unsupported operation.
#[cold]
#[inline(never)]
pub fn unsupported(msg: &str) -> ! {
    panic!("unsupported: {}", msg)
}
