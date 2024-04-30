//! "Dummy" implementations of some system primitives for MIRI emulation.
//!
//! Note that at this time this is just enough to run some tests in MIRI but
//! notably WebAssembly tests are not executed at this time (MIRI can't execute
//! Cranelift-generated code).

pub mod mmap;
pub mod traphandlers;
pub mod unwind;
pub mod vm;
