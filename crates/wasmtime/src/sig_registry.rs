//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use crate::Module;
use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{SignatureIndex, WasmFuncType};
use wasmtime_runtime::{VMSharedSignatureIndex, VMTrampoline};

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug, Default)]
pub struct SignatureRegistry {
    // Map from a wasm actual function type to the index that it is assigned,
    // shared amongst all wasm modules.
    wasm2index: HashMap<WasmFuncType, VMSharedSignatureIndex>,

    // Map of all known wasm function signatures in this registry. This is
    // keyed by `VMSharedSignatureIndex` above.
    index_map: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    // The WebAssembly type signature, using wasm types.
    wasm: WasmFuncType,
    // The native trampoline used to invoke this type signature from `Func`.
    // Note that the code memory for this trampoline is not owned by this
    // type, but instead it's expected to be owned by the store that this
    // registry lives within.
    trampoline: Option<VMTrampoline>,
}

impl SignatureRegistry {
    /// Registers all signatures within a module into this registry all at once.
    ///
    /// This will also internally register trampolines compiled in the module.
    pub fn register_module(&mut self, module: &Module) {
        // Register a unique index for all types in this module, even if they
        // don't have a trampoline.
        let signatures = &module.types().wasm_signatures;
        for ty in module.compiled_module().module().types.values() {
            if let wasmtime_environ::ModuleType::Function(index) = ty {
                self.register_one(&signatures[*index], None);
            }
        }

        // Once we've got a shared index for all types used then also fill in
        // any trampolines that the module has compiled as well.
        for (index, trampoline) in module.compiled_module().trampolines() {
            let shared = self.wasm2index[&signatures[*index]];
            let entry = &mut self.index_map[shared.bits() as usize];
            if entry.trampoline.is_none() {
                entry.trampoline = Some(*trampoline);
            }
        }
    }

    /// Register a signature and return its unique index.
    pub fn register(
        &mut self,
        wasm: &WasmFuncType,
        trampoline: VMTrampoline,
    ) -> VMSharedSignatureIndex {
        self.register_one(wasm, Some(trampoline))
    }

    fn register_one(
        &mut self,
        wasm: &WasmFuncType,
        trampoline: Option<VMTrampoline>,
    ) -> VMSharedSignatureIndex {
        let len = self.wasm2index.len();

        match self.wasm2index.entry(wasm.clone()) {
            hash_map::Entry::Occupied(entry) => {
                let ret = *entry.get();
                let entry = &mut self.index_map[ret.bits() as usize];
                // If the entry does not previously have a trampoline, then
                // overwrite it with whatever was specified by this function.
                if entry.trampoline.is_none() {
                    entry.trampoline = trampoline;
                }
                ret
            }
            hash_map::Entry::Vacant(entry) => {
                // Keep `signature_hash` len under 2**32 -- VMSharedSignatureIndex::new(std::u32::MAX)
                // is reserved for VMSharedSignatureIndex::default().
                assert!(
                    len < std::u32::MAX as usize,
                    "Invariant check: signature_hash.len() < std::u32::MAX"
                );
                debug_assert_eq!(len, self.index_map.len());
                let index = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
                self.index_map.push(Entry {
                    wasm: wasm.clone(),
                    trampoline,
                });
                entry.insert(index);
                index
            }
        }
    }

    /// Looks up a shared index from the wasm signature itself.
    pub fn lookup(&self, wasm: &WasmFuncType) -> Option<VMSharedSignatureIndex> {
        self.wasm2index.get(wasm).cloned()
    }

    /// Builds a lookup table for a module from the possible module's signature
    /// indices to the shared signature index within this registry.
    pub fn lookup_table(
        &self,
        module: &Module,
    ) -> PrimaryMap<SignatureIndex, VMSharedSignatureIndex> {
        // For module-linking using modules this builds up a map that is
        // too large. This builds up a map for everything in `TypeTables` but
        // that's all the types for all modules in a whole module linking graph,
        // which our `module` may not be using.
        //
        // For all non-module-linking-using modules, though, this is not an
        // issue. This is optimizing for the non-module-linking case right now
        // and it seems like module linking will likely change to the point that
        // this will no longer be an issue in the future.
        let signatures = &module.types().wasm_signatures;
        let mut map = PrimaryMap::with_capacity(signatures.len());
        for wasm in signatures.values() {
            map.push(
                self.wasm2index
                    .get(wasm)
                    .cloned()
                    .unwrap_or(VMSharedSignatureIndex::new(u32::MAX)),
            );
        }
        map
    }

    /// Looks up information known about a shared signature index.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_shared(
        &self,
        idx: VMSharedSignatureIndex,
    ) -> Option<(&WasmFuncType, VMTrampoline)> {
        let (wasm, trampoline) = self
            .index_map
            .get(idx.bits() as usize)
            .map(|e| (&e.wasm, e.trampoline))?;
        Some((wasm, trampoline?))
    }
}
