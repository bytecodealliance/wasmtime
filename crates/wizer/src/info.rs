use std::convert::TryFrom;
use std::ops::Range;

/// Info that we keep track of on a per module-within-a-module-linking-bundle
/// basis.
///
/// These are created during our `parse` pass and then used throughout
/// our later passes.
#[derive(Default)]
pub struct ModuleContext<'a> {
    /// The raw sections from the original Wasm input.
    raw_sections: Vec<wasm_encoder::RawSection<'a>>,

    /// Imports made by this module.
    imports: Vec<wasmparser::Import<'a>>,

    /// A map from global indices to each global's type for all defined,
    /// imported, and aliased globals.
    globals: Vec<wasmparser::GlobalType>,

    /// The index within the global index space where defined globals (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined globals.
    defined_globals_index: Option<u32>,

    /// This module's exports.
    ///
    /// This is used later on, in the rewrite phase, when we are inserting state
    /// instance imports.
    ///
    /// Note that this does *not* include the `__wizer_thing_N` exports that
    /// this instrumentation pass adds.
    exports: Vec<wasmparser::Export<'a>>,

    /// Maps from function index to the function's type index for all functions
    /// defined and imported in this module.
    functions: Vec<u32>,

    /// Maps from table index to the table's type for all tables defined,
    /// imported, and aliased in this module.
    tables: Vec<wasmparser::TableType>,

    /// Maps from memory index to the memory's type for all memories defined,
    /// imported, and aliased in this module.
    memories: Vec<wasmparser::MemoryType>,

    /// The index within the memory index space where defined memories (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined memories.
    defined_memories_index: Option<u32>,

    /// Export names of defined globals injected by the instrumentation pass.
    ///
    /// Note that this only tracks defined mutable globals, not all globals.
    pub(crate) defined_global_exports: Option<Vec<(u32, String)>>,

    /// Export names of defined memories injected by the instrumentation pass.
    pub(crate) defined_memory_exports: Option<Vec<String>>,
}

impl<'a> ModuleContext<'a> {
    /// Add a new raw section to this module info during parsing.
    pub(crate) fn add_raw_section(&mut self, id: u8, range: Range<usize>, full_wasm: &'a [u8]) {
        self.raw_sections.push(wasm_encoder::RawSection {
            id,
            data: &full_wasm[range.start..range.end],
        })
    }

    /// Push a new imported memory into this module's memory index space.
    pub(crate) fn push_imported_memory(&mut self, memory_type: wasmparser::MemoryType) {
        assert!(self.defined_memories_index.is_none());
        self.memories.push(memory_type);
    }

    /// Push a new defined memory into this module's memory index space.
    pub(crate) fn push_defined_memory(&mut self, memory_type: wasmparser::MemoryType) {
        if self.defined_memories_index.is_none() {
            self.defined_memories_index = Some(u32::try_from(self.memories.len()).unwrap());
        }
        self.memories.push(memory_type);
    }

    /// Push a new imported global into this module's global index space.
    pub(crate) fn push_imported_global(&mut self, global_type: wasmparser::GlobalType) {
        assert!(self.defined_globals_index.is_none());
        self.globals.push(global_type);
    }

    /// Push a new defined global into this module's global index space.
    pub(crate) fn push_defined_global(&mut self, global_type: wasmparser::GlobalType) {
        if self.defined_globals_index.is_none() {
            self.defined_globals_index = Some(u32::try_from(self.globals.len()).unwrap());
        }
        self.globals.push(global_type);
    }

    /// Push a new function into this module's function index space.
    pub(crate) fn push_function(&mut self, func_type: u32) {
        self.functions.push(func_type);
    }

    /// Push a new table into this module's table index space.
    pub(crate) fn push_table(&mut self, table_type: wasmparser::TableType) {
        self.tables.push(table_type);
    }

    /// Push a new import into this module.
    pub(crate) fn push_import(&mut self, import: wasmparser::Import<'a>) {
        self.imports.push(import);

        // Add the import to the appropriate index space for our current module.
        match import.ty {
            wasmparser::TypeRef::Memory(ty) => {
                self.push_imported_memory(ty);
            }
            wasmparser::TypeRef::Global(ty) => {
                self.push_imported_global(ty);
            }
            wasmparser::TypeRef::Func(ty_idx) => {
                self.push_function(ty_idx);
            }
            wasmparser::TypeRef::Table(ty) => {
                self.push_table(ty);
            }
            wasmparser::TypeRef::Tag(_) => {
                unreachable!("exceptions are unsupported; checked in validation")
            }
            wasmparser::TypeRef::FuncExact(_) => {
                unreachable!("custom-descriptors are unsupported; checked in validation")
            }
        }
    }

    /// Push an export into this module.
    pub(crate) fn push_export(&mut self, export: wasmparser::Export<'a>) {
        self.exports.push(export);
    }

    /// The number of defined memories in this module.
    pub(crate) fn defined_memories_len(&self) -> usize {
        self.defined_memories_index.map_or(0, |n| {
            let n = usize::try_from(n).unwrap();
            assert!(self.memories.len() > n);
            self.memories.len() - n
        })
    }

    /// Iterate over the defined memories in this module.
    pub(crate) fn defined_memories(
        &self,
    ) -> impl Iterator<Item = (u32, wasmparser::MemoryType)> + '_ {
        self.memories
            .iter()
            .copied()
            .enumerate()
            .skip(
                self.defined_memories_index
                    .map_or(self.memories.len(), |i| usize::try_from(i).unwrap()),
            )
            .map(|(i, m)| (u32::try_from(i).unwrap(), m))
    }

    /// Iterate over the defined globals in this module.
    pub(crate) fn defined_globals(
        &self,
    ) -> impl Iterator<Item = (u32, wasmparser::GlobalType, Option<&str>)> + '_ {
        let mut defined_global_exports = self
            .defined_global_exports
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .peekable();

        self.globals
            .iter()
            .copied()
            .enumerate()
            .skip(
                self.defined_globals_index
                    .map_or(self.globals.len(), |i| usize::try_from(i).unwrap()),
            )
            .map(move |(i, g)| {
                let i = u32::try_from(i).unwrap();
                let name = defined_global_exports
                    .next_if(|(j, _)| *j == i)
                    .map(|(_, name)| name.as_str());
                (i, g, name)
            })
    }

    /// Get a slice of this module's original raw sections.
    pub(crate) fn raw_sections(&self) -> &[wasm_encoder::RawSection<'a>] {
        &self.raw_sections
    }

    /// Get a slice of this module's imports.
    pub(crate) fn imports(&self) -> &[wasmparser::Import<'a>] {
        &self.imports
    }

    /// Get a slice of this module's exports.
    pub(crate) fn exports(&self) -> &[wasmparser::Export<'a>] {
        &self.exports
    }

    pub(crate) fn has_wasi_initialize(&self) -> bool {
        self.exports.iter().any(|e| e.name == "_initialize")
    }
}
