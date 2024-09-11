use crate::{
    abi::{wasm_sig, ABISig, ABI},
    codegen::{control, BlockSig, BuiltinFunction, BuiltinFunctions, OperandSize},
    isa::TargetIsa,
};
use cranelift_codegen::ir::{UserExternalName, UserExternalNameRef};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::mem;
use wasmparser::BlockType;
use wasmtime_environ::{
    BuiltinFunctionIndex, FuncIndex, GlobalIndex, MemoryIndex, MemoryPlan, MemoryStyle,
    ModuleTranslation, ModuleTypesBuilder, PrimaryMap, PtrSize, TableIndex, TablePlan, TypeConvert,
    TypeIndex, VMOffsets, WasmHeapType, WasmValType,
};

#[derive(Debug, Clone, Copy)]
pub struct GlobalData {
    /// The offset of the global.
    pub offset: u32,
    /// True if the global is imported.
    pub imported: bool,
    /// The WebAssembly type of the global.
    pub ty: WasmValType,
}

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

/// Style of the heap.
#[derive(Debug, Copy, Clone)]
pub enum HeapStyle {
    /// Static heap, which has a fixed address.
    Static {
        /// The heap bound in bytes, not including the bytes for the offset
        /// guard pages.
        bound: u64,
    },
    /// Dynamic heap, which can be relocated to a different address when grown.
    /// The bounds are calculated at runtime on every access.
    Dynamic,
}

/// Heap metadata.
///
/// Heaps represent a WebAssembly linear memory.
#[derive(Debug, Copy, Clone)]
pub struct HeapData {
    /// The offset to the base of the heap.
    /// Relative to the VMContext pointer if the WebAssembly memory is locally
    /// defined. Else this is relative to the location of the imported WebAssembly
    /// memory location.
    pub offset: u32,
    /// The offset to the current length field.
    pub current_length_offset: u32,
    /// If the WebAssembly memory is imported, this field contains the offset to locate the
    /// base of the heap.
    pub import_from: Option<u32>,
    /// The memory type (32 or 64).
    pub ty: WasmValType,
    /// The style of the heap.
    pub style: HeapStyle,
    /// The guaranteed minimum size, in bytes.
    pub min_size: u64,
    /// The maximum heap size in bytes.
    pub max_size: Option<u64>,
    /// The log2 of this memory's page size, in bytes.
    ///
    /// By default the page size is 64KiB (0x10000; 2**16; 1<<16; 65536) but the
    /// custom-page-sizes proposal allows opting into a page size of `1`.
    pub page_size_log2: u8,
    /// Size in bytes of the offset guard pages, located after the heap bounds.
    pub offset_guard_size: u64,
}

/// A function callee.
/// It categorizes how the callee should be treated
/// when performing the call.
#[derive(Clone)]
pub(crate) enum Callee {
    /// Locally defined function.
    Local(FuncIndex),
    /// Imported function.
    Import(FuncIndex),
    /// Function reference.
    FuncRef(TypeIndex),
    /// A built-in function.
    Builtin(BuiltinFunction),
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
    /// The built-in functions available to the JIT code.
    pub builtins: &'translation mut BuiltinFunctions,
    /// Track resolved table information.
    resolved_tables: HashMap<TableIndex, TableData>,
    /// Track resolved heap information.
    resolved_heaps: HashMap<MemoryIndex, HeapData>,
    /// A map from [FunctionIndex] to [ABISig], to keep track of the resolved
    /// function callees.
    resolved_callees: HashMap<FuncIndex, ABISig>,
    /// A map from [TypeIndex] to [ABISig], to keep track of the resolved
    /// indirect function signatures.
    resolved_sigs: HashMap<TypeIndex, ABISig>,
    /// A map from [GlobalIndex] to [GlobalData].
    resolved_globals: HashMap<GlobalIndex, GlobalData>,
    /// Pointer size represented as a WebAssembly type.
    ptr_type: WasmValType,
    /// Whether or not to enable Spectre mitigation on heap bounds checks.
    heap_access_spectre_mitigation: bool,
    /// Whether or not to enable Spectre mitigation on table element accesses.
    table_access_spectre_mitigation: bool,
    name_map: PrimaryMap<UserExternalNameRef, UserExternalName>,
    name_intern: HashMap<UserExternalName, UserExternalNameRef>,
}

