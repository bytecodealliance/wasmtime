use crate::PrimaryMap;
use serde_derive::{Deserialize, Serialize};
use std::ops::{Index, Range};
use wasmtime_types::{ModuleInternedRecGroupIndex, ModuleInternedTypeIndex, WasmSubType};

/// All types used in a core wasm module.
///
/// At this time this only contains function types. Note, though, that function
/// types are deduplicated within this [`ModuleTypes`].
///
/// Note that accesing this type is primarily done through the `Index`
/// implementations for this type.
#[derive(Default, Serialize, Deserialize)]
pub struct ModuleTypes {
    rec_groups: PrimaryMap<ModuleInternedRecGroupIndex, Range<ModuleInternedTypeIndex>>,
    wasm_types: PrimaryMap<ModuleInternedTypeIndex, WasmSubType>,
}

impl ModuleTypes {
    /// Returns an iterator over all the wasm function signatures found within
    /// this module.
    pub fn wasm_types(
        &self,
    ) -> impl ExactSizeIterator<Item = (ModuleInternedTypeIndex, &WasmSubType)> {
        self.wasm_types.iter()
    }

    /// Get the type at the specified index, if it exists.
    pub fn get(&self, ty: ModuleInternedTypeIndex) -> Option<&WasmSubType> {
        self.wasm_types.get(ty)
    }

    /// Get an iterator over all recursion groups defined in this module and
    /// their elements.
    pub fn rec_groups(
        &self,
    ) -> impl ExactSizeIterator<Item = (ModuleInternedRecGroupIndex, Range<ModuleInternedTypeIndex>)> + '_
    {
        self.rec_groups.iter().map(|(k, v)| (k, v.clone()))
    }

    /// Get the elements within an already-defined rec group.
    pub fn rec_group_elements(
        &self,
        rec_group: ModuleInternedRecGroupIndex,
    ) -> impl ExactSizeIterator<Item = ModuleInternedTypeIndex> {
        let range = &self.rec_groups[rec_group];
        (range.start.as_u32()..range.end.as_u32()).map(|i| ModuleInternedTypeIndex::from_u32(i))
    }

    /// Returns the number of types interned.
    pub fn len_types(&self) -> usize {
        self.wasm_types.len()
    }

    /// Adds a new type to this interned list of types.
    pub fn push(&mut self, ty: WasmSubType) -> ModuleInternedTypeIndex {
        self.wasm_types.push(ty)
    }

    /// Adds a new rec group to this interned list of types.
    pub fn push_rec_group(
        &mut self,
        range: Range<ModuleInternedTypeIndex>,
    ) -> ModuleInternedRecGroupIndex {
        self.rec_groups.push(range)
    }

    /// Reserves space for `amt` more types.
    pub fn reserve(&mut self, amt: usize) {
        self.wasm_types.reserve(amt)
    }

    /// Returns the next return value of `push_rec_group`.
    pub fn next_rec_group(&self) -> ModuleInternedRecGroupIndex {
        self.rec_groups.next_key()
    }

    /// Returns the next return value of `push`.
    pub fn next_ty(&self) -> ModuleInternedTypeIndex {
        self.wasm_types.next_key()
    }
}

impl Index<ModuleInternedTypeIndex> for ModuleTypes {
    type Output = WasmSubType;

    fn index(&self, sig: ModuleInternedTypeIndex) -> &WasmSubType {
        &self.wasm_types[sig]
    }
}
