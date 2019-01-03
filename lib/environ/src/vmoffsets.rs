//! Offsets and sizes of various structs in wasmtime-runtime's vmcontext
//! module.

use cranelift_codegen::ir;
use cranelift_wasm::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GlobalIndex, MemoryIndex,
    SignatureIndex, TableIndex,
};
use module::Module;

/// This class computes offsets to fields within `VMContext` and other
/// related structs that JIT code accesses directly.
pub struct VMOffsets {
    /// The size in bytes of a pointer on the target.
    pub pointer_size: u8,
    /// The number of signature declarations in the module.
    pub num_signature_ids: u64,
    /// The number of imported functions in the module.
    pub num_imported_functions: u64,
    /// The number of imported tables in the module.
    pub num_imported_tables: u64,
    /// The number of imported memories in the module.
    pub num_imported_memories: u64,
    /// The number of imported globals in the module.
    pub num_imported_globals: u64,
    /// The number of defined tables in the module.
    pub num_defined_tables: u64,
    /// The number of defined memories in the module.
    pub num_defined_memories: u64,
    /// The number of defined globals in the module.
    pub num_defined_globals: u64,
}

impl VMOffsets {
    /// Return a new `VMOffsets` instance, for a given pointer size.
    pub fn new(pointer_size: u8, module: &Module) -> Self {
        Self {
            pointer_size,
            num_signature_ids: module.signatures.len() as u64,
            num_imported_functions: module.imported_funcs.len() as u64,
            num_imported_tables: module.imported_tables.len() as u64,
            num_imported_memories: module.imported_memories.len() as u64,
            num_imported_globals: module.imported_globals.len() as u64,
            num_defined_tables: module.table_plans.len() as u64,
            num_defined_memories: module.memory_plans.len() as u64,
            num_defined_globals: module.globals.len() as u64,
        }
    }
}

