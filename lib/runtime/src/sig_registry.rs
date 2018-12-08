//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use cast;
use cranelift_codegen::ir;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::SignatureIndex;
use std::collections::{hash_map, HashMap};
use vmcontext::VMSharedSignatureIndex;

#[derive(Debug)]
pub struct SignatureRegistry {
    signature_hash: HashMap<ir::Signature, VMSharedSignatureIndex>,
    shared_signatures: PrimaryMap<SignatureIndex, VMSharedSignatureIndex>,
}

impl SignatureRegistry {
    pub fn new() -> Self {
        Self {
            signature_hash: HashMap::new(),
            shared_signatures: PrimaryMap::new(),
        }
    }

    pub fn vmshared_signatures(&mut self) -> *mut VMSharedSignatureIndex {
        self.shared_signatures
            .values_mut()
            .into_slice()
            .as_mut_ptr()
    }

    /// Register the given signature.
    pub fn register(&mut self, sig_index: SignatureIndex, sig: &ir::Signature) {
        // TODO: Refactor this interface so that we're not passing in redundant
        // information.
        debug_assert_eq!(sig_index.index(), self.shared_signatures.len());
        use cranelift_entity::EntityRef;

        let len = self.signature_hash.len();
        let sig_id = match self.signature_hash.entry(sig.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                let sig_id = VMSharedSignatureIndex::new(cast::u32(len).unwrap());
                entry.insert(sig_id);
                sig_id
            }
        };
        self.shared_signatures.push(sig_id);
    }

    /// Return the identifying runtime index for the given signature.
    pub fn lookup(&mut self, sig_index: SignatureIndex) -> VMSharedSignatureIndex {
        self.shared_signatures[sig_index]
    }
}
