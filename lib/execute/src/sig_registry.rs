//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use cast;
use cranelift_codegen::ir;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::SignatureIndex;
use std::collections::{hash_map, HashMap};
use vmcontext::VMSignatureId;

#[derive(Debug)]
pub struct SignatureRegistry {
    signature_hash: HashMap<ir::Signature, VMSignatureId>,
    signature_ids: PrimaryMap<SignatureIndex, VMSignatureId>,
}

impl SignatureRegistry {
    pub fn new() -> Self {
        Self {
            signature_hash: HashMap::new(),
            signature_ids: PrimaryMap::new(),
        }
    }

    pub fn vmsignature_ids(&mut self) -> *mut VMSignatureId {
        self.signature_ids.values_mut().into_slice().as_mut_ptr()
    }

    /// Register the given signature.
    pub fn register(&mut self, sig_index: SignatureIndex, sig: &ir::Signature) {
        // TODO: Refactor this interface so that we're not passing in redundant
        // information.
        debug_assert_eq!(sig_index.index(), self.signature_ids.len());
        use cranelift_entity::EntityRef;

        let len = self.signature_hash.len();
        let sig_id = match self.signature_hash.entry(sig.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                let sig_id = cast::u32(len).unwrap();
                entry.insert(sig_id);
                sig_id
            }
        };
        self.signature_ids.push(sig_id);
    }

    /// Return the identifying runtime index for the given signature.
    pub fn lookup(&mut self, sig_index: SignatureIndex) -> VMSignatureId {
        self.signature_ids[sig_index]
    }
}
