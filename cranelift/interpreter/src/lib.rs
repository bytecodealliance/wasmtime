//! Cranelift IR interpreter.
//!
//! This module is a project for interpreting Cranelift IR.

#![cfg_attr(not(test), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

pub mod address;
pub mod environment;
pub mod frame;
pub mod instruction;
pub mod interpreter;
pub mod state;
pub mod step;
pub mod value;
