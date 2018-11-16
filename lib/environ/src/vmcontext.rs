/// The main fields a JIT needs to access to utilize a WebAssembly linear,
/// memory, namely the start address and the size in bytes.
#[repr(C, packed)]
pub struct VMMemory {
    pub base: *mut u8,
    pub current_length: usize,
}

/// The main fields a JIT needs to access to utilize a WebAssembly table,
/// namely the start address and the number of elements.
#[repr(C, packed)]
pub struct VMTable {
    pub base: *mut u8,
    pub current_num_elements: usize,
}

/// The VM "context", which is pointed to by the `vmctx` arg in Cranelift.
/// This has pointers to the globals, memories, tables, and other runtime
/// state associated with the current instance.
#[repr(C, packed)]
pub struct VMContext {
    /// A pointer to an array of globals.
    pub globals: *mut u8,
    /// A pointer to an array of `VMMemory` instances, indexed by
    /// WebAssembly memory index.
    pub memories: *mut VMMemory,
    /// A pointer to an array of `VMTable` instances, indexed by
    /// WebAssembly table index.
    pub tables: *mut VMTable,
    /// A pointer to extra runtime state that isn't directly accessed
    /// from JIT code.
    pub instance: *mut u8,
}
