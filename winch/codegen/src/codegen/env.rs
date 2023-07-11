use smallvec::{smallvec, SmallVec};
use wasmparser::BlockType;
use wasmtime_environ::{
    FuncIndex, GlobalIndex, ModuleTranslation, PtrSize, TypeConvert, VMOffsets, WasmFuncType,
    WasmType,
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
        let ty = types[types.function_at(idx.as_u32())].unwrap_func();
        let ty = self.translation.module.convert_func_type(ty);
        let import = self.translation.module.is_imported_function(idx);

        Callee {
            ty,
            import,
            index: idx,
        }
    }

    /// Resolves the type of the block in terms of [`wasmtime_environ::WasmType`].
    pub fn resolve_block_type(&self, blockty: BlockType) -> SmallVec<[WasmType; 1]> {
        use BlockType::*;
        match blockty {
            Empty => smallvec![],
            Type(ty) => smallvec![self.translation.module.convert_valtype(ty)],
            _ => unimplemented!("multi-value"),
        }
    }

    /// Resolves the type and offset of a global at the given index.
    pub fn resolve_global_type_and_offset(&self, index: GlobalIndex) -> (WasmType, u32) {
        let ty = self.translation.module.globals[index].wasm_ty;
        let offset = match self.translation.module.defined_global_index(index) {
            Some(defined_index) => self.vmoffsets.vmctx_vmglobal_definition(defined_index),
            None => self.vmoffsets.vmctx_vmglobal_import_from(index),
        };

        (ty, offset)
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
