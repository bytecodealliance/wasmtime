use super::{guest_types, WasiCryptoCtx};

impl super::wasi_ephemeral_crypto_kx::WasiEphemeralCryptoKx for WasiCryptoCtx {
    // --- key exchange

    fn kx_dh(
        &self,
        pk_handle: guest_types::Publickey,
        sk_handle: guest_types::Secretkey,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self.ctx.kx_dh(pk_handle.into(), sk_handle.into())?.into())
    }

    // --- Key encapsulation

    fn kx_encapsulate(
        &self,
        pk_handle: guest_types::Publickey,
    ) -> Result<(guest_types::ArrayOutput, guest_types::ArrayOutput), guest_types::CryptoErrno>
    {
        let (secret_handle, encapsulated_secret_handle) =
            self.ctx.kx_encapsulate(pk_handle.into())?;
        Ok((secret_handle.into(), encapsulated_secret_handle.into()))
    }

    fn kx_decapsulate(
        &self,
        sk_handle: guest_types::Secretkey,
        encapsulated_secret_ptr: &wiggle::GuestPtr<'_, u8>,
        encapsulated_secret_len: guest_types::Size,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        let encapsulated_secret = &*encapsulated_secret_ptr
            .as_array(encapsulated_secret_len)
            .as_slice()?;
        Ok(self
            .ctx
            .kx_decapsulate(sk_handle.into(), encapsulated_secret)?
            .into())
    }
}
