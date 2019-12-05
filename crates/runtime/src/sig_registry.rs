//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use crate::vmcontext::VMSharedSignatureIndex;
use more_asserts::{assert_lt, debug_assert_lt};
use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use wasmtime_environ::ir;

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug)]
pub struct SignatureRegistry {
    signature_hash: HashMap<ir::Signature, VMSharedSignatureIndex>,
}

impl SignatureRegistry {
    /// Create a new `SignatureRegistry`.
    pub fn new() -> Self {
        Self {
            signature_hash: HashMap::new(),
        }
    }

    /// Register a signature and return its unique index.
    pub fn register(&mut self, sig: &ir::Signature) -> VMSharedSignatureIndex {
        let len = self.signature_hash.len();
        match self.signature_hash.entry(sig.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                // Keep `signature_hash` len under 2**32 -- VMSharedSignatureIndex::new(std::u32::MAX)
                // is reserved for VMSharedSignatureIndex::default().
                debug_assert_lt!(
                    len,
                    std::u32::MAX as usize,
                    "Invariant check: signature_hash.len() < std::u32::MAX"
                );
                let sig_id = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
                entry.insert(sig_id);
                sig_id
            }
        }
    }
}
