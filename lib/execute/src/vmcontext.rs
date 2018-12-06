//! This file declares `VMContext` and several related structs which contain
//! fields that JIT code accesses directly.

use cranelift_entity::EntityRef;
use cranelift_wasm::{Global, GlobalIndex, GlobalInit, MemoryIndex, TableIndex};
use instance::Instance;
use std::fmt;
use std::ptr;

/// The fields a JIT needs to access to utilize a WebAssembly linear
/// memory defined within the instance, namely the start address and the
/// size in bytes.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMMemoryDefinition {
    /// The start address.
    base: *mut u8,
    /// The current size of linear memory in bytes.
    current_length: usize,
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
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly linear
/// memory imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMMemoryImport {
    /// A pointer to the imported memory description.
    from: *mut VMMemoryDefinition,
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
    }
}

/// The main fields a JIT needs to access to utilize a WebAssembly linear
/// memory. It must know whether the memory is defined within the instance
/// or imported.
#[repr(C)]
pub union VMMemory {
    /// A linear memory defined within the instance.
    definition: VMMemoryDefinition,

    /// An imported linear memory.
    import: VMMemoryImport,
}

#[cfg(test)]
mod test_vmmemory {
    use super::VMMemory;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmmemory_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMMemory>(),
            usize::from(offsets.size_of_vmmemory())
        );
    }
}

impl VMMemory {
    /// Construct a `VMMemoryDefinition` variant of `VMMemory`.
    pub fn definition(base: *mut u8, current_length: usize) -> Self {
        Self {
            definition: VMMemoryDefinition {
                base,
                current_length,
            },
        }
    }

    /// Construct a `VMMemoryImmport` variant of `VMMemory`.
    pub fn import(from: *mut VMMemoryDefinition) -> Self {
        Self {
            import: VMMemoryImport { from },
        }
    }

    /// Get the underlying `VMMemoryDefinition`.
    pub unsafe fn get_definition(&mut self, is_import: bool) -> &mut VMMemoryDefinition {
        if is_import {
            &mut *self.import.from
        } else {
            &mut self.definition
        }
    }
}

impl fmt::Debug for VMMemory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VMMemory {{")?;
        write!(f, "    definition: {:?},", unsafe { self.definition })?;
        write!(f, "    import: {:?},", unsafe { self.import })?;
        write!(f, "}}")?;
        Ok(())
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
    pub unsafe fn as_i32(&mut self) -> &mut i32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut i32)
    }

    pub unsafe fn as_i64(&mut self) -> &mut i64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut i64)
    }

    pub unsafe fn as_f32(&mut self) -> &mut f32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut f32)
    }

    pub unsafe fn as_f32_bits(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut u32)
    }

    pub unsafe fn as_f64(&mut self) -> &mut f64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut f64)
    }

    pub unsafe fn as_f64_bits(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr() as *mut u8 as *mut u64)
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly global
/// variable imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMGlobalImport {
    /// A pointer to the imported global variable description.
    from: *mut VMGlobalDefinition,
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

/// The main fields a JIT needs to access to utilize a WebAssembly global
/// variable. It must know whether the global variable is defined within the
/// instance or imported.
#[repr(C)]
pub union VMGlobal {
    /// A global variable defined within the instance.
    definition: VMGlobalDefinition,

    /// An imported global variable.
    import: VMGlobalImport,
}

#[cfg(test)]
mod test_vmglobal {
    use super::VMGlobal;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmglobal_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMGlobal>(),
            usize::from(offsets.size_of_vmglobal())
        );
    }
}

impl VMGlobal {
    /// Construct a `VMGlobalDefinition` variant of `VMGlobal`.
    pub fn definition(global: &Global) -> Self {
        let mut result = VMGlobalDefinition { storage: [0; 8] };
        unsafe {
            match global.initializer {
                GlobalInit::I32Const(x) => *result.as_i32() = x,
                GlobalInit::I64Const(x) => *result.as_i64() = x,
                GlobalInit::F32Const(x) => *result.as_f32_bits() = x,
                GlobalInit::F64Const(x) => *result.as_f64_bits() = x,
                GlobalInit::GetGlobal(_x) => unimplemented!("globals init with get_global"),
                GlobalInit::Import => panic!("attempting to initialize imported global"),
            }
        }
        Self { definition: result }
    }

    /// Construct a `VMGlobalImmport` variant of `VMGlobal`.
    pub fn import(from: *mut VMGlobalDefinition) -> Self {
        Self {
            import: VMGlobalImport { from },
        }
    }

    /// Get the underlying `VMGlobalDefinition`.
    pub unsafe fn get_definition(&mut self, is_import: bool) -> &mut VMGlobalDefinition {
        if is_import {
            &mut *self.import.from
        } else {
            &mut self.definition
        }
    }
}

