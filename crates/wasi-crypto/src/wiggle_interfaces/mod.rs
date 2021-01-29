use std::rc::Rc;

pub use wasi_crypto::CryptoCtx as WasiCryptoCtx;

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/spec/witx/wasi_ephemeral_crypto.witx"],
    ctx: WasiCryptoCtx
});

pub mod wasi_modules {
    pub use super::{
        wasi_ephemeral_crypto_asymmetric_common, wasi_ephemeral_crypto_common,
        wasi_ephemeral_crypto_kx, wasi_ephemeral_crypto_signatures,
        wasi_ephemeral_crypto_symmetric,
    };
}

pub use types as guest_types;

mod asymmetric_common;
mod common;
mod error;
mod key_exchange;
mod signatures;
mod symmetric;
