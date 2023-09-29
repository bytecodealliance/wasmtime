use crate::{codegen::BuiltinFunctions, CallingConvention};
use smallvec::{smallvec, SmallVec};
use wasmparser::BlockType;
use wasmtime_environ::{
    FuncIndex, GlobalIndex, ModuleTranslation, ModuleTypes, PtrSize, TableIndex, TypeConvert,
    TypeIndex, VMOffsets, WasmFuncType, WasmType,
};

/// Table metadata.
pub struct TableData {
    /// The offset to the base of the table.
    pub offset: u32,
    /// The offset to the current elements field.
    pub current_elems_offset: u32,
    /// If the table is imported, return the base
    /// offset of the `from` field in `VMTableImport`.
    pub base: Option<u32>,
    /// The size of the table elements, in bytes.
    pub element_size: u8,
}

/// A function callee.
/// It categorizes how the callee should be treated
/// when performing the call.
pub enum Callee {
    /// Locally defined function.
    Local(CalleeInfo),
    /// Imported function.
    Import(CalleeInfo),
    /// Function reference.
    FuncRef(WasmFuncType),
}

/// Metadata about a function callee. Used by the code generation to
/// emit function calls to local or imported functions.
pub struct CalleeInfo {
    /// The function type.
    pub ty: WasmFuncType,
    /// The callee index in the WebAssembly function index space.
    pub index: FuncIndex,
}

/// The function environment.
///
/// Contains all information about the module and runtime that is accessible to
/// to a particular function during code generation.
pub struct FuncEnv<'a, P: PtrSize> {
    /// Offsets to the fields within the `VMContext` ptr.
    pub vmoffsets: VMOffsets<P>,
    /// Metadata about the translation process of a WebAssembly module.
    pub translation: &'a ModuleTranslation<'a>,
    /// Metadata about the builtin functions.
    pub builtins: BuiltinFunctions,
    /// The module's function types.
    pub types: &'a ModuleTypes,
}

pub fn ptr_type_from_ptr_size(size: u8) -> WasmType {
    (size == 8)
        .then(|| WasmType::I64)
        .unwrap_or_else(|| unimplemented!("Support for non-64-bit architectures"))
}

impl<'a, P: PtrSize> FuncEnv<'a, P> {
    /// Create a new function environment.
    pub fn new(
        ptr: P,
        translation: &'a ModuleTranslation,
        types: &'a ModuleTypes,
        call_conv: CallingConvention,
    ) -> Self {
        let vmoffsets = VMOffsets::new(ptr, &translation.module);
        let size = vmoffsets.ptr.size();
        let builtins_base = vmoffsets.vmctx_builtin_functions();
        Self {
            vmoffsets,
            translation,
            builtins: BuiltinFunctions::new(size, call_conv, builtins_base),
            types,
        }
    }

    /// Returns a slice of types representing the caller and callee VMContext types.
    pub(crate) fn vmctx_args_type(&self) -> [WasmType; 2] {
        let ty = self.ptr_type();
        [ty, ty]
    }

    /// Derive the [`WasmType`] from the pointer size.
    pub(crate) fn ptr_type(&self) -> WasmType {
        ptr_type_from_ptr_size(self.ptr_size())
    }

    /// Returns the pointer size for the target ISA.
    fn ptr_size(&self) -> u8 {
        self.vmoffsets.ptr.size()
    }

    /// Resolves a [`Callee::FuncRef`] from a type index.
    pub fn funcref(&self, idx: TypeIndex) -> Callee {
        let sig_index = self.translation.module.types[idx].unwrap_function();
        let ty = self.types[sig_index].clone();
        Callee::FuncRef(ty)
    }

    /// Resolves a function [`Callee`] from an index.
    pub fn callee_from_index(&self, idx: FuncIndex) -> Callee {
        let types = &self.translation.get_types();
        let ty = types[types.function_at(idx.as_u32())].unwrap_func();
        let ty = self.translation.module.convert_func_type(ty);
        let import = self.translation.module.is_imported_function(idx);

        let info = CalleeInfo { ty, index: idx };

        if import {
            Callee::Import(info)
        } else {
            Callee::Local(info)
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

    /// Returns the table information for the given table index.
    pub fn resolve_table_data(&self, index: TableIndex) -> TableData {
        let (from_offset, base_offset, current_elems_offset) =
            match self.translation.module.defined_table_index(index) {
                Some(defined) => (
                    None,
                    self.vmoffsets.vmctx_vmtable_definition_base(defined),
                    self.vmoffsets
                        .vmctx_vmtable_definition_current_elements(defined),
                ),
                None => (
                    Some(self.vmoffsets.vmctx_vmtable_import_from(index)),
                    self.vmoffsets.vmtable_definition_base().into(),
                    self.vmoffsets.vmtable_definition_current_elements().into(),
                ),
            };

        TableData {
            base: from_offset,
            offset: base_offset,
            current_elems_offset,
            element_size: self.vmoffsets.ptr.size(),
        }
    }
}
