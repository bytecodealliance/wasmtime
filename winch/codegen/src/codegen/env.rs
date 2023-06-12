use wasmtime_environ::{
    FuncIndex, ModuleTranslation, PtrSize, TypeConvert, VMOffsets, WasmFuncType,
};

/// The function environment.
///
/// Contains all information about the module and runtime that is accessible to
/// to a particular function during code generation.
pub struct FuncEnv<'a, P> {
    /// Offsets to the fields within the `VMContext` ptr.
    pub vmoffsets: VMOffsets<P>,
    /// Metadata about the translation process of a WebAssembly module.
    pub translation: &'a ModuleTranslation<'a>,
}

impl<'a, P: PtrSize> FuncEnv<'a, P> {
    /// Create a new function environment.
    pub fn new(ptr: P, translation: &'a ModuleTranslation) -> Self {
        let vmoffsets = VMOffsets::new(ptr, &translation.module);
        Self {
            vmoffsets,
            translation,
        }
    }

    /// Resolves a function [`Callee`] from an index.
    pub fn callee_from_index(&self, idx: FuncIndex) -> Callee {
        let types = &self.translation.get_types();
        let ty = types
            .function_at(idx.as_u32())
            .unwrap_or_else(|| panic!("function type at index: {}", idx.as_u32()));
        let ty = self.translation.module.convert_func_type(ty);
        let import = self.translation.module.is_imported_function(idx);

        Callee {
            ty,
            import,
            index: idx,
        }
    }
}

/// Metadata about a function callee.  Use by the code generation
/// to emit function calls.
pub struct Callee {
    /// The function type.
    pub ty: WasmFuncType,
    /// A flag to determine if the callee is imported.
    pub import: bool,
    /// The callee index in the WebAssembly function index space.
    pub index: FuncIndex,
}
