//! This file declares `VMContext` and several related structs which contain
//! fields that JIT code accesses directly.

use cranelift_entity::EntityRef;
use cranelift_wasm::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, Global, GlobalIndex,
    GlobalInit, MemoryIndex, TableIndex,
};
use instance::Instance;
use std::{mem, ptr, u32};

/// A placeholder byte-sized type which is just used to provide some amount of type
/// safety when dealing with pointers to JIT-compiled function bodies. Note that it's
/// deliberately not Copy, as we shouldn't be carelessly copying function body bytes
/// around.
#[repr(C)]
pub struct VMFunctionBody(u8);

#[cfg(test)]
mod test_vmfunction_body {
    use super::VMFunctionBody;
    use std::mem::size_of;

    #[test]
    fn check_vmfunction_body_offsets() {
        assert_eq!(size_of::<VMFunctionBody>(), 1);
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly table
/// imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableImport {
    /// A pointer to the imported table description.
    pub from: *mut VMTableDefinition,

    /// A pointer to the VMContext that owns the table description.
    pub vmctx: *mut VMContext,
}

#[cfg(test)]
mod test_vmtable_import {
    use super::VMTableImport;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmtable_import_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMTableImport>(),
            usize::from(offsets.size_of_vmtable_import())
        );
        assert_eq!(
            offset_of!(VMTableImport, from),
            usize::from(offsets.vmtable_import_from())
        );
        assert_eq!(
            offset_of!(VMTableImport, vmctx),
            usize::from(offsets.vmtable_import_vmctx())
        );
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly linear
/// memory imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMMemoryImport {
    /// A pointer to the imported memory description.
    pub from: *mut VMMemoryDefinition,

    /// A pointer to the VMContext that owns the memory description.
    pub vmctx: *mut VMContext,
}

#[cfg(test)]
mod test_vmmemory_import {
    use super::VMMemoryImport;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmmemory_import_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMMemoryImport>(),
            usize::from(offsets.size_of_vmmemory_import())
        );
        assert_eq!(
            offset_of!(VMMemoryImport, from),
            usize::from(offsets.vmmemory_import_from())
        );
        assert_eq!(
            offset_of!(VMMemoryImport, vmctx),
            usize::from(offsets.vmmemory_import_vmctx())
        );
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly global
/// variable imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMGlobalImport {
    /// A pointer to the imported global variable description.
    pub from: *mut VMGlobalDefinition,
}

#[cfg(test)]
mod test_vmglobal_import {
    use super::VMGlobalImport;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmglobal_import_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMGlobalImport>(),
            usize::from(offsets.size_of_vmglobal_import())
        );
        assert_eq!(
            offset_of!(VMGlobalImport, from),
            usize::from(offsets.vmglobal_import_from())
        );
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly linear
/// memory defined within the instance, namely the start address and the
/// size in bytes.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMMemoryDefinition {
    /// The start address.
    pub base: *mut u8,

    /// The current logical size of this linear memory in bytes.
    pub current_length: usize,
}

#[cfg(test)]
mod test_vmmemory_definition {
    use super::VMMemoryDefinition;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmmemory_definition_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMMemoryDefinition>(),
            usize::from(offsets.size_of_vmmemory_definition())
        );
        assert_eq!(
            offset_of!(VMMemoryDefinition, base),
            usize::from(offsets.vmmemory_definition_base())
        );
        assert_eq!(
            offset_of!(VMMemoryDefinition, current_length),
            usize::from(offsets.vmmemory_definition_current_length())
        );
        /* TODO: Assert that the size of `current_length` matches.
        assert_eq!(
            size_of::<VMMemoryDefinition::current_length>(),
            usize::from(offsets.size_of_vmmemory_definition_current_length())
        );
        */
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly table
/// defined within the instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableDefinition {
    /// Pointer to the table data.
    pub base: *mut u8,

    /// The current number of elements in the table.
    pub current_elements: usize,
}

#[cfg(test)]
mod test_vmtable_definition {
    use super::VMTableDefinition;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmtable_definition_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMTableDefinition>(),
            usize::from(offsets.size_of_vmtable_definition())
        );
        assert_eq!(
            offset_of!(VMTableDefinition, base),
            usize::from(offsets.vmtable_definition_base())
        );
        assert_eq!(
            offset_of!(VMTableDefinition, current_elements),
            usize::from(offsets.vmtable_definition_current_elements())
        );
    }
}

/// The storage for a WebAssembly global defined within the instance.
///
/// TODO: Pack the globals more densely, rather than using the same size
/// for every type.
#[derive(Debug, Copy, Clone)]
#[repr(C, align(8))]
pub struct VMGlobalDefinition {
    storage: [u8; 8],
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test_vmglobal_definition {
    use super::VMGlobalDefinition;
    use std::mem::{align_of, size_of};
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmglobal_definition_alignment() {
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<i32>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<i64>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<f32>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<f64>());
    }

    #[test]
    fn check_vmglobal_definition_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMGlobalDefinition>(),
            usize::from(offsets.size_of_vmglobal_definition())
        );
    }
}

