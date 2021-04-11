//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryFrom;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{SignatureIndex, WasmFuncType};
use wasmtime_runtime::{VMSharedSignatureIndex, VMTrampoline};

/// Represents a mapping of shared signature index to trampolines.
///
/// This is used in various places to store trampolines associated with shared
/// signature indexes.
///
/// As multiple trampolines may exist for a single signature, the map entries
/// are internally reference counted.
#[derive(Default)]
pub struct TrampolineMap(HashMap<VMSharedSignatureIndex, (usize, VMTrampoline)>);

impl TrampolineMap {
    /// Inserts a trampoline into the map.
    pub fn insert(&mut self, index: VMSharedSignatureIndex, trampoline: VMTrampoline) {
        let entry = match self.0.entry(index) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert((0, trampoline)),
        };

        // Increment the ref count
        entry.0 += 1;
    }

    /// Gets a trampoline from the map.
    pub fn get(&self, index: VMSharedSignatureIndex) -> Option<VMTrampoline> {
        self.0.get(&index).map(|(_, trampoline)| *trampoline)
    }

    /// Iterates the shared signature indexes stored in the map.
    ///
    /// A shared signature index will be returned by the iterator for every
    /// trampoline registered for that index, so duplicates may be present.
    ///
    /// This iterator can be used for deregistering signatures with the
    /// signature registry.
    pub fn indexes<'a>(&'a self) -> impl Iterator<Item = VMSharedSignatureIndex> + 'a {
        self.0
            .iter()
            .flat_map(|(index, (count, _))| std::iter::repeat(*index).take(*count))
    }

    /// Determines if the trampoline map is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Represents a map between module signature indexes and
/// shared signature indexes.
pub type SharedSignatures = PrimaryMap<SignatureIndex, VMSharedSignatureIndex>;

#[derive(Debug)]
struct RegistryEntry {
    references: usize,
    ty: WasmFuncType,
}

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug, Default)]
pub struct SignatureRegistry {
    map: HashMap<WasmFuncType, VMSharedSignatureIndex>,
    entries: Vec<Option<RegistryEntry>>,
    free: Vec<VMSharedSignatureIndex>,
}

impl SignatureRegistry {
    /// Registers a module with the signature registry from the collection of
    /// all signatures and trampolines in the module.
    pub fn register_module(
        &mut self,
        signatures: &PrimaryMap<SignatureIndex, WasmFuncType>,
        trampolines: impl Iterator<Item = (SignatureIndex, VMTrampoline)>,
    ) -> (SharedSignatures, TrampolineMap) {
        let mut sigs = SharedSignatures::default();
        let mut map = TrampolineMap::default();

        for (_, ty) in signatures.iter() {
            sigs.push(self.register(ty));
        }

        for (index, trampoline) in trampolines {
            let index = self.map[&signatures[index]];
            map.insert(index, trampoline);
        }

        (sigs, map)
    }

    /// Registers a single signature with the registry.
    ///
    /// This is used for registering host functions created with the Wasmtime API.
    pub fn register(&mut self, ty: &WasmFuncType) -> VMSharedSignatureIndex {
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

    /// Unregisters a collection of shared indexes from the registry.
    pub fn unregister(&mut self, indexes: impl Iterator<Item = VMSharedSignatureIndex>) {
        for index in indexes {
            let removed = {
                let entry = self.entries[index.bits() as usize].as_mut().unwrap();

                debug_assert!(entry.references > 0);
                entry.references -= 1;

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

    /// Looks up a function type from a shared signature index.
    pub fn lookup_type(&self, index: VMSharedSignatureIndex) -> Option<&WasmFuncType> {
        self.entries
            .get(index.bits() as usize)
            .and_then(|e| e.as_ref().map(|e| &e.ty))
    }

    /// Determines if the registry is semantically empty.
    pub fn is_empty(&self) -> bool {
        // If the map is empty, assert that all remaining entries are "free"
        if self.map.is_empty() {
            assert!(self.free.len() == self.entries.len());
            true
        } else {
            false
        }
    }
}