pub fn ptr_type_from_ptr_size(size: u8) -> WasmValType {
    (size == 8)
        .then(|| WasmValType::I64)
        .unwrap_or_else(|| unimplemented!("Support for non-64-bit architectures"))
}

impl<'a, 'translation, 'data, P: PtrSize> FuncEnv<'a, 'translation, 'data, P> {
    /// Create a new function environment.
    pub fn new(
        vmoffsets: &'a VMOffsets<P>,
        translation: &'translation ModuleTranslation<'data>,
        types: &'translation ModuleTypesBuilder,
        builtins: &'translation mut BuiltinFunctions,
        isa: &dyn TargetIsa,
        ptr_type: WasmValType,
    ) -> Self {
        Self {
            vmoffsets,
            translation,
            types,
            resolved_tables: HashMap::new(),
            resolved_heaps: HashMap::new(),
            resolved_callees: HashMap::new(),
            resolved_sigs: HashMap::new(),
            resolved_globals: HashMap::new(),
            ptr_type,
            heap_access_spectre_mitigation: isa.flags().enable_heap_access_spectre_mitigation(),
            table_access_spectre_mitigation: isa.flags().enable_table_access_spectre_mitigation(),
            builtins,
            name_map: Default::default(),
            name_intern: Default::default(),
        }
    }

    /// Derive the [`WasmType`] from the pointer size.
    pub(crate) fn ptr_type(&self) -> WasmValType {
        self.ptr_type
    }

    /// Resolves a [`Callee::FuncRef`] from a type index.
    pub(crate) fn funcref(&mut self, idx: TypeIndex) -> Callee {
        Callee::FuncRef(idx)
    }

    /// Resolves a function [`Callee`] from an index.
    pub(crate) fn callee_from_index(&mut self, idx: FuncIndex) -> Callee {
        let import = self.translation.module.is_imported_function(idx);
        if import {
            Callee::Import(idx)
        } else {
            Callee::Local(idx)
        }
    }

    /// Converts a [wasmparser::BlockType] into a [BlockSig].
    pub(crate) fn resolve_block_sig(&self, ty: BlockType) -> BlockSig {
        use BlockType::*;
        match ty {
            Empty => BlockSig::new(control::BlockType::void()),
            Type(ty) => {
                let ty = TypeConverter::new(self.translation, self.types).convert_valtype(ty);
                BlockSig::new(control::BlockType::single(ty))
            }
            FuncType(idx) => {
                let sig_index = self.translation.module.types[TypeIndex::from_u32(idx)];
                let sig = self.types[sig_index].unwrap_func();
                BlockSig::new(control::BlockType::func(sig.clone()))
            }
        }
    }

