/// This class computes offsets to fields within `VMContext` and other
/// related structs that JIT code accesses directly.
pub struct VMOffsets {
    pointer_size: u8,
}

impl VMOffsets {
    /// Return a new `VMOffsets` instance, for a given pointer size.
    pub fn new(pointer_size: u8) -> Self {
        Self { pointer_size }
    }
}

/// Offsets for `wasmtime_execute::VMMemory`.
impl VMOffsets {
    /// The offset of the `base` field.
    pub fn vmmemory_base(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `current_length` field.
    pub fn vmmemory_current_length(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMMemory`.
    pub fn size_of_vmmemory(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `wasmtime_execute::VMGlobal`.
impl VMOffsets {
    /// Return the size of `VMGlobal`.
    pub fn size_of_vmglobal(&self) -> u8 {
        8
    }
}

/// Offsets for `wasmtime_execute::VMTable`.
impl VMOffsets {
    /// The offset of the `base` field.
    pub fn vmtable_base(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `current_elements` field.
    pub fn vmtable_current_elements(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMTable`.
    pub fn size_of_vmtable(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `wasmtime_execute::VMSignatureId`.
impl VMOffsets {
    /// Return the size of `VMSignatureId`.
    pub fn size_of_vmsignature_id(&self) -> u8 {
        4
    }
}

/// Offsets for `wasmtime_execute::VMCallerCheckedAnyfunc`.
impl VMOffsets {
    /// The offset of the `func_ptr` field.
    pub fn vmcaller_checked_anyfunc_func_ptr(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `type_id` field.
    pub fn vmcaller_checked_anyfunc_type_id(&self) -> u8 {
        1 * self.pointer_size
    }

    /// Return the size of `VMTable`.
    pub fn size_of_vmcaller_checked_anyfunc(&self) -> u8 {
        2 * self.pointer_size
    }
}

/// Offsets for `wasmtime_execute::VMContext`.
impl VMOffsets {
    /// The offset of the `memories` field.
    pub fn vmctx_memories(&self) -> u8 {
        0 * self.pointer_size
    }

    /// The offset of the `globals` field.
    pub fn vmctx_globals(&self) -> u8 {
        1 * self.pointer_size
    }

    /// The offset of the `tables` field.
    pub fn vmctx_tables(&self) -> u8 {
        2 * self.pointer_size
    }

    /// The offset of the `signature_ids` field.
    pub fn vmctx_signature_ids(&self) -> u8 {
        3 * self.pointer_size
    }

    /// Return the size of `VMContext`.
    #[allow(dead_code)]
    pub fn size_of_vmctx(&self) -> u8 {
        4 * self.pointer_size
    }

    /// Return the offset from the `memories` pointer to `VMMemory` index `index`.
    pub fn index_vmmemory(&self, index: u32) -> i32 {
        cast::i32(
            index
                .checked_mul(u32::from(self.size_of_vmmemory()))
                .unwrap(),
        )
        .unwrap()
    }

    /// Return the offset from the `globals` pointer to `VMGlobal` index `index`.
    pub fn index_vmglobal(&self, index: u32) -> i32 {
        cast::i32(
            index
                .checked_mul(u32::from(self.size_of_vmglobal()))
                .unwrap(),
        )
        .unwrap()
    }

    /// Return the offset from the `tables` pointer to `VMTable` index `index`.
    pub fn index_vmtable(&self, index: u32) -> i32 {
        cast::i32(
            index
                .checked_mul(u32::from(self.size_of_vmtable()))
                .unwrap(),
        )
        .unwrap()
    }

    /// Return the offset from the `memories` pointer to the `base` field in
    /// `VMMemory` index `index`.
    pub fn index_vmmemory_base(&self, index: u32) -> i32 {
        self.index_vmmemory(index)
            .checked_add(i32::from(self.vmmemory_base()))
            .unwrap()
    }

    /// Return the offset from the `memories` pointer to the `current_length` field in
    /// `VMMemory` index `index`.
    pub fn index_vmmemory_current_length(&self, index: u32) -> i32 {
        self.index_vmmemory(index)
            .checked_add(i32::from(self.vmmemory_current_length()))
            .unwrap()
    }

    /// Return the offset from the `tables` pointer to the `base` field in
    /// `VMTable` index `index`.
    pub fn index_vmtable_base(&self, index: u32) -> i32 {
        self.index_vmtable(index)
            .checked_add(i32::from(self.vmtable_base()))
            .unwrap()
    }

    /// Return the offset from the `tables` pointer to the `current_elements` field in
    /// `VMTable` index `index`.
    pub fn index_vmtable_current_elements(&self, index: u32) -> i32 {
        self.index_vmtable(index)
            .checked_add(i32::from(self.vmtable_current_elements()))
            .unwrap()
    }
}
