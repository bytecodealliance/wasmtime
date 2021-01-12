use super::state::*;
use super::*;

use ::sha2::{Digest, Sha256, Sha512, Sha512Trunc256};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum HashVariant {
    Sha256(Sha256),
    Sha512(Sha512),
    Sha512_256(Sha512Trunc256),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Sha2SymmetricState {
    pub alg: SymmetricAlgorithm,
    options: Option<SymmetricOptions>,
    ctx: HashVariant,
}

impl Sha2SymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<&SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        if key.is_some() {
            return Err(CryptoError::KeyNotSupported);
        }
        let ctx = match alg {
            SymmetricAlgorithm::Sha256 => HashVariant::Sha256(Sha256::new()),
            SymmetricAlgorithm::Sha512 => HashVariant::Sha512(Sha512::new()),
            SymmetricAlgorithm::Sha512_256 => HashVariant::Sha512_256(Sha512Trunc256::new()),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        Ok(Sha2SymmetricState { alg, options, ctx })
    }
}

impl SymmetricStateLike for Sha2SymmetricState {
    fn alg(&self) -> SymmetricAlgorithm {
        self.alg
    }

    fn options_get(&self, name: &str) -> Result<Vec<u8>, CryptoError> {
        self.options
            .as_ref()
            .ok_or(CryptoError::OptionNotSet)?
            .get(name)
    }

    fn options_get_u64(&self, name: &str) -> Result<u64, CryptoError> {
        self.options
            .as_ref()
            .ok_or(CryptoError::OptionNotSet)?
            .get_u64(name)
    }

    fn absorb(&mut self, data: &[u8]) -> Result<(), CryptoError> {
        match &mut self.ctx {
            HashVariant::Sha256(x) => x.update(data),
            HashVariant::Sha512(x) => x.update(data),
            HashVariant::Sha512_256(x) => x.update(data),
        };
        Ok(())
    }

    fn squeeze(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
        let raw = match &self.ctx {
            HashVariant::Sha256(x) => x.clone().finalize().to_vec(),
            HashVariant::Sha512(x) => x.clone().finalize().to_vec(),
            HashVariant::Sha512_256(x) => x.clone().finalize().to_vec(),
        };
        ensure!(raw.len() >= out.len(), CryptoError::InvalidLength);
        out.copy_from_slice(&raw[..out.len()]);
        Ok(())
    }
}
