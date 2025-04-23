//! Code generation library for Winch.

// Unless this library is compiled with `all-arch`, the rust compiler
// is going to emit dead code warnings. This directive is fine as long
// as we configure to run CI at least once with the `all-arch` feature
// enabled.
#![cfg_attr(not(feature = "all-arch"), allow(dead_code))]

mod abi;
pub use codegen::{BuiltinFunctions, FuncEnv};
mod codegen;
mod frame;
pub mod isa;
pub use isa::*;
mod masm;
mod regalloc;
mod regset;
mod stack;
mod visitor;
