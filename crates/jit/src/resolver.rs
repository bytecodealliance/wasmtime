//! Define the `Resolver` trait, allowing custom resolution for external
//! references.

use wasmtime_runtime::Export;

/// Import resolver connects imports with available exported values.
pub trait Resolver {
    /// Resolves an import a WebAssembly module to an export it's hooked up to.
    ///
    /// The `index` provided is the index of the import in the wasm module
    /// that's being resolved. For example 1 means that it's the second import
    /// listed in the wasm module.
    ///
    /// The `module` and `field` arguments provided are the module/field names
    /// listed on the import itself.
    fn resolve(&mut self, index: u32, module: &str, field: &str) -> Option<Export>;
}

/// `Resolver` implementation that always resolves to `None`.
pub struct NullResolver {}

impl Resolver for NullResolver {
    fn resolve(&mut self, _idx: u32, _module: &str, _field: &str) -> Option<Export> {
        None
    }
}
