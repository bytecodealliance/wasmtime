//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use crate::vmcontext::VMSharedSignatureIndex;
use more_asserts::assert_lt;
use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use wasmtime_environ::{ir, wasm::WasmFuncType};

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug, Default)]
pub struct SignatureRegistry {
    wasm2index: HashMap<WasmFuncType, VMSharedSignatureIndex>,

    // Maps the index to the original Wasm signature.
    index2wasm: HashMap<VMSharedSignatureIndex, WasmFuncType>,

    // Maps the index to the native signature.
    index2native: HashMap<VMSharedSignatureIndex, ir::Signature>,
}

impl SignatureRegistry {
    /// Register a signature and return its unique index.
    pub fn register(
        &mut self,
        wasm: WasmFuncType,
        native: ir::Signature,
    ) -> VMSharedSignatureIndex {
        let len = self.wasm2index.len();

        match self.wasm2index.entry(wasm.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                // Keep `signature_hash` len under 2**32 -- VMSharedSignatureIndex::new(std::u32::MAX)
                // is reserved for VMSharedSignatureIndex::default().
                assert_lt!(
                    len,
                    std::u32::MAX as usize,
                    "Invariant check: signature_hash.len() < std::u32::MAX"
                );
                let index = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
                entry.insert(index);
                self.index2wasm.insert(index, wasm);
                self.index2native.insert(index, native);
                index
            }
        }
    }

    /// Looks up a shared native signature within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_native(&self, idx: VMSharedSignatureIndex) -> Option<ir::Signature> {
        self.index2native.get(&idx).cloned()
    }

    /// Looks up a shared Wasm signature within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_wasm(&self, idx: VMSharedSignatureIndex) -> Option<WasmFuncType> {
        self.index2wasm.get(&idx).cloned()
    }

    /// Looks up both a shared Wasm function signature and its associated native
    /// `ir::Signature` within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_wasm_and_native_signatures(
        &self,
        idx: VMSharedSignatureIndex,
    ) -> Option<(WasmFuncType, ir::Signature)> {
        let wasm = self.lookup_wasm(idx)?;
        let native = self.lookup_native(idx)?;
        Some((wasm, native))
    }
}