/// Offsets for `VMFunctionImport`.
impl VMOffsets {
    /// The offset of the `body` field.
    #[allow(clippy::erasing_op)]
    pub fn vmfunction_import_body(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `vmctx` field.
    #[allow(clippy::identity_op)]
    pub fn vmfunction_import_vmctx(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMFunctionImport`.
    pub fn size_of_vmfunction_import(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `*const VMFunctionBody`.
impl VMOffsets {
    /// The size of the `current_elements` field.
    #[allow(clippy::identity_op)]
    pub fn size_of_vmfunction_body_ptr(&self) -> u8 {
        1 * self.pointer_size
    }
}

/// Offsets for `VMTableImport`.
impl VMOffsets {
    /// The offset of the `from` field.
    #[allow(clippy::erasing_op)]
    pub fn vmtable_import_from(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `vmctx` field.
    #[allow(clippy::identity_op)]
    pub fn vmtable_import_vmctx(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMTableImport`.
    pub fn size_of_vmtable_import(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `VMTableDefinition`.
impl VMOffsets {
    /// The offset of the `base` field.
    #[allow(clippy::erasing_op)]
    pub fn vmtable_definition_base(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `current_elements` field.
    #[allow(clippy::identity_op)]
    pub fn vmtable_definition_current_elements(&self) -> u8 {
        1 * self.pointer_size
    }

    /// The size of the `current_elements` field.
    pub fn size_of_vmtable_definition_current_elements(&self) -> u8 {
        4
    }

    /// Return the size of `VMTableDefinition`.
    pub fn size_of_vmtable_definition(&self) -> u8 {
        2 * self.pointer_size
    }

    /// The type of the `current_elements` field.
    pub fn type_of_vmtable_definition_current_elements(&self) -> ir::Type {
        ir::Type::int(u16::from(self.size_of_vmtable_definition_current_elements()) * 8).unwrap()
    }
}

/// Offsets for `VMMemoryImport`.
impl VMOffsets {
    /// The offset of the `from` field.
    #[allow(clippy::erasing_op)]
    pub fn vmmemory_import_from(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `vmctx` field.
    #[allow(clippy::identity_op)]
    pub fn vmmemory_import_vmctx(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMMemoryImport`.
    pub fn size_of_vmmemory_import(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `VMMemoryDefinition`.
impl VMOffsets {
    /// The offset of the `base` field.
    #[allow(clippy::erasing_op)]
    pub fn vmmemory_definition_base(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `current_length` field.
    #[allow(clippy::identity_op)]
    pub fn vmmemory_definition_current_length(&self) -> u8 {
        1 * self.pointer_size
    }

    /// The size of the `current_length` field.
    pub fn size_of_vmmemory_definition_current_length(&self) -> u8 {
        4
    }

    /// Return the size of `VMMemoryDefinition`.
    pub fn size_of_vmmemory_definition(&self) -> u8 {
        2 * self.pointer_size
    }

    /// The type of the `current_length` field.
    pub fn type_of_vmmemory_definition_current_length(&self) -> ir::Type {
        ir::Type::int(u16::from(self.size_of_vmmemory_definition_current_length()) * 8).unwrap()
    }
}

/// Offsets for `VMGlobalImport`.
impl VMOffsets {
    /// The offset of the `from` field.
    #[allow(clippy::erasing_op)]
    pub fn vmglobal_import_from(&self) -> u8 {
        0 * self.pointer_size
    }

    /// Return the size of `VMGlobalImport`.
    #[allow(clippy::identity_op)]
    pub fn size_of_vmglobal_import(&self) -> u8 {
        1 * self.pointer_size
    }
}

/// Offsets for `VMGlobalDefinition`.
impl VMOffsets {
    /// Return the size of `VMGlobalDefinition`.
    pub fn size_of_vmglobal_definition(&self) -> u8 {
        8
    }
}

/// Offsets for `VMSharedSignatureIndex`.
impl VMOffsets {
    /// Return the size of `VMSharedSignatureIndex`.
    pub fn size_of_vmshared_signature_index(&self) -> u8 {
        4
    }
}

/// Offsets for `VMCallerCheckedAnyfunc`.
impl VMOffsets {
    /// The offset of the `func_ptr` field.
    #[allow(clippy::erasing_op)]
    pub fn vmcaller_checked_anyfunc_func_ptr(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `type_index` field.
    #[allow(clippy::identity_op)]
    pub fn vmcaller_checked_anyfunc_type_index(&self) -> u8 {
        1 * self.pointer_size
    }

    /// The offset of the `vmctx` field.
    pub fn vmcaller_checked_anyfunc_vmctx(&self) -> u8 {
        2 * self.pointer_size
    }

    /// Return the size of `VMCallerCheckedAnyfunc`.
    pub fn size_of_vmcaller_checked_anyfunc(&self) -> u8 {
        3 * self.pointer_size
    }
}

/// Offsets for `VMContext`.
impl VMOffsets {
    /// The offset of the `signature_ids` array.
    pub fn vmctx_signature_ids_begin(&self) -> u64 {
        0
    }

    /// The offset of the `tables` array.
    #[allow(clippy::erasing_op)]
    pub fn vmctx_imported_functions_begin(&self) -> u64 {
        self.vmctx_signature_ids_begin()
            + self.num_signature_ids * u64::from(self.size_of_vmshared_signature_index())
    }

    /// The offset of the `tables` array.
    #[allow(clippy::identity_op)]
    pub fn vmctx_imported_tables_begin(&self) -> u64 {
        self.vmctx_imported_functions_begin()
            + self.num_imported_functions * u64::from(self.size_of_vmfunction_import())
    }

    /// The offset of the `memories` array.
    pub fn vmctx_imported_memories_begin(&self) -> u64 {
        self.vmctx_imported_tables_begin()
            + self.num_imported_tables * u64::from(self.size_of_vmtable_import())
    }

    /// The offset of the `globals` array.
    pub fn vmctx_imported_globals_begin(&self) -> u64 {
        self.vmctx_imported_memories_begin()
            + self.num_imported_memories * u64::from(self.size_of_vmmemory_import())
    }

    /// The offset of the `tables` array.
    pub fn vmctx_tables_begin(&self) -> u64 {
        self.vmctx_imported_globals_begin()
            + self.num_imported_globals * u64::from(self.size_of_vmglobal_import())
    }

    /// The offset of the `memories` array.
    pub fn vmctx_memories_begin(&self) -> u64 {
        self.vmctx_tables_begin()
            + self.num_defined_tables * u64::from(self.size_of_vmtable_definition())
    }

    /// The offset of the `globals` array.
    pub fn vmctx_globals_begin(&self) -> u64 {
        self.vmctx_memories_begin()
            + self.num_defined_memories * u64::from(self.size_of_vmmemory_definition())
    }

    /// Return the size of the `VMContext` allocation.
    #[allow(dead_code)]
    pub fn size_of_vmctx(&self) -> u64 {
        self.vmctx_globals_begin()
            + self.num_defined_globals * u64::from(self.size_of_vmglobal_definition())
    }

    /// Return the offset to `VMSharedSignatureId` index `index`.
    pub fn vmctx_vmshared_signature_id(&self, index: SignatureIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_signature_ids);
        self.vmctx_signature_ids_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmshared_signature_index()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMFunctionImport` index `index`.
    pub fn vmctx_vmfunction_import(&self, index: FuncIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_imported_functions);
        self.vmctx_imported_functions_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmfunction_import()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMTableImport` index `index`.
    pub fn vmctx_vmtable_import(&self, index: TableIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_imported_tables);
        self.vmctx_imported_tables_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmtable_import()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMMemoryImport` index `index`.
    pub fn vmctx_vmmemory_import(&self, index: MemoryIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_imported_memories);
        self.vmctx_imported_memories_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmmemory_import()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMGlobalImport` index `index`.
    pub fn vmctx_vmglobal_import(&self, index: GlobalIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_imported_globals);
        self.vmctx_imported_globals_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmglobal_import()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMTableDefinition` index `index`.
    pub fn vmctx_vmtable_definition(&self, index: DefinedTableIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_defined_tables);
        self.vmctx_tables_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmtable_definition()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to `VMMemoryDefinition` index `index`.
    pub fn vmctx_vmmemory_definition(&self, index: DefinedMemoryIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_defined_memories);
        self.vmctx_memories_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmmemory_definition()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to the `VMGlobalDefinition` index `index`.
    pub fn vmctx_vmglobal_definition(&self, index: DefinedGlobalIndex) -> u64 {
        assert!(u64::from(index.as_u32()) < self.num_defined_globals);
        self.vmctx_globals_begin()
            .checked_add(
                u64::from(index.as_u32())
                    .checked_mul(u64::from(self.size_of_vmglobal_definition()))
                    .unwrap(),
            )
            .unwrap()
    }

    /// Return the offset to the `body` field in `*const VMFunctionBody` index `index`.
    pub fn vmctx_vmfunction_import_body(&self, index: FuncIndex) -> u64 {
        self.vmctx_vmfunction_import(index)
            .checked_add(u64::from(self.vmfunction_import_body()))
            .unwrap()
    }

    /// Return the offset to the `vmctx` field in `*const VMFunctionBody` index `index`.
    pub fn vmctx_vmfunction_import_vmctx(&self, index: FuncIndex) -> u64 {
        self.vmctx_vmfunction_import(index)
            .checked_add(u64::from(self.vmfunction_import_vmctx()))
            .unwrap()
    }

    /// Return the offset to the `from` field in `VMTableImport` index `index`.
    pub fn vmctx_vmtable_import_from(&self, index: TableIndex) -> u64 {
        self.vmctx_vmtable_import(index)
            .checked_add(u64::from(self.vmtable_import_from()))
            .unwrap()
    }

    /// Return the offset to the `base` field in `VMTableDefinition` index `index`.
    pub fn vmctx_vmtable_definition_base(&self, index: DefinedTableIndex) -> u64 {
        self.vmctx_vmtable_definition(index)
            .checked_add(u64::from(self.vmtable_definition_base()))
            .unwrap()
    }

    /// Return the offset to the `current_elements` field in `VMTableDefinition` index `index`.
    pub fn vmctx_vmtable_definition_current_elements(&self, index: DefinedTableIndex) -> u64 {
        self.vmctx_vmtable_definition(index)
            .checked_add(u64::from(self.vmtable_definition_current_elements()))
            .unwrap()
    }

    /// Return the offset to the `from` field in `VMMemoryImport` index `index`.
    pub fn vmctx_vmmemory_import_from(&self, index: MemoryIndex) -> u64 {
        self.vmctx_vmmemory_import(index)
            .checked_add(u64::from(self.vmmemory_import_from()))
            .unwrap()
    }

    /// Return the offset to the `vmctx` field in `VMMemoryImport` index `index`.
    pub fn vmctx_vmmemory_import_vmctx(&self, index: MemoryIndex) -> u64 {
        self.vmctx_vmmemory_import(index)
            .checked_add(u64::from(self.vmmemory_import_vmctx()))
            .unwrap()
    }

    /// Return the offset to the `base` field in `VMMemoryDefinition` index `index`.
    pub fn vmctx_vmmemory_definition_base(&self, index: DefinedMemoryIndex) -> u64 {
        self.vmctx_vmmemory_definition(index)
            .checked_add(u64::from(self.vmmemory_definition_base()))
            .unwrap()
    }

    /// Return the offset to the `current_length` field in `VMMemoryDefinition` index `index`.
    pub fn vmctx_vmmemory_definition_current_length(&self, index: DefinedMemoryIndex) -> u64 {
        self.vmctx_vmmemory_definition(index)
            .checked_add(u64::from(self.vmmemory_definition_current_length()))
            .unwrap()
    }

    /// Return the offset to the `from` field in `VMGlobalImport` index `index`.
    pub fn vmctx_vmglobal_import_from(&self, index: GlobalIndex) -> u64 {
        self.vmctx_vmglobal_import(index)
            .checked_add(u64::from(self.vmglobal_import_from()))
            .unwrap()
    }
}
