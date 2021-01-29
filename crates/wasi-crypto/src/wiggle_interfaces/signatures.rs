use super::{guest_types, WasiCryptoCtx};

use wasi_crypto::SignatureEncoding;

impl super::wasi_ephemeral_crypto_signatures::WasiEphemeralCryptoSignatures for WasiCryptoCtx {
    // --- signature

    fn signature_export(
        &self,
        signature_handle: guest_types::Signature,
        encoding: guest_types::SignatureEncoding,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .ctx
            .signature_export(signature_handle.into(), encoding.into())?
            .into())
    }

    fn signature_import(
        &self,
        alg_str: &wiggle::GuestPtr<'_, str>,
        encoded_ptr: &wiggle::GuestPtr<'_, u8>,
        encoded_len: guest_types::Size,
        encoding: guest_types::SignatureEncoding,
    ) -> Result<guest_types::Signature, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let encoded = &*encoded_ptr.as_array(encoded_len).as_slice()?;
        Ok(self
            .ctx
            .signature_import(alg_str, encoded, encoding.into())?
            .into())
    }

    fn signature_state_open(
        &self,
        kp_handle: guest_types::Keypair,
    ) -> Result<guest_types::SignatureState, guest_types::CryptoErrno> {
        Ok(self.ctx.signature_state_open(kp_handle.into())?.into())
    }

    fn signature_state_update(
        &self,
        state_handle: guest_types::SignatureState,
        input_ptr: &wiggle::GuestPtr<'_, u8>,
        input_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let input = &*input_ptr.as_array(input_len).as_slice()?;
        Ok(self
            .ctx
            .signature_state_update(state_handle.into(), input)?)
    }

    fn signature_state_sign(
        &self,
        signature_state_handle: guest_types::SignatureState,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .ctx
            .signature_state_sign(signature_state_handle.into())?
            .into())
    }

    fn signature_state_close(
        &self,
        signature_state_handle: guest_types::SignatureState,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self
            .ctx
            .signature_state_close(signature_state_handle.into())?)
    }

    fn signature_verification_state_open(
        &self,
        pk_handle: guest_types::Publickey,
    ) -> Result<guest_types::SignatureVerificationState, guest_types::CryptoErrno> {
        Ok(self
            .ctx
            .signature_verification_state_open(pk_handle.into())?
            .into())
    }

    fn signature_verification_state_update(
        &self,
        verification_state_handle: guest_types::SignatureVerificationState,
        input_ptr: &wiggle::GuestPtr<'_, u8>,
        input_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let input: &[u8] = &*input_ptr.as_array(input_len).as_slice()?;
        Ok(self
            .ctx
            .signature_verification_state_update(verification_state_handle.into(), input)?)
    }

    fn signature_verification_state_verify(
        &self,
        verification_state_handle: guest_types::SignatureVerificationState,
        signature_handle: guest_types::Signature,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.ctx.signature_verification_state_verify(
            verification_state_handle.into(),
            signature_handle.into(),
        )?)
    }

    fn signature_verification_state_close(
        &self,
        verification_state_handle: guest_types::SignatureVerificationState,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self
            .ctx
            .signature_verification_state_close(verification_state_handle.into())?)
    }

    fn signature_close(
        &self,
        signature_handle: guest_types::Signature,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.ctx.signature_close(signature_handle.into())?)
    }
}

impl From<guest_types::SignatureEncoding> for SignatureEncoding {
    fn from(encoding: guest_types::SignatureEncoding) -> Self {
        match encoding {
            guest_types::SignatureEncoding::Raw => SignatureEncoding::Raw,
            guest_types::SignatureEncoding::Der => SignatureEncoding::Der,
        }
    }
}
