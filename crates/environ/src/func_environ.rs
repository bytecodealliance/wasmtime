/// An index type for builtin functions.
#[derive(Copy, Clone, Debug)]
pub struct BuiltinFunctionIndex(u32);

impl BuiltinFunctionIndex {
    /// Returns an index for wasm's `memory.grow` builtin function.
    pub const fn get_memory32_grow_index() -> Self {
        Self(0)
    }
    /// Returns an index for wasm's imported `memory.grow` builtin function.
    pub const fn get_imported_memory32_grow_index() -> Self {
        Self(1)
    }
    /// Returns an index for wasm's `memory.size` builtin function.
    pub const fn get_memory32_size_index() -> Self {
        Self(2)
    }
    /// Returns an index for wasm's imported `memory.size` builtin function.
    pub const fn get_imported_memory32_size_index() -> Self {
        Self(3)
    }
    /// Returns an index for wasm's `table.copy` when both tables are locally
    /// defined.
    pub const fn get_table_copy_index() -> Self {
        Self(4)
    }
    /// Returns an index for wasm's `table.init`.
    pub const fn get_table_init_index() -> Self {
        Self(5)
    }
    /// Returns an index for wasm's `elem.drop`.
    pub const fn get_elem_drop_index() -> Self {
        Self(6)
    }
    /// Returns an index for wasm's `memory.copy` for locally defined memories.
    pub const fn get_defined_memory_copy_index() -> Self {
        Self(7)
    }
    /// Returns an index for wasm's `memory.copy` for imported memories.
    pub const fn get_imported_memory_copy_index() -> Self {
        Self(8)
    }
    /// Returns an index for wasm's `memory.fill` for locally defined memories.
    pub const fn get_memory_fill_index() -> Self {
        Self(9)
    }
    /// Returns an index for wasm's `memory.fill` for imported memories.
    pub const fn get_imported_memory_fill_index() -> Self {
        Self(10)
    }
    /// Returns an index for wasm's `memory.init` instruction.
    pub const fn get_memory_init_index() -> Self {
        Self(11)
    }
    /// Returns an index for wasm's `data.drop` instruction.
    pub const fn get_data_drop_index() -> Self {
        Self(12)
    }
    /// Returns the total number of builtin functions.
    pub const fn builtin_functions_total_number() -> u32 {
        13
    }

    /// Return the index as an u32 number.
    pub const fn index(&self) -> u32 {
        self.0
    }
}
