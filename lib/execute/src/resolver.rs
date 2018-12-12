use wasmtime_runtime::Export;

/// Import resolver connects imports with available exported values.
pub trait Resolver {
    /// Resolve the given module/field combo.
    fn resolve(&mut self, module: &str, field: &str) -> Option<Export>;
}

/// `Resolver` implementation that always resolves to `None`.
pub struct NullResolver {}

impl Resolver for NullResolver {
    fn resolve(&mut self, _module: &str, _field: &str) -> Option<Export> {
        None
    }
}
