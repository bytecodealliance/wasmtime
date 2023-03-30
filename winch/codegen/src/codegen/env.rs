use wasmparser::FuncType;

/// Function environment used the by the code generation to
/// resolve module and runtime-specific information.
pub trait FuncEnv {
    /// Get the callee information from a given function index.
    fn callee_from_index(&self, index: u32) -> Callee;
}

/// Metadata about a function callee.  Use by the code generation
/// to emit function calls.
pub struct Callee {
    /// The function type.
    pub ty: FuncType,
    /// A flag to determine if the callee is imported.
    pub import: bool,
    /// The callee index in the WebAssembly function index space.
    pub index: u32,
}
