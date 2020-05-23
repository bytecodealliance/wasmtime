//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use crate::vmcontext::VMSharedSignatureIndex;
use more_asserts::assert_lt;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::RwLock;
use wasmtime_environ::{ir, wasm::WasmFuncType};

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug)]
pub struct SignatureRegistry {
    // This structure is stored in a `Compiler` and is intended to be shared
    // across many instances. Ideally instances can themselves be sent across
    // threads, and ideally we can compile across many threads. As a result we
    // use interior mutability here with a lock to avoid having callers to
    // externally synchronize calls to compilation.
    inner: RwLock<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    wasm2index: HashMap<WasmFuncType, VMSharedSignatureIndex>,

    // Maps the index to the original Wasm signature.
    index2wasm: HashMap<VMSharedSignatureIndex, WasmFuncType>,

    // Maps the index to the native signature.
    index2native: HashMap<VMSharedSignatureIndex, ir::Signature>,
}

impl SignatureRegistry {
    /// Create a new `SignatureRegistry`.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Register a signature and return its unique index.
    pub fn register(&self, wasm: WasmFuncType, native: ir::Signature) -> VMSharedSignatureIndex {
        let Inner {
            wasm2index,
            index2wasm,
            index2native,
        } = &mut *self.inner.write().unwrap();
        let len = wasm2index.len();

        *wasm2index.entry(wasm.clone()).or_insert_with(|| {
            // Keep `signature_hash` len under 2**32 -- VMSharedSignatureIndex::new(std::u32::MAX)
            // is reserved for VMSharedSignatureIndex::default().
            assert_lt!(
                len,
                std::u32::MAX as usize,
                "Invariant check: signature_hash.len() < std::u32::MAX"
            );
            let index = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
            index2wasm.insert(index, wasm);
            index2native.insert(index, native);
            index
        })
    }

    /// Looks up a shared native signature within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_native(&self, idx: VMSharedSignatureIndex) -> Option<ir::Signature> {
        self.inner.read().unwrap().index2native.get(&idx).cloned()
    }

    /// Looks up a shared Wasm signature within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup_wasm(&self, idx: VMSharedSignatureIndex) -> Option<WasmFuncType> {
        self.inner.read().unwrap().index2wasm.get(&idx).cloned()
    }
}
