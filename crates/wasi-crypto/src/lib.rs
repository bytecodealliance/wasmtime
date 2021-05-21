use std::borrow::BorrowMut;

mod wiggle_interfaces;

pub use wiggle_interfaces::WasiCryptoCtx;

pub fn add_to_linker<T>(linker: &mut wasmtime::Linker<T>) -> anyhow::Result<()>
where
    T: BorrowMut<WasiCryptoCtx>,
{
    use wiggle_interfaces::wasi_modules as w;
    w::wasi_ephemeral_crypto_common::add_to_linker(linker)?;
    w::wasi_ephemeral_crypto_asymetric_common::add_to_linker(linker)?;
    w::wasi_ephemeral_crypto_signatures::add_to_linker(linker)?;
    w::wasi_ephemeral_crypto_symmetric::add_to_linker(linker)?;
    Ok(())
}
