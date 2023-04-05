//! This crate implements Winch's function compilation environment,
//! which allows Winch's code generation to resolve module and runtime
//! specific information.  This crate mainly implements the
//! `winch_codegen::FuncEnv` trait.

use wasmparser::types::Types;
use wasmtime_environ::{FuncIndex, Module};
use winch_codegen::{self, Callee, TargetIsa};

/// Function environment containing module and runtime specific
/// information.
pub struct FuncEnv<'a> {
    /// The translated WebAssembly module.
    pub module: &'a Module,
    /// Type information about a module, once it has been validated.
    pub types: &'a Types,
    /// The current ISA.
    pub isa: &'a Box<dyn TargetIsa>,
}

impl<'a> winch_codegen::FuncEnv for FuncEnv<'a> {
    fn callee_from_index(&self, index: u32) -> Callee {
        let func = self
            .types
            .function_at(index)
            .unwrap_or_else(|| panic!("function type at index: {}", index));

        Callee {
            ty: func.clone(),
            import: self.module.is_imported_function(FuncIndex::from_u32(index)),
            index,
        }
    }
}

impl<'a> FuncEnv<'a> {
    /// Create a new function environment.
    pub fn new(module: &'a Module, types: &'a Types, isa: &'a Box<dyn TargetIsa>) -> Self {
        Self { module, types, isa }
    }
}
