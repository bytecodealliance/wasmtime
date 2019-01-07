//! A basic `Variable` implementation.
//!
//! `FunctionBuilderContext`, `FunctionBuilder`, and related types have a `Variable`
//! type parameter, to allow frontends that identify variables with
//! their own index types to use them directly. Frontends which don't
//! can use the `Variable` defined here.

use core::u32;
use cranelift_codegen::entity::EntityRef;

///! An opaque reference to a variable.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Variable(u32);

impl Variable {
    /// Create a new Variable with the given index.
    pub fn with_u32(index: u32) -> Self {
        debug_assert!(index < u32::MAX);
        Variable(index)
    }
}

impl EntityRef for Variable {
    fn new(index: usize) -> Self {
        debug_assert!(index < (u32::MAX as usize));
        Variable(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}