impl VMGlobalDefinition {
    /// Construct a `VMGlobalDefinition`.
    pub fn new(global: &Global) -> Self {
        let mut result = Self { storage: [0; 8] };
        match global.initializer {
            GlobalInit::I32Const(x) => *unsafe { result.as_i32_mut() } = x,
            GlobalInit::I64Const(x) => *unsafe { result.as_i64_mut() } = x,
            GlobalInit::F32Const(x) => *unsafe { result.as_f32_bits_mut() } = x,
            GlobalInit::F64Const(x) => *unsafe { result.as_f64_bits_mut() } = x,
            GlobalInit::GetGlobal(_x) => unimplemented!("globals init with get_global"),
            GlobalInit::Import => panic!("attempting to initialize imported global"),
        }
        result
    }

    /// Return a reference to the value as an i32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i32(&self) -> &i32 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const i32)
    }

    /// Return a mutable reference to the value as an i32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i32_mut(&mut self) -> &mut i32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut i32)
    }

    /// Return a reference to the value as an i64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i64(&self) -> &i64 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const i64)
    }

    /// Return a mutable reference to the value as an i64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i64_mut(&mut self) -> &mut i64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut i64)
    }

    /// Return a reference to the value as an f32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32(&self) -> &f32 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const f32)
    }

    /// Return a mutable reference to the value as an f32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_mut(&mut self) -> &mut f32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut f32)
    }

    /// Return a reference to the value as f32 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_bits(&self) -> &u32 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const u32)
    }

    /// Return a mutable reference to the value as f32 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_bits_mut(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut u32)
    }

    /// Return a reference to the value as an f64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64(&self) -> &f64 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const f64)
    }

    /// Return a mutable reference to the value as an f64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_mut(&mut self) -> &mut f64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut f64)
    }

    /// Return a reference to the value as f64 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_bits(&self) -> &u64 {
        &*(self.storage.as_ref().as_ptr() as *const u8 as *const u64)
    }

    /// Return a mutable reference to the value as f64 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_bits_mut(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut u64)
    }
}

/// An index into the shared signature registry, usable for checking signatures
/// at indirect calls.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct VMSharedSignatureIndex(u32);

#[cfg(test)]
mod test_vmshared_signature_index {
    use super::VMSharedSignatureIndex;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmshared_signature_index() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMSharedSignatureIndex>(),
            usize::from(offsets.size_of_vmshared_signature_index())
        );
    }
}

impl VMSharedSignatureIndex {
    pub fn new(value: u32) -> Self {
        VMSharedSignatureIndex(value)
    }
}

/// The VM caller-checked "anyfunc" record, for caller-side signature checking.
/// It consists of the actual function pointer and a signature id to be checked
/// by the caller.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VMCallerCheckedAnyfunc {
    pub func_ptr: *const VMFunctionBody,
    pub type_index: VMSharedSignatureIndex,
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test_vmcaller_checked_anyfunc {
    use super::VMCallerCheckedAnyfunc;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmcaller_checked_anyfunc_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMCallerCheckedAnyfunc>(),
            usize::from(offsets.size_of_vmcaller_checked_anyfunc())
        );
        assert_eq!(
            offset_of!(VMCallerCheckedAnyfunc, func_ptr),
            usize::from(offsets.vmcaller_checked_anyfunc_func_ptr())
        );
        assert_eq!(
            offset_of!(VMCallerCheckedAnyfunc, type_index),
            usize::from(offsets.vmcaller_checked_anyfunc_type_index())
        );
    }
}

impl Default for VMCallerCheckedAnyfunc {
    fn default() -> Self {
        Self {
            func_ptr: ptr::null_mut(),
            type_index: VMSharedSignatureIndex::new(u32::MAX),
        }
    }
}

/// The VM "context", which is pointed to by the `vmctx` arg in Cranelift.
/// This has pointers to the globals, memories, tables, and other runtime
/// state associated with the current instance.
///
/// TODO: The number of memories, globals, tables, and signature IDs does
/// not change dynamically, and pointer arrays are not indexed dynamically,
/// so these fields could all be contiguously allocated.
#[derive(Debug)]
#[repr(C)]
pub struct VMContext {
    /// A pointer to an array of `*const VMFunctionBody` instances, indexed by `FuncIndex`.
    imported_functions: *const *const VMFunctionBody,

    /// A pointer to an array of `VMTableImport` instances, indexed by `TableIndex`.
    imported_tables: *mut VMTableImport,

    /// A pointer to an array of `VMMemoryImport` instances, indexed by `MemoryIndex`.
    imported_memories: *mut VMMemoryImport,

    /// A pointer to an array of `VMGlobalImport` instances, indexed by `GlobalIndex`.
    imported_globals: *mut VMGlobalImport,

    /// A pointer to an array of locally-defined `VMTableDefinition` instances,
    /// indexed by `DefinedTableIndex`.
    tables: *mut VMTableDefinition,

    /// A pointer to an array of locally-defined `VMMemoryDefinition` instances,
    /// indexed by `DefinedMemoryIndex`.
    memories: *mut VMMemoryDefinition,

