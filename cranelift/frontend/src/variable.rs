//! A basic `Variable` implementation.
//!
//! Frontends can use any indexing scheme they see fit and
//! generate the appropriate `Variable` instances.
//!
//! Note: The `Variable` is used by Cranelift to index an array containing
//! information about your mutable variables. Thus, when you create a new
//! `Variable` you should make sure that the index is provided by a counter
//! incremented by 1 each time you encounter a new mutable variable.

use core::u32;
use cranelift_codegen::entity::EntityRef;

///! An opaque reference to a variable.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Variable(u32);

impl Variable {
    /// Create a new Variable with the given index.
    pub fn with_u32(index: u32) -> Self {
        debug_assert!(index < u32::MAX);
        Self(index)
    }
}

impl EntityRef for Variable {
    fn new(index: usize) -> Self {
        debug_assert!(index < (u32::MAX as usize));
        Self(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}
