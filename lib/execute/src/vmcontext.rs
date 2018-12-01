//! This file declares `VMContext` and several related structs which contain
//! fields that JIT code accesses directly.

use std::ptr::{size_of, align_of};

/// The main fields a JIT needs to access to utilize a WebAssembly linear,
/// memory, namely the start address and the size in bytes.
#[repr(C, packed)]
pub struct VMMemory {
    pub base: *mut u8,
    pub current_length: usize,
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test {
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmmemory_offsets() {
        let offsets = VMOffsets::new(size_of<*mut u8>());
        assert_eq!(size_of<VMMemory>(), offsets.size_of_vmmemory());
        assert_eq!(offset_of!(VMMemory, base), offsets.vmmemory_base());
        assert_eq!(offset_of!(VMMemory, current_length), offsets.vmmemory_current_length());
    }
}

impl VMMemory {
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

#[repr(C, packed, align(8))]
pub struct VMGlobal {
    pub storage: [u8; 8],
    // If more elements are added here, remember to add offset_of tests below!
}

/// The storage for a WebAssembly global.
#[cfg(test)]
mod test {
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmglobal_alignment() {
        assert!(align_of<VMGlobal>() <= align_of<i32>());
        assert!(align_of<VMGlobal>() >= align_of<i64>());
        assert!(align_of<VMGlobal>() >= align_of<f32>());
        assert!(align_of<VMGlobal>() >= align_of<f64>());
    }

    #[test]
    fn check_vmglobal_offsets() {
        let offsets = VMOffsets::new(size_of<*mut u8>());
        assert_eq!(size_of<VMGlobal>(), offsets.size_of_vmglobal());
    }
}

/// The main fields a JIT needs to access to utilize a WebAssembly table,
/// namely the start address and the number of elements.
#[repr(C, packed)]
pub struct VMTableStorage {
    pub base: *mut u8,
    pub current_elements: usize,
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test {
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmtable_offsets() {
        let offsets = VMOffsets::new(size_of<*mut u8>());
        assert_eq!(size_of<VMTableStorage>(), offsets.size_of_vmtable());
        assert_eq!(offset_of!(VMTableStorage, base), offsets.vmtable_base());
        assert_eq!(offset_of!(VMTableStorage, current_elements), offsets.vmtable_current_elements());
    }
}

impl VMTableStorage {
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

/// The VM "context", which is pointed to by the `vmctx` arg in Cranelift.
/// This has pointers to the globals, memories, tables, and other runtime
/// state associated with the current instance.
#[repr(C, packed)]
pub struct VMContext {
    /// A pointer to an array of `VMMemory` instances, indexed by
    /// WebAssembly memory index.
    pub memories: *mut VMMemory,
    /// A pointer to an array of globals.
    pub globals: *mut u8,
    /// A pointer to an array of `VMTableStorage` instances, indexed by
    /// WebAssembly table index.
    pub tables: *mut VMTableStorage,
    /// A pointer to extra runtime state that isn't directly accessed
    /// from JIT code.
    pub instance: *mut u8,
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test {
    use wasmtime_environ::VMOffsets;

    #[test]
    fn check_vmctx_offsets() {
        let offsets = VMOffsets::new(size_of<*mut u8>());
        assert_eq!(size_of<VMContext>(), offsets.size_of_vmctx());
        assert_eq!(offset_of!(VMContext, globals), offsets.vmctx_globals());
        assert_eq!(offset_of!(VMContext, memories), offsets.vmctx_memories());
        assert_eq!(offset_of!(VMContext, tables), offsets.vmctx_tables());
        assert_eq!(offset_of!(VMContext, instance), offsets.vmctx_instance());
    }
}

impl VMContext {
    unsafe pub fn global_storage(&mut self, index: usize) -> *mut u8 {
        globals.add(index * global_size)
    }

    unsafe pub fn global_i32(&mut self, index: usize) -> &mut i32 {
        self.global_storage(index) as &mut i32
    }

    unsafe pub fn global_i64(&mut self, index: usize) -> &mut i64 {
        self.global_storage(index) as &mut i64
    }

    unsafe pub fn global_f32(&mut self, index: usize) -> &mut f32 {
        self.global_storage(index) as &mut f32
    }

    unsafe pub fn global_f64(&mut self, index: usize) -> &mut f64 {
        self.global_storage(index) as &mut f64
    }

    unsafe pub fn memory(&mut self, index: usize) -> &mut VMMemory {
        memories.add(index) as &mut VMMemory
    }

    unsafe pub fn table(&mut self, index: usize) -> &mut VMTableStorage {
        tables.add(index) as &mut VMTableStorage
    }
}