impl fmt::Debug for VMGlobal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VMGlobal {{")?;
        write!(f, "    definition: {:?},", unsafe { self.definition })?;
        write!(f, "    import: {:?},", unsafe { self.import })?;
        write!(f, "}}")?;
        Ok(())
    }
}

/// The fields a JIT needs to access to utilize a WebAssembly table
/// defined within the instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableDefinition {
    base: *mut u8,
    current_elements: usize,
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

/// The fields a JIT needs to access to utilize a WebAssembly table
/// imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableImport {
    /// A pointer to the imported table description.
    from: *mut VMTableDefinition,
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
    }
}

/// The main fields a JIT needs to access to utilize a WebAssembly table.
/// It must know whether the table is defined within the instance
/// or imported.
#[repr(C)]
pub union VMTable {
    /// A table defined within the instance.
    definition: VMTableDefinition,

    /// An imported table.
    import: VMTableImport,
}

#[cfg(test)]
mod test_vmtable {
    use super::VMTable;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmtable_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(size_of::<VMTable>(), usize::from(offsets.size_of_vmtable()));
    }
}

impl VMTable {
    /// Construct a `VMTableDefinition` variant of `VMTable`.
    pub fn definition(base: *mut u8, current_elements: usize) -> Self {
        Self {
            definition: VMTableDefinition {
                base,
                current_elements,
            },
        }
    }

    /// Construct a `VMTableImmport` variant of `VMTable`.
    pub fn import(from: *mut VMTableDefinition) -> Self {
        Self {
            import: VMTableImport { from },
        }
    }

    /// Get the underlying `VMTableDefinition`.
    pub unsafe fn get_definition(&mut self, is_import: bool) -> &mut VMTableDefinition {
        if is_import {
            &mut *self.import.from
        } else {
            &mut self.definition
        }
    }
}

impl fmt::Debug for VMTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VMTable {{")?;
        write!(f, "    definition: {:?},", unsafe { self.definition })?;
        write!(f, "    import: {:?},", unsafe { self.import })?;
        write!(f, "}}")?;
        Ok(())
    }
}

/// The type of the `type_id` field in `VMCallerCheckedAnyfunc`.
pub type VMSignatureId = u32;

#[cfg(test)]
mod test_vmsignature_id {
    use super::VMSignatureId;
    use std::mem::size_of;
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmcaller_checked_anyfunc_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMSignatureId>(),
            usize::from(offsets.size_of_vmsignature_id())
        );
    }
}

/// The VM caller-checked "anyfunc" record, for caller-side signature checking.
/// It consists of the actual function pointer and a signature id to be checked
/// by the caller.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VMCallerCheckedAnyfunc {
    pub func_ptr: *const u8,
    pub type_id: VMSignatureId,
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
            offset_of!(VMCallerCheckedAnyfunc, type_id),
            usize::from(offsets.vmcaller_checked_anyfunc_type_id())
        );
    }
}

impl Default for VMCallerCheckedAnyfunc {
    fn default() -> Self {
        Self {
            func_ptr: ptr::null_mut(),
            type_id: 0,
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
    /// A pointer to an array of `VMMemory` instances, indexed by
    /// WebAssembly memory index.
    memories: *mut VMMemory,
    /// A pointer to an array of globals.
    globals: *mut VMGlobal,
    /// A pointer to an array of `VMTable` instances, indexed by
    /// WebAssembly table index.
    tables: *mut VMTable,
    /// Signature identifiers for signature-checking indirect calls.
    signature_ids: *mut u32,
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
        memories: *mut VMMemory,
        globals: *mut VMGlobal,
        tables: *mut VMTable,
        signature_ids: *mut u32,
    ) -> Self {
        Self {
            memories,
            globals,
            tables,
            signature_ids,
        }
    }

    /// Return the base pointer of the globals array.
    pub unsafe fn global(&mut self, index: GlobalIndex) -> &mut VMGlobal {
        &mut *self.globals.add(index.index())
    }

    /// Return a mutable reference to linear memory `index`.
    pub unsafe fn memory(&mut self, index: MemoryIndex) -> &mut VMMemory {
        &mut *self.memories.add(index.index())
    }

    /// Return a mutable reference to table `index`.
    pub unsafe fn table(&mut self, index: TableIndex) -> &mut VMTable {
        &mut *self.tables.add(index.index())
    }

    /// Return a mutable reference to the associated `Instance`.
    pub unsafe fn instance(&mut self) -> &mut Instance {
        &mut *((self as *mut VMContext as *mut u8).offset(-Instance::vmctx_offset())
            as *mut Instance)
    }
}
