//! A basic `Variable` implementation.
//!
//! `ILBuilder`, `FunctionBuilder`, and related types have a `Variable`
//! type parameter, to allow frontends that identify variables with
//! their own index types to use them directly. Frontends which don't
//! can use the `Variable` defined here.

use cretonne::entity::EntityRef;
use std::u32;

///! An opaque reference to a variable.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Variable(u32);

impl EntityRef for Variable {
    fn new(index: usize) -> Self {
        assert!(index < (u32::MAX as usize));
        Variable(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}
