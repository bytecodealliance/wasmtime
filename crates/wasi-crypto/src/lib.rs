mod wiggle_interfaces;

pub use wiggle_interfaces::WasiCryptoCtx;

pub fn add_to_linker<T>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiCryptoCtx + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    use wiggle_interfaces::wasi_modules as w;
    w::wasi_ephemeral_crypto_common::add_to_linker(linker, get_cx)?;
    w::wasi_ephemeral_crypto_asymmetric_common::add_to_linker(linker, get_cx)?;
    w::wasi_ephemeral_crypto_signatures::add_to_linker(linker, get_cx)?;
    w::wasi_ephemeral_crypto_symmetric::add_to_linker(linker, get_cx)?;
    Ok(())
}
