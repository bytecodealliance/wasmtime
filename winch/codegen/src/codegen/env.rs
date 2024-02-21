use crate::{
    abi::{wasm_sig, ABISig, ABI},
    codegen::{control, BlockSig, BuiltinFunction, BuiltinFunctions, OperandSize},
    isa::TargetIsa,
};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use wasmparser::BlockType;
use wasmtime_environ::{
    FuncIndex, GlobalIndex, MemoryIndex, MemoryPlan, MemoryStyle, ModuleTranslation,
    ModuleTypesBuilder, PtrSize, TableIndex, TablePlan, TypeConvert, TypeIndex, VMOffsets,
    WasmHeapType, WasmValType, WASM_PAGE_SIZE,
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
    /// Size in bytes of the offset guard pages, located after the heap bounds.
    pub offset_guard_size: u64,
}

/// A function callee.
/// It categorizes how the callee should be treated
/// when performing the call.
#[derive(Clone)]
pub(crate) enum Callee<'a> {
    /// Locally defined function.
    Local(&'a CalleeInfo),
    /// Imported function.
    Import(&'a CalleeInfo),
    /// Function reference.
    FuncRef(&'a ABISig),
    /// A built-in function.
    Builtin(BuiltinFunction),
}

impl Callee<'_> {
    /// Returns the [ABISig] of the [Callee].
    pub(crate) fn sig(&self) -> &ABISig {
        match self {
            Self::Local(info) | Self::Import(info) => &info.sig,
            Self::FuncRef(sig) => sig,
            Self::Builtin(b) => b.sig(),
        }
    }
}

/// Metadata about a function callee. Used by the code generation to
/// emit function calls to local or imported functions.
#[derive(Clone)]
pub struct CalleeInfo {
    /// The function's ABI signature.
    pub(crate) sig: ABISig,
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
    /// The built-in functions available to the JIT code.
    pub builtins: &'translation mut BuiltinFunctions,
    /// Track resolved table information.
    resolved_tables: HashMap<TableIndex, TableData>,
    /// Track resolved heap information.
    resolved_heaps: HashMap<MemoryIndex, HeapData>,
    /// A map from [FunctionIndex] to [CalleeInfo], to keep track of the resolved
    /// function callees.
    resolved_callees: HashMap<FuncIndex, CalleeInfo>,
    /// A map from [TypeIndex] to [ABISig], to keep track of the resolved
    /// indirect function signatures.
    resolved_sigs: HashMap<TypeIndex, ABISig>,
    /// A map from [GlobalIndex] to [GlobalData].
    resolved_globals: HashMap<GlobalIndex, GlobalData>,
    /// Pointer size represented as a WebAssembly type.
    ptr_type: WasmValType,
    /// Whether or not to enable Spectre mitigation on heap bounds checks.
    heap_access_spectre_mitigation: bool,
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
            builtins,
        }
    }

    /// Derive the [`WasmType`] from the pointer size.
    pub(crate) fn ptr_type(&self) -> WasmValType {
        self.ptr_type
    }

    /// Resolves a [`Callee::FuncRef`] from a type index.
    pub(crate) fn funcref<A>(&mut self, idx: TypeIndex) -> Callee
    where
        A: ABI,
    {
        let val = || {
            let sig_index = self.translation.module.types[idx].unwrap_function();
            let ty = &self.types[sig_index];
            let sig = wasm_sig::<A>(ty);
            sig
        };
        Callee::FuncRef(self.resolved_sigs.entry(idx).or_insert_with(val))
    }

    /// Resolves a function [`Callee`] from an index.
    pub(crate) fn callee_from_index<A>(&mut self, idx: FuncIndex) -> Callee
    where
        A: ABI,
    {
        let types = self.translation.get_types();
        let ty = types[types.core_function_at(idx.as_u32())].unwrap_func();
        let ty = self.convert_func_type(ty);
        let import = self.translation.module.is_imported_function(idx);
        let val = || {
            let sig = wasm_sig::<A>(&ty);
            CalleeInfo { sig, index: idx }
        };

        if import {
            Callee::Import(self.resolved_callees.entry(idx).or_insert_with(val))
        } else {
            Callee::Local(self.resolved_callees.entry(idx).or_insert_with(val))
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

    /// Resolves [GlobalData] of a global at the given index.
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

    /// Resolve a [HeapData] from a [MemoryIndex].
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
                    ty: if plan.memory.memory64 {
                        // TODO: Add support for 64-bit memories.
                        unimplemented!("memory64")
                    } else {
                        WasmValType::I32
                    },
                    min_size,
                    max_size,
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

fn heap_style_and_offset_guard_size(plan: &MemoryPlan) -> (HeapStyle, u64) {
    match plan {
        MemoryPlan {
            style: MemoryStyle::Static { bound },
            offset_guard_size,
            ..
        } => (
            HeapStyle::Static {
                bound: bound * u64::from(WASM_PAGE_SIZE),
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
        plan.memory
            .minimum
            .checked_mul(u64::from(WASM_PAGE_SIZE))
            .unwrap_or_else(|| {
                // 2^64 as a minimum doesn't fin in a 64 bit integer.
                // So in this case, the minimum is clamped to u64::MAX.
                u64::MAX
            }),
        plan.memory
            .maximum
            .and_then(|max| max.checked_mul(u64::from(WASM_PAGE_SIZE))),
    )
}
