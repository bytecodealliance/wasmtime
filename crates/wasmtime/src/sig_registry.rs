//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use wasmtime_environ::{ir, wasm::WasmFuncType};
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
    // The native signature we're using for this wasm type signature.
    native: ir::Signature,
    // The native trampoline used to invoke this type signature from `Func`.
    // Note that the code memory for this trampoline is not owned by this
    // type, but instead it's expected to be owned by the store that this
    // registry lives within.
    trampoline: VMTrampoline,
}

impl SignatureRegistry {
    /// Register a signature and return its unique index.
    pub fn register(
        &mut self,
        wasm: &WasmFuncType,
        native: &ir::Signature,
        trampoline: VMTrampoline,
    ) -> VMSharedSignatureIndex {
        let len = self.wasm2index.len();

        match self.wasm2index.entry(wasm.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
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
                    native: native.clone(),
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

    /// Looks up information known about a shared signature index.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_shared(
        &self,
        idx: VMSharedSignatureIndex,
    ) -> Option<(&WasmFuncType, &ir::Signature, VMTrampoline)> {
        self.index_map
            .get(idx.bits() as usize)
            .map(|e| (&e.wasm, &e.native, e.trampoline))
    }
}
