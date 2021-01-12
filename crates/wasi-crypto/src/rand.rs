use crate::CryptoError;
use rand_core::{self, CryptoRng, OsRng, RngCore};

pub struct SecureRandom;

impl SecureRandom {
    pub fn new() -> Self {
        SecureRandom
    }

    pub fn fill(&mut self, bytes: &mut [u8]) -> Result<(), CryptoError> {
        self.try_fill_bytes(bytes)
            .map_err(|_| CryptoError::RNGError)
    }
}

impl CryptoRng for SecureRandom {}

impl RngCore for SecureRandom {
    fn next_u32(&mut self) -> u32 {
        OsRng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        OsRng.next_u64()
    }

    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        OsRng.fill_bytes(bytes);
    }

    fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), rand_core::Error> {
        OsRng.try_fill_bytes(bytes)
    }
}
