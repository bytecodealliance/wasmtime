//! This file declares `VMContext` and several related structs which contain
//! fields that JIT code accesses directly.

use cranelift_entity::EntityRef;
use cranelift_wasm::{GlobalIndex, MemoryIndex, TableIndex};
use instance::Instance;
use std::mem::size_of;
use std::ptr;
use std::slice;

/// The main fields a JIT needs to access to utilize a WebAssembly linear,
/// memory, namely the start address and the size in bytes.
#[derive(Debug)]
#[repr(C)]
pub struct VMMemory {
    base: *mut u8,
    current_length: usize,
    // If more elements are added here, remember to add offset_of tests below!
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
        assert_eq!(
            offset_of!(VMMemory, base),
            usize::from(offsets.vmmemory_base())
        );
        assert_eq!(
            offset_of!(VMMemory, current_length),
            usize::from(offsets.vmmemory_current_length())
        );
    }
}

impl VMMemory {
    pub fn new(base: *mut u8, current_length: usize) -> Self {
        Self {
            base,
            current_length,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base, self.current_length) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.base, self.current_length) }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.base
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.base
    }

    pub fn len(&self) -> usize {
        self.current_length
    }
}

/// The storage for a WebAssembly global.
///
/// TODO: Pack the globals more densely, rather than using the same size
/// for every type.
#[derive(Debug, Clone)]
#[repr(C, align(8))]
pub struct VMGlobal {
    storage: [u8; 8],
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test_vmglobal {
    use super::VMGlobal;
    use std::mem::{align_of, size_of};
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmglobal_alignment() {
        assert!(align_of::<VMGlobal>() >= align_of::<i32>());
        assert!(align_of::<VMGlobal>() >= align_of::<i64>());
        assert!(align_of::<VMGlobal>() >= align_of::<f32>());
        assert!(align_of::<VMGlobal>() >= align_of::<f64>());
    }

    #[test]
    fn check_vmglobal_offsets() {
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8);
        assert_eq!(
            size_of::<VMGlobal>(),
            usize::from(offsets.size_of_vmglobal())
        );
    }
}

impl Default for VMGlobal {
    fn default() -> Self {
        VMGlobal { storage: [0; 8] }
    }
}

#[derive(Debug)]
/// The main fields a JIT needs to access to utilize a WebAssembly table,
/// namely the start address and the number of elements.
#[repr(C)]
pub struct VMTable {
    base: *mut u8,
    current_elements: usize,
    // If more elements are added here, remember to add offset_of tests below!
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
        assert_eq!(
            offset_of!(VMTable, base),
            usize::from(offsets.vmtable_base())
        );
        assert_eq!(
            offset_of!(VMTable, current_elements),
            usize::from(offsets.vmtable_current_elements())
        );
    }
}

impl VMTable {
    pub fn new(base: *mut u8, current_elements: usize) -> Self {
        Self {
            base,
            current_elements,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base, self.current_elements) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.base, self.current_elements) }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.base
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.base
    }

    pub fn len(&self) -> usize {
        self.current_elements
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
    pub unsafe fn global_storage(&mut self, index: GlobalIndex) -> *mut VMGlobal {
        self.globals.add(index.index() * size_of::<VMGlobal>())
    }

    /// Return a mutable reference to global `index` which has type i32.
    pub unsafe fn global_i32(&mut self, index: GlobalIndex) -> &mut i32 {
        &mut *(self.global_storage(index) as *mut i32)
    }

    /// Return a mutable reference to global `index` which has type i64.
    pub unsafe fn global_i64(&mut self, index: GlobalIndex) -> &mut i64 {
        &mut *(self.global_storage(index) as *mut i64)
    }

    /// Return a mutable reference to global `index` which has type f32.
    pub unsafe fn global_f32(&mut self, index: GlobalIndex) -> &mut f32 {
        &mut *(self.global_storage(index) as *mut f32)
    }

    /// Return a mutable reference to global `index` which has type f64.
    pub unsafe fn global_f64(&mut self, index: GlobalIndex) -> &mut f64 {
        &mut *(self.global_storage(index) as *mut f64)
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
