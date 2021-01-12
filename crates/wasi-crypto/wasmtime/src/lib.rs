pub use wasi_crypto::WasiCryptoCtx;

mod witx {
    pub use wasi_crypto::wasi_modules::*;
}

wasmtime_wiggle::wasmtime_integration!({
    target: witx,
    witx: ["$OUT_DIR/wasi_ephemeral_crypto.witx"],
    ctx: WasiCryptoCtx,
    modules: {
        wasi_ephemeral_crypto_common =>
            {
                name: WasiCryptoCommon,
                docs: "wasi-crypto - Common module."
            },
        wasi_ephemeral_crypto_asymmetric_common =>
            {
                name: WasiCryptoAsymmetricCommon,
                docs: "wasi-crypto - Common module for asymmetric operations."
            },
        wasi_ephemeral_crypto_signatures =>
            {
                name: WasiCryptoSignatures,
                docs: "wasi-crypto - Signature module."
            },
        wasi_ephemeral_crypto_symmetric =>
            {
                name: WasiCryptoSymmetric,
                docs: "wasi-crypto - Symmetric cryptography module."
            }
    }
});