    /// Resolves `GlobalData` of a global at the given index.
    pub fn resolve_global(&mut self, index: GlobalIndex) -> GlobalData {
        let ty = self.translation.module.globals[index].wasm_ty;
        let val = || match self.translation.module.defined_global_index(index) {
            Some(defined_index) => GlobalData {
                offset: self.vmoffsets.vmctx_vmglobal_definition(defined_index),
                imported: false,
                ty,
            },
            None => GlobalData {
                offset: self.vmoffsets.vmctx_vmglobal_import_from(index),
                imported: true,
                ty,
            },
        };

        *self.resolved_globals.entry(index).or_insert_with(val)
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

    /// Resolve a `HeapData` from a [MemoryIndex].
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

                let plan = &self.translation.module.memory_plans[index];
                let (min_size, max_size) = heap_limits(&plan);
                let (style, offset_guard_size) = heap_style_and_offset_guard_size(&plan);

                *entry.insert(HeapData {
                    offset: base_offset,
                    import_from,
                    current_length_offset,
                    style,
                    ty: match plan.memory.idx_type {
                        wasmtime_environ::IndexType::I32 => WasmValType::I32,
                        wasmtime_environ::IndexType::I64 => WasmValType::I64,
                    },
                    min_size,
                    max_size,
                    page_size_log2: plan.memory.page_size_log2,
                    offset_guard_size,
                })
            }
        }
    }

    /// Get a [`TablePlan`] from a [`TableIndex`].
    pub fn table_plan(&mut self, index: TableIndex) -> &TablePlan {
        &self.translation.module.table_plans[index]
    }

    /// Returns true if Spectre mitigations are enabled for heap bounds check.
    pub fn heap_access_spectre_mitigation(&self) -> bool {
        self.heap_access_spectre_mitigation
    }

    /// Returns true if Spectre mitigations are enabled for table element
    /// accesses.
    pub fn table_access_spectre_mitigation(&self) -> bool {
        self.table_access_spectre_mitigation
    }

    pub(crate) fn callee_sig<'b, A>(&'b mut self, callee: &'b Callee) -> &'b ABISig
    where
        A: ABI,
    {
        match callee {
            Callee::Local(idx) | Callee::Import(idx) => {
                let types = self.translation.get_types();
                let ty = types[types.core_function_at(idx.as_u32())].unwrap_func();
                let val = || {
                    let converter = TypeConverter::new(self.translation, self.types);
                    let ty = converter.convert_func_type(&ty);
                    wasm_sig::<A>(&ty)
                };
                self.resolved_callees.entry(*idx).or_insert_with(val)
            }
            Callee::FuncRef(idx) => {
                let val = || {
                    let sig_index = self.translation.module.types[*idx];
                    let ty = self.types[sig_index].unwrap_func();
                    let sig = wasm_sig::<A>(ty);
                    sig
                };
                self.resolved_sigs.entry(*idx).or_insert_with(val)
            }
            Callee::Builtin(b) => b.sig(),
        }
    }

    /// Creates a name to reference the `builtin` provided.
    pub fn name_builtin(&mut self, builtin: BuiltinFunctionIndex) -> UserExternalNameRef {
        self.intern_name(UserExternalName {
            namespace: wasmtime_cranelift::NS_WASMTIME_BUILTIN,
            index: builtin.index(),
        })
    }

    /// Creates a name to reference the wasm function `index` provided.
    pub fn name_wasm(&mut self, index: FuncIndex) -> UserExternalNameRef {
        self.intern_name(UserExternalName {
            namespace: wasmtime_cranelift::NS_WASM_FUNC,
            index: index.as_u32(),
        })
    }

    /// Interns `name` into a `UserExternalNameRef` and ensures that duplicate
    /// instances of `name` are given a unique name ref index.
    fn intern_name(&mut self, name: UserExternalName) -> UserExternalNameRef {
        *self
            .name_intern
            .entry(name.clone())
            .or_insert_with(|| self.name_map.push(name))
    }

    /// Extracts the name map that was created while translating this function.
    pub fn take_name_map(&mut self) -> PrimaryMap<UserExternalNameRef, UserExternalName> {
        self.name_intern.clear();
        mem::take(&mut self.name_map)
    }
}

/// A wrapper struct over a reference to a [ModuleTranslation] and
/// [ModuleTypesBuilder].
pub(crate) struct TypeConverter<'a, 'data: 'a> {
    translation: &'a ModuleTranslation<'data>,
    types: &'a ModuleTypesBuilder,
}

impl TypeConvert for TypeConverter<'_, '_> {
    fn lookup_heap_type(&self, idx: wasmparser::UnpackedIndex) -> WasmHeapType {
        wasmtime_environ::WasmparserTypeConverter::new(self.types, &self.translation.module)
            .lookup_heap_type(idx)
    }

    fn lookup_type_index(
        &self,
        index: wasmparser::UnpackedIndex,
    ) -> wasmtime_environ::EngineOrModuleTypeIndex {
        wasmtime_environ::WasmparserTypeConverter::new(self.types, &self.translation.module)
            .lookup_type_index(index)
    }
}

impl<'a, 'data> TypeConverter<'a, 'data> {
    pub fn new(translation: &'a ModuleTranslation<'data>, types: &'a ModuleTypesBuilder) -> Self {
        Self { translation, types }
    }
}

fn heap_style_and_offset_guard_size(plan: &MemoryPlan) -> (HeapStyle, u64) {
    match plan {
        MemoryPlan {
            style: MemoryStyle::Static { byte_reservation },
            offset_guard_size,
            ..
        } => (
            HeapStyle::Static {
                bound: *byte_reservation,
            },
            *offset_guard_size,
        ),

        MemoryPlan {
            style: MemoryStyle::Dynamic { .. },
            offset_guard_size,
            ..
        } => (HeapStyle::Dynamic, *offset_guard_size),
    }
}

fn heap_limits(plan: &MemoryPlan) -> (u64, Option<u64>) {
    (
        plan.memory.minimum_byte_size().unwrap_or_else(|_| {
            // 2^64 as a minimum doesn't fin in a 64 bit integer.
            // So in this case, the minimum is clamped to u64::MAX.
            u64::MAX
        }),
        plan.memory.maximum_byte_size().ok(),
    )
}
