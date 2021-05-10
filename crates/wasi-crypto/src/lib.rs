use std::borrow::BorrowMut;

mod wiggle_interfaces;

pub use wiggle_interfaces::WasiCryptoCtx;

pub fn add_to_linker<T>(linker: &mut wasmtime::Linker<T>) -> anyhow::Result<()>
where
    T: BorrowMut<WasiCryptoCtx>,
{
    add_wasi_crypto_common_to_linker(linker)?;
    add_wasi_crypto_asymmetric_common_to_linker(linker)?;
    add_wasi_crypto_signatures_to_linker(linker)?;
    add_wasi_crypto_symmetric_to_linker(linker)?;
    Ok(())
}

wasmtime_wiggle::wasmtime_integration!({
    target: wiggle_interfaces::wasi_modules,
    witx: ["$CARGO_MANIFEST_DIR/spec/witx/wasi_ephemeral_crypto.witx"],
    ctx: WasiCryptoCtx,
    modules: {
        wasi_ephemeral_crypto_common =>
            {
                name: wasi_crypto_common,
                docs: "wasi-crypto - Common module."
            },
        wasi_ephemeral_crypto_asymmetric_common =>
            {
                name: wasi_crypto_asymmetric_common,
                docs: "wasi-crypto - Common module for asymmetric operations."
            },
        wasi_ephemeral_crypto_signatures =>
            {
                name: wasi_crypto_signatures,
                docs: "wasi-crypto - Signature module."
            },
        wasi_ephemeral_crypto_symmetric =>
            {
                name: wasi_crypto_symmetric,
                docs: "wasi-crypto - Symmetric cryptography module."
            }
    }
});
