//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! casting, and etc.

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::RwLock,
};
use std::{convert::TryFrom, sync::Arc};
use wasmtime_environ::{ModuleTypes, PrimaryMap, TypeIndex, WasmFuncType};
use wasmtime_runtime::VMSharedTypeIndex;

/// Represents a collection of shared types.
///
/// This is used to register shared types with a shared type registry.
///
/// The collection will unregister any contained types with the registry
/// when dropped.
#[derive(Debug)]
pub struct TypeCollection {
    registry: Arc<RwLock<TypeRegistryInner>>,
    types: PrimaryMap<TypeIndex, VMSharedTypeIndex>,
    reverse_types: HashMap<VMSharedTypeIndex, TypeIndex>,
}

impl TypeCollection {
    /// Creates a type collection for a module given the module's types.
    pub fn new_for_module(registry: &TypeRegistry, types: &ModuleTypes) -> Self {
        let types = registry.0.write().unwrap().register_for_module(types);
        let reverse_types = types.iter().map(|(k, v)| (*v, k)).collect();

        Self {
            registry: registry.0.clone(),
            types,
            reverse_types,
        }
    }

    /// Treats the type collection as a map from a module type index to
    /// registered shared type indexes.
    ///
    /// This is used for looking up module shared type indexes during module
    /// instantiation.
    pub fn as_module_map(&self) -> &PrimaryMap<TypeIndex, VMSharedTypeIndex> {
        &self.types
    }

    /// Gets the shared type index given a module type index.
    #[inline]
    pub fn shared_type(&self, index: TypeIndex) -> Option<VMSharedTypeIndex> {
        self.types.get(index).copied()
    }

    /// Get the module-local type index for the given shared type index.
    pub fn module_local_type(&self, index: VMSharedTypeIndex) -> Option<TypeIndex> {
        self.reverse_types.get(&index).copied()
    }
}

impl Drop for TypeCollection {
    fn drop(&mut self) {
        if !self.types.is_empty() {
            self.registry.write().unwrap().unregister_types(self);
        }
    }
}

#[derive(Debug)]
struct RegistryEntry {
    references: usize,
    ty: WasmFuncType,
}

#[derive(Debug, Default)]
struct TypeRegistryInner {
    // A map from the Wasm function type to a `VMSharedTypeIndex`, for all
    // the Wasm function types we have already registered.
    map: HashMap<WasmFuncType, VMSharedTypeIndex>,

    // A map from `VMSharedTypeIndex::bits()` to the type index's
    // associated data, such as the underlying Wasm type.
    entries: Vec<Option<RegistryEntry>>,

    // A free list of the `VMSharedTypeIndex`es that are no longer being
    // used by anything, and can therefore be reused.
    //
    // This is a size optimization, and not strictly necessary for correctness:
    // we reuse entries rather than leak them and have logical holes in our
    // `self.entries` list.
    free: Vec<VMSharedTypeIndex>,
}

impl TypeRegistryInner {
    fn register_for_module(
        &mut self,
        types: &ModuleTypes,
    ) -> PrimaryMap<TypeIndex, VMSharedTypeIndex> {
        let mut map = PrimaryMap::default();
        for (idx, ty) in types.wasm_types() {
            let b = map.push(self.register(ty));
            assert_eq!(idx, b);
        }
        map
    }

    fn register(&mut self, ty: &WasmFuncType) -> VMSharedTypeIndex {
        let len = self.map.len();

        let index = match self.map.entry(ty.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let (index, entry) = match self.free.pop() {
                    Some(index) => (index, &mut self.entries[index.bits() as usize]),
                    None => {
                        // Keep `index_map`'s length under `u32::MAX` because
                        // `u32::MAX` is reserved for `VMSharedTypeIndex`'s
                        // default value.
                        assert!(
                            len < std::u32::MAX as usize,
                            "Invariant check: index_map.len() < std::u32::MAX"
                        );
                        debug_assert_eq!(len, self.entries.len());

                        let index = VMSharedTypeIndex::new(u32::try_from(len).unwrap());
                        self.entries.push(None);

                        (index, self.entries.last_mut().unwrap())
                    }
                };

                // The entry should be missing for one just allocated or
                // taken from the free list
                assert!(entry.is_none());

                *entry = Some(RegistryEntry {
                    references: 0,
                    ty: ty.clone(),
                });

                *e.insert(index)
            }
        };

        self.entries[index.bits() as usize]
            .as_mut()
            .unwrap()
            .references += 1;

        index
    }

    fn unregister_types(&mut self, collection: &TypeCollection) {
        for (_, index) in collection.types.iter() {
            self.unregister_entry(*index, 1);
        }
    }

    fn unregister_entry(&mut self, index: VMSharedTypeIndex, count: usize) {
        let removed = {
            let entry = self.entries[index.bits() as usize].as_mut().unwrap();

            debug_assert!(entry.references >= count);
            entry.references -= count;

            if entry.references == 0 {
                self.map.remove(&entry.ty);
                self.free.push(index);
                true
            } else {
                false
            }
        };

        if removed {
            self.entries[index.bits() as usize] = None;
        }
    }
}

// `TypeRegistryInner` implements `Drop` in debug builds to assert that
// all types have been unregistered for the registry.
#[cfg(debug_assertions)]
impl Drop for TypeRegistryInner {
    fn drop(&mut self) {
        assert!(
            self.map.is_empty(),
            "type registry not empty: still have registered types in self.map"
        );
        assert_eq!(
            self.free.len(),
            self.entries.len(),
            "type registery not empty: not all entries in free list"
        );
    }
}

/// Implements a shared type registry.
///
/// WebAssembly requires that the caller and callee types in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// types, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug)]
pub struct TypeRegistry(Arc<RwLock<TypeRegistryInner>>);

impl TypeRegistry {
    /// Creates a new shared type registry.
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(TypeRegistryInner::default())))
    }

    /// Looks up a function type from a shared type index.
    pub fn lookup_type(&self, index: VMSharedTypeIndex) -> Option<WasmFuncType> {
        self.0
            .read()
            .unwrap()
            .entries
            .get(index.bits() as usize)
            .and_then(|e| e.as_ref().map(|e| &e.ty).cloned())
    }

    /// Registers a single function with the collection.
    ///
    /// Returns the shared type index for the function.
    pub fn register(&self, ty: &WasmFuncType) -> VMSharedTypeIndex {
        self.0.write().unwrap().register(ty)
    }

    /// Registers a single function with the collection.
    ///
    /// Returns the shared type index for the function.
    pub unsafe fn unregister(&self, sig: VMSharedTypeIndex) {
        self.0.write().unwrap().unregister_entry(sig, 1)
    }
}
