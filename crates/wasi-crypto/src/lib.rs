mod wiggle_interfaces;

pub use wiggle_interfaces::WasiCryptoCtx;

wasmtime_wiggle::wasmtime_integration!({
    target: wiggle_interfaces::wasi_modules,
    witx: ["$CARGO_MANIFEST_DIR/spec/witx/wasi_ephemeral_crypto.witx"],
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
