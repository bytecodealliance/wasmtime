//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use module::Module;

/// An Instance of a WebAssemby module.
#[derive(Debug)]
pub struct Compilation<'module> {
    /// The module this `Compilation` is compiled from.
    pub module: &'module Module,

    /// Compiled machine code for the function bodies.
    pub functions: Vec<Vec<u8>>,
}

impl<'module> Compilation<'module> {
    /// Allocates the runtime data structures with the given flags.
    pub fn new(module: &'module Module, functions: Vec<Vec<u8>>) -> Self {
        Self { module, functions }
    }
}
