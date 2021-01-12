#![allow(
    clippy::unit_arg,
    clippy::useless_conversion,
    clippy::new_without_default,
    clippy::new_ret_no_self,
    clippy::too_many_arguments
)]
#![allow(unused_imports, dead_code)]
#[macro_use]
extern crate derivative;

mod array_output;
mod asymmetric_common;
mod error;
mod handles;
mod key_exchange;
mod options;
mod rand;
mod secrets_manager;
mod signatures;
mod symmetric;
mod version;
mod wasi_glue;

use std::collections::HashMap;
use std::rc::Rc;

use crate::types as guest_types;
use array_output::*;
use asymmetric_common::*;
use handles::*;
use key_exchange::*;
use options::*;
use secrets_manager::*;
use signatures::*;
use symmetric::*;

pub use asymmetric_common::{KeyPairEncoding, PublicKeyEncoding};
pub use error::CryptoError;
pub use handles::Handle;
pub use signatures::SignatureEncoding;
pub use version::Version;

pub fn witx_interfaces() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert(
        "proposal_common.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/proposal_common.witx"
        )),
    );
    map.insert(
        "proposal_asymmetric_common.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/proposal_asymmetric_common.witx"
        )),
    );
    map.insert(
        "proposal_signatures.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/proposal_signatures.witx"
        )),
    );
    map.insert(
        "proposal_symmetric.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/proposal_symmetric.witx"
        )),
    );
    map.insert(
        "proposal_kx.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/proposal_kx.witx"
        )),
    );
    map.insert(
        "wasi_ephemeral_crypto.witx",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/witx/wasi_ephemeral_crypto.witx"
        )),
    );
    map
}

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/spec/witx/wasi_ephemeral_crypto.witx"],
    ctx: WasiCryptoCtx
});

pub mod wasi_modules {
    pub use crate::{
        wasi_ephemeral_crypto_asymmetric_common, wasi_ephemeral_crypto_common,
        wasi_ephemeral_crypto_kx, wasi_ephemeral_crypto_signatures,
        wasi_ephemeral_crypto_symmetric,
    };
}

pub struct HandleManagers {
    pub array_output: HandlesManager<ArrayOutput>,
    pub options: HandlesManager<Options>,
    pub keypair: HandlesManager<KeyPair>,
    pub publickey: HandlesManager<PublicKey>,
    pub secretkey: HandlesManager<SecretKey>,
    pub signature_state: HandlesManager<SignatureState>,
    pub signature: HandlesManager<Signature>,
    pub signature_verification_state: HandlesManager<SignatureVerificationState>,
    pub symmetric_state: HandlesManager<SymmetricState>,
    pub symmetric_key: HandlesManager<SymmetricKey>,
    pub symmetric_tag: HandlesManager<SymmetricTag>,
    pub secrets_manager: HandlesManager<SecretsManager>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlgorithmType {
    Signatures,
    Symmetric,
    KeyExchange,
}

impl From<guest_types::AlgorithmType> for AlgorithmType {
    fn from(options_type: guest_types::AlgorithmType) -> Self {
        match options_type {
            guest_types::AlgorithmType::Signatures => AlgorithmType::Signatures,
            guest_types::AlgorithmType::Symmetric => AlgorithmType::Symmetric,
            guest_types::AlgorithmType::KeyExchange => AlgorithmType::KeyExchange,
        }
    }
}

pub struct CryptoCtx {
    pub(crate) handles: HandleManagers,
}

#[derive(Clone)]
pub struct WasiCryptoCtx {
    ctx: Rc<CryptoCtx>,
}

impl CryptoCtx {
    pub fn new() -> Self {
        CryptoCtx {
            handles: HandleManagers {
                array_output: HandlesManager::new(0x00),
                options: HandlesManager::new(0x01),
                keypair: HandlesManager::new(0x02),
                publickey: HandlesManager::new(0x03),
                secretkey: HandlesManager::new(0x04),
                signature_state: HandlesManager::new(0x05),
                signature: HandlesManager::new(0x06),
                signature_verification_state: HandlesManager::new(0x07),
                symmetric_state: HandlesManager::new(0x08),
                symmetric_key: HandlesManager::new(0x09),
                symmetric_tag: HandlesManager::new(0x0a),
                secrets_manager: HandlesManager::new(0x0b),
            },
        }
    }
}

impl WasiCryptoCtx {
    pub fn new() -> Self {
        WasiCryptoCtx {
            ctx: Rc::new(CryptoCtx::new()),
        }
    }
}
