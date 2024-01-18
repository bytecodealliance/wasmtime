use crate::codegen::{control, BlockSig, BuiltinFunction, OperandSize};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use wasmparser::BlockType;
use wasmtime_environ::{
    FuncIndex, GlobalIndex, MemoryIndex, ModuleTranslation, ModuleTypesBuilder, PtrSize,
    TableIndex, TablePlan, TypeConvert, TypeIndex, VMOffsets, WasmFuncType, WasmHeapType, WasmType,
};

/// Table metadata.
#[derive(Debug, Copy, Clone)]
pub struct TableData {
    /// The offset to the base of the table.
    pub offset: u32,
    /// The offset to the current elements field.
    pub current_elems_offset: u32,
    /// If the table is imported, this field contains the offset to locate the
    /// base of the table data.
    pub import_from: Option<u32>,
    /// The size of the table elements.
    pub(crate) element_size: OperandSize,
    /// The size of the current elements field.
    pub(crate) current_elements_size: OperandSize,
}

/// Heap metadata.
#[derive(Debug, Copy, Clone)]
pub struct HeapData {
    /// The offset to the base of the heap.
    pub offset: u32,
    /// The offset to the current length field.
    pub current_length_offset: u32,
    /// If the heap is imported, this field contains the offset to locate the
    /// base of the heap.
    pub import_from: Option<u32>,
    /// The memory type (32 or 64).
    pub ty: WasmType,
}

/// A function callee.
/// It categorizes how the callee should be treated
/// when performing the call.
#[derive(Clone)]
pub enum Callee {
    /// Locally defined function.
    Local(CalleeInfo),
    /// Imported function.
    Import(CalleeInfo),
    /// Function reference.
    FuncRef(WasmFuncType),
    /// A built-in function.
    Builtin(BuiltinFunction),
}

/// Metadata about a function callee. Used by the code generation to
/// emit function calls to local or imported functions.
#[derive(Clone)]
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
pub struct FuncEnv<'a, 'translation: 'a, 'data: 'translation, P: PtrSize> {
    /// Offsets to the fields within the `VMContext` ptr.
    pub vmoffsets: &'a VMOffsets<P>,
    /// Metadata about the translation process of a WebAssembly module.
    pub translation: &'translation ModuleTranslation<'data>,
    /// The module's function types.
    pub types: &'translation ModuleTypesBuilder,
    /// Track resolved table information.
    resolved_tables: HashMap<TableIndex, TableData>,
    resolved_heaps: HashMap<MemoryIndex, HeapData>,
}

pub fn ptr_type_from_ptr_size(size: u8) -> WasmType {
    (size == 8)
        .then(|| WasmType::I64)
        .unwrap_or_else(|| unimplemented!("Support for non-64-bit architectures"))
}

impl<'a, 'translation, 'data, P: PtrSize> FuncEnv<'a, 'translation, 'data, P> {
    /// Create a new function environment.
    pub fn new(
        vmoffsets: &'a VMOffsets<P>,
        translation: &'translation ModuleTranslation<'data>,
        types: &'translation ModuleTypesBuilder,
    ) -> Self {
        Self {
            vmoffsets,
            translation,
            types,
            resolved_tables: HashMap::new(),
            resolved_heaps: HashMap::new(),
        }
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
        let ty = types[types.core_function_at(idx.as_u32())].unwrap_func();
        let ty = self.convert_func_type(ty);
        let import = self.translation.module.is_imported_function(idx);

        let info = CalleeInfo { ty, index: idx };

        if import {
            Callee::Import(info)
        } else {
            Callee::Local(info)
        }
    }

    /// Converts a [wasmparser::BlockType] into a [BlockSig].
    pub(crate) fn resolve_block_sig(&self, ty: BlockType) -> BlockSig {
        use BlockType::*;
        match ty {
            Empty => BlockSig::new(control::BlockType::void()),
            Type(ty) => {
                let ty = self.convert_valtype(ty);
                BlockSig::new(control::BlockType::single(ty))
            }
            FuncType(idx) => {
                let sig_index =
                    self.translation.module.types[TypeIndex::from_u32(idx)].unwrap_function();
                let sig = &self.types[sig_index];
                BlockSig::new(control::BlockType::func(sig.clone()))
            }
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
    pub fn resolve_table_data(&mut self, index: TableIndex) -> TableData {
        match self.resolved_tables.entry(index) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => {
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

                *entry.insert(TableData {
                    import_from: from_offset,
                    offset: base_offset,
                    current_elems_offset,
                    element_size: OperandSize::from_bytes(self.vmoffsets.ptr.size()),
                    current_elements_size: OperandSize::from_bytes(
                        self.vmoffsets.size_of_vmtable_definition_current_elements(),
                    ),
                })
            }
        }
    }

    /// Resolved a [HeapData] from a [MemoryIndex].
    // TODO: (@saulecabrera)
    // Handle shared memories when implementing support for Wasm Threads.
    pub fn resolve_heap(&mut self, index: MemoryIndex) -> HeapData {
        match self.resolved_heaps.entry(index) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => {
                let (import_from, base_offset, current_length_offset) =
                    match self.translation.module.defined_memory_index(index) {
                        Some(defined) => {
                            let owned = self.translation.module.owned_memory_index(defined);
                            (
                                None,
                                self.vmoffsets.vmctx_vmmemory_definition_base(owned),
                                self.vmoffsets
                                    .vmctx_vmmemory_definition_current_length(owned),
                            )
                        }
                        None => (
                            Some(self.vmoffsets.vmctx_vmmemory_import_from(index)),
                            self.vmoffsets.ptr.vmmemory_definition_base().into(),
                            self.vmoffsets
                                .ptr
                                .vmmemory_definition_current_length()
                                .into(),
                        ),
                    };

                *entry.insert(HeapData {
                    offset: base_offset,
                    import_from,
                    current_length_offset,
                    ty: if self.translation.module.memory_plans[index].memory.memory64 {
                        WasmType::I64
                    } else {
                        WasmType::I32
                    },
                })
            }
        }
    }

    /// Get a [`TablePlan`] from a [`TableIndex`].
    pub fn table_plan(&mut self, index: TableIndex) -> &TablePlan {
        &self.translation.module.table_plans[index]
    }
}

impl<P: PtrSize> TypeConvert for FuncEnv<'_, '_, '_, P> {
    fn lookup_heap_type(&self, idx: wasmparser::UnpackedIndex) -> WasmHeapType {
        wasmtime_environ::WasmparserTypeConverter {
            module: &self.translation.module,
            types: self.types,
        }
        .lookup_heap_type(idx)
    }
}
