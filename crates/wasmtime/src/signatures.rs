//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::RwLock,
};
use std::{convert::TryFrom, sync::Arc};
use wasmtime_environ::{PrimaryMap, SignatureIndex, TypeTables, WasmFuncType};
use wasmtime_runtime::{VMSharedSignatureIndex, VMTrampoline};

/// Represents a collection of shared signatures.
///
/// This is used to register shared signatures with a shared signature registry.
///
/// The collection will unregister any contained signatures with the registry
/// when dropped.
#[derive(Debug)]
pub struct SignatureCollection {
    registry: Arc<RwLock<SignatureRegistryInner>>,
    signatures: PrimaryMap<SignatureIndex, VMSharedSignatureIndex>,
    trampolines: HashMap<VMSharedSignatureIndex, (usize, VMTrampoline)>,
}

impl SignatureCollection {
    /// Creates a signature collection for a module given the module's signatures
    /// and trampolines.
    pub fn new_for_module(
        registry: &SignatureRegistry,
        types: &TypeTables,
        trampolines: impl Iterator<Item = (SignatureIndex, VMTrampoline)>,
    ) -> Self {
        let (signatures, trampolines) = registry
            .0
            .write()
            .unwrap()
            .register_for_module(types, trampolines);

        Self {
            registry: registry.0.clone(),
            signatures,
            trampolines,
        }
    }

    /// Treats the signature collection as a map from a module signature index to
    /// registered shared signature indexes.
    ///
    /// This is used for looking up module shared signature indexes during module
    /// instantiation.
    pub fn as_module_map(&self) -> &PrimaryMap<SignatureIndex, VMSharedSignatureIndex> {
        &self.signatures
    }

    /// Gets the shared signature index given a module signature index.
    pub fn shared_signature(&self, index: SignatureIndex) -> Option<VMSharedSignatureIndex> {
        self.signatures.get(index).copied()
    }

    /// Gets a trampoline for a registered signature.
    pub fn trampoline(&self, index: VMSharedSignatureIndex) -> Option<VMTrampoline> {
        self.trampolines
            .get(&index)
            .map(|(_, trampoline)| *trampoline)
    }
}

impl Drop for SignatureCollection {
    fn drop(&mut self) {
        if !self.signatures.is_empty() || !self.trampolines.is_empty() {
            self.registry.write().unwrap().unregister_signatures(self);
        }
    }
}

#[derive(Debug)]
struct RegistryEntry {
    references: usize,
    ty: WasmFuncType,
}

#[derive(Debug, Default)]
struct SignatureRegistryInner {
    map: HashMap<WasmFuncType, VMSharedSignatureIndex>,
    entries: Vec<Option<RegistryEntry>>,
    free: Vec<VMSharedSignatureIndex>,
}

impl SignatureRegistryInner {
    fn register_for_module(
        &mut self,
        types: &TypeTables,
        trampolines: impl Iterator<Item = (SignatureIndex, VMTrampoline)>,
    ) -> (
        PrimaryMap<SignatureIndex, VMSharedSignatureIndex>,
        HashMap<VMSharedSignatureIndex, (usize, VMTrampoline)>,
    ) {
        let mut sigs = PrimaryMap::default();
        let mut map = HashMap::default();

        for (idx, ty) in types.wasm_signatures() {
            let b = sigs.push(self.register(ty));
            assert_eq!(idx, b);
        }

        for (index, trampoline) in trampolines {
            map.insert(sigs[index], (1, trampoline));
        }

        (sigs, map)
    }

    fn register(&mut self, ty: &WasmFuncType) -> VMSharedSignatureIndex {
        let len = self.map.len();

        let index = match self.map.entry(ty.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let (index, entry) = match self.free.pop() {
                    Some(index) => (index, &mut self.entries[index.bits() as usize]),
                    None => {
                        // Keep `index_map` len under 2**32 -- VMSharedSignatureIndex::new(std::u32::MAX)
                        // is reserved for VMSharedSignatureIndex::default().
                        assert!(
                            len < std::u32::MAX as usize,
                            "Invariant check: index_map.len() < std::u32::MAX"
                        );
                        debug_assert_eq!(len, self.entries.len());

                        let index = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
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

    fn unregister_signatures(&mut self, collection: &SignatureCollection) {
        // If the collection has a populated signatures map, use it to deregister
        // This is always 1:1 from entry to registration
        if !collection.signatures.is_empty() {
            for (_, index) in collection.signatures.iter() {
                self.unregister_entry(*index, 1);
            }
        } else {
            // Otherwise, use the trampolines map, which has reference counts related
            // to the stored index
            for (index, (count, _)) in collection.trampolines.iter() {
                self.unregister_entry(*index, *count);
            }
        }
    }

    fn unregister_entry(&mut self, index: VMSharedSignatureIndex, count: usize) {
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

// `SignatureRegistryInner` implements `Drop` in debug builds to assert that
// all signatures have been unregistered for the registry.
#[cfg(debug_assertions)]
impl Drop for SignatureRegistryInner {
    fn drop(&mut self) {
        assert!(
            self.map.is_empty() && self.free.len() == self.entries.len(),
            "signature registry not empty"
        );
    }
}

/// Implements a shared signature registry.
///
/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug)]
pub struct SignatureRegistry(Arc<RwLock<SignatureRegistryInner>>);

impl SignatureRegistry {
    /// Creates a new shared signature registry.
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(SignatureRegistryInner::default())))
    }

    /// Looks up a function type from a shared signature index.
    pub fn lookup_type(&self, index: VMSharedSignatureIndex) -> Option<WasmFuncType> {
        self.0
            .read()
            .unwrap()
            .entries
            .get(index.bits() as usize)
            .and_then(|e| e.as_ref().map(|e| &e.ty).cloned())
    }

    /// Registers a single function with the collection.
    ///
    /// Returns the shared signature index for the function.
    pub fn register(&self, ty: &WasmFuncType) -> VMSharedSignatureIndex {
        self.0.write().unwrap().register(ty)
    }

    /// Registers a single function with the collection.
    ///
    /// Returns the shared signature index for the function.
    pub unsafe fn unregister(&self, sig: VMSharedSignatureIndex) {
        self.0.write().unwrap().unregister_entry(sig, 1)
    }
}