    /// A pointer to an array of locally-defined `VMGlobalDefinition` instances,
    /// indexed by `DefinedGlobalIndex`.
    globals: *mut VMGlobalDefinition,

    /// Signature identifiers for signature-checking indirect calls.
    signature_ids: *mut VMSharedSignatureIndex,
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test {
    use super::VMContext;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmctx_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(size_of::<VMContext>(), usize::from(offsets.size_of_vmctx()));
        assert_eq!(
            offset_of!(VMContext, memories),
            usize::from(offsets.vmctx_memories())
        );
        assert_eq!(
            offset_of!(VMContext, globals),
            usize::from(offsets.vmctx_globals())
        );
        assert_eq!(
            offset_of!(VMContext, tables),
            usize::from(offsets.vmctx_tables())
        );
        assert_eq!(
            offset_of!(VMContext, signature_ids),
            usize::from(offsets.vmctx_signature_ids())
        );
    }
}

impl VMContext {
    /// Create a new `VMContext` instance.
    pub fn new(
        imported_functions: *const *const VMFunctionBody,
        imported_tables: *mut VMTableImport,
        imported_memories: *mut VMMemoryImport,
        imported_globals: *mut VMGlobalImport,
        tables: *mut VMTableDefinition,
        memories: *mut VMMemoryDefinition,
        globals: *mut VMGlobalDefinition,
        signature_ids: *mut VMSharedSignatureIndex,
    ) -> Self {
        Self {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            tables,
            memories,
            globals,
            signature_ids,
        }
    }

    /// Return a reference to imported function `index`.
    pub unsafe fn imported_function(&self, index: FuncIndex) -> *const VMFunctionBody {
        *self.imported_functions.add(index.index())
    }

    /// Return a reference to imported table `index`.
    pub unsafe fn imported_table(&self, index: TableIndex) -> &VMTableImport {
        &*self.imported_tables.add(index.index())
    }

    /// Return a mutable reference to imported table `index`.
    pub unsafe fn imported_table_mut(&mut self, index: TableIndex) -> &mut VMTableImport {
        &mut *self.imported_tables.add(index.index())
    }

    /// Return a reference to imported memory `index`.
    pub unsafe fn imported_memory(&self, index: MemoryIndex) -> &VMMemoryImport {
        &*self.imported_memories.add(index.index())
    }

    /// Return a mutable reference to imported memory `index`.
    pub unsafe fn imported_memory_mut(&mut self, index: MemoryIndex) -> &mut VMMemoryImport {
        &mut *self.imported_memories.add(index.index())
    }

    /// Return a reference to imported global `index`.
    pub unsafe fn imported_global(&self, index: GlobalIndex) -> &VMGlobalImport {
        &*self.imported_globals.add(index.index())
    }

    /// Return a mutable reference to imported global `index`.
    pub unsafe fn imported_global_mut(&mut self, index: GlobalIndex) -> &mut VMGlobalImport {
        &mut *self.imported_globals.add(index.index())
    }

    /// Return a reference to locally-defined table `index`.
    pub unsafe fn table(&self, index: DefinedTableIndex) -> &VMTableDefinition {
        &*self.tables.add(index.index())
    }

    /// Return a mutable reference to locally-defined table `index`.
    pub unsafe fn table_mut(&mut self, index: DefinedTableIndex) -> &mut VMTableDefinition {
        &mut *self.tables.add(index.index())
    }

    /// Return a reference to locally-defined linear memory `index`.
    pub unsafe fn memory(&self, index: DefinedMemoryIndex) -> &VMMemoryDefinition {
        &*self.memories.add(index.index())
    }

    /// Return a mutable reference to locally-defined linear memory `index`.
    pub unsafe fn memory_mut(&mut self, index: DefinedMemoryIndex) -> &mut VMMemoryDefinition {
        &mut *self.memories.add(index.index())
    }

    /// Return a reference to locally-defined global variable `index`.
    pub unsafe fn global(&self, index: DefinedGlobalIndex) -> &VMGlobalDefinition {
        &*self.globals.add(index.index())
    }

    /// Return a mutable reference to locally-defined global variable `index`.
    pub unsafe fn global_mut(&mut self, index: DefinedGlobalIndex) -> &mut VMGlobalDefinition {
        &mut *self.globals.add(index.index())
    }

    /// Return a mutable reference to the associated `Instance`.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn instance(&mut self) -> &mut Instance {
        &mut *((self as *mut Self as *mut u8).offset(-Instance::vmctx_offset()) as *mut Instance)
    }

    /// Return the memory index for the given `VMMemoryDefinition`.
    pub fn memory_index(&self, memory: &mut VMMemoryDefinition) -> DefinedMemoryIndex {
        // TODO: Use `offset_from` once it stablizes.
        let begin = self.memories;
        let end: *mut VMMemoryDefinition = memory;
        DefinedMemoryIndex::new(
            (end as usize - begin as usize) / mem::size_of::<VMMemoryDefinition>(),
        )
    }
}
