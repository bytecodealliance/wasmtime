//! Cranelift unwinder.
#![doc = include_str!("../README.md")]
#![no_std]
#![expect(unsafe_op_in_unsafe_fn, reason = "crate isn't migrated yet")]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

#[cfg(feature = "cranelift")]
extern crate alloc;

mod stackwalk;
pub use stackwalk::*;
mod arch;
#[allow(unused_imports)] // `arch` becomes empty on platforms without native-code backends.
pub use arch::*;
mod exception_table;
pub use exception_table::*;
mod throw;
pub use throw::*;
